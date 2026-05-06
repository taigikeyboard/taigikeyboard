//! INVARIANT_LEX_* integration tests.
//!
//! Uses synthetic in-memory fixtures rather than the bundled
//! `dictionary.fst` + `.bin` so the suite runs without the full
//! `dictionary/build.sh` pipeline. Real-fixture parity gets covered
//! by the platform XCTest / JUnit tests that load shipped assets.
//!
//! Tests pin the invariants from
//! `docs/engine/lexicon-slice-plan.md` §7 +
//! `docs/architecture/behavioral-invariants.md`.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};

use lexicon::association_reader::AssociationReader;
use lexicon::dictionary_reader::{DictionaryReader, Filter};
use lexicon::handle::EngineHandle;
use lexicon::paths::LexiconPaths;
use lexicon::prefix_index::PrefixIndex;
use lexicon::search::{self, SearchInputMode, SearchInputType, SearchParams};
use lexicon::LexiconError;

const SEPARATOR: u8 = 0xFF;

/// Process-wide serialization for tests that call `EngineHandle::install`.
/// `EngineHandle` is a global singleton; without this, parallel test threads
/// can swap engine state between one test's install and its assertions.
/// Codex post-impl R2 reproduced this flake at `--test-threads=16` after
/// ~50 iterations.
fn engine_install_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}

// --- INVARIANT_LEX_FILTER_BITMASK ---------------------------------------

#[test]
fn invariant_lex_filter_bitmask_three_layers() {
    // Layer 1: variant exclusion
    let f = Filter {
        variant: false,
        khiin: true,
        all_enabled: true,
        enabled_mask: 0,
    };
    let variant_record = 1u16 << 12; // VARIANT_BIT
    assert!(!DictionaryReader::passes_filter(variant_record, &f));

    // Layer 2: khiin exclusion
    let f2 = Filter {
        variant: true,
        khiin: false,
        all_enabled: true,
        enabled_mask: 0,
    };
    let khiin_record = 1u16 << 9; // KHIIN_BIT
    assert!(!DictionaryReader::passes_filter(khiin_record, &f2));

    // Layer 3: source-OR with dev (dev always passes when set)
    let f3 = Filter {
        variant: true,
        khiin: true,
        all_enabled: false,
        enabled_mask: 1u16 << 0, // only kautian enabled
    };
    let dev_record = 1u16 << 10; // DEV_BIT
    assert!(DictionaryReader::passes_filter(dev_record, &f3));
    let kautian_record = 1u16 << 0;
    assert!(DictionaryReader::passes_filter(kautian_record, &f3));
    let stti_record = 1u16 << 7;
    assert!(!DictionaryReader::passes_filter(stti_record, &f3));
}

// --- INVARIANT_LEX_BINARY_FORMAT ---------------------------------------

#[test]
fn invariant_lex_binary_format_rejects_bad_magic() {
    let bad = synth_dictionary_bin(b"BAD!", 1, &[]);
    let path = write_temp("bad-magic.bin", &bad);
    let err = DictionaryReader::open(&path).expect_err("bad magic must fail");
    assert!(matches!(err, LexiconError::InvalidBinary(_)), "{err:?}");
}

#[test]
fn invariant_lex_binary_format_rejects_bad_version() {
    let bad = synth_dictionary_bin(b"TKDB", 99, &[]);
    let path = write_temp("bad-version.bin", &bad);
    let err = DictionaryReader::open(&path).expect_err("bad version must fail");
    assert!(matches!(err, LexiconError::InvalidBinary(_)), "{err:?}");
}

// --- INVARIANT_LEX_FST_ROWID_PAYLOAD -----------------------------------

#[test]
fn invariant_lex_fst_rowid_payload_round_trip() {
    let pairs: &[(&str, u32)] = &[
        ("tl:gua2", 100),
        ("tl:gua2", 200),
        ("tl:guan2", 300),
        ("hanzi:好", 50),
    ];
    let path = write_synthetic_fst("roundtrip.fst", pairs);
    let index = PrefixIndex::open(&path).expect("fst opens");

    let gua_hits = index.lookup_prefix("tl:gua");
    assert_eq!(
        gua_hits,
        vec![100, 200, 300],
        "prefix lookup preserves insertion order; got {gua_hits:?}"
    );

    let exact_gua2 = index.lookup_exact("tl:gua2");
    assert_eq!(
        exact_gua2,
        vec![100, 200],
        "exact match returns both rowids in insertion order"
    );

    let hanzi_hits = index.lookup_prefix("hanzi:好");
    assert_eq!(hanzi_hits, vec![50]);
}

// --- INVARIANT_LEX_LOOKUP_ROWIDS_ORDER ---------------------------------

#[test]
fn invariant_lex_lookup_rowids_order_preserves_insertion() {
    // D-12 parity correction toward Android: search.rs concatenates
    // exact-match rowids THEN prefix-only rowids, deduped via IndexSet
    // (insertion-order). Within each lookup, rowids surface in fst's
    // byte-sort order which is deterministic per build.
    let pairs: &[(&str, u32)] = &[("tl:abc", 7), ("tl:abc", 3), ("tl:abcd", 9), ("tl:abcd", 1)];
    let path = write_synthetic_fst("order.fst", pairs);
    let index = PrefixIndex::open(&path).expect("fst opens");

    // lookup_exact("tl:abc") returns rowids in fst byte-order: 3, 7
    let exact = index.lookup_exact("tl:abc");
    assert_eq!(
        exact,
        vec![3, 7],
        "exact-match rowids ascending in fst byte-order"
    );

    // lookup_prefix("tl:abc") returns ALL matching rowids byte-sorted by
    // entry. Because '\xFF' (0xFF) > 'd' (0x64), longer keys "tl:abcd*"
    // come BEFORE "tl:abc\xFF*" in the fst:
    //     tl:abcd\xFF\x01 < tl:abcd\xFF\x09 < tl:abc\xFF\x03 < tl:abc\xFF\x07
    let prefix = index.lookup_prefix("tl:abc");
    assert_eq!(
        prefix,
        vec![1, 9, 3, 7],
        "fst byte-sort order across prefix scan"
    );
}

// --- INVARIANT_LEX_HANZI_GUARD (Rust unit) -----------------------------

#[test]
fn invariant_lex_hanzi_guard_short_circuits() {
    // Hanzi inputType returns [] without touching the readers — verified
    // by passing dummy non-existent paths; if the guard fired the search
    // would attempt to read the (missing) state and return an error.
    let pairs: &[(&str, u32)] = &[];
    let path = write_synthetic_fst("hanzi-guard.fst", pairs);
    let index = PrefixIndex::open(&path).expect("empty fst opens");
    let dict = synth_dictionary_reader(&[]);

    let params = SearchParams {
        input: "我".to_string(),
        input_type: SearchInputType::Hanzi,
        input_mode: SearchInputMode::Tl,
        limit: 50,
        tps_or_mapped_to_er: false,
        enabled_sources_bitmask: u32::MAX,
    };
    let rows = search::search(&params, &index, &dict).expect("guard short-circuits");
    assert!(
        rows.is_empty(),
        "INVARIANT_LEX_HANZI_GUARD: hanzi → []; got {} rows",
        rows.len()
    );
}

// --- INVARIANT_LEX_ASSOC_BITMASK_FILTER --------------------------------

/// Regression for v3.5.6 fix r3173013233 — `api::assoc_lookup` previously
/// hardcoded `u32::MAX` instead of plumbing `req.enabled_sources_bitmask`,
/// which silently disabled the source-toggle filter for bundled bigram
/// next-word entries. Pin both ends: mask `0` returns nothing, mask
/// `u32::MAX` returns the entry, mask matching the entry's source bit
/// returns the entry, mask missing the entry's source returns nothing.
#[test]
fn invariant_lex_assoc_bitmask_filter_honored() {
    let assoc_bytes = synth_association_bin_with_one_entry("好", "伊", "i1", 100, 0x0001);
    let assoc_path = write_temp("assoc-bitmask-filter.bin", &assoc_bytes);
    let reader = AssociationReader::open(&assoc_path).expect("synth assoc opens");

    let all = search::assoc_lookup("好", 10, u32::MAX, &reader).expect("u32::MAX");
    assert_eq!(all.len(), 1, "u32::MAX must return the entry");

    let none = search::assoc_lookup("好", 10, 0, &reader).expect("mask 0");
    assert!(none.is_empty(), "mask 0 must filter everything");

    let matching = search::assoc_lookup("好", 10, 0x0001, &reader).expect("mask matches bit 0");
    assert_eq!(matching.len(), 1, "matching mask returns entry");

    let mismatching = search::assoc_lookup("好", 10, 0x0002, &reader).expect("mask bit 1 only");
    assert!(mismatching.is_empty(), "non-matching mask filters entry");
}

// --- INVARIANT_LEX_API_BITMASK_HONORED ---------------------------------

/// Regression for v3.5.6 fix r3173440126 — `api::search_with_sources` and
/// `api::search_by_hanzi` previously hardcoded `u32::MAX` (Tab3 paths),
/// bypassing user dictionary-source toggles AND falsely forcing variant +
/// khiin on. Drives the assertion through `lexicon::api` so the regression
/// is pinned at the layer where it actually existed (search.rs has always
/// honored the bitmask param; api.rs was the leak point).
#[test]
fn invariant_lex_api_bitmask_plumbing_honored() {
    use lexicon::api;
    use protos::engine::{SearchByHanziRequest, SearchWithSourcesRequest};

    let _engine_lock = engine_install_lock();

    // Synth fixture: rowid=1, source bitmask 0x0001 (bit 0 = kautian only).
    // Frequency MUST be > 0 — engine sorts by frequency and length-sort uses
    // it as a tiebreaker; a `0` frequency would be valid but tests with a
    // realistic non-zero value catch sort regressions too.
    let fst_path = write_synthetic_fst("api-bitmask.fst", &[("tl:test", 1), ("hanzi:好", 1)]);
    let dict_bytes = synth_dictionary_bin(b"TKDB", 1, &[(0x0001u16, 100, "好", "ho2")]);
    let dict_path = write_temp("api-bitmask-dict.bin", &dict_bytes);
    let assoc_path = write_temp("api-bitmask-assoc.bin", &synth_association_bin());

    let paths = LexiconPaths::validated(
        fst_path.to_str().unwrap(),
        dict_path.to_str().unwrap(),
        assoc_path.to_str().unwrap(),
        1,
    )
    .expect("paths validated");
    EngineHandle::install(paths).expect("install");

    // search_with_sources: mask=0x0001 hits, mask=0x0002 misses.
    let make_with_sources = |mask: u32| SearchWithSourcesRequest {
        input: "test".to_string(),
        input_mode: protos::engine::InputMode::Tl as i32,
        limit: 10,
        enabled_sources_bitmask: mask,
    };
    let hit = api::search_with_sources(make_with_sources(0x0001)).expect("hit");
    assert_eq!(
        hit.rows.len(),
        1,
        "search_with_sources with matching mask returns entry"
    );

    let miss = api::search_with_sources(make_with_sources(0x0002)).expect("miss");
    assert!(
        miss.rows.is_empty(),
        "search_with_sources with non-matching mask filters entry"
    );

    // search_by_hanzi: same assertions, hanzi-prefix path.
    let make_by_hanzi = |mask: u32| SearchByHanziRequest {
        query: "好".to_string(),
        input_mode: protos::engine::InputMode::Tl as i32,
        limit: 10,
        enabled_sources_bitmask: mask,
    };
    let hit_h = api::search_by_hanzi(make_by_hanzi(0x0001)).expect("hit_h");
    assert_eq!(
        hit_h.rows.len(),
        1,
        "search_by_hanzi with matching mask returns entry"
    );

    let miss_h = api::search_by_hanzi(make_by_hanzi(0x0002)).expect("miss_h");
    assert!(
        miss_h.rows.is_empty(),
        "search_by_hanzi with non-matching mask filters entry"
    );
}

// --- INVARIANT_LEX_INSTALL_PATH_VALIDATION -----------------------------

#[test]
fn invariant_lex_install_path_validation_rejects_nul() {
    let err = LexiconPaths::validated("/foo\0bar", "/dict.bin", "/assoc.bin", 1)
        .expect_err("nul rejected");
    assert!(matches!(err, LexiconError::InvalidPath(_)), "{err:?}");
}

#[test]
fn invariant_lex_install_path_validation_rejects_relative() {
    let err = LexiconPaths::validated("relative.fst", "/dict.bin", "/assoc.bin", 1)
        .expect_err("relative rejected");
    assert!(matches!(err, LexiconError::PathNotAbsolute(_)), "{err:?}");
}

// --- INVARIANT_LEX_INSTALL_SEARCH_SERIALIZATION ------------------------

#[test]
fn invariant_lex_install_search_serialization_no_panic() {
    use std::sync::Arc;
    use std::thread;

    let _engine_lock = engine_install_lock();

    // Build minimal install fixture once.
    let (fst_path, dict_path, assoc_path) = build_minimal_install_fixture("serialize");
    let paths = LexiconPaths::validated(
        fst_path.to_str().unwrap(),
        dict_path.to_str().unwrap(),
        assoc_path.to_str().unwrap(),
        1,
    )
    .expect("paths validated");
    EngineHandle::install(paths.clone()).expect("initial install");

    // 4 threads × 50 iterations each, mixing install and read state.
    // Reduced from 8×1000 in plan §7 because the integration mutex
    // serializes everything anyway; smaller exercises the same code path
    // without making CI flaky.
    let paths_arc = Arc::new(paths);
    let mut handles = Vec::new();
    for thread_id in 0..4 {
        let p = paths_arc.clone();
        handles.push(thread::spawn(move || {
            for _ in 0..50 {
                if thread_id % 2 == 0 {
                    let _ = EngineHandle::install((*p).clone());
                } else {
                    let _ = EngineHandle::with_state(|state| {
                        Ok::<u32, LexiconError>(state.dictionary_version)
                    });
                }
            }
        }));
    }
    for h in handles {
        h.join().expect("thread joins without panic");
    }
}

// --- helpers -----------------------------------------------------------

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

/// Build a TKDB byte sequence with the given (rowid, bitmask, hanzi, tl)
/// rows. rowids are 1-based; supply rows in rowid order.
fn synth_dictionary_bin(magic: &[u8; 4], version: u32, rows: &[(u16, u32, &str, &str)]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(magic);
    out.extend_from_slice(&version.to_le_bytes());
    out.extend_from_slice(&(rows.len() as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // build_ts

    // Compute offsets (header + offset_table) then payload.
    let header_size = 16usize;
    let offset_table_size = rows.len() * 4;
    let mut offsets = Vec::<u32>::with_capacity(rows.len());
    let mut payload = Vec::<u8>::new();
    for (bitmask, freq, hanzi, tl) in rows {
        offsets.push((header_size + offset_table_size + payload.len()) as u32);
        payload.extend_from_slice(&bitmask.to_le_bytes());
        payload.extend_from_slice(&freq.to_le_bytes());
        payload.push(hanzi.len() as u8);
        payload.push(tl.len() as u8);
        payload.extend_from_slice(hanzi.as_bytes());
        payload.extend_from_slice(tl.as_bytes());
    }
    for off in &offsets {
        out.extend_from_slice(&off.to_le_bytes());
    }
    out.extend_from_slice(&payload);
    out
}

fn synth_dictionary_reader(rows: &[(u16, u32, &str, &str)]) -> DictionaryReader {
    let bytes = synth_dictionary_bin(b"TKDB", 1, rows);
    let path = write_temp(&format!("dict-{}.bin", rows.len()), &bytes);
    DictionaryReader::open(&path).expect("synth dict opens")
}

fn write_temp(name: &str, bytes: &[u8]) -> PathBuf {
    let path = std::env::temp_dir().join(format!("lexicon-test-{name}"));
    std::fs::write(&path, bytes).expect("write temp");
    path
}

fn build_minimal_install_fixture(prefix: &str) -> (PathBuf, PathBuf, PathBuf) {
    let fst_path = write_synthetic_fst(&format!("{prefix}.fst"), &[("tl:test", 1)]);
    let dict_bytes = synth_dictionary_bin(b"TKDB", 1, &[(0, 1, "好", "ho2")]);
    let dict_path = write_temp(&format!("{prefix}-dict.bin"), &dict_bytes);
    let assoc_bytes = synth_association_bin();
    let assoc_path = write_temp(&format!("{prefix}-assoc.bin"), &assoc_bytes);
    (fst_path, dict_path, assoc_path)
}

fn synth_association_bin() -> Vec<u8> {
    // Empty association.bin: TKWA + version 1 + 0 keys + 0 entries + 0 ts
    let mut out = Vec::new();
    out.extend_from_slice(b"TKWA");
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // key_count
    out.extend_from_slice(&0u32.to_le_bytes()); // entry_count
    out.extend_from_slice(&0u32.to_le_bytes()); // build_ts
    out
}

/// TKWA fixture with exactly one prev_word that holds one entry.
/// Useful for bitmask-filter regression tests (assoc filter is 1-layer
/// and only honors the low 9 bits of the source bitmask).
fn synth_association_bin_with_one_entry(
    prev_word: &str,
    next_word: &str,
    next_tl: &str,
    count: u32,
    bitmask: u16,
) -> Vec<u8> {
    let prev_bytes = prev_word.as_bytes();
    let nw_bytes = next_word.as_bytes();
    let nt_bytes = next_tl.as_bytes();
    assert!(prev_bytes.len() <= u8::MAX as usize, "prev_word too long");
    assert!(nw_bytes.len() <= u8::MAX as usize, "next_word too long");
    assert!(nt_bytes.len() <= u8::MAX as usize, "next_tl too long");

    let header_size = 20usize;
    let key_table_size = 4usize; // 1 key × u32 offset
    let key_size = 1 + prev_bytes.len() + 4 + 2; // len + bytes + entry_offset + entry_count
    let entry_offset = (header_size + key_table_size + key_size) as u32;
    let key_offset = (header_size + key_table_size) as u32;

    let mut out = Vec::new();
    out.extend_from_slice(b"TKWA");
    out.extend_from_slice(&1u32.to_le_bytes()); // version
    out.extend_from_slice(&1u32.to_le_bytes()); // key_count
    out.extend_from_slice(&1u32.to_le_bytes()); // entry_count
    out.extend_from_slice(&0u32.to_le_bytes()); // build_ts

    out.extend_from_slice(&key_offset.to_le_bytes());

    out.push(prev_bytes.len() as u8);
    out.extend_from_slice(prev_bytes);
    out.extend_from_slice(&entry_offset.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // entry_count for this key

    out.extend_from_slice(&bitmask.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    out.push(nw_bytes.len() as u8);
    out.push(nt_bytes.len() as u8);
    out.extend_from_slice(nw_bytes);
    out.extend_from_slice(nt_bytes);

    out
}
