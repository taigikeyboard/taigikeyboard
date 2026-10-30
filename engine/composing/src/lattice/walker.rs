//! v3.5.8 S2/S5 — whole-sentence best-path walker (min-cost).
//!
//! Single-pass relaxation over the S1 segmentation lattice
//! (`super::Lattice`). McBopomofo Gramambular shape
//! (`references/McBopomofo/Source/Engine/gramambular2/reading_grid.cpp:132`):
//! nodes are shadow byte offsets, edges are forward-only, so the
//! natural ascending-offset order **is** a topological order — one
//! forward pass over the already-`(start,end)`-sorted edge list is a
//! valid relaxation (every edge into offset `S` has `end == S` and
//! `start < S`, so it is processed before any edge whose `start == S`).
//! No separate topological sort, no full Viterbi: O(V + E), mobile
//! budget friendly. This stays valid under min-cost — the graph is
//! still a forward-only DAG (Codex pre-impl S5 Q1, 2026-05-17).
//!
//! **S5**: the objective is now `min Σ edge_cost`
//! (`super::cost::edge_cost`), a faithful port of the khiin word-level
//! DP segmenter (`references/khiin-rs/khiin/src/data/segmenter.rs`
//! `segment_min_cost`) — the same optimum as McBopomofo's
//! `max Σ log P` relaxation. The S2/S3 `max Σ edge_score` objective
//! structurally rewarded over-segmentation (`taiuan` → `乾伊有俺`);
//! see `cost.rs` and `docs/releases/v3.5.8/plan.md` §整句 lattice + walker S5.
//!
//! The walker is **pure and shadow-space native**. It never looks at
//! the dictionary, the raw byte space, or proto types: the caller
//! (`continuous::fetch_walker_slot0_inner`, which holds `LexiconHandle`
//! state) injects an `edge_choice` provider that maps a shadow edge
//! `(start, end)` to its best content. This keeps the
//! composing↔lexicon boundary clean and reuses the lexicon candidate
//! construction instead of duplicating it (Codex pre-impl S2 Q1b).

use super::{
    cost::{edge_cost, BigramEvidence},
    Lattice,
};
use lexicon::dictionary_reader::NO_ROWID;

/// The walker bigram's pair counts (`docs/architecture/bigram-lm-roadmap.md`
/// D6), looked up by the edges' `dictionary.bin` rowids. The production
/// source is the loaded `DictionaryReader`; under an empty table every walk is
/// the unigram walk.
pub(crate) trait WalkerBigram {
    /// One previous word's continuation summary, resolved once per state.
    type Continuations;
    /// `None` when `previous_rowid`'s word has no recorded continuation.
    fn continuations(&self, previous_rowid: u32) -> Option<Self::Continuations>;
    /// The evidence pricing `next_rowid`'s word after that previous word.
    fn evidence(&self, continuations: &Self::Continuations, next_rowid: u32) -> BigramEvidence;
}

/// Content the caller resolved for one lattice edge `(start, end)`.
/// Built by the dispatch-injected provider from the best dictionary
/// candidate for the edge's toneless key, or — when the edge has no
/// dict hit — synthesized from the edge's own toneless roman so the
/// no-hanji roman path is the walker's natural best path (Bug 2 / §1
/// subsumed, not a fallback).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EdgeChoice {
    /// The `dictionary.bin` rowid the walker bigram prices this edge's word
    /// by (`EdgeBest::bigram_rowid`, the context-free winner). [`NO_ROWID`]
    /// for an OOV edge or a custom / learned winner: those take and give no
    /// bigram penalty.
    pub rowid: u32,
    /// Romanization for this edge. The synthesized candidate's roman
    /// line joins these with a single ASCII space (§10.2 segmented
    /// rule: roman line gets word-boundary spaces).
    pub roman: String,
    /// Hanji for this edge if the chosen dict candidate had one;
    /// `None` for a pure-roman (no dict hit) edge.
    pub hanji: Option<String>,
    /// The chosen word's canonical TL — the reading half of its
    /// `(Hanji, canonical-TL)` identity (`AGENTS.md` Core Principle 6). Empty
    /// for a synthesized OOV edge, which has none.
    pub canonical_tl: String,
    /// `true` iff this edge resolved to a **lexicon-backed hit**: a
    /// `dict.bin` record (provider `Some(c)` branch) or a
    /// `custom_dictionary.db` entry (v3.5.8 S6 custom-precedence
    /// branch). It is `false` ONLY for a synthesized pure-roman OOV
    /// edge. The no-dict carve-out keys off this flag and must never
    /// be re-derived from `hanji.is_none()`, `frequency == 0`, or
    /// `is_custom`, because a record (or custom entry) may be
    /// roman-only and a zero frequency is representable, so those
    /// would misclassify (Codex pre-impl S5 Q2 + S6 Q7, BLOCK). A
    /// custom edge stays `true`; reverting it to `false` would
    /// re-enable the all-OOV carve-out on a custom-only path.
    pub dict_hit: bool,
    /// v3.5.8 S6 (Codex pre-impl S6 Q4) — `true` iff the edge pick
    /// (`lexicon::best_candidate_for_key_with_barriers`) chose a
    /// `custom_dictionary.db` entry. Provenance only: propagated to the
    /// synthesized slot-0 `RawCandidate.is_custom` when ANY winning edge is
    /// custom; the walker cost objective does NOT read this (a custom edge
    /// competes via [`Self::walker_cost`], Codex S6 Q1 — NOT a cost
    /// special-case).
    pub is_custom: bool,
    /// Corpus cost (milli-nats) the edge is scored with on the **dict**
    /// branch of [`super::cost::edge_cost`]: the key's lowest
    /// `walker_cost` across its homophones
    /// (`lexicon::EdgeBest::span_walker_cost` — segmentation evidence,
    /// not the chosen word's own). [`edge_cost`] caps it at the model-derived
    /// custom-word cost when [`Self::user_row`] is set.
    /// **Unused** for a synthesized OOV edge (RC0: OOV
    /// cost is char-keyed, never corpus-priced).
    pub walker_cost: u16,
    /// A custom or learned row sits under the edge's key: the edge is priced
    /// at most at the custom-word cost — after the walker bigram's penalty —
    /// so the span the user filed or once composed as one word keeps winning
    /// the segmentation (the #69 span-vs-word decoupling).
    pub user_row: bool,
    /// Syllable count of the chosen candidate (`>= 1`). Dict/custom:
    /// the record's / greedy-longest span syllable count, drives the
    /// khiin `n_syls^0.2` bias. OOV: the real min-syllable-hop count of
    /// the span (`shadow::span_min_syllable_count`, NOT a hardcoded
    /// `1`) — RC0 metadata only (it feeds the synthesized candidate's
    /// syllable sum; the OOV *cost* ignores it).
    pub syllable_count: u8,
    /// Toneless-key char count for this edge (khiin's `word_len`). The
    /// khiin length normalization in [`edge_cost`] is not faithful
    /// without it, so it is carried explicitly rather than re-derived
    /// (Codex pre-impl S5 Q1, 2026-05-17, BLOCK).
    pub toneless_len: usize,
    /// v3.5.8 S3 — time-decayed user-frequency boost delta for this
    /// edge's chosen candidate (`ranking::decayed_user_weight_delta`,
    /// `0.0..=4.0`). `0.0` for a no-dict edge or one with no user
    /// history. Computed caller-side in `continuous::fetch_walker_slot0_inner`
    /// so the walker stays pure and shadow-space native (Codex pre-impl
    /// S3 Q4d seam). Applied as a log-space cost discount in
    /// [`edge_cost`] (Codex pre-impl S5 Q3). Closes Continuous-input
    /// Gap B → goal G2.
    pub user_weight_delta: f64,
}

impl EdgeChoice {
    /// The rowid this edge takes part in the walker bigram with, on either
    /// side of a pair; `None` for an edge outside it.
    fn bigram_word(&self) -> Option<u32> {
        (self.dict_hit && self.rowid != NO_ROWID).then_some(self.rowid)
    }
}

/// The walker's best full-buffer path. `choices[i]` is the resolved
/// content of `edges[i]`; `edges` is contiguous and covers
/// `0..shadow_len`. `cost` is the accumulated `Σ edge_cost`
/// (**lower = better**).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BestPath {
    pub edges: Vec<(usize, usize)>,
    pub choices: Vec<EdgeChoice>,
    pub cost: f64,
}

/// Relax `lattice` to the **minimum-`Σ edge_cost`** path from offset
/// `0` to `shadow_len`. Returns `None` when no edge chain spans the
/// whole buffer (e.g. a sub-syllable partial-prefix buffer) — the
/// caller then leaves the existing span-local list untouched (no
/// slot-0 synthesis), preserving pre-S2 behavior.
/// `custom_word_cost` is priced by the caller's loaded dictionary model;
/// the walker uses that same value for every selected-word cap.
///
/// `edge_choice(start, end)` returns the resolved content for that
/// shadow edge, or `None` to drop the edge from consideration (e.g.
/// an empty toneless key). An edge dropped here simply does not relax;
/// the buffer can still be spanned via other edges.
///
/// **State = `(offset, incoming edge)`** (D6): an edge's cost depends on
/// the previous edge's pick through `bigram`, so each edge into an offset
/// keeps its own best prefix instead of one node per offset. Each edge
/// still has the one pick `edge_choice` resolves (no top-N alternatives).
/// Under an empty table an edge's cost no longer depends on the prefix, so
/// the best state at every offset is the old per-offset node and the walk
/// returns the same path, choices and cost as the unigram walker.
///
/// Tie handling: among paths of equal accumulated cost the first one
/// reached in the ascending-`(start, end)` edge order wins — a state's
/// prefix is the first minimum among the states at its start, and the
/// sink is the first minimum among the states at `shadow_len`. `lattice.edges()` is
/// sorted, so this is deterministic. No edge-count tiebreak: the S2
/// "more edges wins" lever existed only to force the no-dict
/// per-syllable split, which S5 moves to the explicit
/// `continuous::fetch_walker_slot0_inner` carve-out — keeping it here would
/// re-introduce the over-segmentation pressure min-cost exists to
/// remove (Codex pre-impl S5 Q5, 2026-05-17).
pub(crate) fn walk_best<B: WalkerBigram>(
    lattice: &Lattice,
    shadow_len: usize,
    custom_word_cost: u16,
    bigram: &B,
    mut edge_choice: impl FnMut(usize, usize) -> Option<EdgeChoice>,
) -> Option<BestPath> {
    if shadow_len == 0 {
        return None;
    }

    // One state per edge that reached its end offset. `states[0]` is the
    // root (offset 0, no edge); `by_offset[offset]` lists the states at
    // `offset` in edge order, which is the tie order. An empty offset is
    // "unreached", `+∞` for the relaxation.
    struct State<C> {
        cost: f64,
        start: usize,
        // Index of the prefix's last state; `None` for the root.
        prev: Option<usize>,
        choice: Option<EdgeChoice>,
        // The pick's continuations, resolved once for every edge leaving it.
        continuations: Option<C>,
    }
    let mut states: Vec<State<B::Continuations>> = vec![State {
        cost: 0.0,
        start: 0,
        prev: None,
        choice: None,
        continuations: None,
    }];
    let mut by_offset: Vec<Vec<usize>> = vec![Vec::new(); shadow_len + 1];
    by_offset[0].push(0);

    // `lattice.edges()` is sorted ascending by `(start, end)`. Because
    // edges are forward-only (`start < end`), every edge feeding
    // offset `S` has `end == S` with `start < S` and therefore sorts
    // before any edge whose `start == S` — so every state at `S` exists
    // before the first edge leaving `S` relaxes, without a separate
    // topological sort.
    for &(start, end) in lattice.edges() {
        let Some(incoming) = by_offset.get(start).filter(|states| !states.is_empty()) else {
            continue; // `start` not yet reachable from 0.
        };
        let Some(choice) = edge_choice(start, end) else {
            continue; // caller dropped this edge.
        };
        // Replace only on a strictly lower cost, so on an exact tie the
        // first state at `start` (first-reached prefix) is kept.
        let mut best: Option<(f64, usize)> = None;
        for &index in incoming {
            let state = &states[index];
            let evidence = state
                .continuations
                .as_ref()
                .zip(choice.bigram_word())
                .map(|(continuations, rowid)| bigram.evidence(continuations, rowid));
            // OOV pricing is selected from the explicit `dict_hit` flag, never
            // `frequency == 0` (Codex pre-impl Q2).
            let cost = state.cost + edge_cost(&choice, custom_word_cost, evidence.as_ref());
            if best.is_none_or(|(best_cost, _)| cost < best_cost) {
                best = Some((cost, index));
            }
        }
        let Some((cost, prev)) = best else {
            continue;
        };
        let Some(arrivals) = by_offset.get_mut(end) else {
            continue; // an edge past the buffer reaches no path.
        };
        arrivals.push(states.len());
        let continuations = choice
            .bigram_word()
            .and_then(|rowid| bigram.continuations(rowid));
        states.push(State {
            cost,
            start,
            prev: Some(prev),
            choice: Some(choice),
            continuations,
        });
    }

    // No edge chain reached the buffer end → no full-buffer path.
    // `min_by` keeps the first of equal minimums: the first-reached path.
    let mut cur = by_offset[shadow_len]
        .iter()
        .copied()
        .min_by(|&a, &b| states[a].cost.total_cmp(&states[b].cost))?;
    let total = states[cur].cost;

    // Reconstruct back to the root via `prev` links.
    let mut edges_rev: Vec<(usize, usize)> = Vec::new();
    let mut choices_rev: Vec<EdgeChoice> = Vec::new();
    let mut end = shadow_len;
    while let Some(prev) = states[cur].prev {
        let state = &mut states[cur];
        edges_rev.push((state.start, end));
        choices_rev.push(state.choice.take()?);
        end = state.start;
        cur = prev;
    }
    edges_rev.reverse();
    choices_rev.reverse();

    Some(BestPath {
        edges: edges_rev,
        choices: choices_rev,
        cost: total,
    })
}

#[cfg(test)]
mod tests {
    const FIXTURE_CUSTOM_WORD_COST: u16 = 7_879;

    use super::*;

    /// The unigram walk: `walk_best` under an empty bigram table.
    fn walk(
        lat: &Lattice,
        shadow_len: usize,
        edge_choice: impl FnMut(usize, usize) -> Option<EdgeChoice>,
    ) -> Option<BestPath> {
        walk_best(
            lat,
            shadow_len,
            FIXTURE_CUSTOM_WORD_COST,
            &FakeBigram::default(),
            edge_choice,
        )
    }

    /// A dictionary edge priced at `walker_cost` milli-nats. Test values
    /// are production `dictionary.bin` costs (E1 P2b, 2026-10-08).
    fn dict(roman: &str, hanji: &str, walker_cost: u16, syll: u8, len: usize) -> EdgeChoice {
        dict_u(roman, hanji, walker_cost, syll, len, 0.0)
    }
    /// `dict` with an explicit S3 decayed user-weight delta.
    fn dict_u(
        roman: &str,
        hanji: &str,
        walker_cost: u16,
        syll: u8,
        len: usize,
        delta: f64,
    ) -> EdgeChoice {
        EdgeChoice {
            rowid: NO_ROWID,
            roman: roman.to_owned(),
            hanji: Some(hanji.to_owned()),
            canonical_tl: roman.to_owned(),
            dict_hit: true,
            is_custom: false,
            walker_cost,
            user_row: false,
            syllable_count: syll,
            toneless_len: len,
            user_weight_delta: delta,
        }
    }
    /// A single-syllable OOV (no-dict-hit) edge, as the dispatch
    /// provider builds one for a span with no `dict.bin`/custom hit.
    fn roman(r: &str) -> EdgeChoice {
        roman_n(r, 1)
    }
    /// A multi-syllable OOV edge. v3.5.8 RC0: OOV cost is now
    /// `OOV_PER_CHAR_PENALTY * toneless_len` (char-keyed); `syll` here
    /// is synth syllable-sum metadata and does NOT affect the edge
    /// cost. `toneless_len` (= `r.chars().count()`) is the cost driver.
    fn roman_n(r: &str, syll: u8) -> EdgeChoice {
        EdgeChoice {
            rowid: NO_ROWID,
            roman: r.to_owned(),
            hanji: None,
            canonical_tl: String::new(),
            dict_hit: false,
            is_custom: false,
            walker_cost: 0,
            user_row: false,
            syllable_count: syll,
            toneless_len: r.chars().count(),
            user_weight_delta: 0.0,
        }
    }
    /// v3.5.8 S6 — a `custom_dictionary.db` edge as the dispatch
    /// provider builds it: `dict_hit:true` (lexicon-backed),
    /// `is_custom:true`, priced at most at the model-derived custom-word cost.
    fn custom(roman: &str, hanji: &str, walker_cost: u16, syll: u8, len: usize) -> EdgeChoice {
        EdgeChoice {
            rowid: NO_ROWID,
            roman: roman.to_owned(),
            hanji: Some(hanji.to_owned()),
            canonical_tl: roman.to_owned(),
            dict_hit: true,
            is_custom: true,
            walker_cost,
            user_row: true,
            syllable_count: syll,
            toneless_len: len,
            user_weight_delta: 0.0,
        }
    }

    // `walker` is a submodule of `lattice`, so `Lattice`'s private
    // `edges` field is in scope here — construct directly, same as the
    // `builder.rs` unit tests do.
    fn lattice(mut edges: Vec<(usize, usize)>) -> Lattice {
        edges.sort_unstable();
        Lattice { edges }
    }

    #[test]
    fn picks_real_phrase_path_over_high_frequency_single_chars() {
        // The motivating bug with real dictionary costs.
        // `taiuan` shadow len 6. Phrase path (0,6) 台灣 (2 syll) vs the
        // all-atomic single-char path 乾 伊 有 俺 — the path the broken
        // S2/S3 max-Σ objective produced. Min-cost must pick the phrase.
        let lat = lattice(vec![(0, 2), (0, 6), (2, 3), (3, 4), (4, 6)]);
        let path = walk(&lat, 6, |s, e| match (s, e) {
            (0, 6) => Some(dict("tâi-uân", "台灣", 7_166, 2, 6)),
            (0, 2) => Some(dict("ta", "乾", 10_647, 1, 2)),
            (2, 3) => Some(dict("i", "伊", 4_400, 1, 1)),
            (3, 4) => Some(dict("ū", "有", 4_496, 1, 1)),
            (4, 6) => Some(dict("án", "俺", 11_166, 1, 2)),
            _ => None,
        })
        .expect("full path");
        assert_eq!(path.edges, vec![(0, 6)]);
        assert_eq!(path.choices[0].hanji.as_deref(), Some("台灣"));
    }

    #[test]
    fn picks_two_phrase_path_for_taiuantaigi() {
        // `taiuantaigi` shadow len 11. Two-phrase path
        // (0,6) 台灣 + (6,11) 台語 must beat both the single
        // (0,11) blob and the all-atomic single-char path.
        let lat = lattice(vec![
            (0, 3),
            (0, 6),
            (3, 6),
            (6, 9),
            (6, 11),
            (9, 11),
            (0, 11),
        ]);
        let path = walk(&lat, 11, |s, e| match (s, e) {
            (0, 6) => Some(dict("tâi-uân", "台灣", 7_166, 2, 6)),
            (6, 11) => Some(dict("tâi-gí", "台語", 8_800, 2, 5)),
            (0, 3) => Some(dict("tâi", "台", 8_561, 1, 3)),
            (3, 6) => Some(dict("uân", "灣", 11_795, 1, 3)),
            (6, 9) => Some(dict("tâi", "台", 8_561, 1, 3)),
            (9, 11) => Some(dict("gí", "語", 10_571, 1, 2)),
            (0, 11) => None,
            _ => None,
        })
        .expect("full path");
        assert_eq!(path.edges, vec![(0, 6), (6, 11)]);
        let hanji: String = path
            .choices
            .iter()
            .filter_map(|c| c.hanji.clone())
            .collect();
        assert_eq!(hanji, "台灣台語");
    }

    #[test]
    fn no_dict_path_collapses_to_fewest_edges_under_min_cost() {
        // `taiuantai` — no dict hits anywhere. v3.5.8 RC0: every OOV
        // edge costs `OOV_PER_CHAR_PENALTY * toneless_len`, so ALL
        // OOV-only segmentations covering the same buffer have the
        // SAME total (Σ char counts = buffer length, constant). The
        // walker replaces only on a strictly-lower cost, so the
        // first-reached path at offset 9 — the single (0,9) blob,
        // reached directly from 0 before the (6,9) relaxation — is
        // kept. The user-facing per-syllable romanization is NOT
        // produced here; it is the explicit
        // `continuous::fetch_walker_slot0_inner` carve-out (Codex pre-impl S5
        // Q2). This pins that an all-OOV buffer still resolves to the
        // fewest-edge blob (which the carve-out then renders
        // per-syllable) under the RC0 BIG-per-char model.
        let lat = lattice(vec![(0, 3), (0, 6), (0, 9), (3, 6), (3, 9), (6, 9)]);
        let path = walk(&lat, 9, |s, e| match (s, e) {
            (0, 3) => Some(roman("tai")),
            (3, 6) => Some(roman("uan")),
            (6, 9) => Some(roman("tai")),
            (0, 6) => Some(roman_n("taiuan", 2)),
            (3, 9) => Some(roman_n("uantai", 2)),
            (0, 9) => Some(roman_n("taiuantai", 3)),
            _ => None,
        })
        .expect("full path");
        assert_eq!(path.edges, vec![(0, 9)]);
        assert!(path.choices.iter().all(|c| !c.dict_hit));
    }

    #[test]
    fn oov_blob_loses_to_dict_covering_path() {
        // RC0 at the walker level. `taiuanta` (shadow len 8): a
        // whole-buffer OOV blob edge (0,8) competes with the
        // dict-covering path 台灣(0,6, 2 syll) + 焦(6,8, 1 syll).
        // Under RC0 the blob costs
        // `8 * OOV_PER_CHAR_PENALTY ≈ 8e10`, dwarfing the `ln`-scale
        // dict path, so the walker picks the dict path → `any dict_hit`
        // true → `continuous::fetch_walker_slot0_inner` renders hanji, never
        // bare `"tai uan ta"`.
        let lat = lattice(vec![(0, 6), (0, 8), (6, 8)]);
        let path = walk(&lat, 8, |s, e| match (s, e) {
            (0, 6) => Some(dict("tâi-uân", "台灣", 7_166, 2, 6)),
            (6, 8) => Some(dict("ta", "焦", 10_453, 1, 2)),
            (0, 8) => Some(roman_n("taiuanta", 3)),
            _ => None,
        })
        .expect("full path");
        assert_eq!(path.edges, vec![(0, 6), (6, 8)]);
        assert!(
            path.choices.iter().all(|c| c.dict_hit),
            "dict-covering path must win so the slot-0 synth is hanji"
        );
        let hanji: String = path
            .choices
            .iter()
            .filter_map(|c| c.hanji.clone())
            .collect();
        assert_eq!(hanji, "台灣焦");
    }

    #[test]
    fn rc0_long_dict_path_beats_whole_buffer_oov_blob() {
        // RC0 at the walker level — the reproduced bug shape.
        // `ginalangtsiahpngbesai` (= 囡仔人食飯袂使): a 7-edge
        // dict-covering path vs a single whole-buffer OOV blob edge.
        // Pre-RC0 the smooth OOV pricing made the 1-edge blob cheaper
        // than the 7-edge dict path (each dict edge paid a per-edge
        // `−ln p` toll, the blob paid one discounted toll) →
        // bare roman. RC0: the blob costs `21 * OOV_PER_CHAR_PENALTY`,
        // so the dict path wins regardless of how many edges it needs.
        let lat = lattice(vec![
            (0, 3),
            (3, 6),
            (6, 9),
            (9, 12),
            (12, 15),
            (15, 18),
            (18, 21),
            (0, 21),
        ]);
        let path = walk(&lat, 21, |s, e| match (s, e) {
            (0, 3) => Some(dict("gín", "囡", 12_306, 1, 3)),
            (3, 6) => Some(dict("á", "仔", 7_643, 1, 3)),
            (6, 9) => Some(dict("lâng", "人", 4_557, 1, 3)),
            (9, 12) => Some(dict("tsia̍h", "食", 6_503, 1, 3)),
            (12, 15) => Some(dict("pn̄g", "飯", 9_333, 1, 3)),
            (15, 18) => Some(dict("bē", "袂", 6_845, 1, 3)),
            (18, 21) => Some(dict("sái", "使", 9_995, 1, 3)),
            (0, 21) => Some(roman_n("ginalangtsiahpngbesai", 7)),
            _ => None,
        })
        .expect("full path");
        assert_eq!(path.edges.len(), 7, "must take the 7-edge dict path");
        assert!(
            path.choices.iter().all(|c| c.dict_hit),
            "RC0: a dict-coverable buffer never collapses to an OOV blob"
        );
        let hanji: String = path
            .choices
            .iter()
            .filter_map(|c| c.hanji.clone())
            .collect();
        assert_eq!(hanji, "囡仔人食飯袂使");
    }

    #[test]
    fn user_preference_flips_the_chosen_segmentation_path() {
        // v3.5.8 S3/S5 — Gap B → G2. Same buffer (shadow len 6), two
        // covering segmentations:
        //   A: one 2-syllable phrase edge (0,6), unseen in the corpus.
        //   B: two hot 1-syllable edges (0,3)+(3,6), cheap.
        // Without user history the hot single chars (B) are cheaper.
        // After the user repeatedly selects the phrase (fresh max
        // decayed delta) the phrase edge is capped at the user-entry
        // tier and discounted, and path A wins; the hot single chars get
        // NO discount (single-syllable damping, SCALE = 0.0).
        // trace: cold 13.181 / 6^0.2 × 2^0.2 = 10.581 > (4.5 + 4.5) / 3^0.2 = 7.225;
        //        warm 7.878 / 6^0.2 × 2^0.2 − ln 5 = 6.324 − 1.609 = 4.715
        let lat = lattice(vec![(0, 3), (0, 6), (3, 6)]);
        let edges = |s, e, phrase_delta: f64| match (s, e) {
            (0, 6) => Some(dict_u("tâi-gí", "臺語", 13_181, 2, 6, phrase_delta)),
            // Hot single chars carry a huge delta too — it must be
            // ignored in the path objective (Q4c BLOCK guard).
            (0, 3) => Some(dict_u("tâi", "台", 4_500, 1, 3, 4.0)),
            (3, 6) => Some(dict_u("gí", "語", 4_500, 1, 3, 4.0)),
            _ => None,
        };

        // Cold start (no user history on the phrase) → hot singles win.
        let cold = walk(&lat, 6, |s, e| edges(s, e, 0.0)).expect("full path");
        assert_eq!(cold.edges, vec![(0, 3), (3, 6)]);

        // After repeated user selection (fresh, fully saturated decayed
        // delta = 4.0 on the phrase edge) → the phrase path wins.
        let warm = walk(&lat, 6, |s, e| edges(s, e, 4.0)).expect("full path");
        assert_eq!(warm.edges, vec![(0, 6)]);
        let hanji: String = warm
            .choices
            .iter()
            .filter_map(|c| c.hanji.clone())
            .collect();
        assert_eq!(hanji, "臺語");
    }

    #[test]
    fn custom_edge_wins_path_and_carries_is_custom() {
        // v3.5.8 S6 — `taigi` (shadow len 5). A custom_dictionary.db
        // entry covering (0,5) priced at the fixture custom-word cost (2-syll)
        // must beat the single-char split 台 + 語 — same arithmetic as
        // `cost::tests::custom_two_syllable_beats_the_cheapest_single_char_split`.
        // The winning slot-0 path's choice must carry `is_custom`.
        let custom_cost = FIXTURE_CUSTOM_WORD_COST;
        let lat = lattice(vec![(0, 3), (0, 5), (3, 5)]);
        let path = walk(&lat, 5, |s, e| match (s, e) {
            (0, 5) => Some(custom("tâi-gí", "台語", custom_cost, 2, 5)),
            (0, 3) => Some(dict("tâi", "台", 8_561, 1, 3)),
            (3, 5) => Some(dict("gí", "語", 10_571, 1, 2)),
            _ => None,
        })
        .expect("full path");
        assert_eq!(path.edges, vec![(0, 5)]);
        assert!(path.choices[0].is_custom, "custom edge must flag is_custom");
        assert!(
            path.choices[0].dict_hit,
            "custom edge is a lexicon-backed hit (no-dict carve-out must not fire)"
        );
        assert_eq!(path.choices[0].hanji.as_deref(), Some("台語"));
    }

    #[test]
    fn no_full_path_returns_none() {
        // Edges leave a gap (3..6 unreachable) so offset 9 is never
        // relaxed from 0 → no full-buffer path.
        let lat = lattice(vec![(0, 3), (6, 9)]);
        assert!(walk(&lat, 9, |_, _| Some(roman("x"))).is_none());
    }

    #[test]
    fn empty_shadow_returns_none() {
        let lat = lattice(vec![]);
        assert!(walk(&lat, 0, |_, _| Some(roman("x"))).is_none());
    }

    #[test]
    fn dropped_edge_does_not_break_alternative_path() {
        // Provider drops the phrase edge (0,6); the walker must still
        // span the buffer via the atomic edges.
        let lat = lattice(vec![(0, 3), (0, 6), (3, 6)]);
        let path = walk(&lat, 6, |s, e| match (s, e) {
            (0, 6) => None, // dropped (e.g. empty toneless key)
            (0, 3) => Some(roman("tai")),
            (3, 6) => Some(roman("bak")),
            _ => None,
        })
        .expect("full path via atomic edges");
        assert_eq!(path.edges, vec![(0, 3), (3, 6)]);
    }

    /// A dictionary edge whose pick is the word at `rowid`.
    fn word(rowid: u32, hanji: &str, walker_cost: u16, syll: u8, len: usize) -> EdgeChoice {
        EdgeChoice {
            rowid,
            ..dict(hanji, hanji, walker_cost, syll, len)
        }
    }

    /// A bigram table: previous rowid → (C, N), and the pair counts.
    #[derive(Debug, Clone, Default)]
    struct FakeBigram {
        continuations: std::collections::HashMap<u32, (u32, u32)>,
        pairs: std::collections::HashMap<(u32, u32), u32>,
    }

    impl FakeBigram {
        fn previous(mut self, rowid: u32, total: u32, distinct: u32) -> Self {
            self.continuations.insert(rowid, (total, distinct));
            self
        }
        fn pair(mut self, previous: u32, next: u32, count: u32) -> Self {
            self.pairs.insert((previous, next), count);
            self
        }
    }

    impl WalkerBigram for FakeBigram {
        type Continuations = (u32, u32, u32);

        fn continuations(&self, previous_rowid: u32) -> Option<Self::Continuations> {
            let &(total, distinct) = self.continuations.get(&previous_rowid)?;
            Some((previous_rowid, total, distinct))
        }

        fn evidence(&self, continuations: &Self::Continuations, next_rowid: u32) -> BigramEvidence {
            let &(previous, total, distinct) = continuations;
            BigramEvidence {
                total,
                distinct,
                pair_count: self
                    .pairs
                    .get(&(previous, next_rowid))
                    .copied()
                    .unwrap_or(0),
                discount: 0.75,
            }
        }
    }

    // Production rowid-free stand-ins for the reported words.
    const U_HAVE: u32 = 1;
    const E_THE: u32 = 2;
    const E_SHOE: u32 = 3;
    const KHI_GO: u32 = 4;
    const KHI_KHI_RISE: u32 = 5;

    /// The production pair counts behind the two reports (`word_bigrams.tsv`
    /// minus `taigi_typing`): 有 C 48 740 N 3 370, 有→的 411; 去 C 14 233
    /// N 1 432, 去→去 27.
    fn reported_pairs() -> FakeBigram {
        FakeBigram::default()
            .previous(U_HAVE, 48_740, 3_370)
            .pair(U_HAVE, E_THE, 411)
            .previous(KHI_GO, 14_233, 1_432)
            .pair(KHI_GO, KHI_GO, 27)
    }

    #[test]
    fn bigram_penalty_keeps_ue5_whole() {
        // trace: unigram 有 4.496 + 的 4.173 = 8.669 < 鞋 (key min 10.024) / 2^0.2 = 8.726;
        // bigram 的 after 有: −ln(410.25/48 740 + 0.75·3 370/48 740 · e^−4.173) = 4.687,
        // split 4.496 + 4.687 = 9.183 > 8.726.
        let lat = lattice(vec![(0, 1), (0, 2), (1, 2)]);
        let edges = |s, e| match (s, e) {
            (0, 1) => Some(word(U_HAVE, "有", 4_496, 1, 1)),
            (1, 2) => Some(word(E_THE, "的", 4_173, 1, 1)),
            (0, 2) => Some(word(E_SHOE, "鞋", 10_024, 1, 2)),
            _ => None,
        };
        let unigram = walk(&lat, 2, edges).unwrap();
        assert_eq!(unigram.edges, vec![(0, 1), (1, 2)]);
        let path = walk_best(&lat, 2, FIXTURE_CUSTOM_WORD_COST, &reported_pairs(), edges).unwrap();
        assert_eq!(path.edges, vec![(0, 2)]);
        assert!((path.cost - 10.024 / 2f64.powf(0.2)).abs() < 1e-9);
    }

    #[test]
    fn bigram_penalty_keeps_khikhi_whole() {
        // trace: 去 5.461 / 3^0.2 = 4.384, unigram split 8.768 < 起去 11.552 × (2/6)^0.2 = 9.273;
        // 去 after 去: −ln(26.25/14 233 + 0.75·1 432/14 233 · e^−5.461) = 6.135 → 4.925,
        // split 9.309 > 9.273 (the thin margin D6 records).
        let lat = lattice(vec![(0, 3), (0, 6), (3, 6)]);
        let edges = |s, e| match (s, e) {
            (0, 3) | (3, 6) => Some(word(KHI_GO, "去", 5_461, 1, 3)),
            (0, 6) => Some(word(KHI_KHI_RISE, "起去", 11_552, 2, 6)),
            _ => None,
        };
        let unigram = walk(&lat, 6, edges).unwrap();
        assert_eq!(unigram.edges, vec![(0, 3), (3, 6)]);
        let path = walk_best(&lat, 6, FIXTURE_CUSTOM_WORD_COST, &reported_pairs(), edges).unwrap();
        assert_eq!(path.edges, vec![(0, 6)]);
    }

    #[test]
    fn rowid_free_picks_take_no_penalty() {
        // A custom / learned / OOV pick has no bigram word: 有 then a
        // rowid-free 的 is the unigram walk.
        let lat = lattice(vec![(0, 1), (0, 2), (1, 2)]);
        let path = walk_best(
            &lat,
            2,
            FIXTURE_CUSTOM_WORD_COST,
            &reported_pairs(),
            |s, e| match (s, e) {
                (0, 1) => Some(word(U_HAVE, "有", 4_496, 1, 1)),
                (1, 2) => Some(word(NO_ROWID, "的", 4_173, 1, 1)),
                (0, 2) => Some(word(E_SHOE, "鞋", 10_024, 1, 2)),
                _ => None,
            },
        )
        .unwrap();
        assert_eq!(path.edges, vec![(0, 1), (1, 2)]);
    }

    #[test]
    fn each_incoming_edge_keeps_its_own_prefix() {
        // Two edges reach offset 3: 甲 path (1 + 4 = 5.0) beats 乙 (5.5)
        // alone, but only 乙 is a recorded predecessor of 丙. A per-offset
        // walker would keep 甲 and pay 甲→丙's penalty (ln(100/7.5) = 2.590);
        // the (offset, incoming edge) state prices 乙→丙 at its unigram.
        // trace: 甲 path 5.0 + 4.0 + 2.590 = 11.590 > 乙 path 5.5 + 4.0 = 9.5.
        let lat = lattice(vec![(0, 1), (0, 3), (1, 3), (3, 4)]);
        let bigram = FakeBigram::default()
            .previous(10, 100, 10)
            .previous(11, 100, 10)
            .pair(11, 12, 90);
        let path = walk_best(&lat, 4, FIXTURE_CUSTOM_WORD_COST, &bigram, |s, e| {
            match (s, e) {
                (0, 1) => Some(word(20, "丁", 1_000, 1, 1)),
                (1, 3) => Some(word(10, "甲", 4_000, 1, 1)),
                (0, 3) => Some(word(11, "乙", 5_500, 1, 1)),
                (3, 4) => Some(word(12, "丙", 4_000, 1, 1)),
                _ => None,
            }
        })
        .unwrap();
        assert_eq!(path.edges, vec![(0, 3), (3, 4)]);
        assert!((path.cost - 9.5).abs() < 1e-9);
    }

    #[test]
    fn equal_cost_paths_keep_the_first_reached() {
        // trace: 甲 (0,2) 8.0 and 乙 4.0 + 丙 4.0 = 8.0 tie at the sink; (0,2)
        // sorts before (1,2), so the one-edge path is kept, as before the bigram.
        let lat = lattice(vec![(0, 1), (0, 2), (1, 2)]);
        let path = walk(&lat, 2, |s, e| match (s, e) {
            (0, 2) => Some(word(1, "甲", 8_000, 1, 1)),
            (0, 1) => Some(word(2, "乙", 4_000, 1, 1)),
            (1, 2) => Some(word(3, "丙", 4_000, 1, 1)),
            _ => None,
        })
        .unwrap();
        assert_eq!(path.edges, vec![(0, 2)]);
    }

    /// The unigram walker as of HEAD~1 (one node per offset), kept as the
    /// oracle for the empty-table identity (`bigram-lm-roadmap.md` D6).
    fn unigram_walk(
        lattice: &Lattice,
        shadow_len: usize,
        mut edge_choice: impl FnMut(usize, usize) -> Option<EdgeChoice>,
    ) -> Option<BestPath> {
        use std::collections::BTreeMap;
        if shadow_len == 0 {
            return None;
        }
        let mut best: BTreeMap<usize, (f64, usize, Option<EdgeChoice>)> =
            BTreeMap::from([(0, (0.0, 0, None))]);
        for &(start, end) in lattice.edges() {
            let Some(&(start_cost, _, _)) = best.get(&start) else {
                continue;
            };
            let Some(choice) = edge_choice(start, end) else {
                continue;
            };
            let cost =
                start_cost + super::super::cost::edge_cost(&choice, FIXTURE_CUSTOM_WORD_COST, None);
            if best.get(&end).is_none_or(|current| cost < current.0) {
                best.insert(end, (cost, start, Some(choice)));
            }
        }
        let total = best.get(&shadow_len)?.0;
        let (mut edges, mut choices, mut cur) = (Vec::new(), Vec::new(), shadow_len);
        while cur != 0 {
            let (_, prev, choice) = best.get(&cur)?;
            edges.push((*prev, cur));
            choices.push(choice.clone()?);
            cur = *prev;
        }
        edges.reverse();
        choices.reverse();
        Some(BestPath {
            edges,
            choices,
            cost: total,
        })
    }

    /// A random edge's content: dictionary (rowid 0..4, so picks repeat) or OOV.
    /// Prices and lengths come from short lists so equal-cost paths, and the tie
    /// order, occur.
    fn arbitrary_choice() -> impl proptest::strategy::Strategy<Value = Option<EdgeChoice>> {
        use proptest::prelude::*;
        proptest::option::weighted(
            0.9,
            (
                0u32..4,
                proptest::sample::select(vec![0u16, 4_000, 8_000, 13_181]),
                proptest::sample::select(vec![1u8, 1, 1, 2]),
                proptest::sample::select(vec![1usize, 1, 1, 4]),
                prop_oneof![3 => Just(0.0), 1 => 0.0..4.0],
                proptest::bool::weighted(0.9),
            ),
        )
        .prop_map(|choice| {
            choice.map(|(rowid, walker_cost, syll, len, delta, dict_hit)| {
                if dict_hit {
                    EdgeChoice {
                        user_weight_delta: delta,
                        ..word(rowid, "字", walker_cost, syll, len)
                    }
                } else {
                    roman_n(&"a".repeat(len), syll)
                }
            })
        })
    }

    /// A buffer length, its edges with their content, and a bigram table.
    type ArbitraryWalk = (usize, Vec<((usize, usize), Option<EdgeChoice>)>, FakeBigram);

    /// A random lattice over offsets `0..=len` (edges up to 4 wide), each
    /// edge with its content, plus a bigram table over the four rowids.
    fn arbitrary_walk() -> impl proptest::strategy::Strategy<Value = ArbitraryWalk> {
        use proptest::prelude::*;
        (1usize..9).prop_flat_map(|len| {
            let spans: Vec<(usize, usize)> = (0..len)
                .flat_map(|s| (s + 1..=(s + 4).min(len)).map(move |e| (s, e)))
                .collect();
            let edges = spans
                .into_iter()
                .map(|span| {
                    (proptest::bool::weighted(0.7), arbitrary_choice())
                        .prop_map(move |(kept, c)| (span, kept.then_some(c).flatten()))
                })
                .collect::<Vec<_>>();
            let table = proptest::collection::vec(
                (1u32..500, 1u32..50, proptest::collection::vec(0u32..400, 4)),
                4,
            )
            .prop_map(|rows| {
                rows.into_iter().enumerate().fold(
                    FakeBigram::default(),
                    |table, (previous, (total, distinct, pairs))| {
                        let previous = previous as u32;
                        pairs.into_iter().enumerate().fold(
                            table.previous(previous, total, distinct),
                            |t, (next, count)| t.pair(previous, next as u32, count),
                        )
                    },
                )
            });
            (Just(len), edges, table)
        })
    }

    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig::with_cases(2_048))]

        // (a) Empty table: the same edges, choices and cost as the HEAD~1 walker.
        #[test]
        fn empty_bigram_walk_is_the_unigram_walk((len, edges, _) in arbitrary_walk()) {
            let lat = lattice(edges.iter().map(|(span, _)| *span).collect());
            let content = |s, e| edges.iter().find(|(span, _)| *span == (s, e)).and_then(|(_, c)| c.clone());
            let walked = walk(&lat, len, content);
            proptest::prop_assert_eq!(walked, unigram_walk(&lat, len, content));
        }

        // (b) With data: penalty-only never lowers the best cost, and a
        // single-edge path is priced exactly as without the table.
        #[test]
        fn bigram_walk_never_undercuts_the_unigram_walk((len, edges, table) in arbitrary_walk()) {
            let lat = lattice(edges.iter().map(|(span, _)| *span).collect());
            let content = |s, e| edges.iter().find(|(span, _)| *span == (s, e)).and_then(|(_, c)| c.clone());
            let unigram = unigram_walk(&lat, len, content);
            let walked = walk_best(&lat, len, FIXTURE_CUSTOM_WORD_COST, &table, content);
            proptest::prop_assert_eq!(walked.is_some(), unigram.is_some());
            if let (Some(walked), Some(unigram)) = (walked, unigram) {
                proptest::prop_assert!(walked.cost >= unigram.cost);
                if let [choice] = walked.choices.as_slice() {
                    let alone = super::super::cost::edge_cost(choice, FIXTURE_CUSTOM_WORD_COST, None);
                    proptest::prop_assert_eq!(walked.cost, alone);
                }
            }
        }
    }
}
