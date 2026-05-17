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
//! see `cost.rs` and `docs/roadmap.md` §整句 lattice + walker S5.
//!
//! The walker is **pure and shadow-space native**. It never looks at
//! the dictionary, the raw byte space, or proto types: the caller
//! (`dispatch::fetch_walker_slot0`, which holds `LexiconHandle`
//! state) injects an `edge_choice` provider that maps a shadow edge
//! `(start, end)` to its best content. This keeps the
//! composing↔lexicon boundary clean and reuses the lexicon candidate
//! construction instead of duplicating it (Codex pre-impl S2 Q1b).

// 中文: S2/S5 — 全句最佳路徑 walker(min Σ edge_cost,khiin segment_min_cost 忠實移植 = McBopomofo max Σ log P)。
// 中文: McBopomofo 形狀:byte offset 天然拓樸序 → 單趟前向 relaxation 即合法(min-cost 下圖仍 forward-only DAG)。
// 中文: S2/S3 的 max Σ edge_score 結構性獎勵過度切分(taiuan→乾伊有俺)→ S5 改 min-cost(見 cost.rs / roadmap S5)。
// 中文: walker 純函式、shadow-space;每條 edge 內容由 caller(dispatch,持 LexiconHandle)注入 (Codex S2 Q1b)。

use super::{cost::edge_cost, Lattice};

/// Content the caller resolved for one lattice edge `(start, end)`.
/// Built by the dispatch-injected provider from the best dictionary
/// candidate for the edge's toneless key, or — when the edge has no
/// dict hit — synthesized from the edge's own toneless roman so the
/// no-hanji roman path is the walker's natural best path (Bug 2 / §1
/// subsumed, not a fallback).
// 中文: caller 為單條 edge 解出的內容;有字典命中用最佳候選,無命中用該段 toneless 羅馬字。
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
    /// branch in `dispatch::fetch_walker_slot0`). Provenance only:
    /// propagated to the synthesized slot-0 `RawCandidate.is_custom`
    /// when ANY winning edge is custom; the walker cost objective does
    /// NOT read this (a custom edge competes via [`Self::frequency`] =
    /// `CUSTOM_EFFECTIVE_FREQ`, Codex S6 Q1 — NOT a cost special-case).
    pub is_custom: bool,
    /// Frequency the edge is scored with in [`super::cost::edge_cost`]:
    /// the chosen `dict.bin` candidate's raw frequency, `0` for a
    /// synthesized OOV edge, or `super::cost::CUSTOM_EFFECTIVE_FREQ`
    /// for a custom edge (S6 effective-frequency proxy).
    pub frequency: u32,
    /// Syllable count of the chosen candidate (`>= 1`; `1` for a
    /// synthesized pure-roman edge; greedy-longest segment count of the
    /// edge span for a custom edge).
    pub syllable_count: u8,
    /// Toneless-key char count for this edge (khiin's `word_len`). The
    /// khiin length normalization in [`edge_cost`] is not faithful
    /// without it, so it is carried explicitly rather than re-derived
    /// (Codex pre-impl S5 Q1, 2026-05-17, BLOCK).
    pub toneless_len: usize,
    /// v3.5.8 S3 — time-decayed user-frequency boost delta for this
    /// edge's chosen candidate (`ranking::decayed_user_weight_delta`,
    /// `0.0..=4.0`). `0.0` for a no-dict edge or one with no user
    /// history. Computed caller-side in `dispatch::fetch_walker_slot0`
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
// 中文: walker 的全 buffer 最佳路徑;edges 連續覆蓋 0..shadow_len,cost = Σ edge_cost(越小越好)。
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
/// `dispatch::fetch_walker_slot0` carve-out — keeping it here would
/// re-introduce the over-segmentation pressure min-cost exists to
/// remove (Codex pre-impl S5 Q5, 2026-05-17).
// 中文: 對 lattice 跑單趟鬆弛求 0→shadow_len 的最小 Σ edge_cost 路徑;
// 中文:   無法整段覆蓋時回 None(sub-syllable partial-prefix)→ caller 不合成 slot 0,維持 pre-S2。
// 中文: 同分取 ascending edge order 先到者(strict < 才取代);無 edge-count tiebreak
// 中文:   (S2 "more edges wins" 只為逼出 no-dict 逐音節,S5 移到 fetch_walker_slot0 carve-out;留著會重新引入過度切分壓力)。
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
    fn roman(r: &str) -> EdgeChoice {
        EdgeChoice {
            roman: r.to_owned(),
            hanji: None,
            dict_hit: false,
            is_custom: false,
            frequency: 0,
            syllable_count: 1,
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
        // `taiuantai` — no dict hits anywhere. Every edge pays the same
        // ~ln(CORPUS) toll, so min-cost prefers the FEWEST edges → the
        // single (0,9) blob. The user-facing per-syllable romanization
        // is NOT produced here; it is the explicit
        // `dispatch::fetch_walker_slot0` no-dict carve-out (Codex
        // pre-impl S5 Q2). This test pins the walker-level behavior.
        let lat = lattice(vec![(0, 3), (0, 6), (0, 9), (3, 6), (3, 9), (6, 9)]);
        let path = walk_best(&lat, 9, |s, e| match (s, e) {
            (0, 3) => Some(roman("tai")),
            (3, 6) => Some(roman("uan")),
            (6, 9) => Some(roman("tai")),
            (0, 6) => Some(roman("taiuan")),
            (3, 9) => Some(roman("uantai")),
            (0, 9) => Some(roman("taiuantai")),
            _ => None,
        })
        .expect("full path");
        assert_eq!(path.edges, vec![(0, 9)]);
        assert!(path.choices.iter().all(|c| !c.dict_hit));
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
