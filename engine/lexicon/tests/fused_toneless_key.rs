//! Engine-side contract test for the fused-toneless multi-syllable key
//! used by the Roman toneless lookup path (v3.5.8 連續輸入 Phase 1b).
//!
//! Background — the v3.5.8 roadmap (`docs/releases/v3.5.8/plan.md` § Phase 1b) originally
//! planned an FST-builder derivation rule that would emit an extra fused
//! toneless variant for every multi-syllable entry. Pre-impl audit on
//! 2026-05-10 (this PR) showed the upstream `notone` CSV stage already
//! produces fused `tl_notone` / `poj_notone` (`dictionary/common/notone.py`
//! strips both `\d` and `-`), and `create_fst.py` indexes those columns
//! directly. So the FST already contains a fused toneless key for every
//! multi-syllable entry without any builder change. Phase 1b is therefore
//! N/A — see roadmap §Phase 1b for the full reasoning.
//!
//! Scope of this contract — Roman toneless lookup with separator-stripped
//! keys ONLY. Out of scope: the `ⁿ` / `o͘` ↔ `nn` / `oo` ASCII-folding
//! mismatch between `tl_notone` and engine input normalization, and TPS
//! bopomofo explicit-tone keys. Both are tracked separately.
//!
//! What this test pins — given a synthetic FST whose multi-syllable entry
//! shares a fused toneless key (`tl:tsua`) with a single-syllable entry,
//! `lexicon::search::search()` must surface BOTH rowids when the user types
//! the toneless input `tsua`. Drives the Phase 3/5 走查範例 Case A
//! (`紙(syll=1) + 珠仔(syll=2)` retrieved from the same toneless input).
//!
//! Regression source-of-truth — if this test breaks, audit
//! `dictionary/common/notone.py::remove_tone` first. The character class
//! `[\d\-]` must continue to strip BOTH digits AND hyphens for multi-syllable
//! `tl_num` (e.g. `tsu1a2`) → fused `tl_notone` (`tsua`).

// 中文: Phase 1b 引擎側合約測試 — 固定「fused toneless key 同時索引到單音節 + 多音節 entry」這個事實。
// 中文: 上游 notone stage 已 fused;builder 不需 derive 新 key。若此 test 壞了,先看 dictionary/common/notone.py。

use std::path::PathBuf;

use lexicon::dictionary_reader::DictionaryReader;
use lexicon::prefix_index::PrefixIndex;
use lexicon::search::{self, SearchInputMode, SearchInputType, SearchParams};

mod common;
use common::{build_tkdb_v2, write_temp};

const SEPARATOR: u8 = 0xFF;

/// Toneless input `tsua` must retrieve both the single-syllable `紙` and
/// the multi-syllable `珠仔` from a synthetic FST whose `tl:tsua` key
/// indexes both rowids. Mirrors the production pipeline's behaviour after
/// the upstream notone stage fuses `tsu1a2 → tsua`.
#[test]
fn fused_toneless_key_retrieves_single_and_multi_syllable_entries() {
    // Frequencies kept realistic and non-zero so the post-lookup
    // frequency-desc sort in `search::collect_filtered_sorted` runs against
    // sensible inputs. Assertion is membership + exact set size — an
    // exact+prefix dedup regression that returned duplicates would inflate
    // the set and trip the size check.
    let fst_path = write_synthetic_fst(
        "phase1b-fused-toneless.fst",
        &[("tl:tsua", 1), ("tl:tsua", 2)],
    );
    let dict_bytes = build_tkdb_v2(
        b"TKDB",
        &[
            (0u16, 5_000u32, 1u8, "紙", "tsuá"),
            (0u16, 100u32, 2u8, "珠仔", "tsu-á"),
        ],
    );
    let dict_path = write_temp("phase1b-fused-toneless-dict.bin", &dict_bytes);

    let prefix_index = PrefixIndex::open(&fst_path).expect("open synthetic fst");
    let dict = DictionaryReader::open(&dict_path).expect("open synthetic dict");

    let params = SearchParams {
        input: "tsua".to_string(),
        input_type: SearchInputType::RomanNoTone,
        input_mode: SearchInputMode::Tl,
        limit: 10,
        enabled_sources_bitmask: u32::MAX,
    };
    let rows = search::search(&params, &prefix_index, &dict).expect("search succeeds");

    let hanji_results: Vec<&str> = rows.iter().filter_map(|r| r.hanji.as_deref()).collect();
    assert_eq!(
        hanji_results.len(),
        2,
        "exactly the two fixture entries must surface (no dedup regression): {hanji_results:?}",
    );
    assert!(
        hanji_results.contains(&"紙"),
        "single-syllable 紙 must be in results: {hanji_results:?}",
    );
    assert!(
        hanji_results.contains(&"珠仔"),
        "multi-syllable 珠仔 must be retrieved from fused toneless key: {hanji_results:?}",
    );
}

/// POJ mirror of the TL contract — `poj_notone` is also fused upstream
/// (same `remove_tone()` path), so toneless POJ input must retrieve
/// multi-syllable entries through `poj:<fused>` keys.
#[test]
fn fused_toneless_key_works_for_poj_path() {
    let fst_path = write_synthetic_fst(
        "phase1b-fused-toneless-poj.fst",
        &[("poj:chua", 1), ("poj:chua", 2)],
    );
    let dict_bytes = build_tkdb_v2(
        b"TKDB",
        &[
            (0u16, 3_134u32, 1u8, "紙", "chóa"),
            (0u16, 10u32, 2u8, "珠仔", "chu-á"),
        ],
    );
    let dict_path = write_temp("phase1b-fused-toneless-poj-dict.bin", &dict_bytes);

    let prefix_index = PrefixIndex::open(&fst_path).expect("open synthetic fst");
    let dict = DictionaryReader::open(&dict_path).expect("open synthetic dict");

    let params = SearchParams {
        input: "chua".to_string(),
        input_type: SearchInputType::RomanNoTone,
        input_mode: SearchInputMode::Poj,
        limit: 10,
        enabled_sources_bitmask: u32::MAX,
    };
    let rows = search::search(&params, &prefix_index, &dict).expect("search succeeds");
    let hanji_results: Vec<&str> = rows.iter().filter_map(|r| r.hanji.as_deref()).collect();
    assert_eq!(hanji_results.len(), 2, "POJ side: {hanji_results:?}");
    assert!(hanji_results.contains(&"珠仔"));
}

/// Pin the FST byte-order behaviour for the fused toneless key — both
/// rowids must be discoverable through `lookup_exact`. We assert on the
/// set of rowids (not order), because `PrefixIndex::lookup_exact` walks
/// the FST in byte order rather than insertion order — see
/// `engine/lexicon/tests/parity.rs` for the shared expectation.
#[test]
fn lookup_exact_returns_all_rowids_under_fused_toneless_key() {
    let fst_path = write_synthetic_fst(
        "phase1b-lookup-exact.fst",
        &[("tl:tsua", 1), ("tl:tsua", 2)],
    );
    let prefix_index = PrefixIndex::open(&fst_path).expect("open synthetic fst");
    let mut rowids: Vec<u32> = prefix_index.lookup_exact("tl:tsua");
    rowids.sort_unstable();
    assert_eq!(rowids, vec![1u32, 2u32]);
}

fn write_synthetic_fst(name: &str, pairs: &[(&str, u32)]) -> PathBuf {
    use fst::SetBuilder;
    let path = std::env::temp_dir().join(format!("lexicon-test-{name}"));
    let mut entries: Vec<Vec<u8>> = pairs
        .iter()
        .map(|(key, rowid)| {
            let mut e = Vec::with_capacity(key.len() + 1 + 4);
            e.extend_from_slice(key.as_bytes());
            e.push(SEPARATOR);
            e.extend_from_slice(&rowid.to_le_bytes());
            e
        })
        .collect();
    entries.sort_unstable();
    entries.dedup();
    let file = std::fs::File::create(&path).expect("create fst");
    let mut builder = SetBuilder::new(std::io::BufWriter::new(file)).expect("builder");
    for entry in &entries {
        builder.insert(entry).expect("insert");
    }
    builder.finish().expect("finish");
    path
}
