//! Continuous candidate construction: dictionary / custom / learned rows to
//! [`RawCandidate`], plus the `(roman, hanji, span)` dedupe.

use ranking::{calculate_continuous_score, source_tier_rank, FrequencyMap};

use super::sort_key::sort_by_sort_key;
use super::{
    derive_mode, ConsumedSpan, ContinuousFetchCtx, CustomEntry, LearnedEntry, RawCandidate,
    FORM_NOTONE,
};
use crate::dictionary_reader::DictionaryRecord;

/// Shared tail of every continuous fetch: merge `custom_dictionary.db`
/// hits into the dictionary candidates `out`, collapse `(roman, hanji,
/// span)` duplicates, then apply the eight-dimension [`SortKey`](super::sort_key::SortKey) sort.
/// `coverage_kind` is stamped on the custom synths — `COVERAGE_KIND_FULL`
/// on the exact path, `COVERAGE_KIND_PARTIAL_PREFIX` on the partial-prefix
/// path so §15.5's "partial-prefix ranks strictly below full-syllable" rule
/// is preserved there (Codex pre-impl D6).
///
/// **Custom merge** (v3.5.8 Phase 9 Item 12): each custom entry is
/// synthesized as a full-buffer candidate (`consumed_span = (0, raw_len)`,
/// `is_custom = true` → `source_tier_rank` rank 0) and appended AFTER the
/// FST hits so a `(roman, hanji)` duplicate keeps the earlier-inserted
/// `dict.bin` candidate only when source ranks tie (they never do — custom
/// rank 0 < every `dict.bin` rank ≥ 1, so the custom entry always wins its
/// collision). The legacy custom dict is prefix-visible, so the
/// partial-prefix path merges too — Continuous must not hide the user's
/// custom word while they are still typing toward the first syllable
/// boundary. See `docs/engine/continuous-input-ranking.md` §10.10.
///
/// A custom entry is synthesized whole-buffer, so the buffer's
/// [`TonePin`](super::TonePin) applies to it exactly as to a dictionary hit: with the
/// tail syllable space-closed (§41), or a syllable typed with its digit
/// (§17), an entry whose reading disagrees there is not what the user
/// asked for. Custom matching itself is toneless, so skipping this would
/// let the tone promise leak through the custom source, which is appended
/// AFTER dictionary filtering. On the partial-prefix path the typed body
/// is a strict prefix of the entry's reading, so the check lands on the
/// syllables typed so far, not on the entry's own tail.
///
/// **Dedupe** (Item 12): `dict.bin` is already collapsed by
/// `dictionary/build/merge_csv.py:107`'s `groupby(["hanzi", "_tl_key"])`,
/// so the only realistic duplicate is custom-vs-`dict.bin` sharing a
/// `(roman, hanji)` pair. MUST run BEFORE the `SortKey` sort: the winner is
/// the lowest `source_tier_rank` survivor (custom rank 0 beats any
/// `dict.bin` tier), which is NOT what the full 8-dim sort would pick (it
/// weighs `score`/`freq` ahead of `source_rank`, so a high-freq `dict.bin`
/// duplicate could otherwise mask the user's custom entry). `(roman,
/// hanji)` is the dual key (Codex pre-impl D1) so romanization variants of
/// the same hanji are preserved; S2 extends it with `consumed_span` (Codex
/// pre-impl S2 Q1d) — both the custom synth and its `dict.bin` duplicate
/// are emitted at the same `(0, raw_len)` span so the collapse still fires.
///
/// **Sort** (Phase 9.1): `stable_idx` is stamped from pre-sort element
/// position via `enumerate()` BEFORE any sorting machinery runs, so the
/// index reflects insertion order (caller-provided `keys` order × FST
/// byte-sort) and is independent of how the sort algorithm invokes the key
/// extractor — `slice::sort_by_cached_key` does NOT contractually pin that
/// order; earlier revisions incrementing a counter inside the closure were
/// silently relying on stdlib internals (Codex PR #262 r3216153007; PR-9.1
/// PR-bot R1 fix `be86f5f7`). No truncate here: the partial-prefix bounded
/// wrapper applies `PARTIAL_PREFIX_OUTPUT_CAP` itself, and Step 4b in
/// `composing::continuous` takes the un-truncated pool.
pub(super) fn merge_custom_dedupe_sort(
    mut out: Vec<RawCandidate>,
    ctx: &ContinuousFetchCtx<'_>,
    raw_len: u32,
    coverage_kind: u8,
) -> Vec<RawCandidate> {
    for entry in ctx.custom {
        if !ctx.tone_pin.admits_custom(&entry.roman, ctx.mode) {
            continue;
        }
        out.push(custom_entry_to_candidate(
            entry,
            raw_len,
            ctx.freq_map,
            ctx.now_ms,
            coverage_kind,
            ctx.mode,
        ));
    }
    // Learned phrases (§50) — same whole-buffer span as a custom row; the
    // `(roman, hanji, span)` dedupe below keeps the `dict.bin` / custom
    // duplicate over it (lowest `source_tier_rank` wins, a learned row has
    // the default rank), so a learned pair that the dictionary also carries
    // is listed once, from the dictionary.
    for entry in ctx.learned {
        if !ctx.tone_pin.admits(None, &entry.canonical_tl) {
            continue;
        }
        out.push(learned_entry_to_candidate(
            entry,
            (0, raw_len),
            ctx.freq_map,
            ctx.now_ms,
            coverage_kind,
        ));
    }
    dedupe_by_roman_hanji_span(&mut out);
    sort_by_sort_key(out, raw_len)
}

pub(super) fn record_to_candidate(
    record: DictionaryRecord,
    effective_bitmask: u16,
    consumed_span: ConsumedSpan,
    freq_map: &FrequencyMap,
    now_ms: i64,
    coverage_kind: u8,
) -> RawCandidate {
    let DictionaryRecord {
        bitmask: _,
        frequency,
        syllable_count,
        hanzi,
        tl,
        kautian_subtag: _,
    } = record;
    // EFFECTIVE bitmask (kautian bit dropped when its subcollection is
    // disabled) drives `source_tier_rank` so a multi-source survivor ranks by
    // its other source's tier, not kautian's (DD6 ranking-weight drop).
    let bitmask = effective_bitmask;
    let mode = derive_mode(hanzi.as_deref());
    // Phase 9 Item 5: `roman` = `tl` alongside `display_text`. `hanji`
    // mirrors `DictionaryRecord.hanzi` verbatim so the proto3 `optional`
    // field can preserve the absent-vs-empty distinction. R2 identity
    // sidechannel: `DictionaryRecord.tl` is already canonical TL, so
    // `canonical_tl` equals `roman` here BEFORE the composing-layer recase /
    // POJ-render passes rewrite `roman`. `display_text` is derived last so
    // each source string is cloned once, not twice.
    let roman = tl;
    let canonical_tl = roman.clone();
    let hanji = hanzi;
    let display_text = hanji.clone().unwrap_or_else(|| roman.clone());
    // Phase 9.3a + R5 pair-key (#7): look up the candidate's
    // user-frequency snapshot by the `(display_text, canonical_tl)`
    // identity — the same pair the platform writes to `user_frequency.db`
    // on commit (`display_text` key + `canonical_tl` reading). The
    // tolerant `get` falls back to the legacy `tl == ""` bucket on an
    // exact miss; absent entries fall through to `FrequencyData::default()`
    // (count = 0, last_used_ms = 0) → `user_weight = 0.0`, the cold-start
    // neutral path.
    let user_weight = freq_map
        .get(&display_text, &canonical_tl)
        .user_weight(now_ms);
    let score = calculate_continuous_score(frequency, syllable_count);
    RawCandidate {
        consumed_span,
        syllable_count,
        display_text,
        roman,
        hanji,
        canonical_tl,
        score,
        form: FORM_NOTONE,
        frequency,
        bitmask,
        mode,
        user_weight,
        coverage_kind,
        // dict.bin FST hit — `source_tier_rank` derives the rank from
        // `bitmask`; `is_custom = false` keeps the kautian/taigitv/…
        // ordering. Only `custom_entry_to_candidate` sets `true`.
        is_custom: false,
    }
}

/// v3.5.8 Phase 9 Item 12 — synthesize a [`RawCandidate`] from a
/// platform-supplied [`CustomEntry`] (`custom_dictionary.db` row).
///
/// Shape decisions (Codex pre-impl 2026-05-15, D3 / D4):
///
/// - `consumed_span = (0, raw_len)` — custom entries are outside the
///   FST/syllabifier span model, so they commit the whole buffer as
///   one block (final-commit), mirroring the legacy lexicon path's
///   treatment of custom dict and Item 10 partial-prefix Q15.4. With
///   `consumed_span_end == raw_len` the downstream `SortKey.tier` is
///   `0` (full-buffer) — NO forced Tier-1 promotion
///   (`docs/releases/v3.5.8/plan.md` § Phase 9: "無強制 Tier 1 promotion").
/// - `frequency = 0`, `syllable_count = 1` — `custom_dictionary.db`
///   carries no `dict.bin`-comparable frequency. `is_custom = true`
///   gives `source_tier_rank` rank `0`, which is what governs the
///   `(roman, hanji)` dedupe winner and prior-axis ties; it does NOT
///   globally float custom above `dict.bin` because `SortKey` weighs
///   `score`/`freq` ahead of `source_rank`. This is intentional —
///   Item 12's job is duplicate elimination + custom-wins-collision,
///   not a global custom-priority tier
///   (`docs/engine/continuous-input-ranking.md` §10.10).
/// - `display_text = hanji.unwrap_or(canonical_tl_form(roman, mode))` —
///   v3.5.9 B-4 closes the bounded asymmetry the pre-B-4
///   `unwrap_or(roman)` contract documented at
///   [`crate::dedupe_by_roman_hanji_span`] callers and
///   `composing::continuous::dedupe_rendered_continuous`: a POJ-form
///   roman now folds to canonical TL so the commit +
///   `user_frequency.db` write key collides with the same word coming
///   from `dict.bin` (which writes `record.tl`, already canonical TL).
///   Identical wire contract to [`record_to_candidate`] across modes.
/// - `mode = derive_mode(hanji)` — TAILO when `hanji` is `None`.
/// - user-frequency boost / recency are applied identically to
///   `dict.bin` candidates (custom entries can also be user-selected).
pub(super) fn custom_entry_to_candidate(
    entry: &CustomEntry,
    raw_len: u32,
    freq_map: &FrequencyMap,
    now_ms: i64,
    coverage_kind: u8,
    input_mode: phonetics::InputMode,
) -> RawCandidate {
    let roman = entry.roman.clone();
    let hanji = entry.hanji.clone();
    let mode = derive_mode(hanji.as_deref());
    // v3.5.9 B-4 — `display_text` is the platform's
    // `user_frequency.db` commit key. When `hanji` is absent the
    // fallback is the roman, which may have been stored in the user's
    // native form (POJ display form on a POJ-mode entry). Folding
    // through `canonical_tl_form` keeps the freq key mode-invariant
    // so a custom entry typed in POJ and the same word coming back
    // from `dict.bin` share one frequency bucket. `roman` itself is
    // left raw (the walker / `custom_toneless_key` need it in the
    // user's native form so POJ-family lattice keys match against
    // POJ-form custom roman — see `composing::shadow::custom_toneless_key`).
    // R2 identity sidechannel: fold the user's native-form roman (POJ
    // display form on a POJ-mode entry) to canonical TL ONCE, then reuse
    // for both the hanji-absent `display_text` fallback and the
    // `(hanji, canonical-TL)` identity key. Populated for hanji-PRESENT
    // entries too (Codex pre-impl 2026-06-03 BLOCK: identity is the pair,
    // not gated on hanji). TPS-mode `canonical_tl_form` is identity
    // (Bopomofo, no TL) → `canonical_tl` stays Bopomofo for a TPS-OOV
    // hanji-absent custom entry; the platform treats a non-TL form as
    // "no canonical TL" only for the wire-empty case, so the existing
    // TPS round-trip is unchanged.
    let canonical_tl = phonetics::api::canonical_tl_form(&roman, input_mode);
    let display_text = hanji.clone().unwrap_or_else(|| canonical_tl.clone());
    // R5 pair-key (#7): same `(display_text, canonical_tl)` identity as
    // `record_to_candidate`. For a hanji-absent custom/OOV entry
    // `display_text == canonical_tl`; the tolerant `get` still falls back
    // to the legacy `tl == ""` bucket on an exact miss.
    let user_weight = freq_map
        .get(&display_text, &canonical_tl)
        .user_weight(now_ms);
    // `frequency = 0` (D4) → score 0; the candidate floats on
    // `user_weight` / `source_rank` / dedupe, not raw freq.
    let score = calculate_continuous_score(0, 1);
    RawCandidate {
        consumed_span: (0, raw_len),
        syllable_count: 1,
        display_text,
        roman,
        hanji,
        canonical_tl,
        score,
        form: FORM_NOTONE,
        frequency: 0,
        // No `dict.bin` source bits; rank is forced to 0 via
        // `is_custom = true` in `SortKey::new` /
        // `dedupe_by_roman_hanji_span` (`source_tier_rank` short-circuits).
        bitmask: 0,
        mode,
        user_weight,
        coverage_kind,
        is_custom: true,
    }
}

/// Learned phrases (§50) — one [`LearnedEntry`] as a candidate at `span`:
/// a `dict.bin` record the dictionary does not carry, with no frequency and
/// no source bits, so [`SortKey`](super::sort_key::SortKey) gives it the default source rank (below
/// every dictionary source and below a manual custom row) and the
/// `(roman, hanji, span)` dedupe keeps a dictionary / custom duplicate over
/// it. `syllable_count` is read off the TL's separators for the walker's
/// `n_syls` bias.
pub(super) fn learned_entry_to_candidate(
    entry: &LearnedEntry,
    span: ConsumedSpan,
    freq_map: &FrequencyMap,
    now_ms: i64,
    coverage_kind: u8,
) -> RawCandidate {
    let syllable_count = phonetics::api::tl_syllables(&entry.canonical_tl)
        .count()
        .clamp(1, u8::MAX as usize) as u8;
    record_to_candidate(
        DictionaryRecord {
            bitmask: 0,
            frequency: 0,
            hanzi: Some(entry.hanji.clone()),
            tl: entry.canonical_tl.clone(),
            syllable_count,
            kautian_subtag: 0,
        },
        0,
        span,
        freq_map,
        now_ms,
        coverage_kind,
    )
}

/// v3.5.8 Phase 9 Item 12 — `(roman, hanji)` dedupe (Codex pre-impl
/// D1 + D2, 2026-05-15). **v3.5.8 S2: key extended to
/// `(roman, hanji, consumed_span)`** (Codex pre-impl S2 Q1d,
/// 2026-05-16). Runs on the merged `dict.bin` + custom candidate
/// vector BEFORE the `SortKey` sort.
///
/// - **Key**: the triple `(roman, hanji, consumed_span)`. The
///   `(roman, hanji)` pair (D1) keeps romanization variants of the
///   same hanji distinct; adding `consumed_span` keeps the **same
///   word at different spans** distinct — once the whole-sentence
///   walker / path-step candidates exist (S2) the same `(roman,
///   hanji)` legitimately recurs at different spans and must NOT be
///   collapsed (which the pre-S2 `(roman, hanji)`-only key would
///   wrongly do). The Item-12 custom-vs-`dict.bin` collapse is
///   preserved: both the custom synth and its `dict.bin` duplicate
///   are emitted at the **same** full-buffer span `(0, raw_len)`
///   (see the `custom_entry_to_candidate` call sites above), so the
///   span-augmented key still collides and custom still wins by
///   source rank.
/// - **Winner** (D2): the survivor with the lowest
///   `source_tier_rank(bitmask, is_custom)` (custom = rank 0 beats
///   every `dict.bin` tier ≥ 1). On a rank tie the earlier-inserted
///   candidate wins (deterministic; `dict.bin` hits are inserted
///   before custom, so a `dict.bin`-vs-`dict.bin` tie — which
///   `merge_csv.py` already precludes in production — keeps the first
///   FST hit).
/// - Survivor **insertion order is preserved** so the downstream
///   `SortKey.stable_idx` stays deterministic.
pub(super) fn dedupe_by_roman_hanji_span(out: &mut Vec<RawCandidate>) {
    use std::collections::{HashMap, HashSet};
    // key (borrowed from `out`) → (winning source rank, index of winner).
    let mut best: HashMap<(&str, Option<&str>, ConsumedSpan), (u8, usize)> =
        HashMap::with_capacity(out.len());
    for (i, c) in out.iter().enumerate() {
        let rank = source_tier_rank(c.bitmask, c.is_custom);
        let key = (c.roman.as_str(), c.hanji.as_deref(), c.consumed_span);
        // Strictly lower rank replaces; equal rank keeps the earlier index
        // (no replace) → deterministic tie-break.
        let slot = best.entry(key).or_insert((rank, i));
        if rank < slot.0 {
            *slot = (rank, i);
        }
    }
    if best.len() == out.len() {
        return; // no duplicates — common production path, skip rebuild.
    }
    let winners: HashSet<usize> = best.into_values().map(|(_, idx)| idx).collect();
    let mut idx = 0usize;
    out.retain(|_| {
        let keep = winners.contains(&idx);
        idx += 1;
        keep
    });
}

#[cfg(test)]
mod record_to_candidate_carrier_tests {
    //! v3.5.8 Phase 9 Item 5 — `record_to_candidate` populates the
    //! `roman` + `hanji` sidechannels alongside `display_text` so the
    //! proto3 wire carries both for dual-line UI render. These tests
    //! pin the field-population rule across the three `CandidateMode`
    //! axes (HANT / TAILO / MIXED).
    use super::*;
    use crate::continuous::{CandidateMode, COVERAGE_KIND_FULL};

    fn record(tl: &str, hanzi: Option<&str>) -> DictionaryRecord {
        DictionaryRecord {
            bitmask: 0,
            frequency: 0,
            syllable_count: 1,
            kautian_subtag: 0,
            hanzi: hanzi.map(str::to_owned),
            tl: tl.to_owned(),
        }
    }

    #[test]
    fn hant_record_emits_roman_and_some_hanji() {
        let cand = record_to_candidate(
            record("tâi-uân", Some("臺灣")),
            0,
            (0, 7),
            &FrequencyMap::new(),
            0,
            COVERAGE_KIND_FULL,
        );
        assert_eq!(cand.roman, "tâi-uân");
        assert_eq!(cand.hanji.as_deref(), Some("臺灣"));
        assert_eq!(cand.display_text, "臺灣");
        // R2: identity sidechannel = canonical TL even though display_text
        // is the hanji. This is what the platform round-trips into the
        // NextWord association as `next_tl`/`prev_tl`.
        assert_eq!(cand.canonical_tl, "tâi-uân");
        assert_eq!(cand.mode, CandidateMode::Hant);
    }

    #[test]
    fn tailo_record_emits_roman_and_none_hanji() {
        // `hanzi = None` → TAILO path; `display_text` falls back to TL,
        // `roman` stays equal to TL, `hanji` is wire-absent
        // (proto3 `optional` distinguishes None from Some("")).
        let cand = record_to_candidate(
            record("tāi", None),
            0,
            (0, 3),
            &FrequencyMap::new(),
            0,
            COVERAGE_KIND_FULL,
        );
        assert_eq!(cand.roman, "tāi");
        assert_eq!(cand.hanji, None);
        assert_eq!(cand.display_text, "tāi");
        assert_eq!(cand.mode, CandidateMode::Tailo);
    }

    #[test]
    fn mixed_record_emits_roman_and_hanji_with_latin() {
        // MIXED = hanji string contains Latin letters after NFKD.
        // `display_text` keeps the MIXED hanji string verbatim;
        // `roman` still equals the pure TL romanization.
        let cand = record_to_candidate(
            record("hip-siòng", Some("hip相")),
            0,
            (0, 9),
            &FrequencyMap::new(),
            0,
            COVERAGE_KIND_FULL,
        );
        assert_eq!(cand.roman, "hip-siòng");
        assert_eq!(cand.hanji.as_deref(), Some("hip相"));
        assert_eq!(cand.display_text, "hip相");
        assert_eq!(cand.mode, CandidateMode::Mixed);
    }
}

#[cfg(test)]
mod item12_custom_dedupe_tests {
    //! v3.5.8 Phase 9 Item 12 — `custom_dictionary.db` synthesis +
    //! `(roman, hanji)` dedupe. Pins Codex pre-impl decisions D1
    //! (dual `(roman, hanji)` key), D2 (lowest `source_tier_rank`
    //! winner, rank-tie → earlier insertion), D3 (full-buffer span),
    //! D4 (`frequency = 0`, `syllable_count = 1`, `is_custom` drives
    //! rank 0). Spec: `docs/engine/continuous-input-ranking.md`
    //! §10.10.
    use super::*;
    use crate::continuous::sort_key::SortKey;
    use crate::continuous::{CandidateMode, COVERAGE_KIND_FULL, COVERAGE_KIND_PARTIAL_PREFIX};

    /// Minimal non-custom `dict.bin`-shaped candidate. `bitmask` picks
    /// the source rank; all sort-noise dims are neutralized so a test
    /// isolates the dedupe / source-rank axis.
    fn dict_cand(roman: &str, hanji: Option<&str>, bitmask: u16) -> RawCandidate {
        RawCandidate {
            consumed_span: (0, 6),
            syllable_count: 1,
            display_text: hanji.unwrap_or(roman).to_owned(),
            roman: roman.to_owned(),
            hanji: hanji.map(str::to_owned),
            canonical_tl: roman.to_owned(),
            score: 1.0,
            form: FORM_NOTONE,
            frequency: 100,
            bitmask,
            mode: derive_mode(hanji),
            user_weight: 0.0,
            coverage_kind: COVERAGE_KIND_FULL,
            is_custom: false,
        }
    }

    #[test]
    fn custom_entry_to_candidate_hant_shape() {
        // D3 + D4: full-buffer span, freq 0, syll 1, is_custom true,
        // coverage_kind passed through, display = hanji, mode HANT.
        // `input_mode = Tl` for hanji-present cases — canonicalize
        // path doesn't execute (hanji is always preferred).
        let c = custom_entry_to_candidate(
            &CustomEntry {
                roman: "tâi-gí".to_owned(),
                hanji: Some("台語".to_owned()),
            },
            9,
            &FrequencyMap::new(),
            0,
            COVERAGE_KIND_FULL,
            phonetics::InputMode::Tl,
        );
        assert!(c.is_custom);
        assert_eq!(c.consumed_span, (0, 9));
        assert_eq!(c.frequency, 0);
        assert_eq!(c.syllable_count, 1);
        assert_eq!(c.roman, "tâi-gí");
        assert_eq!(c.hanji.as_deref(), Some("台語"));
        assert_eq!(c.display_text, "台語");
        // R2 (Codex BLOCK): identity sidechannel populated for
        // hanji-PRESENT custom entries too — `(hanji, canonical-TL)` is
        // the word identity, NOT gated on hanji absence.
        assert_eq!(c.canonical_tl, "tâi-gí");
        assert_eq!(c.mode, CandidateMode::Hant);
        assert_eq!(c.coverage_kind, COVERAGE_KIND_FULL);
    }

    #[test]
    fn custom_entry_to_candidate_tailo_when_no_hanji() {
        // hanji None → display falls back to `canonical_tl_form(roman,
        // mode)`. Fixture uses canonical-TL `guá` so the fold is
        // observably identity (TL form is idempotent through the
        // rewrite chain). Mode TAILO; partial-prefix coverage
        // honored (D6).
        let c = custom_entry_to_candidate(
            &CustomEntry {
                roman: "gu\u{00e1}".to_owned(),
                hanji: None,
            },
            3,
            &FrequencyMap::new(),
            0,
            COVERAGE_KIND_PARTIAL_PREFIX,
            phonetics::InputMode::Tl,
        );
        assert_eq!(c.display_text, "gu\u{00e1}");
        assert_eq!(c.hanji, None);
        assert_eq!(c.mode, CandidateMode::Tailo);
        assert_eq!(c.coverage_kind, COVERAGE_KIND_PARTIAL_PREFIX);
        assert!(c.is_custom);
    }

    #[test]
    fn custom_entry_to_candidate_poj_form_in_tl_mode_folds_canonical_tl() {
        // PR #310 r3278520895 (Codex bot P2): a POJ-form custom entry
        // (`góa`, hanji absent) loaded while the active input mode
        // is `Tl` must still fold to canonical TL `guá` for the
        // `display_text` commit key — otherwise the same word coming
        // through `dict.bin` (writing `record.tl = "guá"`) keys into
        // a different `user_frequency.db` bucket and learning splits
        // cross-mode.
        let c = custom_entry_to_candidate(
            &CustomEntry {
                roman: "g\u{00f3}a".to_owned(), // POJ display form
                hanji: None,
            },
            3,
            &FrequencyMap::new(),
            0,
            COVERAGE_KIND_FULL,
            phonetics::InputMode::Tl,
        );
        assert_eq!(c.roman, "g\u{00f3}a", "roman must stay raw");
        assert_eq!(
            c.display_text, "gu\u{00e1}",
            "display_text must fold to canonical TL even in Tl mode"
        );
        // R2: identity sidechannel folds the raw POJ-form roman to
        // canonical TL — NOT the raw `góa`. This is the TL the NextWord
        // association learns, matching a dict.bin commit's `record.tl`.
        assert_eq!(c.canonical_tl, "gu\u{00e1}");
    }

    #[test]
    fn custom_entry_to_candidate_poj_form_roman_folds_to_canonical_tl_display() {
        // v3.5.9 B-4 (Codex pre-impl BLOCK #1 corrected to α''): a
        // hanji-absent custom entry stored in POJ display form
        // (`góa`, the POJ ASCII used in custom_dictionary.db when the
        // user typed in POJ mode) must surface a TL canonical
        // `display_text` so the `user_frequency.db` commit key
        // collides with the TL-mode equivalent. `roman` itself is left
        // raw — see `composing::shadow::custom_toneless_key`, which
        // needs the POJ-form roman to match POJ lattice keys.
        let c = custom_entry_to_candidate(
            &CustomEntry {
                roman: "g\u{00f3}a".to_owned(), // POJ `góa` (TL would be `guá`)
                hanji: None,
            },
            3,
            &FrequencyMap::new(),
            0,
            COVERAGE_KIND_FULL,
            phonetics::InputMode::Poj,
        );
        assert_eq!(c.roman, "g\u{00f3}a", "roman must stay raw POJ form");
        assert_eq!(
            c.display_text, "gu\u{00e1}",
            "display_text must be canonical TL `guá` for cross-mode freq-key collision"
        );
        assert_eq!(c.hanji, None);
    }

    #[test]
    fn dedupe_custom_wins_over_dict_collision() {
        // D2: same `(roman, hanji)` from a high-freq `dict.bin` entry
        // (kautian, rank 1) and a custom entry (rank 0). The custom
        // survivor wins regardless of the dict entry's higher freq /
        // earlier insertion.
        let mut out = vec![
            dict_cand("tâi-gí", Some("台語"), 1 << 0), // kautian, rank 1
            custom_entry_to_candidate(
                &CustomEntry {
                    roman: "tâi-gí".to_owned(),
                    hanji: Some("台語".to_owned()),
                },
                6,
                &FrequencyMap::new(),
                0,
                COVERAGE_KIND_FULL,
                phonetics::InputMode::Tl,
            ),
        ];
        dedupe_by_roman_hanji_span(&mut out);
        assert_eq!(out.len(), 1, "collision must collapse to one");
        assert!(out[0].is_custom, "custom (rank 0) must win the collision");
    }

    #[test]
    fn dedupe_dual_key_preserves_roman_variants() {
        // D1: same hanji, different roman → distinct `(roman, hanji)`
        // keys, both survive (single `display_text` key would wrongly
        // collapse them).
        let mut out = vec![
            dict_cand("tâi-gí", Some("台語"), 1 << 0),
            dict_cand("tâi-gír", Some("台語"), 1 << 0),
        ];
        dedupe_by_roman_hanji_span(&mut out);
        assert_eq!(
            out.len(),
            2,
            "roman variants of same hanji must both survive"
        );
    }

    #[test]
    fn dedupe_rank_tie_keeps_earlier_insertion_and_order() {
        // D2 tie-break: two same-rank non-custom collisions keep the
        // earlier-inserted one; unrelated entries keep insertion order
        // so the downstream `SortKey.stable_idx` stays deterministic.
        let mut first = dict_cand("a", Some("甲"), 1 << 0);
        first.frequency = 10; // earlier insertion, lower freq
        let mut second = dict_cand("a", Some("甲"), 1 << 0);
        second.frequency = 999; // later insertion, higher freq — must lose
        let other = dict_cand("b", Some("乙"), 1 << 0);
        let mut out = vec![first, other.clone(), second];
        dedupe_by_roman_hanji_span(&mut out);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].frequency, 10, "rank tie keeps earlier insertion");
        assert_eq!(out[1].roman, "b", "non-duplicate keeps its position");
    }

    #[test]
    fn dedupe_noop_when_no_duplicates() {
        let mut out = vec![
            dict_cand("a", Some("甲"), 1 << 0),
            dict_cand("b", Some("乙"), 1 << 0),
        ];
        dedupe_by_roman_hanji_span(&mut out);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn dedupe_span_aware_keeps_same_word_at_different_spans() {
        // v3.5.8 S2 (Codex pre-impl Q1d): the same `(roman, hanji)` at
        // DIFFERENT `consumed_span`s must both survive — once the
        // whole-sentence walker / path-step candidates exist the same
        // word legitimately recurs at different spans. The pre-S2
        // `(roman, hanji)`-only key would have wrongly collapsed these.
        let mut a = dict_cand("tâi", Some("台"), 1 << 0);
        a.consumed_span = (0, 3);
        let mut b = dict_cand("tâi", Some("台"), 1 << 0);
        b.consumed_span = (6, 9);
        let mut out = vec![a, b];
        dedupe_by_roman_hanji_span(&mut out);
        assert_eq!(
            out.len(),
            2,
            "same (roman,hanji) at different spans must both survive"
        );
    }

    #[test]
    fn dedupe_span_aware_still_collapses_custom_vs_dict_at_full_buffer() {
        // Regression guard for Codex Q1d: the Item-12 custom-vs-`dict.bin`
        // collapse must NOT regress under the span-augmented key. Both
        // are emitted at the SAME full-buffer span `(0, raw_len)` in
        // production (`fetch_candidates_for_keys_with_barriers`), so the triple key
        // still collides and custom (rank 0) still wins.
        let mut dict = dict_cand("tâi-gí", Some("台語"), 1 << 0);
        dict.consumed_span = (0, 6);
        let custom = custom_entry_to_candidate(
            &CustomEntry {
                roman: "tâi-gí".to_owned(),
                hanji: Some("台語".to_owned()),
            },
            6,
            &FrequencyMap::new(),
            0,
            COVERAGE_KIND_FULL,
            phonetics::InputMode::Tl,
        );
        let mut out = vec![dict, custom];
        dedupe_by_roman_hanji_span(&mut out);
        assert_eq!(out.len(), 1, "same-span custom/dict collision collapses");
        assert!(out[0].is_custom, "custom (rank 0) still wins the collision");
    }

    #[test]
    fn is_custom_forces_source_rank_zero_in_sortkey() {
        // The `is_custom` axis must reach `SortKey` via
        // `source_tier_rank(bitmask, is_custom)` — a custom candidate
        // (no source bits) sorts ahead of a default-source dict
        // candidate when every prior dim is equal.
        let custom = custom_entry_to_candidate(
            &CustomEntry {
                roman: "x".to_owned(),
                hanji: Some("某".to_owned()),
            },
            3,
            &FrequencyMap::new(),
            0,
            COVERAGE_KIND_FULL,
            phonetics::InputMode::Tl,
        );
        // Default-source dict candidate: same span (tier 0), same
        // score/freq/recency so only `source_rank` differs.
        let mut dict = dict_cand("y", Some("乙"), 0); // no known source bit → rank 5
        dict.consumed_span = (0, 3);
        dict.frequency = 0;
        dict.score = custom.score;
        let k_custom = SortKey::new(&custom, 3, 0);
        let k_dict = SortKey::new(&dict, 3, 1);
        assert!(
            k_custom < k_dict,
            "is_custom → source_rank 0 must outrank default source rank"
        );
    }
}
