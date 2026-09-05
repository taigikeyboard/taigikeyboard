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

use super::{cost::edge_cost, Lattice};

/// Content the caller resolved for one lattice edge `(start, end)`.
/// Built by the dispatch-injected provider from the best dictionary
/// candidate for the edge's toneless key, or — when the edge has no
/// dict hit — synthesized from the edge's own toneless roman so the
/// no-hanji roman path is the walker's natural best path (Bug 2 / §1
/// subsumed, not a fallback).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EdgeChoice {
    /// Romanization for this edge. The synthesized candidate's roman
    /// line joins these with a single ASCII space (§10.2 segmented
    /// rule: roman line gets word-boundary spaces).
    pub roman: String,
    /// Hanji for this edge if the chosen dict candidate had one;
    /// `None` for a pure-roman (no dict hit) edge.
    pub hanji: Option<String>,
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
    /// v3.5.8 S6 (Codex pre-impl S6 Q4) — `true` iff this edge resolved
    /// to a `custom_dictionary.db` entry (the S6 custom-precedence
    /// branch in `continuous::fetch_walker_slot0_inner`). Provenance only:
    /// propagated to the synthesized slot-0 `RawCandidate.is_custom`
    /// when ANY winning edge is custom; the walker cost objective does
    /// NOT read this (a custom edge competes via [`Self::frequency`] =
    /// `CUSTOM_EFFECTIVE_FREQ`, Codex S6 Q1 — NOT a cost special-case).
    pub is_custom: bool,
    /// Frequency the edge is scored with on the **dict** branch of
    /// [`super::cost::edge_cost`]: the chosen `dict.bin` candidate's
    /// raw frequency, or `super::cost::CUSTOM_EFFECTIVE_FREQ` for a
    /// custom edge (S6 proxy). `0` and **unused** for a synthesized OOV
    /// edge (RC0: OOV cost is char-keyed, never frequency-based).
    pub frequency: u32,
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
///
/// `edge_choice(start, end)` returns the resolved content for that
/// shadow edge, or `None` to drop the edge from consideration (e.g.
/// an empty toneless key). An edge dropped here simply does not relax;
/// the buffer can still be spanned via other edges.
///
/// Tie handling: among paths of equal accumulated cost the first one
/// reached in the ascending-`(start, end)` edge order wins
/// (`replace` only on a *strictly* lower cost). `lattice.edges()` is
/// sorted, so this is deterministic. No edge-count tiebreak: the S2
/// "more edges wins" lever existed only to force the no-dict
/// per-syllable split, which S5 moves to the explicit
/// `continuous::fetch_walker_slot0_inner` carve-out — keeping it here would
/// re-introduce the over-segmentation pressure min-cost exists to
/// remove (Codex pre-impl S5 Q5, 2026-05-17).
pub(crate) fn walk_best(
    lattice: &Lattice,
    shadow_len: usize,
    mut edge_choice: impl FnMut(usize, usize) -> Option<EdgeChoice>,
) -> Option<BestPath> {
    if shadow_len == 0 {
        return None;
    }

    // best[offset] = (accumulated cost, prev offset, chosen content of
    // the edge that arrived here). `0` is seeded at cost 0.0; an absent
    // entry means "unreached", which is `+∞` for the relaxation
    // comparison (Codex pre-impl S5 Q5: no need to physically seed
    // every offset for a BTreeMap DP).
    use std::collections::BTreeMap;
    struct Node {
        cost: f64,
        prev: usize,
        choice: Option<EdgeChoice>,
    }
    let mut best: BTreeMap<usize, Node> = BTreeMap::new();
    best.insert(
        0,
        Node {
            cost: 0.0,
            prev: 0,
            choice: None,
        },
    );

    // `lattice.edges()` is sorted ascending by `(start, end)`. Because
    // edges are forward-only (`start < end`), every edge feeding
    // offset `S` has `end == S` with `start < S` and therefore sorts
    // before any edge whose `start == S` — so a single forward pass is
    // a valid relaxation without a separate topological sort. This
    // holds for min-cost just as for the prior max objective: the
    // graph is unchanged, only the comparison flips.
    for &(start, end) in lattice.edges() {
        let Some(&Node {
            cost: start_cost, ..
        }) = best.get(&start)
        else {
            continue; // `start` not yet reachable from 0.
        };
        let Some(choice) = edge_choice(start, end) else {
            continue; // caller dropped this edge.
        };
        let cand_cost = start_cost
            + edge_cost(
                choice.frequency,
                choice.syllable_count,
                choice.toneless_len,
                choice.user_weight_delta,
                // OOV pricing is selected from the explicit dict-hit
                // flag, never `frequency == 0` (Codex pre-impl Q2).
                choice.dict_hit,
            );
        // Replace only on a strictly lower cost. On an exact tie the
        // first-reached (lower `(start, end)`) path is kept — no
        // edge-count semantics (Codex pre-impl S5 Q5).
        let replace = match best.get(&end) {
            None => true,
            Some(cur) => cand_cost < cur.cost,
        };
        if replace {
            best.insert(
                end,
                Node {
                    cost: cand_cost,
                    prev: start,
                    choice: Some(choice),
                },
            );
        }
    }

    // No edge chain reached the buffer end → no full-buffer path.
    let sink = best.get(&shadow_len)?;
    let total = sink.cost;

    // Reconstruct back to 0 via `prev` pointers.
    let mut edges_rev: Vec<(usize, usize)> = Vec::new();
    let mut choices_rev: Vec<EdgeChoice> = Vec::new();
    let mut cur = shadow_len;
    while cur != 0 {
        let node = best.get(&cur)?;
        let choice = node.choice.clone()?;
        edges_rev.push((node.prev, cur));
        choices_rev.push(choice);
        cur = node.prev;
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
    use super::*;

    fn dict(roman: &str, hanji: &str, freq: u32, syll: u8, len: usize) -> EdgeChoice {
        dict_u(roman, hanji, freq, syll, len, 0.0)
    }
    /// `dict` with an explicit S3 decayed user-weight delta.
    fn dict_u(roman: &str, hanji: &str, freq: u32, syll: u8, len: usize, delta: f64) -> EdgeChoice {
        EdgeChoice {
            roman: roman.to_owned(),
            hanji: Some(hanji.to_owned()),
            dict_hit: true,
            is_custom: false,
            frequency: freq,
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
            roman: r.to_owned(),
            hanji: None,
            dict_hit: false,
            is_custom: false,
            frequency: 0,
            syllable_count: syll,
            toneless_len: r.chars().count(),
            user_weight_delta: 0.0,
        }
    }
    /// v3.5.8 S6 — a `custom_dictionary.db` edge as the dispatch
    /// provider builds it: `dict_hit:true` (lexicon-backed),
    /// `is_custom:true`, scored at the `CUSTOM_EFFECTIVE_FREQ` proxy.
    fn custom(roman: &str, hanji: &str, freq: u32, syll: u8, len: usize) -> EdgeChoice {
        EdgeChoice {
            roman: roman.to_owned(),
            hanji: Some(hanji.to_owned()),
            dict_hit: true,
            is_custom: true,
            frequency: freq,
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
        // The motivating bug with real dictionary frequencies.
        // `taiuan` shadow len 6. Phrase path (0,6) 台灣 (freq 1379,
        // 2 syll) vs the all-atomic single-char path 乾(2145) 伊(63255)
        // 有(53685) 俺(10635) — the path the broken S2/S3 max-Σ
        // objective produced. Min-cost must pick the phrase.
        let lat = lattice(vec![(0, 2), (0, 6), (2, 3), (3, 4), (4, 6)]);
        let path = walk_best(&lat, 6, |s, e| match (s, e) {
            (0, 6) => Some(dict("tâi-uân", "台灣", 1379, 2, 6)),
            (0, 2) => Some(dict("ta", "乾", 2145, 1, 2)),
            (2, 3) => Some(dict("i", "伊", 63255, 1, 1)),
            (3, 4) => Some(dict("ū", "有", 53685, 1, 1)),
            (4, 6) => Some(dict("án", "俺", 10635, 1, 2)),
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
        let path = walk_best(&lat, 11, |s, e| match (s, e) {
            (0, 6) => Some(dict("tâi-uân", "台灣", 1379, 2, 6)),
            (6, 11) => Some(dict("tâi-gí", "台語", 300, 2, 5)),
            (0, 3) => Some(dict("tâi", "台", 2145, 1, 3)),
            (3, 6) => Some(dict("uân", "灣", 200, 1, 3)),
            (6, 9) => Some(dict("tâi", "台", 2145, 1, 3)),
            (9, 11) => Some(dict("gí", "語", 4000, 1, 2)),
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
        let path = walk_best(&lat, 9, |s, e| match (s, e) {
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
        // dict-covering path 台灣(0,6, freq 1379, 2 syll) +
        // 焦(6,8, freq 2145, 1 syll). Under RC0 the blob costs
        // `8 * OOV_PER_CHAR_PENALTY ≈ 8e10`, dwarfing the `ln`-scale
        // dict path, so the walker picks the dict path → `any dict_hit`
        // true → `continuous::fetch_walker_slot0_inner` renders hanji, never
        // bare `"tai uan ta"`.
        let lat = lattice(vec![(0, 6), (0, 8), (6, 8)]);
        let path = walk_best(&lat, 8, |s, e| match (s, e) {
            (0, 6) => Some(dict("tâi-uân", "台灣", 1379, 2, 6)),
            (6, 8) => Some(dict("ta", "焦", 2145, 1, 2)),
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
        // `ln(CORPUS/freq)` toll, the blob paid one discounted toll) →
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
        let path = walk_best(&lat, 21, |s, e| match (s, e) {
            (0, 3) => Some(dict("gín", "囡", 8068, 1, 3)),
            (3, 6) => Some(dict("á", "仔", 1148, 1, 3)),
            (6, 9) => Some(dict("lâng", "人", 52526, 1, 3)),
            (9, 12) => Some(dict("tsia̍h", "食", 15378, 1, 3)),
            (12, 15) => Some(dict("pn̄g", "飯", 9000, 1, 3)),
            (15, 18) => Some(dict("bē", "袂", 8000, 1, 3)),
            (18, 21) => Some(dict("sái", "使", 7000, 1, 3)),
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
        //   A: one 2-syllable phrase edge (0,6), LOW dict freq.
        //   B: two hot 1-syllable edges (0,3)+(3,6), HIGH dict freq.
        // Without user history the hot single chars (B) are cheaper.
        // After the user repeatedly selects the phrase (fresh max
        // decayed delta) the phrase edge gets a log-space discount and
        // path A wins; the hot single chars get NO discount
        // (single-syllable damping, SCALE = 0.0).
        let lat = lattice(vec![(0, 3), (0, 6), (3, 6)]);
        let edges = |s, e, phrase_delta: f64| match (s, e) {
            (0, 6) => Some(dict_u("tâi-gí", "臺語", 100, 2, 6, phrase_delta)),
            // Hot single chars carry a huge delta too — it must be
            // ignored in the path objective (Q4c BLOCK guard).
            (0, 3) => Some(dict_u("tâi", "台", 60_000, 1, 3, 4.0)),
            (3, 6) => Some(dict_u("gí", "語", 60_000, 1, 3, 4.0)),
            _ => None,
        };

        // Cold start (no user history on the phrase) → hot singles win.
        let cold = walk_best(&lat, 6, |s, e| edges(s, e, 0.0)).expect("full path");
        assert_eq!(cold.edges, vec![(0, 3), (3, 6)]);

        // After repeated user selection (fresh, fully saturated decayed
        // delta = 4.0 on the phrase edge) → the phrase path wins.
        let warm = walk_best(&lat, 6, |s, e| edges(s, e, 4.0)).expect("full path");
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
        // entry covering (0,5) scored at CUSTOM_EFFECTIVE_FREQ (2-syll)
        // must beat the high-frequency single-char split
        // 台(31281)+語(21976) — same arithmetic as
        // `cost::tests::custom_two_syllable_beats_top_single_char_split`.
        // The winning slot-0 path's choice must carry `is_custom`.
        let cef = super::super::cost::CUSTOM_EFFECTIVE_FREQ;
        let lat = lattice(vec![(0, 3), (0, 5), (3, 5)]);
        let path = walk_best(&lat, 5, |s, e| match (s, e) {
            (0, 5) => Some(custom("tâi-gí", "台語", cef, 2, 5)),
            (0, 3) => Some(dict("tâi", "台", 31_281, 1, 3)),
            (3, 5) => Some(dict("gí", "語", 21_976, 1, 2)),
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
        assert!(walk_best(&lat, 9, |_, _| Some(roman("x"))).is_none());
    }

    #[test]
    fn empty_shadow_returns_none() {
        let lat = lattice(vec![]);
        assert!(walk_best(&lat, 0, |_, _| Some(roman("x"))).is_none());
    }

    #[test]
    fn dropped_edge_does_not_break_alternative_path() {
        // Provider drops the phrase edge (0,6); the walker must still
        // span the buffer via the atomic edges.
        let lat = lattice(vec![(0, 3), (0, 6), (3, 6)]);
        let path = walk_best(&lat, 6, |s, e| match (s, e) {
            (0, 6) => None, // dropped (e.g. empty toneless key)
            (0, 3) => Some(roman("tai")),
            (3, 6) => Some(roman("bak")),
            _ => None,
        })
        .expect("full path via atomic edges");
        assert_eq!(path.edges, vec![(0, 3), (3, 6)]);
    }
}
