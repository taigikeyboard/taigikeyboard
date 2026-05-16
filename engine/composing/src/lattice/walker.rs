//! v3.5.8 S2 — whole-sentence best-path walker.
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
//! budget friendly (plan `/Users/alexsu/.claude/plans/greedy-questing-cake.md`
//! §最佳實踐對齊).
//!
//! The walker is **pure and shadow-space native**. It never looks at
//! the dictionary, the raw byte space, or proto types: the caller
//! (`dispatch::handle_fetch_at_pos`, which holds `LexiconHandle`
//! state) injects an `edge_choice` provider that maps a shadow edge
//! `(start, end)` to its best content. This keeps the
//! composing↔lexicon boundary clean and reuses the lexicon candidate
//! construction instead of duplicating it (Codex pre-impl S2 Q1b,
//! 2026-05-16).

// 中文: S2 — 全句最佳路徑 walker。McBopomofo 形狀:byte offset 天然拓樸序,
// 中文:   單趟 relaxation (edges 已按 (start,end) 排序 → 一趟前向掃描即合法鬆弛)。
// 中文: walker 純函式、shadow-space;不碰字典/raw/proto。每條 edge 的內容由 caller
// 中文:   (dispatch,持 LexiconHandle state) 注入 edge_choice provider 提供 (Codex S2 Q1b)。

use super::{cost::edge_score, Lattice};

/// Content the caller resolved for one lattice edge `(start, end)`.
/// Built by the dispatch-injected provider from the best dictionary
/// candidate for the edge's toneless key, or — when the edge has no
/// dict hit — synthesized from the edge's own toneless roman so the
/// no-hanji roman path is the walker's natural best path (Bug 2 / §1
/// subsumed, not a fallback).
// 中文: caller 為單條 edge 解出的內容;有字典命中用最佳候選,無命中用該段 toneless 羅馬字
// 中文:   → 無漢字 roman 路徑為 walker 自然最佳路徑 (吞 Bug 2/§1,非 fallback)。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EdgeChoice {
    /// Romanization for this edge. The synthesized candidate's roman
    /// line joins these with a single ASCII space (§10.2 segmented
    /// rule: roman line gets word-boundary spaces).
    pub roman: String,
    /// Hanji for this edge if the chosen dict candidate had one;
    /// `None` for a pure-roman (no dict hit) edge.
    pub hanji: Option<String>,
    /// Frequency of the chosen dict candidate (`0` = no dict hit).
    pub frequency: u32,
    /// Syllable count of the chosen dict candidate (`>= 1`; `1` for a
    /// synthesized pure-roman edge).
    pub syllable_count: u8,
}

/// The walker's best full-buffer path. `choices[i]` is the resolved
/// content of `edges[i]`; `edges` is contiguous and covers
/// `0..shadow_len`. `score` is the accumulated `Σ edge_score`.
// 中文: walker 的全 buffer 最佳路徑;edges 連續覆蓋 0..shadow_len,score = Σ edge_score。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BestPath {
    pub edges: Vec<(usize, usize)>,
    pub choices: Vec<EdgeChoice>,
    pub score: f64,
}

/// Relax `lattice` to the maximum-`Σ edge_score` path from offset `0`
/// to `shadow_len`. Returns `None` when no edge chain spans the whole
/// buffer (e.g. a sub-syllable partial-prefix buffer) — the caller
/// then leaves the existing span-local list untouched (no slot-0
/// synthesis), preserving pre-S2 behavior.
///
/// `edge_choice(start, end)` returns the resolved content for that
/// shadow edge, or `None` to drop the edge from consideration (e.g.
/// an empty toneless key). An edge dropped here simply does not relax;
/// the buffer can still be spanned via other edges.
///
/// Tie contract: among paths of equal accumulated `score`, the one
/// with **more edges** wins. With every frequency `0` (no dict hit at
/// all — the `taiuantai` roman case) every `edge_score` is exactly
/// the unit `1.0` (see `cost::edge_score`), so a finer segmentation
/// accumulates a strictly higher total and the all-atomic
/// per-syllable path wins outright — the comparison below resolves
/// the (vanishingly unlikely with real `ln(freq)` sums) exact tie
/// deterministically toward the finer split, matching the documented
/// `tai uan tai` expectation (`docs/roadmap.md` §整句 lattice + walker).
// 中文: 對 lattice 跑單趟鬆弛求 0→shadow_len 的最大 Σ edge_score 路徑;
// 中文:   無法整段覆蓋時回 None (sub-syllable partial-prefix) → caller 不合成 slot 0,維持 pre-S2 行為。
// 中文: tie 契約:同分取 edge 較多者;全零頻時每 edge 恰為 1.0 → 細分總分嚴格較高,
// 中文:   逐音節 (tai uan tai) 路徑勝出。
pub(crate) fn walk_best(
    lattice: &Lattice,
    shadow_len: usize,
    mut edge_choice: impl FnMut(usize, usize) -> Option<EdgeChoice>,
) -> Option<BestPath> {
    if shadow_len == 0 {
        return None;
    }

    // best[offset] = (accumulated score, edge count, prev offset,
    // chosen content of the edge that arrived here). `0` is seeded;
    // every other offset is unreached until relaxed.
    use std::collections::BTreeMap;
    struct Node {
        score: f64,
        edges: usize,
        prev: usize,
        choice: Option<EdgeChoice>,
    }
    let mut best: BTreeMap<usize, Node> = BTreeMap::new();
    best.insert(
        0,
        Node {
            score: 0.0,
            edges: 0,
            prev: 0,
            choice: None,
        },
    );

    // `lattice.edges()` is sorted ascending by `(start, end)`. Because
    // edges are forward-only (`start < end`), every edge feeding
    // offset `S` has `end == S` with `start < S` and therefore sorts
    // before any edge whose `start == S` — so a single forward pass is
    // a valid relaxation without a separate topological sort.
    for &(start, end) in lattice.edges() {
        let Some(&Node {
            score: start_score,
            edges: start_edges,
            ..
        }) = best.get(&start)
        else {
            continue; // `start` not yet reachable from 0.
        };
        let Some(choice) = edge_choice(start, end) else {
            continue; // caller dropped this edge.
        };
        let cand_score = start_score + edge_score(choice.frequency, choice.syllable_count);
        let cand_edges = start_edges + 1;
        let replace = match best.get(&end) {
            None => true,
            // Higher score wins; on an exact score tie prefer the
            // finer (more-edge) segmentation.
            Some(cur) => {
                cand_score > cur.score || (cand_score == cur.score && cand_edges > cur.edges)
            }
        };
        if replace {
            best.insert(
                end,
                Node {
                    score: cand_score,
                    edges: cand_edges,
                    prev: start,
                    choice: Some(choice),
                },
            );
        }
    }

    // No edge chain reached the buffer end → no full-buffer path.
    let sink = best.get(&shadow_len)?;
    let total = sink.score;

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
        score: total,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dict(roman: &str, hanji: &str, freq: u32, syll: u8) -> EdgeChoice {
        EdgeChoice {
            roman: roman.to_owned(),
            hanji: Some(hanji.to_owned()),
            frequency: freq,
            syllable_count: syll,
        }
    }
    fn roman(r: &str) -> EdgeChoice {
        EdgeChoice {
            roman: r.to_owned(),
            hanji: None,
            frequency: 0,
            syllable_count: 1,
        }
    }

    // `walker` is a submodule of `lattice`, so `Lattice`'s private
    // `edges` field is in scope here — construct directly, same as the
    // `builder.rs` unit tests do. `walk_best` only reads `.edges()`
    // (ascending sort is the builder's contract, mirrored here).
    fn lattice(mut edges: Vec<(usize, usize)>) -> Lattice {
        edges.sort_unstable();
        Lattice { edges }
    }

    #[test]
    fn picks_high_frequency_phrase_path_over_single_chars() {
        // `taiuantaigi`-shape: shadow len 11. Phrase path
        // (0,6)臺灣 + (6,11)台語 vs the all-atomic single-char path.
        // Phrase edges carry real dictionary frequency; the atomic
        // chars are low-freq — the walker must pick the phrase path.
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
            (0, 6) => Some(dict("tâi-uân", "臺灣", 5000, 2)),
            (6, 11) => Some(dict("tâi-gí", "台語", 4000, 2)),
            (0, 3) => Some(dict("tâi", "臺", 30, 1)),
            (3, 6) => Some(dict("uân", "灣", 20, 1)),
            (6, 9) => Some(dict("tâi", "台", 30, 1)),
            (9, 11) => Some(dict("gí", "語", 20, 1)),
            (0, 11) => None, // no single dict word covers the buffer
            _ => None,
        })
        .expect("full path");
        assert_eq!(path.edges, vec![(0, 6), (6, 11)]);
        let hanji: String = path
            .choices
            .iter()
            .filter_map(|c| c.hanji.clone())
            .collect();
        assert_eq!(hanji, "臺灣台語");
    }

    #[test]
    fn no_dict_path_is_per_syllable_roman() {
        // `taiuantai` — no dict hits anywhere; every edge_score is the
        // unit 1.0, so the finest (most-edge) segmentation wins → the
        // per-syllable atomic path `tai uan tai`.
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
        assert_eq!(path.edges, vec![(0, 3), (3, 6), (6, 9)]);
        let romans: Vec<&str> = path.choices.iter().map(|c| c.roman.as_str()).collect();
        assert_eq!(romans.join(" "), "tai uan tai");
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
