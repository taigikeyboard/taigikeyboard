//! v3.5.8 S1 — segmentation lattice over the TL/POJ shadow buffer.
//!
//! Builds the multi-start DAG that S2's whole-sentence best-path
//! walker will traverse. S1 itself does NOT walk and is strictly
//! BEHAVIOR-NEUTRAL: `dispatch::build_keys_tl_with_inventory` builds
//! this full DAG but emits ONLY its left-anchored (`start == 0`)
//! projection as keys, which is byte-identical to the pre-S1
//! single-start `valid_span_endings(shadow, 0, …)` output. Nothing
//! the user can see, tap, or commit changes in S1.
//!
//! Interior (`start > 0`) edges are deliberately withheld from the
//! key list until S2: `CommitContinuous` carries only `consumed_bytes`
//! (the span end) and the `(roman, hanji)` dedupe is not span-aware,
//! so a tappable interior candidate would mis-commit (Codex post-impl
//! 2026-05-16 P1 #1/#2). S2 adds the walker, start-aware commit, and
//! interior surfacing together. The full DAG is constructed here so
//! S2 wires onto it with no churn and so this slice proves span-local
//! is the lattice's degenerate left-anchored special case
//! (`docs/roadmap.md` §整句 lattice + walker).
//!
//! Shape follows McBopomofo Gramambular
//! (`references/McBopomofo/Source/Engine/gramambular2/reading_grid.cpp:132`):
//! nodes are byte offsets, edges are forward-only, so the natural
//! offset ordering already IS a topological order — no separate sort
//! pass is needed for the future walker.

// 中文: S1 — TL/POJ shadow 上的切分 lattice;建完整多起點 DAG 供 S2 全句 walker。
// 中文: S1 嚴格行為中性:對外只發左錨投影 (start==0),逐 byte 等同 S1 前;
// 中文:   內段 (start>0) 留 S2 (commit 僅帶 consumed_bytes、dedupe 非 span-aware,Codex post-impl P1)。

mod builder;

pub(crate) use builder::build_lattice;

/// A segmentation lattice over shadow byte offsets. Nodes are byte
/// offsets in `0..=shadow.len()`; an edge `(start, end)` means
/// `shadow[start..end]` is a chain of `1..=max_syllables` valid TL
/// syllables (a dictionary-keyable span). Edges are forward-only
/// (`start < end`).
pub(crate) struct Lattice {
    /// All edges, sorted ascending by `(start, end)`. Left-anchored
    /// edges (`start == 0`) therefore sort first in ascending-`end`
    /// order — identical to the pre-S1 single-start emission order —
    /// so a caller flattening this in-order preserves the existing
    /// candidate `stable_idx` (the ordering-neutrality contract).
    edges: Vec<(usize, usize)>,
}

impl Lattice {
    /// Edges sorted ascending by `(start, end)`. Left-anchored
    /// (`start == 0`) edges come first, matching the pre-S1 order.
    pub(crate) fn edges(&self) -> &[(usize, usize)] {
        &self.edges
    }

    /// Distinct byte offsets touched by any edge endpoint, ascending.
    /// Forward-only edges make this the topological order S2's walker
    /// will relax over (McBopomofo `reading_grid.cpp:132` shape).
    #[cfg(test)]
    pub(crate) fn topological_offsets(&self) -> Vec<usize> {
        let mut offsets: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
        for &(start, end) in &self.edges {
            offsets.insert(start);
            offsets.insert(end);
        }
        offsets.into_iter().collect()
    }
}
