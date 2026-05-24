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
//! Interior (`start > 0`) edges are NOT emitted as user-facing keys.
//! Under Model B (`docs/engine/continuous-input-ranking.md`
//! §10.3/§10.4) commit is forward-only `pending[..consumed_bytes]`,
//! so there is no Model-B-consistent commit for an independently
//! tappable interior candidate. S2 therefore does **not** add a
//! `consumed_start` proto/intent/transition change or interior
//! tappables (Codex pre-impl S2 Q1c = option ii, 2026-05-16): the
//! whole-sentence walker consumes the interior edges INTERNALLY and
//! emits one synthesized full-buffer best path at candidate slot 0;
//! the user-facing commit span stays `(0, end)`. Interior
//! `台語`-style words remain reachable as the next path-step after
//! the prefix is nailed. The full DAG is constructed here so the
//! walker wires onto it with no churn and so this slice proves
//! span-local is the lattice's degenerate left-anchored special case
//! (`docs/releases/v3.5.8/plan.md` §整句 lattice + walker).
//!
//! Shape follows McBopomofo Gramambular
//! (`references/McBopomofo/Source/Engine/gramambular2/reading_grid.cpp:132`):
//! nodes are byte offsets, edges are forward-only, so the natural
//! offset ordering already IS a topological order — no separate sort
//! pass is needed for the walker.
//!
//! v3.5.8 S2 added the `walker` module (`walk_best`, single-pass
//! relaxation) and the `cost` module. **S5** corrected the objective
//! to `min Σ edge_cost` — a faithful khiin `segment_min_cost` port
//! (the S2/S3 `max Σ edge_score` structurally rewarded
//! over-segmentation; see `cost.rs` and the `docs/releases/v3.5.8/plan.md`
//! §整句 lattice + walker S5 section). The walker is pure and
//! shadow-space native; `dispatch::handle_fetch_at_pos` injects the
//! per-edge content provider and explicitly prepends the synthesized
//! full-buffer best path at candidate slot 0 (Codex pre-impl S2
//! Q1/Q1c, 2026-05-16).

// 中文: S1 — TL/POJ shadow 上的切分 lattice;建完整多起點 DAG 供 S2 全句 walker。
// 中文: S1 嚴格行為中性:對外只發左錨投影 (start==0),逐 byte 等同 S1 前。
// 中文: 內段 (start>0) 不發為可點 key:Model B forward-only commit 無對應語意;
// 中文:   S2 (Codex Q1c=ii) 不加 consumed_start / 可點內段,walker 內部吃內段、
// 中文:   只發單一全 buffer slot-0 合成候選,commit span 維持 (0,end)。

mod builder;
mod cost;
mod walker;

pub(crate) use builder::build_lattice;
pub(crate) use cost::CUSTOM_EFFECTIVE_FREQ;
pub(crate) use walker::{walk_best, EdgeChoice};

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
