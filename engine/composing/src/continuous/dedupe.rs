//! Candidate dedupes: after a render pass, across fetch batches, and the
//! TPS same-Hanji collapse.

use lexicon::{ConsumedSpan, RawCandidate};

/// v3.5.8 — collapse continuous candidates that became identical only
/// after a render pass (POJ render, Syllable Separator, typed separators).
/// The pre-render dedupe (`lexicon::dedupe_by_roman_hanji_span`) collapses
/// `(hanji, consumed_span)` + (`roman` or the word identity `canonical_tl`)
/// (2026-10-07),
/// so a hanji-bearing custom entry stored in POJ display form
/// (`roman = "gô͘"`) and a `dict.bin` entry in TL (`roman = "gôo"`) for
/// the same 吳 at one span already collapse there. Two DIFFERENT
/// identities can still render to one visible string — e.g. two
/// separator forms of one reading that a Syllable Separator setting
/// draws identically — and this pass collapses that visible duplicate.
/// The `HashSet::retain` first-wins rule keeps whichever row sits earlier
/// in the **current candidate vector** at the moment this fn runs
/// (post-sort, post whole-sentence prepend), so the prepended
/// best-walk-path candidate at index 0 is never dropped. It is
/// behavior-neutral whenever `hanji` is `Some`: the collision key
/// pins the same hanji and `display_text` (the commit /
/// `user_frequency.db` key) is that hanji for BOTH the custom and the
/// `dict.bin` candidate, so which row survives cannot change what
/// commits.
///
/// B-4 deliberately leaves `entry.roman` raw (the walker /
/// `custom_toneless_key` need it in the user's native form so POJ-family
/// lattice keys match against POJ-form custom roman per
/// `composing::shadow::custom_toneless_key`); the identity sidechannel
/// `canonical_tl` is what the pre-render dedupe compares instead. Called
/// when a render pass can make two identities look alike: POJ mode, a
/// Syllable Separator joiner, or rendered typed separators — for plain
/// TL/English the render is identity so the pre-render dedupe already
/// settled every key. TPS has its own post-render visual collapse via
/// [`dedupe_display_hanji_for_tps`] because the TPS UI hides romanization
/// entirely (hanji-only display), so two `dict.bin` rows differing only
/// in tone (e.g. `灣 / uan` vs `灣 / uân` at the same TPS-toneless key
/// `ㄨㄢ`) survive the pre-render dedupe yet render identically.
pub(super) fn dedupe_rendered_continuous(candidates: &mut Vec<RawCandidate>) {
    retain_first_by_key(candidates, |c| {
        Some((c.roman.clone(), c.hanji.clone(), c.consumed_span))
    });
}

/// Keeps the first candidate seen per `key`; a candidate whose key is `None`
/// is always kept. The one body behind every post-sort "first-seen wins"
/// pass (rendered / TPS-hanji / Romanization Only roman), so their survivor rule cannot
/// drift.
pub(crate) fn retain_first_by_key<K: std::hash::Hash + Eq>(
    candidates: &mut Vec<RawCandidate>,
    key: impl Fn(&RawCandidate) -> Option<K>,
) {
    let mut seen = std::collections::HashSet::with_capacity(candidates.len());
    candidates.retain(|c| key(c).is_none_or(|k| seen.insert(k)));
}

/// Cross-batch exclude: drop from `batch` every candidate already in
/// `existing` — the same word identity `(canonical_tl, hanji,
/// consumed_span)` (Core Principle #6), or the same recased `(roman,
/// hanji, consumed_span)`. The per-fetch `dedupe_by_roman_hanji_span`
/// inside the lexicon does not span batches, so a dict row reachable
/// through two fetches of the same buffer (Step 4b extension vs the FULL
/// block, Step 4c abbreviation vs a partial hit) would otherwise show
/// twice. Identity catches a custom row whose stored roman differs from
/// the dict row's only in form (NFD import, POJ spelling); the recased
/// roman still catches a walker slot 0, whose `canonical_tl` went through
/// `canonical_tl_form` while a dict row's is verbatim. Borrowed keys —
/// `existing` is not mutated during the retain, so the set holds `&str`
/// slices into it and costs no per-keystroke String allocs.
pub(super) fn retain_absent_from(existing: &[RawCandidate], batch: &mut Vec<RawCandidate>) {
    if batch.is_empty() {
        return;
    }
    // One set holds both spellings of every existing row: a canonical TL
    // equal to another row's recased roman, at the same hanji and span, is
    // the same word either way.
    let seen: std::collections::HashSet<(&str, Option<&str>, ConsumedSpan)> = existing
        .iter()
        .flat_map(|x| {
            [x.canonical_tl.as_str(), x.roman.as_str()]
                .map(|reading| (reading, x.hanji.as_deref(), x.consumed_span))
        })
        .collect();
    batch.retain(|x| {
        !seen.contains(&(x.canonical_tl.as_str(), x.hanji.as_deref(), x.consumed_span))
            && !seen.contains(&(x.roman.as_str(), x.hanji.as_deref(), x.consumed_span))
    });
}

/// TPS-mode visual dedupe: collapse candidates sharing `(hanji,
/// consumed_span)` because the TPS candidate strip hides romanization
/// (TPS = hanji-first input mode per `docs/contributing/phonetics.md`).
/// Two `dict.bin` rows like `灣 / uan` (tone 1) and `灣 / uân` (tone 5)
/// both index under `tps:ㄨㄢ`, so the build pipeline legitimately
/// emits both rowids at the same FST key. The pre-sort
/// `lexicon::dedupe_by_roman_hanji_span` keys on `(hanji, span)` + (`roman`
/// or `canonical_tl`) and preserves tone variants; that is correct for TL/POJ
/// (the UI shows distinct `uan / 灣` vs `uân / 灣` rows) but produces
/// a visible duplicate in TPS mode where only `灣` is rendered.
///
/// Runs AFTER the sort + walker slot-0 prepend + (no-op for TPS) POJ
/// render pass so first-seen = highest-ranked. `consumed_span` stays
/// in the key so the same hanji at different spans survives — e.g. a
/// 1-syllable `灣` covering only the first syllable `(0, n1)` and a
/// 1-syllable `灣` re-emitted by the walker at the full-buffer span
/// `(0, raw_len)` are distinct commit surfaces; the rationale matches
/// `lexicon::dedupe_by_roman_hanji_span`. `hanji = None` or empty
/// hanji passes through (roman-only rows are unique by roman); upstream
/// `lexicon::dedupe_by_roman_hanji_span` already collapses exact
/// `(hanji, span)` + (`roman` or `canonical_tl`) duplicates so this branch
/// never reaches a truly identical pair.
pub(super) fn dedupe_display_hanji_for_tps(candidates: &mut Vec<RawCandidate>) {
    retain_first_by_key(candidates, |c| {
        c.hanji
            .as_deref()
            .filter(|h| !h.is_empty())
            .map(|h| (h.to_owned(), c.consumed_span))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use lexicon::dictionary_reader::NO_ROWID;
    use lexicon::COVERAGE_KIND_FULL;
    use ranking::WALKER_COST_UNPRICED;

    #[test]
    fn dedupe_rendered_continuous_drops_post_render_collision_keeping_first() {
        fn mk(roman: &str, hanji: Option<&str>, span: (u32, u32), is_custom: bool) -> RawCandidate {
            RawCandidate {
                rowid: NO_ROWID,
                consumed_span: span,
                syllable_count: 1,
                display_text: hanji.map(String::from).unwrap_or_else(|| roman.to_string()),
                roman: roman.to_string(),
                hanji: hanji.map(String::from),
                canonical_tl: roman.to_string(),
                score: 0.0,
                frequency: 0,
                walker_cost: WALKER_COST_UNPRICED,
                bitmask: 0,
                user_weight: 0.0,
                context_rank: ranking::CONTEXT_RANK_NONE,
                coverage_kind: COVERAGE_KIND_FULL,
                is_custom,
            }
        }
        // Post-render: a custom entry stored `gô͘` and a dict.bin entry
        // stored `gôo`; both already rendered to `gô͘` here, same hanji
        // + span ⇒ a visible duplicate. First-wins keeps index 0 so the
        // prepended whole-sentence best candidate is never dropped.
        let mut collide = vec![
            mk("gô\u{0358}", Some("鵝"), (0, 6), false), // dict, rendered
            mk("gô\u{0358}", Some("鵝"), (0, 6), true),  // custom, rendered
        ];
        dedupe_rendered_continuous(&mut collide);
        assert_eq!(collide.len(), 1);
        assert!(
            !collide[0].is_custom,
            "first-wins must keep the earlier (index-0) row"
        );
        // Distinct on ANY of (roman, hanji, consumed_span) ⇒ untouched
        // (the S2 same-word-different-span invariant survives).
        let mut keep = vec![
            mk("go\u{0358}", Some("鵝"), (0, 6), false),
            mk("go\u{0358}", Some("吳"), (0, 6), false), // different hanji
            mk("go\u{0358}", Some("鵝"), (0, 3), false), // different span
            mk("sa\u{207f}", None, (0, 6), false),       // different roman
        ];
        let before = keep.len();
        dedupe_rendered_continuous(&mut keep);
        assert_eq!(keep.len(), before, "distinct keys must all survive");
    }

    #[test]
    fn dedupe_display_hanji_for_tps_collapses_same_hanji_same_span() {
        fn mk(roman: &str, hanji: Option<&str>, span: (u32, u32)) -> RawCandidate {
            RawCandidate {
                rowid: NO_ROWID,
                consumed_span: span,
                syllable_count: 1,
                display_text: hanji.map(String::from).unwrap_or_else(|| roman.to_string()),
                roman: roman.to_string(),
                hanji: hanji.map(String::from),
                canonical_tl: roman.to_string(),
                score: 0.0,
                frequency: 0,
                walker_cost: WALKER_COST_UNPRICED,
                bitmask: 0,
                user_weight: 0.0,
                context_rank: ranking::CONTEXT_RANK_NONE,
                coverage_kind: COVERAGE_KIND_FULL,
                is_custom: false,
            }
        }
        // The reported bug: TPS `ㄨㄢ` → dict has 灣/uan (tone 1) + 灣/uân
        // (tone 5) at the same `tps:ㄨㄢ` toneless key; same hanji, same
        // span, different roman. TPS UI shows hanji only ⇒ duplicate.
        let mut visible_dup = vec![mk("uan", Some("灣"), (0, 6)), mk("uân", Some("灣"), (0, 6))];
        dedupe_display_hanji_for_tps(&mut visible_dup);
        assert_eq!(visible_dup.len(), 1);
        assert_eq!(visible_dup[0].roman, "uan", "first-wins keeps top-ranked");
    }

    #[test]
    fn dedupe_display_hanji_for_tps_keeps_distinct_spans() {
        fn mk(roman: &str, hanji: Option<&str>, span: (u32, u32)) -> RawCandidate {
            RawCandidate {
                rowid: NO_ROWID,
                consumed_span: span,
                syllable_count: 1,
                display_text: hanji.map(String::from).unwrap_or_else(|| roman.to_string()),
                roman: roman.to_string(),
                hanji: hanji.map(String::from),
                canonical_tl: roman.to_string(),
                score: 0.0,
                frequency: 0,
                walker_cost: WALKER_COST_UNPRICED,
                bitmask: 0,
                user_weight: 0.0,
                context_rank: ranking::CONTEXT_RANK_NONE,
                coverage_kind: COVERAGE_KIND_FULL,
                is_custom: false,
            }
        }
        // Same hanji at different `consumed_span` is a legitimate partial
        // vs full-buffer surface — must NOT collapse (mirrors the S2
        // invariant pinned by `lexicon::dedupe_by_roman_hanji_span`).
        let mut spans = vec![mk("uan", Some("灣"), (0, 3)), mk("uan", Some("灣"), (0, 6))];
        let before = spans.len();
        dedupe_display_hanji_for_tps(&mut spans);
        assert_eq!(
            spans.len(),
            before,
            "distinct spans must all survive (partial vs full-buffer)"
        );
    }

    #[test]
    fn dedupe_display_hanji_for_tps_passes_through_none_and_empty_hanji() {
        fn mk(roman: &str, hanji: Option<&str>, span: (u32, u32)) -> RawCandidate {
            RawCandidate {
                rowid: NO_ROWID,
                consumed_span: span,
                syllable_count: 1,
                display_text: hanji.map(String::from).unwrap_or_else(|| roman.to_string()),
                roman: roman.to_string(),
                hanji: hanji.map(String::from),
                canonical_tl: roman.to_string(),
                score: 0.0,
                frequency: 0,
                walker_cost: WALKER_COST_UNPRICED,
                bitmask: 0,
                user_weight: 0.0,
                context_rank: ranking::CONTEXT_RANK_NONE,
                coverage_kind: COVERAGE_KIND_FULL,
                is_custom: false,
            }
        }
        // Hanji-absent rows render as TPS / roman in the UI and are
        // unique by roman — always survive regardless of repetition on
        // any other axis.
        let mut roman_only = vec![
            mk("ㄉㄞ", None, (0, 3)),
            mk("ㄉㄞ", None, (0, 3)),     // duplicate roman, both kept
            mk("ㄨㄢ", Some(""), (0, 3)), // empty hanji also passes
        ];
        let before = roman_only.len();
        dedupe_display_hanji_for_tps(&mut roman_only);
        assert_eq!(roman_only.len(), before, "None / empty hanji all pass");
    }
}
