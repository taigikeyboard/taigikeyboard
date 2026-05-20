//! v3.5.8 S1 — multi-start lattice construction.
//!
//! Generalizes the pre-S1 single-start
//! `valid_span_endings(shadow, 0, …)` emission into a DAG: for every
//! offset reachable from byte 0 by a chain of valid syllables, emit
//! every span the syllabifier accepts from that offset. The
//! syllabifier algorithm is reused unchanged; the only addition is a
//! `pub(crate)` pre-lowered entry point (`valid_span_endings_lowered`)
//! so the BFS lowercases the shadow once instead of per start
//! (`syllabifier::tl::valid_span_endings_lowered`; Codex PR #284 P1).

// 中文: S1 — 多起點 lattice 建構;對每個從 0 可達的音節邊界跑 syllabifier,
// 中文:   聯集所有 (start,end) span。演算法不變,僅加 pre-lowered 入口避免每 start 重抄。

use std::collections::{BTreeSet, VecDeque};

use lexicon::SyllableInventory;
use phonetics::InputMode;

use super::Lattice;
use crate::syllabifier::tl::valid_span_endings_lowered;

/// v3.5.9 B-1: the lattice builder always queries the **TL** family of
/// the tagged-single-FST syllable inventory because `build_shadow_lattice`
/// canonicalizes its input to TL form via `canonicalize_poj_shadow`
/// before reaching here. v3.5.9 B-2 will reshape `canonicalize_poj_shadow`
/// to preserve POJ ASCII when `mode == InputMode::Poj` and thread a
/// `mode: InputMode` parameter through this seam; until then, hardcoding
/// `Tl` here keeps B-1 behavior-neutral against the pre-B golden suite.
// 中文: B-1 階段 lattice 一律查 tl: 家族 — build_shadow_lattice 在上游已將輸入
// 中文:   canonicalize 為 TL 形式;B-2 重塑 canonicalize_poj_shadow + 加 mode 參數後解除。
const LATTICE_INVENTORY_FAMILY: InputMode = InputMode::Tl;

/// Build the segmentation lattice for `shadow` (the hyphen-stripped,
/// POJ-canonicalized TL buffer).
///
/// BFS over syllable-boundary offsets: from each reachable offset
/// `start`, `valid_span_endings_lowered(&lowered, start, inv,
/// LATTICE_INVENTORY_FAMILY, max_syllables)` returns every ending
/// reachable by `1..=max_syllables` syllable chains — i.e. BOTH the
/// immediate single-syllable (atomic) ending AND the multi-syllable
/// phrase endings. Both kinds are kept as
/// edges: dropping the multi-syllable phrase edges would remove the
/// pre-S1 left-anchored `(0, multi-syllable)` keys, so the flattened
/// output would no longer be a superset of today's (Codex pre-impl
/// 2026-05-16 Q2). `composing` stays lexicon-agnostic here: it emits
/// syllabifier-valid spans and lets `fetch_candidates_for_keys`
/// filter against the dictionary exactly as before (no crate-ownership
/// inversion — Codex Q2 boundary note).
///
/// Pure. `valid_span_endings_lowered` guarantees every returned `end`
/// is `> start`, on a UTF-8 char boundary, and `<= shadow.len()`, so
/// the edges are well-formed by construction.
pub(crate) fn build_lattice(
    shadow: &str,
    inv: &SyllableInventory,
    max_syllables: usize,
) -> Lattice {
    // Lowercase the whole shadow ONCE here, then drive the BFS with
    // `valid_span_endings_lowered`. The public `valid_span_endings`
    // re-runs `to_ascii_lowercase()` on every call, so calling it per
    // reachable start would re-lower the entire buffer O(starts) times
    // on the per-keystroke hot path (Codex PR #284 P1, `r3252344518`).
    // 中文: shadow 在此一次性 to_ascii_lowercase,BFS 改用 pre-lowered 變體;
    // 中文:   否則每個 start 都重抄整個 buffer (Codex PR #284 P1)。
    let lowered = shadow.to_ascii_lowercase();
    let mut edges: Vec<(usize, usize)> = Vec::new();
    let mut visited: BTreeSet<usize> = BTreeSet::new();
    let mut queue: VecDeque<usize> = VecDeque::new();

    visited.insert(0);
    queue.push_back(0);

    while let Some(start) = queue.pop_front() {
        for end in valid_span_endings_lowered(
            &lowered,
            start,
            inv,
            LATTICE_INVENTORY_FAMILY,
            max_syllables,
        ) {
            edges.push((start, end));
            if visited.insert(end) {
                queue.push_back(end);
            }
        }
    }

    // Sort so left-anchored (`start == 0`) edges come first in
    // ascending-`end` order — byte-identical to the pre-S1 emission
    // order, which preserves existing candidate `stable_idx`. `dedup`
    // is a no-op today (BFS visits each `start` once and
    // `valid_span_endings_lowered` returns a deduplicated set) but pins the
    // invariant.
    edges.sort_unstable();
    edges.dedup();

    Lattice { edges }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use fst::SetBuilder;
    use lexicon::SyllableInventory;
    use phonetics::canonicalize_syllable;

    use super::{build_lattice, Lattice};

    const MAX_SYLLABLES: usize = 8;

    // Hermetic inventory builder — same pattern as the integration
    // tests (`composing/tests/build_keys_tl_hyphen.rs`); inline
    // duplication preferred over a shared test-utils crate. v3.5.9 B-1:
    // keys carry the `tl:` family prefix so the inventory matches the
    // tagged-single-FST format `SyllableInventory::contains_in` expects.
    fn build_inventory(samples: &[&str]) -> SyllableInventory {
        let mut keys: Vec<String> = Vec::new();
        for s in samples {
            let (canonical, tone) = canonicalize_syllable(s)
                .unwrap_or_else(|| panic!("sample {s:?} failed canonicalize_syllable"));
            if tone.is_empty() {
                keys.push(format!("tl:{canonical}"));
            } else {
                keys.push(format!("tl:{canonical}{tone}"));
                keys.push(format!("tl:{canonical}"));
            }
        }
        keys.sort();
        keys.dedup();

        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path: PathBuf =
            std::env::temp_dir().join(format!("taigi_lattice_unit_{}_{n}.fst", std::process::id()));
        let file = std::fs::File::create(&path).expect("create fst");
        let mut builder = SetBuilder::new(std::io::BufWriter::new(file)).expect("builder");
        for key in &keys {
            builder.insert(key.as_bytes()).expect("insert");
        }
        builder.finish().expect("finish");
        SyllableInventory::open(&path).expect("open inventory")
    }

    #[test]
    fn build_lattice_emits_atomic_phrase_and_interior_edges() {
        // `taibak` / inv {tai, bak}. The DAG must contain BOTH the
        // atomic edges (`(0,3)`, `(3,6)`) AND the multi-syllable
        // phrase edge `(0,6)` (Codex pre-impl 2026-05-16 Q2 = option
        // b: phrase edges present, not atomic-only). The interior
        // `(3,6)` edge is what S2's walker needs to compose a path
        // and is the part S1 deliberately withholds from the
        // user-facing key list.
        let inv = build_inventory(&["tai5", "bak4"]);
        let lattice = build_lattice("taibak", &inv, MAX_SYLLABLES);
        let edges = lattice.edges();
        assert!(edges.contains(&(0, 3)), "atomic (0,3) missing: {edges:?}");
        assert!(
            edges.contains(&(0, 6)),
            "phrase edge (0,6) missing (Q2 option b): {edges:?}",
        );
        assert!(
            edges.contains(&(3, 6)),
            "interior edge (3,6) missing — S2 walker needs it: {edges:?}",
        );
        // Left-anchored edges sort first in ascending-`end` order.
        assert_eq!(edges[0], (0, 3));
        assert_eq!(edges[1], (0, 6));
    }

    #[test]
    fn build_lattice_finding3_interior_subword_reachable() {
        // `taiuantaigi` (台灣台語-shape) / inv {tai, uan, gi}. The
        // interior sub-word `taigi` (bytes 6..11, the 台語-analogue)
        // is reachable as edge `(6,11)` — the lattice structure that
        // lets S2 surface Finding 3 once commit is start-aware.
        let inv = build_inventory(&["tai1", "uan1", "gi1"]);
        let lattice = build_lattice("taiuantaigi", &inv, MAX_SYLLABLES);
        assert!(
            lattice.edges().contains(&(6, 11)),
            "Finding 3 sub-word edge (6,11) missing: {:?}",
            lattice.edges(),
        );
        // Topological order = ascending offsets, forward-only.
        let offsets = lattice.topological_offsets();
        let mut sorted = offsets.clone();
        sorted.sort_unstable();
        assert_eq!(offsets, sorted, "offsets must be ascending (topological)");
        for &(start, end) in lattice.edges() {
            assert!(start < end, "non-forward edge ({start},{end})");
        }
    }

    #[test]
    fn build_lattice_empty_shadow_yields_no_edges() {
        let inv = build_inventory(&["tai5"]);
        let lattice = build_lattice("", &inv, MAX_SYLLABLES);
        assert!(lattice.edges().is_empty());
        assert_eq!(lattice.topological_offsets(), Vec::<usize>::new());
    }

    #[test]
    fn topological_offsets_are_ascending_and_deduplicated() {
        // A small DAG: 0→3, 0→6, 3→6, 6→11. The natural ascending
        // offset order IS the topological order (forward-only edges),
        // which is the property S2's relaxation walker relies on.
        let lattice = Lattice {
            edges: vec![(0, 3), (0, 6), (3, 6), (6, 11)],
        };
        assert_eq!(lattice.topological_offsets(), vec![0, 3, 6, 11]);
    }

    #[test]
    fn edges_accessor_returns_left_anchored_first() {
        // `build_lattice` sorts edges so left-anchored (`start == 0`)
        // come first in ascending-`end` order — the ordering-neutral
        // flatten contract.
        let lattice = Lattice {
            edges: vec![(0, 3), (0, 6), (3, 6)],
        };
        let mut sorted = lattice.edges().to_vec();
        let original = lattice.edges().to_vec();
        sorted.sort_unstable();
        assert_eq!(original, sorted, "edges() must already be sorted");
        assert_eq!(original[0], (0, 3));
        assert_eq!(original[1], (0, 6));
    }
}
