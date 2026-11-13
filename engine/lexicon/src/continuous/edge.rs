//! Walker edge lookups: the single best candidate and the homophone words for
//! one exact key.

use crate::dictionary_reader::{Filter, WALKER_COST_UNPRICED};
use ranking::CandidateSortKey;

use super::candidate::{
    custom_entry_to_candidate, dedupe_by_roman_hanji_span, learned_entry_to_candidate,
};
use super::exact::exact_candidates_for_key;
use super::tone_pin::TonePin;
use super::{
    ConsumedSpan, ContinuousFetchCtx, CustomEntry, LearnedEntry, RawCandidate, COVERAGE_KIND_FULL,
};

/// v3.5.8 S2 — single best dictionary candidate for one exact FST
/// key. Returns the [`record_to_candidate`] over
/// `prefix_index.lookup_exact(key)` that [`pick_edge_word`] chooses
/// (user-selected word, then the corpus `walker_cost`, then the dictionary
/// score; NaN coerced low via `CandidateSortKey`; ties keep the first
/// inserted row for determinism) together with the key's
/// min walker cost ([`EdgeBest::span_walker_cost`]), or `None` when the
/// key has no dict hit. PR-9.6 — the walker reads the
/// SAME `ctx.enabled_sources_bitmask` filter the span-local path
/// applies (`composing::continuous::assemble_candidates` builds one
/// [`ContinuousFetchCtx`] for both), so a whole-sentence parse never
/// re-surfaces a word whose only source the user toggled off. Only the
/// lookup fields of `ctx` are read (`prefix_index` / `dict` /
/// `freq_map` / `now_ms` / `enabled_sources_bitmask`); `custom` /
/// `mode` / `tone_pin` belong to the whole-buffer merge — this edge's
/// own pin arrives as `tone_pin`.
///
/// The whole-sentence walker (`composing::lattice::walker`) calls
/// this once per lattice edge through a dispatch-injected edge
/// provider so the walker stays pure + shadow-space native and the
/// lexicon candidate construction is **reused, not duplicated**
/// (Codex pre-impl S2 Q1b, 2026-05-16). `consumed_span` is stamped
/// onto the returned candidate verbatim; the walker reads `roman` /
/// `hanji` / `syllable_count` / `user_weight` off the candidate and
/// prices the edge on [`EdgeBest::span_walker_cost`].
///
/// `tps_final_only` is the §35 barrier restriction for this edge's key
/// (byte offsets of Final-only pattern slots, family prefix included).
/// The walker resolves multi-syllable edges that may span a stripped
/// separator, so it needs the same restriction the span-local fetch
/// gets — without it a walker edge could re-read a separator-closed
/// coda as the next syllable's onset. Empty slice = no barriers.
///
/// `custom` and `learned` are the user rows the caller matched to this
/// edge's key and tone pin. They join the dictionary homophones in ONE pick,
/// after the same `(reading, hanji, span)` dedupe the candidate list runs —
/// so the edge's word is the list's own order (USER 2026-10-16: a
/// never-selected custom 屢 `lí` no longer takes the `li2` edge from 你 the
/// user picks, nor from a custom 李 the user picks).
///
/// [`record_to_candidate`]: super::candidate::record_to_candidate
pub fn best_candidate_for_key_with_barriers(
    key: &str,
    tps_final_only: &[usize],
    tone_pin: &TonePin,
    consumed_span: ConsumedSpan,
    custom: &[&CustomEntry],
    learned: &[&LearnedEntry],
    ctx: &ContinuousFetchCtx<'_>,
) -> Option<EdgeBest> {
    let filter = Filter::from_enabled_bitmask(ctx.enabled_sources_bitmask);
    // `span_walker_cost` is the dictionary's own minimum — custom and
    // learned rows join the pick below but carry no corpus probability.
    let mut span_walker_cost = WALKER_COST_UNPRICED;
    let mut homophones: Vec<RawCandidate> = Vec::new();
    exact_candidates_for_key(
        key,
        tps_final_only,
        tone_pin,
        consumed_span,
        &filter,
        ctx,
        |cand| {
            span_walker_cost = span_walker_cost.min(cand.walker_cost);
            homophones.push(cand);
        },
    );
    // Custom rows, then learned phrases (§50). Both join the SAME pick with no corpus cost ([`WALKER_COST_UNPRICED`]) and
    // `frequency = 0`: a never-selected custom row leads the never-selected
    // dictionary homophones (`preference_class`), a learned row wins only
    // on the user's `user_frequency` or the previous word's context, and
    // either is the edge when the dictionary has nothing under the key.
    for entry in custom {
        homophones.push(custom_entry_to_candidate(
            entry,
            consumed_span,
            ctx.freq_map,
            ctx.now_ms,
            ctx.context,
            COVERAGE_KIND_FULL,
            ctx.mode,
        ));
    }
    for entry in learned {
        homophones.push(learned_entry_to_candidate(
            entry,
            consumed_span,
            ctx.freq_map,
            ctx.now_ms,
            ctx.context,
            COVERAGE_KIND_FULL,
        ));
    }
    // A custom row repeating a dictionary word replaces it, as in the list,
    // so slot 0 and the list show the same copy. A learned twin needs no
    // dedupe — it shares its dictionary row's weight and context and is
    // unpriced, so the dictionary row wins the pick anyway — and skipping it
    // keeps the per-edge hot path allocation-free.
    if !custom.is_empty() {
        dedupe_by_roman_hanji_span(&mut homophones);
    }
    let (candidate, bigram_rowid) =
        pick_edge_word(homophones, consumed_span.1, ctx.context.is_empty())?;
    Some(EdgeBest {
        candidate,
        span_walker_cost,
        bigram_rowid,
    })
}

/// The edge's word among its homophones: the [`CandidateSortKey`] the
/// candidate list sorts by (E1 P5a / P5b) — a selected word first, then a
/// context hit, then the word the corpus writes most (`walker_cost`), with the
/// dictionary score / freq / source dims only breaking an equal cost. Every homophone
/// shares `coverage_kind` / `consumed_span`, so `raw_len = consumed_span.1`
/// pins `tier` equal; the insertion index breaks a full tie.
///
/// With a previous-word context (§56) the pick may change the WORD but never
/// the edge's COST, so segmentation is frozen: the context-free winner is
/// found first, and the contextual winner is chosen only among homophones
/// with its `syllable_count`. `context_rank` sits below `user_weight` in the
/// key, so the winner keeps the context-free `user_weight` (the span max) —
/// and `EdgeBest::span_walker_cost` is the key min regardless of the pick —
/// which are the pick's only inputs to `lattice::edge_cost`. The walker
/// bigram prices the context-free winner too: it comes back beside the pick
/// as its rowid ([`EdgeBest::bigram_rowid`]).
fn pick_edge_word(
    homophones: Vec<RawCandidate>,
    raw_len: u32,
    context_free: bool,
) -> Option<(RawCandidate, u32)> {
    let edge_key = |i: usize, cand: &RawCandidate, with_context: bool| {
        let facts = cand.rank_facts();
        if with_context {
            CandidateSortKey::new(&facts, raw_len, i as u32)
        } else {
            CandidateSortKey::without_context(&facts, raw_len, i as u32)
        }
    };
    let (baseline_index, syllable_count) = homophones
        .iter()
        .enumerate()
        .min_by_key(|(i, homophone)| edge_key(*i, homophone, false))
        .map(|(i, cand)| (i, cand.syllable_count))?;
    let index = if context_free {
        baseline_index
    } else {
        homophones
            .iter()
            .enumerate()
            .filter(|(_, cand)| cand.syllable_count == syllable_count)
            .min_by_key(|(i, homophone)| edge_key(*i, homophone, true))
            .map(|(i, _)| i)
            .unwrap_or(baseline_index)
    };
    let bigram_rowid = homophones[baseline_index].rowid;
    Some((homophones.into_iter().nth(index)?, bigram_rowid))
}

/// The words [`best_candidate_for_key_with_barriers`] picks the edge's word
/// among — every dictionary row under `key` that passes the edge's source,
/// tone-pin and barrier filters — as their `display_text`, the word half of
/// the `(display_text, canonical_tl)` pair user frequency is keyed on. A
/// Hanji conversion reads the user's counts for these before its walk.
pub fn homophone_words_for_key(
    key: &str,
    tps_final_only: &[usize],
    tone_pin: &TonePin,
    consumed_span: ConsumedSpan,
    ctx: &ContinuousFetchCtx<'_>,
) -> Vec<String> {
    let filter = Filter::from_enabled_bitmask(ctx.enabled_sources_bitmask);
    let mut words = Vec::new();
    exact_candidates_for_key(
        key,
        tps_final_only,
        tone_pin,
        consumed_span,
        &filter,
        ctx,
        |cand| words.push(cand.display_text),
    );
    words
}

/// One lattice edge's word, as picked by
/// [`best_candidate_for_key_with_barriers`], plus the evidence that the
/// edge's span IS a dictionary word.
#[derive(Debug)]
pub struct EdgeBest {
    /// The homophone the user sees for this edge — user-selected word
    /// first, then the previous word's context, then the corpus
    /// `walker_cost` ([`pick_edge_word`]).
    pub candidate: RawCandidate,
    /// Lowest `dict.bin` `walker_cost` among the key's homophones that
    /// pass the edge's source / tone-pin / barrier filters — not the
    /// picked word's own (E1 D3). The walker prices the edge's
    /// segmentation on this so a rarer preferred homophone does not price
    /// its span out of the best path: "is this span a word" is decoupled
    /// from "which word". (`candidate.user_weight` is likewise the span's
    /// max, since user weight is the leading pick dimension.)
    /// [`WALKER_COST_UNPRICED`] when only user rows sit under the key.
    /// Rationale + numbers: `docs/engine/continuous-input-ranking.md` §3.2.
    pub span_walker_cost: u16,
    /// Rowid of the context-free winner — the word the walker bigram prices
    /// for this edge (`docs/architecture/bigram-lm-roadmap.md` D6). Equal to
    /// `candidate.rowid` without a previous-word context; with one (§56) the
    /// shown word may differ, but the bigram still prices the context-free
    /// word, so the context never moves the segmentation.
    pub bigram_rowid: u32,
}

#[cfg(test)]
mod edge_word_pick_tests {
    use super::*;
    use ranking::{CONTEXT_RANK_BUNDLED, CONTEXT_RANK_NONE, CONTEXT_RANK_USER};

    fn homophone(
        hanji: &str,
        frequency: u32,
        syllable_count: u8,
        user_weight: f64,
        context_rank: u8,
    ) -> RawCandidate {
        RawCandidate {
            rowid: crate::dictionary_reader::NO_ROWID,
            consumed_span: (0, 3),
            syllable_count,
            display_text: hanji.to_owned(),
            roman: "tse".to_owned(),
            hanji: Some(hanji.to_owned()),
            canonical_tl: "tse".to_owned(),
            score: frequency as f32,
            frequency,
            walker_cost: EQUAL_COST,
            bitmask: 0,
            user_weight,
            context_rank,
            coverage_kind: COVERAGE_KIND_FULL,
            is_custom: false,
        }
    }

    /// A typical seen word's corpus cost; the tests that do not exercise the
    /// corpus dimension give every homophone this one.
    const EQUAL_COST: u16 = 7_000;

    fn costed(candidate: RawCandidate, walker_cost: u16) -> RawCandidate {
        RawCandidate {
            walker_cost,
            ..candidate
        }
    }

    fn picked(homophones: Vec<RawCandidate>, context_free: bool) -> Option<String> {
        pick_edge_word(homophones, 3, context_free).map(|(c, _)| c.display_text)
    }

    // E1 P5a: the word the corpus writes most fills the edge, whatever the
    // dictionary frequency (production `koh`: 擱 and 閣 share frequency
    // 29,890; `kap`: 甲 outranks 佮 on source rank).
    #[test]
    fn corpus_cost_takes_the_word_over_frequency() {
        let key = vec![
            costed(homophone("擱", 29_890, 1, 0.0, CONTEXT_RANK_NONE), 9_000),
            costed(homophone("閣", 29_890, 1, 0.0, CONTEXT_RANK_NONE), 6_000),
            costed(homophone("故", 40_000, 1, 0.0, CONTEXT_RANK_NONE), 8_000),
        ];
        assert_eq!(picked(key, true).as_deref(), Some("閣"));
    }

    // E1 P5a: an equal cost (corpus-unseen words share one smoothed cost)
    // leaves the pick to the dictionary score, as before P5a.
    #[test]
    fn equal_cost_falls_to_the_dictionary_score() {
        let key = vec![
            costed(homophone("狗巢", 25, 2, 0.0, CONTEXT_RANK_NONE), 13_180),
            costed(homophone("狗岫", 50, 2, 0.0, CONTEXT_RANK_NONE), 13_180),
        ];
        assert_eq!(picked(key, true).as_deref(), Some("狗岫"));
    }

    // A selected word still leads the corpus: user weight is the first
    // dimension of the pick.
    #[test]
    fn selected_word_beats_the_corpus_choice() {
        let key = vec![
            costed(homophone("教授", 10, 2, 0.0, CONTEXT_RANK_NONE), 7_000),
            costed(homophone("狗岫", 25, 2, 0.2, CONTEXT_RANK_NONE), 13_180),
        ];
        assert_eq!(picked(key, true).as_deref(), Some("狗岫"));
    }

    // Learned phrases (§50) carry no corpus cost: a dictionary homophone
    // wins unless the user's weight or the previous word's context favours
    // the learned row, and a learned row alone is the edge.
    #[test]
    fn learned_row_competes_through_user_weight_and_context_only() {
        let dictionary = || costed(homophone("這", 1000, 1, 0.0, CONTEXT_RANK_NONE), 7_000);
        let learned = |user_weight, context_rank| {
            costed(
                homophone("姊", 0, 1, user_weight, context_rank),
                WALKER_COST_UNPRICED,
            )
        };
        assert_eq!(
            picked(vec![learned(0.0, CONTEXT_RANK_NONE), dictionary()], true).as_deref(),
            Some("這")
        );
        assert_eq!(
            picked(vec![dictionary(), learned(0.3, CONTEXT_RANK_NONE)], true).as_deref(),
            Some("姊")
        );
        assert_eq!(
            picked(vec![dictionary(), learned(0.0, CONTEXT_RANK_USER)], false).as_deref(),
            Some("姊")
        );
        assert_eq!(
            picked(vec![learned(0.0, CONTEXT_RANK_NONE)], true).as_deref(),
            Some("姊")
        );
    }

    // INVARIANT_CONTINUOUS_CONTEXT_RERANK (§56): a context hit takes the
    // edge's word; with every corpus cost equal here, the context-free pick
    // is the frequency order.
    #[test]
    fn context_hit_takes_the_word() {
        let key = || {
            vec![
                homophone("這", 1000, 1, 0.0, CONTEXT_RANK_NONE),
                homophone("濟", 100, 1, 0.0, CONTEXT_RANK_BUNDLED),
                homophone("姊", 10, 1, 0.0, CONTEXT_RANK_USER),
            ]
        };
        assert_eq!(picked(key(), true).as_deref(), Some("這"));
        assert_eq!(picked(key(), false).as_deref(), Some("姊"));
    }

    // INVARIANT_CONTINUOUS_CONTEXT_RERANK: the pick never changes the edge's
    // cost inputs — a hit with another syllable count or a lower user weight
    // than the context-free winner cannot take the edge.
    #[test]
    fn context_hit_keeps_syllable_count_and_user_weight() {
        let two_syllables = vec![
            homophone("這", 1000, 1, 0.0, CONTEXT_RANK_NONE),
            homophone("濟濟", 100, 2, 0.0, CONTEXT_RANK_USER),
        ];
        assert_eq!(picked(two_syllables, false).as_deref(), Some("這"));
        let selected_stranger = vec![
            homophone("這", 1000, 1, 0.5, CONTEXT_RANK_NONE),
            homophone("濟", 100, 1, 0.0, CONTEXT_RANK_USER),
        ];
        assert_eq!(picked(selected_stranger, false).as_deref(), Some("這"));
        let hit_among_equals = vec![
            homophone("這", 1000, 1, 0.5, CONTEXT_RANK_NONE),
            homophone("濟", 100, 1, 0.5, CONTEXT_RANK_USER),
        ];
        assert_eq!(picked(hit_among_equals, false).as_deref(), Some("濟"));
    }

    // INVARIANT_CONTINUOUS_CONTEXT_RERANK with corpus costs: the context-free
    // winner is now the cheapest word, and the contextual pick keeps its
    // `syllable_count` and `user_weight` — with `span_walker_cost` taken
    // over the key regardless of the pick, every `lattice::edge_cost` input
    // is the same with and without the context.
    #[test]
    fn context_hit_keeps_the_corpus_winners_cost_inputs() {
        let key = || {
            [
                costed(homophone("這", 1000, 1, 0.0, CONTEXT_RANK_NONE), 7_000),
                costed(homophone("這兜", 10, 2, 0.0, CONTEXT_RANK_NONE), 5_000),
                costed(homophone("濟", 100, 1, 0.0, CONTEXT_RANK_USER), 9_000),
                costed(homophone("姊妹", 1, 2, 0.0, CONTEXT_RANK_BUNDLED), 12_000),
            ]
            .into_iter()
            .zip(1..)
            .map(|(homophone, rowid)| RawCandidate { rowid, ..homophone })
            .collect::<Vec<_>>()
        };
        let (context_free, free_rowid) = pick_edge_word(key(), 3, true).expect("a word");
        let (contextual, bigram_rowid) = pick_edge_word(key(), 3, false).expect("a word");
        assert_eq!(context_free.display_text, "這兜");
        assert_eq!(contextual.display_text, "姊妹");
        assert_eq!(contextual.syllable_count, context_free.syllable_count);
        assert_eq!(contextual.user_weight, context_free.user_weight);
        // The walker bigram prices 這兜 (rowid 2) under either pick.
        assert_eq!((free_rowid, bigram_rowid), (2, 2));
    }

    #[test]
    fn no_homophone_no_word() {
        assert_eq!(picked(Vec::new(), false), None);
    }
}
