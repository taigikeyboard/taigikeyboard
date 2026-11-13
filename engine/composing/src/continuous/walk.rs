//! The whole-sentence walk and the slot-0 candidate it synthesizes.

use super::edge::{edge_key_map, edge_lookup, learned_edge_key, rows_for_edge, EdgeLookup};
use super::recase::{recase_roman, recase_tl_as_poj_display};

use crate::shadow::{
    custom_toneless_key, greedy_longest_syllabification, span_min_syllable_count,
    strip_tones_for_mode,
};
use lexicon::dictionary_reader::NO_ROWID;
use lexicon::{
    best_candidate_for_key_with_barriers, ConsumedSpan, ContinuousFetchCtx, CustomEntry,
    LearnedEntry, SyllableInventory, COVERAGE_KIND_FULL,
};
use ranking::WALKER_COST_UNPRICED;

/// The slot-0 walker's native result. The seam in [`assemble_candidates`]
/// converts it to a `RawCandidate`; slot 0 is an explicit prepend, never
/// sort-compared, so the rank facts a fetched row carries (`score`,
/// `frequency`, `bitmask`) are stamped `0` there.
///
/// [`assemble_candidates`]: super::assemble::assemble_candidates
pub(super) struct WalkerSlot0 {
    pub consumed_span: ConsumedSpan,
    pub syllable_count: u8,
    pub display_text: String,
    pub roman: String,
    pub hanji: Option<String>,
    /// v3.6.1 R2 — canonical TL identity (`RawCandidate.canonical_tl`),
    /// computed ONCE here from the synth `roman` (the single point that
    /// knows the fold rule) so the seam reads it instead of re-folding.
    pub canonical_tl: String,
    pub user_weight: f64,
    pub coverage_kind: u8,
    pub is_custom: bool,
    /// How many lattice edges — words — the path takes. A list of one word
    /// (a Hanji conversion's `FetchAtPos`) lists a
    /// one-edge path only.
    pub edge_count: usize,
}

/// v3.5.8 S2 (Codex post-impl P1, 2026-05-16) — `consumed_span` for
/// the synthesized slot-0 candidate, or `None` to suppress the synth.
///
/// The walker spans the **shadow** (`0..shadow_len`); its slot-0
/// candidate commits in **raw** byte space under forward-only Model B.
/// They line up only when the shadow's end maps back to the full raw
/// buffer. A **trailing** hyphen (`tai-`, `tai-bak-`) is stripped from
/// the shadow and the S1 `build_hyphen_shadow` contract keeps it in
/// the pending raw buffer, so `shadow_to_raw_end[shadow_len] <
/// raw_len`. Emitting a `(0, raw_len)` full-buffer synth there would
/// wrongly consume / drop that pending `-`, violating the S1
/// trailing-hyphen contract — so suppress the synth and leave the
/// span-local list untouched (pre-S2 behavior). Leading / internal
/// hyphens fold INTO the consumed prefix
/// (`shadow_to_raw_end[shadow_len] == raw_len`) and still synth.
pub(crate) fn synth_consumed_span(
    shadow_to_raw_end: &[usize],
    shadow_len: usize,
    raw_len: u32,
) -> Option<(u32, u32)> {
    // `shadow_to_raw_end` always has length `shadow_len + 1`
    // (`build_hyphen_shadow` / `canonicalize_poj_shadow` contract),
    // so indexing `[shadow_len]` is in bounds.
    (shadow_to_raw_end[shadow_len] as u32 == raw_len).then_some((0, raw_len))
}

/// The walker bigram over the loaded `dictionary.bin` v6 pair counts
/// (`docs/architecture/bigram-lm-roadmap.md` D6).
struct DictionaryBigram<'a> {
    dict: &'a lexicon::dictionary_reader::DictionaryReader,
    discount: f64,
}

impl crate::lattice::WalkerBigram for DictionaryBigram<'_> {
    type Continuations = lexicon::dictionary_reader::BigramContinuations;

    fn continuations(&self, previous_rowid: u32) -> Option<Self::Continuations> {
        self.dict.bigram_continuations(previous_rowid)
    }

    fn evidence(
        &self,
        continuations: &Self::Continuations,
        next_rowid: u32,
    ) -> crate::lattice::BigramEvidence {
        crate::lattice::BigramEvidence {
            total: continuations.total,
            distinct: continuations.distinct,
            pair_count: self.dict.bigram_pair_count(continuations, next_rowid),
            discount: self.discount,
        }
    }
}

/// The whole-sentence walk over `continuous_keys`' lattice: the minimum-cost
/// path spanning the shadow, each edge resolved to its custom, dictionary
/// (with the learned rows competing) or OOV content. `None` when no edge
/// chain spans the buffer. Shared by the slot-0 synthesis
/// ([`fetch_walker_slot0_inner`]) and the preedit conversion
/// (`crate::conversion`), which present the same path differently.
pub(crate) fn walk_buffer(
    continuous_keys: &crate::shadow::ContinuousKeys,
    inv: &SyllableInventory,
    ctx: &ContinuousFetchCtx<'_>,
) -> Option<crate::lattice::BestPath> {
    let crate::shadow::ContinuousKeys {
        shadow,
        lattice,
        barriers,
        ..
    } = continuous_keys;
    let ContinuousFetchCtx {
        custom,
        learned,
        mode,
        ..
    } = *ctx;
    // v3.5.8 S6 (Codex pre-impl S6 Q2/Q6, 2026-05-17) — per-fetch
    // map from a custom entry's normalized toneless key to the
    // entry. `custom_toneless_key` reuses the SAME shadow pipeline
    // the edge keys use, so a hit here is byte-identical to a
    // lattice edge's `tl:{toneless}` (Q2 BLOCK: a plain
    // tone-digit-strip would not fold a POJ/diacritic custom roman
    // like `tâi-uân`). Entries under one key keep `custom` order — the
    // dedupe's insertion order in the edge pick (Codex Q6: explicit, not
    // `HashMap` overwrite/iteration).
    //
    // S6 byte-identity invariant: the SAME `mode` (`ctx.mode`) feeds the
    // caller's `build_shadow_lattice_with_barriers` and
    // `custom_toneless_key` here — a split-brain (POJ-aware edges,
    // mode-blind custom keys) would silently drop custom matches in
    // POJ mode.
    let custom_map = edge_key_map(custom, |entry| custom_toneless_key(&entry.roman, mode));
    // Learned phrases (§50) — keyed through [`learned_edge_key`] so the
    // canonical-TL row lands on the mode's own key family. Consumed by the
    // edge pick below as a COMPETITOR of the dictionary rows, as the custom
    // map is (Codex 2026-09-20 F5).
    let learned_map = edge_key_map(learned, |entry| learned_edge_key(&entry.canonical_tl, mode));

    let custom_word_cost = ctx
        .dict
        .walker_model()
        .cost_of_count(crate::lattice::USER_ENTRY_EFFECTIVE_COUNT);
    let bigram = DictionaryBigram {
        dict: ctx.dict,
        discount: ctx.dict.walker_model().bigram_discount(),
    };
    let path = crate::lattice::walk_best(
        lattice,
        shadow.len(),
        custom_word_cost,
        &bigram,
        |start, end| {
            // The tone pin is computed BEFORE the custom rows are gathered so
            // every edge source answers to the same pin: a custom entry
            // synthesized past the guard would land at slot 0, where no
            // downstream lexicon filter can reach it (Codex post-impl BLOCK,
            // 2026-08-20).
            let EdgeLookup {
                toneless,
                raw_span,
                span_key:
                    crate::shadow::SpanKey {
                        key: dict_key,
                        final_only: edge_final_only,
                        tone_pin: edge_tone_pin,
                    },
            } = edge_lookup(continuous_keys, start, end, mode)?;
            // v3.5.9 B-2 — walker edge key is mode-aware: the same `mode`
            // feeding the shadow + lattice (`build_shadow_lattice_with_barriers`) above
            // also feeds the key prefix here, so the lookup family is
            // consistent with the inventory family that produced the edge.
            let key_prefix = phonetics::KeyFamily::for_input_mode(mode).prefix();
            // Custom row matching stays tone-INSENSITIVE: `custom_map` is
            // keyed by `custom_toneless_key` (toneless), so it is queried with
            // the toneless key — a custom word is a specific user entry, matched
            // per the legacy toneless rule. What the tone pin adds is not a
            // different match key but an eligibility gate on the ENTRY's own
            // reading: with the edge typed with a digit (§17) or closed by the
            // keyboard's space (§41), an entry whose reading disagrees there is
            // not the word the user asked for. Every eligible entry under the
            // key enters the pick, so an entry that carries the typed tone is
            // never hidden behind one that does not.
            //
            // Both pins fire in every mode: a TL / POJ stored custom roman keys
            // under TPS by its Bopomofo face (`custom_toneless_key`), and a
            // custom entry synthesized past the pin becomes slot 0, where no
            // downstream lexicon filter can reach it (Codex post-impl
            // 2026-08-20).
            let custom_key = format!("{key_prefix}{toneless}");
            // Custom rows and learned phrases (§50) under this edge's key join
            // the dictionary's own edge pick
            // (`lexicon::best_candidate_for_key_with_barriers`) as competitors,
            // ordered by the list's own `CandidateSortKey` — no longer a
            // first-wins custom override (USER 2026-10-16: a never-selected
            // custom 屢 `lí` took the `li2` edge from 你, and from a custom 李
            // picked 7×). The dictionary rows honour
            // `ctx.enabled_sources_bitmask` (PR-9.6); user rows are not a
            // toggleable source. When any user row matched, the edge is priced
            // at most at the loaded model's custom-word cost so the span the user
            // filed or once composed as one word keeps winning the segmentation (the #69
            // span-vs-word decoupling: "is this span a word" is the cap, "which
            // word" is the pick).
            let custom_for_edge = rows_for_edge(&custom_map, &custom_key, |entry: &CustomEntry| {
                edge_tone_pin.admits_custom(&entry.roman, mode)
            });
            let learned_for_edge =
                rows_for_edge(&learned_map, &custom_key, |entry: &LearnedEntry| {
                    edge_tone_pin.admits(None, &entry.canonical_tl)
                });
            match best_candidate_for_key_with_barriers(
                &dict_key,
                &edge_final_only,
                &edge_tone_pin,
                raw_span,
                &custom_for_edge,
                &learned_for_edge,
                ctx,
            ) {
                Some(lexicon::EdgeBest {
                    candidate: c,
                    span_walker_cost,
                    bigram_rowid,
                }) => {
                    let user_row = !custom_for_edge.is_empty() || !learned_for_edge.is_empty();
                    // Word = the edge pick (user weight, context, corpus cost). Cost =
                    // `span_walker_cost` (key min) so a rarer preferred
                    // homophone does not lose the segmentation; the S3
                    // path discount reads the pick's own `user_weight`.
                    Some(crate::lattice::EdgeChoice {
                        rowid: bigram_rowid,
                        roman: c.roman,
                        hanji: c.hanji,
                        canonical_tl: c.canonical_tl,
                        // S5 (Codex pre-impl Q2 BLOCK): explicit
                        // dict-hit flag — the `Some(c)` branch IS a
                        // dictionary record. The no-dict carve-out must
                        // key off this, never `hanji.is_none()` /
                        // `frequency == 0`.
                        dict_hit: true,
                        is_custom: c.is_custom,
                        walker_cost: span_walker_cost,
                        user_row,
                        syllable_count: c.syllable_count,
                        // khiin `word_len` for the S5 length
                        // normalization (Codex pre-impl Q1 BLOCK).
                        toneless_len: toneless.chars().count(),
                        user_weight_delta: c.user_weight,
                    })
                }
                // No dict hit: the edge's own toneless roman, SAME code
                // path as a dict edge (`feedback_no_redundant_fallback`)
                // but flagged `dict_hit: false`. A synthesized roman
                // edge has no `user_frequency.db` history key so its
                // walker user weight stays neutral
                // (`user_weight_delta = 0.0`). The user-facing
                // per-syllable romanization for a wholly no-dict buffer
                // is produced by the explicit no-dict carve-out below
                // (keyed off `dict_hit`), NOT by an edge-cost tie lever.
                //
                // v3.5.8 RC0: what stops a multi-syllable OOV blob
                // from undercutting a dict-covering path is now
                // `edge_cost`'s char-keyed per-char `BIG` penalty
                // (`OOV_PER_CHAR_PENALTY * toneless_len`), NOT the
                // syllable count. `syllable_count` here is metadata
                // only — it flows into the synthesized slot-0
                // candidate's syllable sum, so it is still kept honest
                // (not a hardcoded `1`) via a **guaranteed-reachable**
                // min-syllable-hop walk over the lattice's own
                // single-syllable step primitive
                // ([`span_min_syllable_count`]) — NOT
                // `greedy_longest_syllabification(...).unwrap_or(1)`,
                // which can dead-end on a span still lattice-
                // syllabifiable via a non-greedy split and then
                // under-count the synth syllable sum (Codex pre-impl
                // RC0 Q3; PR #290 P1 r3255035136). `None` is
                // unreachable for a real lattice edge (Codex pre-impl
                // Q1/Q2 OK); if the edge/provider invariant is ever
                // broken, fail-closed by dropping the edge — the buffer
                // is still spanned via finer edges.
                None => {
                    // §52: an OOV reading is the span's letters as one blob,
                    // so an edge crossing a typed `-` would render `a-i` as
                    // `ai`. Drop it — the single-syllable edges either side
                    // still span the buffer and take the typed join.
                    let oov_barriers =
                        crate::shadow::oov_reading_barriers(barriers, start, end, mode);
                    if oov_barriers.iter().any(|&b| b < end - start) {
                        return None;
                    }
                    let toneless_len = toneless.chars().count();
                    let syllable_count =
                        span_min_syllable_count(&shadow[start..end], inv, mode, &oov_barriers)?
                            .clamp(1, u8::MAX as usize) as u8;
                    Some(crate::lattice::EdgeChoice {
                        rowid: NO_ROWID,
                        roman: toneless,
                        hanji: None,
                        canonical_tl: String::new(),
                        dict_hit: false,
                        // S6: a synthesized OOV roman edge is not custom.
                        is_custom: false,
                        walker_cost: WALKER_COST_UNPRICED,
                        user_row: false,
                        syllable_count,
                        toneless_len,
                        user_weight_delta: 0.0,
                    })
                }
            }
        },
    );

    path.filter(|path| !path.choices.is_empty())
}

/// v3.5.9 A2 — slot-0 whole-sentence walker inner. Pre-A2
/// `fetch_walker_slot0`'s body minus the `LexiconHandle::with_state`
/// opener, the state `as_ref()?` guards, AND the `build_shadow_lattice_with_barriers`
/// call (now built ONCE in the seam under **D1 fold** and passed in).
/// Returns [`WalkerSlot0`]; the seam converts it to a `RawCandidate`.
///
/// v3.5.9 B-0c — `mode: phonetics::InputMode` replaces the prior
/// `is_poj: bool` (Codex pre-impl SHOULD 2026-05-21 + 2026-05-20 enum
/// sweep). `mode` is taken as a parameter rather than re-derived
/// here so the single-source-of-truth invariant — same `mode` feeds
/// `build_shadow_lattice_with_barriers` (seam) and `custom_toneless_key` (here) —
/// cannot be silently broken by a future seam refactor.
///
/// Returns `None` when the synth must be suppressed (trailing-hyphen
/// shadow short of raw, or the walker cannot span the buffer) — caller
/// leaves the span-local list untouched (pre-S2 behavior preserved).
///
/// `continuous_keys` is the seam's single `build_continuous_keys` result
/// (shadow / lattice / barriers — the per-key vectors are the span-local
/// fetch's, unused here); `ctx` is the same [`ContinuousFetchCtx`] the
/// span-local fetch reads (`mode` / `custom` / `freq_map` / `now_ms` /
/// `enabled_sources_bitmask` + the dictionary readers).
pub(super) fn fetch_walker_slot0_inner(
    raw: &str,
    raw_len: u32,
    continuous_keys: &crate::shadow::ContinuousKeys,
    inv: &SyllableInventory,
    ctx: &ContinuousFetchCtx<'_>,
) -> Option<WalkerSlot0> {
    let crate::shadow::ContinuousKeys {
        shadow,
        shadow_to_raw_end,
        barriers,
        ..
    } = continuous_keys;
    let ContinuousFetchCtx {
        freq_map,
        now_ms,
        mode,
        ..
    } = *ctx;
    // Codex post-impl S2 P1: suppress the synth when a trailing
    // hyphen leaves the shadow short of the raw buffer (a
    // `(0, raw_len)` synth would mis-commit the pending `-`).
    // Cheap early-out before walking.
    let consumed_span = synth_consumed_span(shadow_to_raw_end, shadow.len(), raw_len)?;
    let path = walk_buffer(continuous_keys, inv, ctx)?;

    // v3.5.8 S5 (Codex pre-impl Q2, 2026-05-17) — no-dict carve-out.
    // "No dictionary hit anywhere" is detected via the explicit
    // `dict_hit` flag, NOT `hanji.is_none()` / `frequency == 0` (a
    // dict record may be roman-only; zero frequency is
    // representable — Codex BLOCK). Under the S5 min-cost objective
    // an all-OOV buffer collapses to the fewest-edge blob
    // (`taiuantai`), wrong for a romanization the user wants to
    // read. Re-derive the user-facing roman as the greedy
    // longest-syllable syllabification (`tai uan tai`) — an
    // explicit rule OUTSIDE `edge_cost`, not an OOV cost tuned to
    // fight the corpus normalization. Greedy-longest (the canonical
    // romanization reading) is used rather than literal
    // max-syllable-count: the latter would over-split into
    // sub-syllables (`ta i u an …`), reproducing the very
    // over-segmentation S5 removes, and contradicts the documented
    // `taiuantai → tai uan tai` expectation.
    //
    // v3.5.8 #288 (per-segment case-from-raw) composes with the
    // dict-hit branch: each chosen edge's canonical roman is recased
    // to mirror the user's raw input for that edge's own byte span
    // (per-edge, NOT whole buffer: `hitTUI` keeps segment 2 upper).
    // `path.edges` are shadow offsets; `shadow_to_raw_end` maps them
    // back to the raw buffer. The all-OOV carve-out stays unrecased — it
    // is a synthesized reading, not an edge the user typed a case intent
    // for; greedy-longest segments are independent of `path.edges`.
    let any_dict = path.choices.iter().any(|c| c.dict_hit);
    let (parts, syllable_count) = if any_dict {
        let parts = path
            .edges
            .iter()
            .zip(path.choices.iter())
            .map(
                |(&(s, e), c)| match raw.get(shadow_to_raw_end[s]..shadow_to_raw_end[e]) {
                    Some(seg) => recase_roman(&c.roman, seg, mode),
                    None => c.roman.clone(),
                },
            )
            .collect::<Vec<_>>();
        let s = path
            .choices
            .iter()
            .map(|c| u32::from(c.syllable_count))
            .sum::<u32>()
            .min(u32::from(u8::MAX)) as u8;
        (parts, s)
    } else {
        match greedy_longest_syllabification(
            shadow,
            inv,
            mode,
            &crate::shadow::oov_reading_barriers(barriers, 0, shadow.len(), mode),
        ) {
            Some(segs) if !segs.is_empty() => {
                // v3.5.9 D / C-3b — mode-aware tone strip. TPS no-dict
                // synth strips Bopomofo tone marks per syllable; TL/POJ
                // keep the byte-identical ASCII-digit drop.
                let parts = segs
                    .iter()
                    .map(|&(s, e)| strip_tones_for_mode(&shadow[s..e], mode))
                    .collect::<Vec<_>>();
                let s = segs.len().min(u8::MAX as usize) as u8;
                (parts, s)
            }
            // Cannot cleanly syllabify the buffer → leave the
            // span-local list untouched (pre-S2 behavior, mirrors
            // the synth_consumed_span trailing-hyphen suppression).
            _ => return None,
        }
    };
    // §10.2 slot-0 word space. The separators the user typed are rendered
    // onto it by Step 5 (§55) like on every other row, so `canonical_tl`
    // (folded from a space join too) never forks on a typed separator.
    let roman = parts.join(" ");
    // A roman-only dictionary word in the path keeps the sentence's Hanji: the
    // word stands in romanization (Han-Lo mixed, MOE `VT_MIXED`). An OOV edge
    // still leaves the whole path to the romanization.
    let hanji_spelled = any_dict
        && path.choices.iter().any(|c| c.hanji.is_some())
        && path.choices.iter().all(|c| c.hanji.is_some() || c.dict_hit);
    let hanji: Option<String> = hanji_spelled.then(|| {
        path.edges
            .iter()
            .zip(&path.choices)
            .zip(&parts)
            .map(|((&(s, e), choice), part)| match (&choice.hanji, mode) {
                (Some(hanji), _) => hanji.clone(),
                // The typed Bopomofo, what the layout's roman-only cell shows.
                (None, phonetics::InputMode::Tps) => raw
                    .get(shadow_to_raw_end[s]..shadow_to_raw_end[e])
                    .unwrap_or(part)
                    .to_owned(),
                (None, phonetics::InputMode::Poj) => recase_tl_as_poj_display(part),
                (None, _) => part.clone(),
            })
            .collect()
    });
    // v3.5.9 B-4 (Codex pre-impl BLOCK #2 close): the walker
    // greedy-longest OOV synth builds `roman` from the
    // mode-aware shadow — POJ mode preserves POJ ASCII per B-2,
    // so a hanji-absent synth candidate would otherwise key
    // `user_frequency.db` as POJ ASCII (`chiah goa`) and split
    // from the same word's TL-mode commit key (`tsiah gua`).
    // `canonical_tl_form` folds POJ-mode synth roman to TL so
    // the freq key is mode-invariant; `roman` itself stays in
    // its native form so the downstream `recase_tl_as_poj_display`
    // pass renders correctly (POJ-mode roundtrip via
    // `tl_display_to_poj_display`). §9 #2 user-history contract
    // is now satisfied across all three candidate paths:
    // `dict.bin` (record.tl is canonical TL), custom dict (this
    // change to `custom_entry_to_candidate`), and walker synth
    // (this site).
    // R2: fold the synth roman to canonical TL ONCE here — this is the
    // single point that knows the synth's fold rule. Reused for the
    // hanji-absent `display_text` fallback AND carried on `WalkerSlot0`
    // as the `(hanji, canonical-TL)` identity sidechannel, so the seam
    // (`assemble_candidates`) reads it instead of re-folding `slot0.roman`.
    // The fold drops the per-edge recase: a pick counted under `Kóng` would
    // rank nothing typed `kong`.
    let canonical_tl = phonetics::api::canonical_tl_form(&roman, mode);
    let display_text = hanji.clone().unwrap_or_else(|| canonical_tl.clone());
    // R5 pair-key (#7): walker synth keyed by (display_text,
    // canonical_tl); `get` returns the neutral default on a miss.
    let user_weight = freq_map
        .get(&display_text, &canonical_tl)
        .user_weight(now_ms);
    Some(WalkerSlot0 {
        consumed_span,
        syllable_count,
        display_text,
        roman,
        hanji,
        canonical_tl,
        user_weight,
        coverage_kind: COVERAGE_KIND_FULL,
        // v3.5.8 S6 (Codex pre-impl S6 Q4): provenance truth — a
        // synthesized full-buffer path containing ≥1 custom edge is
        // custom-influenced. Informational at slot 0 (explicit
        // prepend, not sorted; the seam dedupe keys on
        // `(roman,hanji,consumed_span)` not `is_custom`), but a
        // truthful flag keeps future ranking/dedupe changes sound.
        is_custom: path.choices.iter().any(|c| c.is_custom),
        edge_count: path.edges.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ----- v3.5.8 S2 — synth_consumed_span trailing-hyphen guard -----
    // (Codex post-impl S2 P1 regression). Production runs
    // `canonicalize_poj_shadow` then `build_hyphen_shadow`; for the
    // pure-ASCII inputs here canonicalize is identity (see the
    // matching `canonicalize_poj_shadow_pure_ascii_is_identity_fast_path`
    // case in `composing::shadow`'s test module), so
    // `crate::shadow::build_hyphen_shadow(raw)` yields the same
    // `shadow_to_raw_end` the walker path computes.

    #[test]
    fn synth_consumed_span_trailing_hyphen_suppresses_synth() {
        use crate::shadow::build_hyphen_shadow;
        // `tai-` / `tai-bak-`: the trailing `-` stays in the pending
        // raw buffer (S1 contract) so the shadow is short of raw_len —
        // the full-buffer synth MUST be suppressed (returns None)
        // rather than mis-commit / drop the pending `-`.
        for raw in ["tai-", "tai-bak-"] {
            let (shadow, map) = build_hyphen_shadow(raw);
            assert!(
                synth_consumed_span(&map, shadow.len(), raw.len() as u32).is_none(),
                "trailing-hyphen {raw:?} must suppress slot-0 synth"
            );
        }
    }

    #[test]
    fn synth_consumed_span_full_coverage_emits_zero_to_raw_len() {
        use crate::shadow::build_hyphen_shadow;
        // No trailing hyphen → shadow maps to the whole raw buffer.
        // Leading (`-tai`) and internal (`tai-bak`) hyphens fold INTO
        // the consumed prefix, so those still get a `(0, raw_len)`
        // full-buffer synth.
        for raw in ["taibak", "tai-bak", "-tai"] {
            let (shadow, map) = build_hyphen_shadow(raw);
            assert_eq!(
                synth_consumed_span(&map, shadow.len(), raw.len() as u32),
                Some((0, raw.len() as u32)),
                "{raw:?} fully covered → (0, raw_len) synth"
            );
        }
    }
}
