//! The seam orchestrator: the 6-step [`assemble_candidates`] contract.

use super::dedupe::{dedupe_rendered_continuous, dedupe_same_commit_hanji, retain_absent_from};
use super::partial_prefix::fetch_via_lexicon_partial_inner;
use super::recase::{recase_all, recase_roman, recase_tl_as_poj_display, shows_poj_roman_in_hanji};
use super::walk::{fetch_walker_slot0_inner, synth_consumed_span};

use crate::shadow::{build_continuous_keys, build_partial_prefix_key};
use lexicon::dictionary_reader::NO_ROWID;
use lexicon::{
    fetch_candidates_for_keys_with_barriers, fetch_partial_prefix_candidates,
    fetch_partial_prefix_candidates_unbounded, ConsumedSpan, ContinuousFetchCtx, CustomEntry,
    EngineHandle as LexiconHandle, LearnedEntry, RawCandidate, COVERAGE_KIND_FULL,
    PARTIAL_PREFIX_OUTPUT_CAP,
};
use phonetics::api::roman_reading_eq;
use ranking::{
    sort_by_candidate_key, FrequencyMap, COVERAGE_KIND_PARTIAL_PREFIX, WALKER_COST_UNPRICED,
};

/// The shape of a continuous candidate list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ListShape {
    /// The whole tail's list: the walker's path over it — a sentence of
    /// several words at slot 0, one word sorted with the rest (Step 4d) — and
    /// the left-anchored words in ranking order.
    Sentence,
    /// A Hanji conversion's list of one word (H4): the walker's path only
    /// when it is one word — so a reading the dictionary has no word for
    /// keeps its row — then the sentence list's order within a tier, longer
    /// words first among the front-segment rows.
    Word,
}

/// The rows [`assemble_candidates`] lists, and whether the Step 4d post-pass
/// moved a one-word walker path ahead of one-syllable prefix picks.
#[derive(Debug, Default)]
pub(crate) struct Assembled {
    pub(crate) candidates: Vec<RawCandidate>,
    pub(crate) walker_moved: bool,
}

/// A picked one-syllable word reached as a prefix extension (`ko` → a
/// picked 講 `kóng`): the store keys a pick by its reading, so it may come
/// from a longer typed syllable.
pub(crate) fn is_one_syllable_prefix_pick(candidate: &RawCandidate) -> bool {
    candidate.user_weight > 0.0
        && candidate.syllable_count == 1
        && candidate.coverage_kind == COVERAGE_KIND_PARTIAL_PREFIX
}

/// Step 4d post-pass: the one-word walker path leads the one-syllable prefix
/// picks among the picks the sort put directly before it (`ko` → 哥 before a
/// picked 講) — the short-word sweep [`picked_extension_outranks_sentence`]
/// refuses a sentence. Rule and examples: `docs/engine/continuous-input-ranking.md`
/// step 4d. `rows` pairs each row with its pre-sort index; the path is index 0
/// (Step 4 `insert(0, …)`). Returns whether the path moved.
fn lead_one_syllable_prefix_picks(rows: &mut [(usize, RawCandidate)]) -> bool {
    let Some(at) = rows.iter().position(|(index, _)| *index == 0) else {
        return false;
    };
    let run = rows[..at]
        .iter()
        .rev()
        .take_while(|(_, candidate)| candidate.user_weight > 0.0)
        .count();
    let start = at - run;
    if !rows[start..at]
        .iter()
        .any(|(_, candidate)| is_one_syllable_prefix_pick(candidate))
    {
        return false;
    }
    // Stable: kept picks first, then the path, then the moved picks.
    rows[start..=at].sort_by_key(|(index, candidate)| {
        if *index == 0 {
            1
        } else if is_one_syllable_prefix_pick(candidate) {
            2
        } else {
            0
        }
    });
    true
}

/// Whether the top ranked row is a picked word that takes slot 0 from the
/// walker's sentence at `candidates[0]`.
///
/// The walker spans `0..raw_len`, so a word whose key extends past the input
/// (`antsua` → 按怎 `án-tsuánn`) has no edge and could never fill slot 0
/// however often it was picked. Only a multi-syllable word qualifies: the
/// store keys a pick by its reading, not by what was typed, so a one-syllable
/// pick made on `sian` would otherwise take `sia` from the sentence `sī a`,
/// the short-word sweep the walker's own single-syllable delta scale refuses
/// (`lattice::cost::WALKER_SINGLE_SYLLABLE_USER_DELTA_SCALE`).
fn picked_extension_outranks_sentence(candidates: &[RawCandidate], raw_len: u32) -> bool {
    let [sentence, top_ranked, ..] = candidates else {
        return false;
    };
    top_ranked.coverage_kind == COVERAGE_KIND_PARTIAL_PREFIX
        && top_ranked.consumed_span.1 == raw_len
        && !is_one_syllable_prefix_pick(top_ranked)
        && top_ranked.user_weight > sentence.user_weight
}

/// The shown `hanji` of each candidate (`render_hanji_display`); `display_text`
/// keeps the stored spelling, the identity key.
fn apply_hanji_display(
    candidates: &mut [RawCandidate],
    khinsiann: bool,
    mode: phonetics::InputMode,
) {
    if !khinsiann && mode != phonetics::InputMode::Poj {
        return;
    }
    for cand in candidates {
        let poj_roman = shows_poj_roman_in_hanji(mode, cand.is_custom);
        if let Some(hanji) = cand.hanji.as_mut() {
            if let std::borrow::Cow::Owned(rendered) = phonetics::api::render_hanji_display(
                hanji,
                &cand.canonical_tl,
                khinsiann,
                poj_roman,
            ) {
                *hanji = rendered;
            }
        }
    }
}

/// v3.5.9 A2 seam — the 6-step `assemble_candidates` contract.
///
/// Replaces the inline `let mut candidates = if keys.is_empty() {...} else
/// {...}` block + POJ presentation pass that lived in
/// `requests::handle_fetch_at_pos` before A2. See module docs for the
/// 6-step contract and the **D1 fold** lifecycle invariant.
///
/// Inputs come from `handle_fetch_at_pos` (`mode` from `parse_input_mode`
/// plus the `contains_tps` raw-buffer upgrade; `freq_map` / `custom` /
/// `learned` from the fetch's `UserRows`). Every TPS-vs-non-TPS branch derives from
/// `mode == InputMode::Tps` internally — v3.5.9 D / C-3b dropped the
/// parallel `is_tps: bool` arg (single mode axis, no split-brain).
/// `mode == Poj` similarly derives the POJ branch — v3.5.9 B-0c
/// dropped the prior `is_poj: bool` arg.
///
/// `enabled_sources_bitmask` is the user's dictionary source filter,
/// resolved from the request's `AppConfig.dictionary_sources` (`u32::MAX` =
/// every source, `0` = none). It flows
/// straight into `ContinuousFetchCtx` so both inner fetchers decode it
/// via `Filter::from_enabled_bitmask` — identical filtering to the Tab3
/// browse path (12 sources + variant + khiin + kautian subcollection).
/// Output is the unwrapped `Vec<RawCandidate>` the caller maps to
/// `CandidateMessage` via `raw_to_proto_candidate`.
///
/// `shape` is the list's: the whole tail's, or a Hanji conversion's list of
/// the word before the caret ([`ListShape`]).
#[allow(clippy::too_many_arguments)]
pub(crate) fn assemble_candidates(
    raw: &str,
    freq_map: &FrequencyMap,
    now_ms: i64,
    custom: &[CustomEntry],
    learned: &[LearnedEntry],
    mode: phonetics::InputMode,
    context: &ranking::ContextRanks,
    config: &protos::engine::AppConfig,
    shape: ListShape,
) -> Assembled {
    let raw_len = raw.len() as u32;
    let enabled_sources_bitmask = crate::requests::source_filter_bitmask(config);
    let syllable_joiner = config.rendered_syllable_joiner();
    let force_lowercase_nasal_marker = config.force_lowercase_nasal_marker;
    let dedupe_commit_hanji = super::dedupe::should_dedupe_commit_hanji(config, mode);
    let khinsiann_in_hanji = config.khinsiann_in_hanji;
    LexiconHandle::with_state(|state| {
        let inv = state.syllable_inventory.as_ref();
        let prefix = state.prefix_index.as_ref();
        let dict = state.dictionary.as_ref();
        // Whole-buffer tone pin (§17 typed digits / §41 space-closed TPS
        // tail / §52 typed boundaries), computed once per seam invocation
        // for the sources that carry no span key of their own.
        let tone_pin = crate::shadow::whole_buffer_tone_pin(raw, mode, inv);

        // v3.5.9 D7 — build the shared lexicon-fetch context once per
        // seam invocation. `Some` only when BOTH `prefix_index` and
        // `dictionary` resolved; either inner fetcher (span-local or
        // partial-prefix) needs both.
        // `enabled_sources_bitmask` is the user's source filter
        // (resolved from the request's `AppConfig.dictionary_sources`), so
        // the span-local + partial-prefix fetchers apply the SAME
        // `Filter` (sources + variant + khiin + kautian subcollection)
        // the Tab3 browse path applies. `u32::MAX` keeps every source on.
        let lex_ctx = prefix
            .zip(dict)
            .map(|(prefix_index, dict)| ContinuousFetchCtx {
                enabled_sources_bitmask,
                freq_map,
                // The previous word's continuations (§56) — the sort's
                // context dimension and the walker's first-edge word pick.
                context,
                now_ms,
                custom,
                learned,
                prefix_index,
                dict,
                // v3.5.9 B-4 — thread the active input mode through so
                // `lexicon::custom_entry_to_candidate` can fold a
                // POJ-form custom roman's `display_text` (the
                // `user_frequency.db` commit key) to canonical TL,
                // keeping the freq key mode-invariant. Lattice / FST
                // key prefix is decided above and embedded in `keys`,
                // so this is purely a freq-key axis.
                mode,
                // Whole-buffer tone pin for the sources that carry no span
                // key of their own (custom entries, partial-prefix
                // extensions). Computed from `raw` by the same rule the
                // per-key pins use.
                tone_pin,
                // The key a custom entry is measured against on the exact
                // span-local path — the same whole-buffer key the
                // partial-prefix paths answer.
                whole_buffer_key: (!custom.is_empty())
                    .then(|| build_partial_prefix_key(raw, mode).map(|(_, key)| key))
                    .flatten(),
                khinsiann_in_hanji,
            });

        // ---- Step 1: build keys + shadow/lattice (D1 fold).
        // v3.5.9 D / C-3b — all modes (Tl / Poj / Tps / English) share
        // the same shadow-pipeline path: build the shadow + lattice ONCE
        // here and project the left-anchored keys via mode-aware
        // [`phonetics::KeyFamily::for_input_mode`] + [`shadow::strip_tones_for_mode`];
        // walker reuses the SAME lattice (no rebuild). The legacy
        // `build_keys_tps` short-circuit retired with C-3b — TPS now
        // walks the same path TL/POJ already do, the only difference
        // being the inventory family selected by `mode` inside
        // [`crate::syllabifier::valid_span_endings_lowered`].
        // Inv absent → empty keys and walker skipped (graceful
        // degradation; matches pre-A2 / pre-C-3b behavior).
        // Literal left-anchored keys + §35 barrier metadata. A word can be
        // hidden because the per-keystroke auto-correct picked the wrong one
        // of two locally-indistinguishable glyph readings (考卷 / 毋是 /
        // 雞胸); the recovery is the ambiguity-aware LOOKUP
        // (`lookup_exact_tps_readings`) fed by this metadata — the key text
        // stays the user's letters. `None` (no inventory) = empty keys +
        // walker skipped.
        let continuous_keys = inv.map(|inv| build_continuous_keys(raw, inv, mode));
        let keys: &[(ConsumedSpan, String)] = continuous_keys
            .as_ref()
            .map_or(&[], |continuous_keys| &continuous_keys.keys);

        // Rows ahead of the ranked list: 1 once the walker prepends a
        // sentence of several words (Step 4), which no later sort may move.
        let mut walker_slot0_len = 0usize;
        // Whether that slot is a sentence the walker composed from several
        // edges, not one word: the only slot a picked extension may take.
        let mut walker_slot0_is_sentence = false;
        // Whether a block was appended after the first sorted one (the §35
        // bare merge, Step 4b, Step 4c) — the only case Step 4d must re-sort,
        // along with a one-word path the walker placed at slot 0.
        let mut appended_blocks = false;
        // Whether Step 4 put a one-word path into the sort, at index 0.
        let mut release_one_word = false;
        let mut walker_moved = false;

        // ---- Step 2: empty-keys partial-prefix vs span-local fetch.
        let mut candidates: Vec<RawCandidate> = if keys.is_empty() {
            // v3.5.8 Phase 9 Item 10 — partial-prefix fallthrough.
            // The syllabifier produced no valid ending (e.g. `raw =
            // "gu"`, `"t"`, `"ㄉ"`), so the lookup-exact path is dead.
            // Fall through to `lookup_prefix` so the user still sees
            // engine candidates while typing toward the first syllable
            // boundary. Spec: `docs/engine/
            // continuous-candidate-display.md` §15.3.D + §15.5.
            //
            // Mode-aware for ALL modes: TL/POJ/English emit `tl:`/`poj:`
            // family keys (since v3.5.9 B-2); TPS emits the `tps:` family
            // (Fork 7b activated by 2026-05-27 dogfood — a leading
            // Bopomofo initial `ㄉ` scans `tps:ㄉ` byte-range and returns
            // every dictionary row whose `tps_notone` starts with `ㄉ`,
            // matching librime / khiin-rs / McBopomofo leading-prefix
            // behavior). [`crate::shadow::build_partial_prefix_key`]
            // returns `None` for empty-after-tone-strip input (bare tone
            // mark / hyphen-only / digit-only), so no unbounded scan.
            if let Some(ctx) = lex_ctx.as_ref() {
                let mut partial = fetch_via_lexicon_partial_inner(
                    raw,
                    raw_len,
                    mode,
                    ctx,
                    fetch_partial_prefix_candidates,
                );
                apply_hanji_display(&mut partial, khinsiann_in_hanji, mode);
                partial
            } else {
                Vec::new()
            }
        } else {
            // ---- Step 2b: span-local fetch.
            let mut c = match (continuous_keys.as_ref(), lex_ctx.as_ref()) {
                (Some(continuous_keys), Some(ctx)) => fetch_candidates_for_keys_with_barriers(
                    &continuous_keys.keys,
                    &continuous_keys.final_only,
                    &continuous_keys.tone_pins,
                    raw_len,
                    ctx,
                ),
                _ => Vec::new(),
            };
            // §35 bare-nasal merge (Codex pre-impl Q3): a single-glyph TPS
            // buffer whose ONLY span keys come from ambiguity expansion
            // (bare `ㄇ` → the syllabic `ㆬ` span) must not silently drop
            // the partial-prefix continuation list the literal glyph used
            // to reach — the user is mid-word at least as often as done.
            // Run BOTH and let the (canonical_tl, hanji, span) dedupe collapse
            // overlaps; span results stay first.
            if let Some(ctx) = lex_ctx.as_ref() {
                let literal_glyph_in_inventory = inv.is_some_and(|inv| {
                    inv.contains_in(phonetics::InputMode::Tps, &crate::shadow::lookup_form(raw))
                });
                let bare_expanded_only = mode == phonetics::InputMode::Tps
                    && raw.chars().count() == 1
                    && raw.chars().next().is_some_and(phonetics::is_tps_char)
                    && !literal_glyph_in_inventory;
                if bare_expanded_only {
                    c.extend(fetch_via_lexicon_partial_inner(
                        raw,
                        raw_len,
                        mode,
                        ctx,
                        fetch_partial_prefix_candidates,
                    ));
                    appended_blocks = true;
                }
            }
            // ---- Step 3: per-candidate recase loop.
            // v3.5.8 (Codex pre-impl 2A locus = candidate
            // construction): case each span-local candidate's roman
            // to mirror the user's raw input for its consumed span,
            // so the displayed candidate already shows `Hit` /
            // `tui` and tap commits it verbatim (display == commit).
            // This is the single casing source (the platform
            // `SuggestionCaseTransformer`s were removed 2026-10-04).
            // Only the presentation `roman` is touched —
            // `display_text` (canonical freq/NextWord key) and
            // `hanji` are deliberately left intact.
            for cand in &mut c {
                let (cs, ce) = cand.consumed_span;
                if let Some(seg) = raw.get(cs as usize..ce as usize) {
                    cand.roman = recase_roman(&cand.roman, seg, mode);
                }
            }
            apply_hanji_display(&mut c, khinsiann_in_hanji, mode);
            // ---- Step 4: walker slot-0 prepend (all modes).
            // v3.5.8 S2 — whole-sentence walker. The synthesized
            // full-buffer best path is explicitly prepended at slot 0
            // (Codex pre-impl S2 Q1 — the 8-dim `CandidateSortKey` cannot
            // guarantee slot 0 on its own). Span-aware de-dup against
            // the synth (Codex pre-impl S2 Q1d): drop any span-local
            // candidate with the synth's `(hanji, consumed_span)` and
            // either its `canonical_tl` identity or its recased roman, so
            // slot 0 is unique (e.g. a real left-anchored full-buffer
            // dict word equal to the walker path — keep the walker's
            // at slot 0, not a duplicate slot N).
            //
            // v3.5.9 D / C-3b — TPS now participates in the walker
            // (was excluded pre-C-3b because the lattice builder was
            // TL/POJ-only and `build_keys_tps` short-circuited the
            // shadow path). The unified `valid_span_endings_lowered`
            // dispatcher routes TPS through its own inventory-gated
            // scanner inside the same `build_shadow_lattice_with_barriers` /
            // `build_lattice` call, so the walker sees a real DAG and
            // can emit a slot-0 whole-sentence best path for TPS too.
            //
            // A2 D1 fold: the walker receives the pre-built
            // `(shadow, shadow_to_raw_end, lattice, inv)` plus
            // `prefix`/`dict` from the single seam `with_state`
            // scope — no second `build_shadow_lattice_with_barriers` per fetch.
            {
                if let (Some(continuous_keys), Some(inv), Some(ctx)) =
                    (continuous_keys.as_ref(), inv, lex_ctx.as_ref())
                {
                    if let Some(slot0) =
                        fetch_walker_slot0_inner(raw, raw_len, continuous_keys, inv, ctx)
                            .filter(|slot0| shape == ListShape::Sentence || slot0.edge_count == 1)
                    {
                        let composed_from_edges = slot0.edge_count >= 2;
                        // R2 identity sidechannel: read the canonical TL the
                        // walker already folded once (`WalkerSlot0.canonical_tl`,
                        // from the synth `roman`) — do NOT re-fold here. In the
                        // promote branch below `slot0_cand` is discarded and the
                        // promoted dict row carries its own `record.tl`
                        // `canonical_tl`.
                        let mut slot0_cand = RawCandidate {
                            rowid: NO_ROWID,
                            consumed_span: slot0.consumed_span,
                            syllable_count: slot0.syllable_count,
                            display_text: slot0.display_text,
                            roman: slot0.roman,
                            hanji: slot0.hanji,
                            canonical_tl: slot0.canonical_tl,
                            // see `WalkerSlot0`
                            score: 0.0,
                            frequency: 0,
                            walker_cost: WALKER_COST_UNPRICED,
                            bitmask: 0,
                            user_weight: slot0.user_weight,
                            context_rank: ranking::CONTEXT_RANK_NONE,
                            coverage_kind: slot0.coverage_kind,
                            is_custom: slot0.is_custom,
                        };
                        // `hoogua` bug (2026-06-02): the walker synth
                        // joins per-syllable canonical romans with a
                        // SPACE (`fetch_walker_slot0_inner` ~`.join(" ")`).
                        // For a genuine multi-word reading (台語齒盤) that
                        // space-join IS the desired display. But when the
                        // synth's full-buffer `(hanji, span)` coincides
                        // with a single lexical dict word that stores its
                        // own separator form (`-` hyphen / `--` neutral tone), the
                        // space-join is a malformed rendering of that word
                        // (`予我` synth `hōo guá` vs dict `hōo--guá`). The
                        // existing identity / roman dedupe below can't
                        // collapse the pair because `roman` and
                        // `canonical_tl` differ ONLY in the separator, so the malformed synth
                        // wins slot 0 and the canonical dict row sinks.
                        //
                        // Fix (display layer, NOT the cost/segmentation
                        // primitive — diagnosis §S5 / behavioral-invariants
                        // §18 lesson): when an existing FULL-coverage,
                        // full-span dict candidate shares the synth's hanji
                        // and is the SAME reading (separator-insensitive,
                        // tone-preserving — `roman_reading_eq`), promote
                        // that dict row's canonical roman to slot 0 and
                        // drop the synth. Gates:
                        // * `coverage_kind == COVERAGE_KIND_FULL` + same
                        //   `consumed_span` → a real whole-buffer dict word.
                        // * `roman_reading_eq` → `hōo guá` matches
                        //   `hōo--guá` (予我) but NOT `hōo-guā` (戶外,
                        //   tone 7 ≠ tone 2) — respects Core Principle #7
                        //   word identity (Hanji + canonical romanization).
                        // * `!slot0_cand.is_custom` → never replace a
                        //   custom-influenced walker path with a non-custom
                        //   dict row (would drop the user's custom-dict
                        //   effect; Codex pre-impl regression #3).
                        // * no exact `(roman, hanji, span)` dict twin → an
                        //   identical-form synth (`iā sī`, a one-edge `gín-á`)
                        //   stays and the dedupe below drops its twin;
                        //   promoting its sibling `gín--á` would swap the
                        //   walker's pick (behavioral-invariants §22). The
                        //   twin's dictionary fields are copied onto the
                        //   synth first: the synth is built with an empty
                        //   source, score, frequency and corpus cost, and
                        //   dropping the twin otherwise leaves `khì` / 去
                        //   looking unrecorded and sorting last on cost.
                        let is_same_full_span_word = |x: &RawCandidate| {
                            x.coverage_kind == COVERAGE_KIND_FULL
                                && x.consumed_span == slot0_cand.consumed_span
                                && x.hanji == slot0_cand.hanji
                        };
                        let exact_idx = c
                            .iter()
                            .position(|x| is_same_full_span_word(x) && x.roman == slot0_cand.roman);
                        let same_reading_idx = c.iter().position(|x| {
                            is_same_full_span_word(x)
                                && roman_reading_eq(&x.roman, &slot0_cand.roman)
                        });
                        walker_slot0_is_sentence =
                            composed_from_edges && same_reading_idx.is_none();
                        if let Some(i) = exact_idx.filter(|_| !slot0_cand.is_custom) {
                            let twin = &c[i];
                            slot0_cand.rowid = twin.rowid;
                            slot0_cand.score = twin.score;
                            slot0_cand.frequency = twin.frequency;
                            slot0_cand.walker_cost = twin.walker_cost;
                            slot0_cand.bitmask = twin.bitmask;
                            slot0_cand.syllable_count = twin.syllable_count;
                            if twin.user_weight > slot0_cand.user_weight {
                                slot0_cand.user_weight = twin.user_weight;
                            }
                            if twin.context_rank < slot0_cand.context_rank {
                                slot0_cand.context_rank = twin.context_rank;
                            }
                        }
                        let promote_idx = same_reading_idx
                            .filter(|_| !slot0_cand.is_custom && exact_idx.is_none());
                        // Slot 0 is either the promoted canonical dict row
                        // (removed from its current position) or the synth.
                        // Both then run the SAME dedupe — same word identity
                        // or same recased roman, as [`retain_absent_from`] —
                        // + `insert(0, …)` so slot 0 stays unique. The
                        // identity half is a defensive mirror here: the
                        // walker's edge pick and the lexicon list run the
                        // same dedupe (a custom row replaces its dictionary
                        // twin in both), so today slot 0 and its leftover
                        // twin always share the recased roman.
                        let slot0 = match promote_idx {
                            Some(i) => c.remove(i),
                            None => slot0_cand,
                        };
                        c.retain(|x| {
                            !(x.hanji == slot0.hanji
                                && x.consumed_span == slot0.consumed_span
                                && (x.canonical_tl == slot0.canonical_tl || x.roman == slot0.roman))
                        });
                        c.insert(0, slot0);
                        walker_slot0_len = 1;
                        // A one-word path is one word like any other row, so it
                        // joins the twelve stages (ranking-lab proposal, 2026-11):
                        // a word picked through the abbreviation `kip` can pass
                        // 級, then the Step 4d post-pass places it. A path of
                        // several words stays pinned at slot 0.
                        if !composed_from_edges {
                            walker_slot0_len = 0;
                            release_one_word = true;
                        }
                    }
                }
            }
            // ---- Step 4b: whole-input prefix-extension scan.
            //
            // Mainstream IME parity. Dict rows whose toneless FST key has
            // `raw` as a strict prefix (key extends BEYOND raw, e.g.
            // `tl:taigir` for input `taigi` → `tâi-gír`/`台語`) are not
            // reachable from the span-local + walker pair:
            // * The span-local fetch (`fetch_candidates_for_keys_with_barriers`)
            //   calls `lookup_exact` per left-anchored lattice edge, so it
            //   only matches keys EQUAL to a `(0, end)` toneless prefix.
            // * Walker spans `0..raw_len` only, so it cannot extend past
            //   the buffer length to surface a longer key.
            // * The `keys.is_empty()` partial-prefix branch only fires
            //   when the syllabifier produced no valid ending at all.
            //
            // Step 4b runs the same `fetch_partial_prefix_candidates`
            // machinery that branch uses against the whole-buffer key
            // built by `build_partial_prefix_key(raw, mode)`. The result
            // carries `COVERAGE_KIND_PARTIAL_PREFIX`; Step 4d places it by
            // `CandidateSortKey` among the FULL rows (tier first: it covers
            // the whole input, so it leads front-segment rows).
            //
            // References:
            // - librime `src/rime/gear/table_translator.cc:178` →
            //   `dictionary.cc:308-309` `LookupWords(predictive=true)` →
            //   `prism_->ExpandSearch` is the default `table_translator`
            //   path; both user dict and main dict surface prefix
            //   extensions for an input already on a valid syllable
            //   boundary.
            // - khiin-rs `khiin/src/input/converter.rs:58-69`
            //   `candidates_for_splittable` keeps `elem.raw_text().len()
            //   >= query.len()` via `select_conversions_for_multiple` on
            //   prefix-trie hits.
            // - Our own `engine/lexicon/src/search.rs::search` normal-
            //   mode autocomplete already does `lookup_exact ++
            //   lookup_prefix` via `IndexSet` order-preserving merge.
            //   The continuous path is the outlier.
            //
            // **Trailing-hyphen gate.** When `synth_consumed_span` returns
            // `None` (shadow ends short of `raw_len` because the raw
            // buffer carries a pending hyphen — `tai-`, `tai-bak-`),
            // Step 4b must NOT emit a `(0, raw_len)`-span candidate that
            // would mis-commit the pending `-`. Mirrors the walker
            // slot-0 trailing-hyphen suppression at
            // `fetch_walker_slot0_inner`'s `synth_consumed_span?`
            // early-out, same contract.
            //
            // **Recase.** Recase is centralized in
            // [`fetch_via_lexicon_partial_inner`] so the empty-keys
            // branch and Step 4b apply the same per-candidate case
            // mapping — without that centralization the display of a
            // partial-prefix hit would case-flip as the user crosses
            // the first syllable boundary. Multi-syllable extension
            // candidates whose syllable count exceeds the raw segment
            // share the single-bucket case treatment documented at
            // [`recase_roman`] (per-syllable case intent is not
            // recoverable past the raw segment length).
            //
            // **Cross-batch dedupe.** Both fetchers run their own internal
            // `dedupe_by_roman_hanji_span` pre-sort, but that does not
            // span batches. A partial-prefix hit on the same dict row
            // already present in the FULL block (walker slot-0 recases
            // edge-by-edge in `fetch_walker_slot0_inner`; span-local
            // recases here at Step 3) normalizes to the same `(roman,
            // hanji, consumed_span)` triple, or carries the same
            // `(canonical_tl, hanji, consumed_span)` identity, and collapses.
            if let (Some(continuous_keys), Some(ctx)) = (continuous_keys.as_ref(), lex_ctx.as_ref())
            {
                if synth_consumed_span(
                    &continuous_keys.shadow_to_raw_end,
                    continuous_keys.shadow.len(),
                    raw_len,
                )
                .is_some()
                {
                    // v3.5.9 Codex PR #351 r3321758666 — use the un-truncated
                    // pool so the FULL-block exclude is applied BEFORE the
                    // `PARTIAL_PREFIX_OUTPUT_CAP` truncate. Otherwise, for
                    // inputs whose exact key has many homophones (e.g.
                    // `tl_notone = hong` has 30+ exact rows in production),
                    // the bounded fetcher would return 30 duplicates of the
                    // FULL block, and the post-fetch dedupe would drop all
                    // 30 — leaving zero extension candidates visible even
                    // though `hong-tshia` / `hong-thai` etc. exist in the
                    // prefix range. The un-bounded variant returns the
                    // sorted pool (≤ `PARTIAL_PREFIX_HYDRATE_CAP` after
                    // dedupe); composing-side filters out FULL duplicates,
                    // then applies `PARTIAL_PREFIX_OUTPUT_CAP` itself.
                    let mut ext = fetch_via_lexicon_partial_inner(
                        raw,
                        raw_len,
                        mode,
                        ctx,
                        fetch_partial_prefix_candidates_unbounded,
                    );
                    apply_hanji_display(&mut ext, khinsiann_in_hanji, mode);
                    // Borrowed key shape — `c` is not mutated during the
                    // retain on `ext`, so HashSet entries can hold &str
                    // slices into `c` and avoid per-keystroke String allocs.
                    //
                    // Recase byte-identity holds for the dominant case:
                    // span-local single-edge FULL hits AND walker slot-0
                    // single-buffer paths, both at span (0, raw_len), both
                    // recased against the same raw segment as `ext`.
                    //
                    // Known divergence (Codex post-impl SHIP-WITH-FIXES
                    // 2026-05-29): walker slot-0 joins per-edge-recased
                    // syllables with a space (see [`fetch_walker_slot0_inner`]),
                    // whereas `ext` recases the full raw segment as one
                    // string. For multi-edge walker paths with mixed-case
                    // per-syllable intent like `TaIGi`, walker produces
                    // `Tâi Gí` while `ext` produces `Tâi-gí` for the same
                    // dict row — the exclude misses the duplicate. Visible
                    // only when ≥30 homophones share an exact key AND raw
                    // is mixed-case per-syllable, so deferred to a
                    // follow-up. The normal lowercase path (the v3.5.9
                    // continuous dogfood baseline) is unaffected.
                    retain_absent_from(&c, &mut ext);
                    // Truncate AFTER exclude so the visible top-N is the
                    // best-scoring subset of the non-duplicate extensions,
                    // not the post-truncate dregs of a homophone-dominated
                    // pool. The lexicon-side sort is preserved (the
                    // un-truncated pool was already sorted by CandidateSortKey),
                    // so `truncate` keeps the global score order.
                    ext.truncate(PARTIAL_PREFIX_OUTPUT_CAP);
                    appended_blocks |= !ext.is_empty();
                    c.extend(ext);
                }
            }
            c
        };
        // ---- Step 4c: whole-buffer abbreviation lookup (both branches).
        //
        // `ss` → 鎖匙 `só-sî`, TPS `ㄙㄒ` → 鎖匙 (§46). The gate is the
        // buffer's shape (`shadow::abbrev_query_key`), not "no exact keys":
        // `mk` has the left-anchored syllable `m` and is still the
        // abbreviation of 物件. Hits carry `COVERAGE_KIND_ABBREV`, which
        // `CandidateSortKey` ranks below PARTIAL within a tier (Step 4d) —
        // for a valid onset like `ts` the single-syllable partial hits stay
        // ahead and the abbreviated words trail them; a whole-input
        // abbreviation leads front-segment rows (`mk` 物件 before `m` 毋). Same recase → exclude → truncate
        // order as Step 4b: a row can be both a partial extension and an
        // abbreviation hit (`m̄-khíng` under `mk`), and truncating before
        // the exclude would lose real candidates to the duplicates cut.
        // The §34 literal is prepended later by the dispatch caller.
        if let Some(ctx) = lex_ctx.as_ref() {
            if let Some(key) = crate::shadow::abbrev_query_key(raw, mode) {
                let mut abbrev = lexicon::fetch_abbrev_candidates(&key, raw_len, ctx);
                recase_all(&mut abbrev, raw, mode);
                apply_hanji_display(&mut abbrev, khinsiann_in_hanji, mode);
                retain_absent_from(&candidates, &mut abbrev);
                abbrev.truncate(PARTIAL_PREFIX_OUTPUT_CAP);
                appended_blocks |= !abbrev.is_empty();
                candidates.extend(abbrev);
            }
        }
        // ---- Step 4d: one order over the merged blocks (USER 2026-10-12).
        // Each block above arrives sorted, but `CandidateSortKey` leads with
        // the tier: a Step 4b extension or Step 4c abbreviation covering the
        // whole input outranks a span-local row covering only its front, so
        // the concatenation is re-sorted (stable — each block's own order and
        // its exclude → truncate are kept). A walker slot 0 of several words
        // stays put, unless a picked word the walker cannot reach outranks
        // its sentence; a one-word slot 0 is sorted with the rest, then
        // [`lead_one_syllable_prefix_picks`] places it.
        if appended_blocks || release_one_word {
            let ranked = candidates.split_off(walker_slot0_len.min(candidates.len()));
            if release_one_word {
                // Each row keeps its pre-sort index; the released path is 0.
                let mut sorted = sort_by_candidate_key(
                    ranked.into_iter().enumerate().collect(),
                    raw_len,
                    |(_, candidate): &(usize, RawCandidate)| candidate.rank_facts(),
                );
                walker_moved = lead_one_syllable_prefix_picks(&mut sorted);
                candidates.extend(sorted.into_iter().map(|(_, candidate)| candidate));
            } else {
                candidates.extend(sort_by_candidate_key(
                    ranked,
                    raw_len,
                    RawCandidate::rank_facts,
                ));
            }
            if walker_slot0_is_sentence && picked_extension_outranks_sentence(&candidates, raw_len)
            {
                candidates.swap(0, 1);
            }
        }
        // ---- Step 5: presentation pass (typed separators, POJ render,
        // Syllable Separator, ⁿ case).
        // §55 — the separator at every typed boundary is the one the user
        // typed (`phonetics::api::render_typed_separators`). `roman` only:
        // `display_text` / `canonical_tl` keep the record's form, so the
        // `(hanji, canonical_tl)` identity never forks on a typed
        // separator. Every row is left-anchored at 0 (span-local keys and
        // the walker alike), so the whole shadow is each row's typed
        // body. Before the POJ render (same `-` / space tokens) and before
        // the rendered dedupe; TL / POJ short-circuit (the walk itself
        // refuses TPS / English).
        let mut typed_separators_rendered = false;
        if let Some(continuous_keys) = continuous_keys
            .as_ref()
            .filter(|k| !k.hyphen_runs.is_empty())
            .filter(|_| matches!(mode, phonetics::InputMode::Tl | phonetics::InputMode::Poj))
        {
            for cand in &mut candidates {
                // A one-token roman has no separator to rewrite.
                if !cand.roman.contains(['-', ' ']) {
                    continue;
                }
                if let Some(roman) = phonetics::api::render_typed_separators(
                    mode,
                    &continuous_keys.shadow,
                    &continuous_keys.hyphen_runs,
                    &cand.canonical_tl,
                    &cand.roman,
                ) {
                    cand.roman = roman;
                    typed_separators_rendered = true;
                }
            }
        }
        // v3.5.8 — POJ-display render. The Continuous platform
        // builders are mode-agnostic by design (Item 13: "the
        // engine owns input-mode handling"); mirror the engine-side
        // NextWord POJ render (`engine/nextword/src/filter.rs`).
        // Rewrite the presentation `roman` TL→POJ (`oo`→`o͘`,
        // `nn`→`ⁿ`, …) for EVERY emitted candidate — span-local
        // (recased above), the walker slot-0 best candidate
        // inserted at index 0, and the partial-prefix branch — so
        // display AND the platform-formatted commit (both derive
        // from `roman`) are POJ. `display_text` (the canonical
        // commit + `user_frequency.db` key) is deliberately
        // untouched; `hanji` was rendered at Step 3
        // (`apply_hanji_display`). No-op for TL / English /
        // (legacy-mapped) TPS.
        if mode == phonetics::InputMode::Poj {
            for cand in &mut candidates {
                cand.roman = recase_tl_as_poj_display(&cand.roman);
            }
        }
        // Syllable Separator (§49) — same presentation seam, same field:
        // only `roman`, for dictionary, custom and walker rows alike;
        // `display_text` / `canonical_tl` keep the dictionary form. After
        // the POJ render (which splits on `-`), before the rendered
        // dedupe (which keys on what the cell shows) and before the §34
        // literal prepend in dispatch (the literal keeps typed hyphens).
        // `None` under Hyphen and on the TPS layout.
        if let Some(joiner) = syllable_joiner {
            for cand in &mut candidates {
                if cand.roman.contains('-') {
                    cand.roman = phonetics::api::syllable_joiner_display(&cand.roman, joiner);
                }
            }
        }
        // ⁿ becomes ᴺ in capitals (§53) — same seam, same field: the one nasal-marker case
        // rule the preedit follows (`normalize_tone`), triggered by the
        // marker, not the mode. `raise_case` leaves the marker as stored,
        // so without this a Caps Lock strip read `SIAⁿ` under a `SIAᴺ`
        // preedit.
        for cand in &mut candidates {
            if let std::borrow::Cow::Owned(roman) =
                phonetics::case_transform::apply_nasal_marker_case(
                    &cand.roman,
                    force_lowercase_nasal_marker,
                )
            {
                cand.roman = roman;
            }
        }
        if mode == phonetics::InputMode::Poj
            || syllable_joiner.is_some()
            || typed_separators_rendered
        {
            dedupe_rendered_continuous(&mut candidates);
        }
        // Same-commit-Hanji visual dedupe — when every cell displays and
        // commits plain Hanji alone (TPS layout/buffer or Hanji-first paired
        // display with Annotate in Brackets off),
        // collapse duplicate `(hanji, consumed_span)` rows keeping the
        // highest-ranked first-seen row.
        if dedupe_commit_hanji {
            dedupe_same_commit_hanji(&mut candidates);
        }
        // A word's list: the Step 4d order within a tier, so the whole-input
        // rows (end == raw_len, which no row passes) read as in the sentence
        // list, and longer words first among the front-segment rows whatever
        // the counts or the custom source (H4). Stable, so each length keeps
        // its ranking. The Step 4 filter lets only a one-edge path in, so no
        // slot is pinned ahead of this sort; a dictionary word the walker
        // reaches over several edges (`hoogua` → 予我) is a whole-input row
        // and leads with its tier.
        if shape == ListShape::Word {
            candidates.sort_by_key(|candidate| std::cmp::Reverse(candidate.consumed_span.1));
        }
        Ok(Assembled {
            candidates,
            walker_moved,
        })
    })
    .unwrap_or_default()
    // Step 6 (wrap into `ContinuousResponse` via `raw_to_proto_candidate`
    // + `with_continuous`) stays in `requests::handle_fetch_at_pos`.
}
