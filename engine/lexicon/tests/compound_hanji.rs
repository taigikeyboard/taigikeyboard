//! §10.2 — `compound_hanji_exists` hermetic regression.
//!
//! Locks the dictionary-compound oracle the composing render/commit join
//! (`composing::api::nailed_prefix`) uses to decide whether a contiguous
//! run of manually-nailed single-syllable segments reconstructs a known
//! n-syllable compound (查某 → `tsa-bóo`, 紅尾冬 → `Âng-bóe-tang`) and
//! therefore renders its internal boundaries as hyphens. Synthetic FST
//! + dict.bin so the contract does not depend on shipped dictionary
//! data.
//!
//! v3.5.9 extended the v3.5.8 §10.2 Option A bigram-only oracle to
//! parametric `syllable_count` so the caller's longest-match loop can
//! ask "is `hanji` an n-syllable compound?" for any `n >= 2`.

mod common;

use std::path::PathBuf;

use common::{build_tkdb_v3, write_temp};
use lexicon::compound_hanji_exists;
use lexicon::dictionary_reader::DictionaryReader;
use lexicon::prefix_index::PrefixIndex;

const SEPARATOR: u8 = 0xFF;

/// `(key, rowid)` → FST `key || 0xFF || rowid_le_4`. `lookup_exact`
/// resolves `rowid`; `DictionaryReader::record` is 1-based, so FST
/// rowid `N` maps to `build_tkdb_v3` row index `N-1`.
fn write_synthetic_fst(name: &str, pairs: &[(&str, u32)]) -> PathBuf {
    use fst::SetBuilder;
    use std::sync::atomic::{AtomicU64, Ordering};
    // Per-process atomic counter + pid namespacing so concurrent tests
    // within the same `cargo test` binary cannot race on the same path
    // (mirrors `common::write_temp` + `tests/span_local_fetch.rs:155-156`).
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let pid = std::process::id();
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("lexicon-test-{name}-{pid}-{n}"));
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

#[test]
fn compound_hanji_exists_contract_matrix() {
    // rowid (1-based) ↔ row index:
    //   1 查某   syll 2  → the reported compound (true)
    //   2 逐家   syll 1  → syllable_count gate (false even though 2 CJK)
    //   3 查某人 syll 3  → 3-syll word, NOT a 2-syll bigram (false) and
    //                       proves lookup_exact("hanzi:查某") parity does
    //                       not bleed into the longer sibling key
    //   4 麻煩   syll 2  → second positive, distinct from row 1
    //   5 好     syll 2  → reached via a *mismatched* FST key (drift sim)
    let fst_path = write_synthetic_fst(
        "compound-hanji.fst",
        &[
            ("hanzi:查某", 1),
            ("hanzi:逐家", 2),
            ("hanzi:查某人", 3),
            ("hanzi:麻煩", 4),
            // Intentionally points at rowid 5 whose record.hanzi == "好":
            // exercises the defensive `record.hanzi == Some(hanji)`
            // re-check (wrong-rowid / future index drift).
            ("hanzi:壞", 5),
        ],
    );
    let dict_bytes = build_tkdb_v3(
        b"TKDB",
        &[
            (0x0001u16, 500u32, 2u8, "查某", "tsa-bóo"),
            (0x0001u16, 400u32, 1u8, "逐家", "ta̍k-ke"),
            (0x0001u16, 300u32, 3u8, "查某人", "tsa-bóo-lâng"),
            (0x0001u16, 200u32, 2u8, "麻煩", "mâ-huân"),
            (0x0001u16, 100u32, 2u8, "好", "hó"),
        ],
    );
    let dict_path = write_temp("compound-hanji-dict.bin", &dict_bytes);
    let prefix = PrefixIndex::open(&fst_path).expect("open synthetic fst");
    let dict = DictionaryReader::open(&dict_path).expect("open synthetic dict");

    // Direct FST length-parity pin (Codex post-impl 2026-05-18): prove
    // `lookup_exact` does NOT bleed the exact "hanzi:查某" key into the
    // longer sibling "hanzi:查某人", and that a single-hanji prefix of a
    // compound key has no exact entry. Asserted on the raw rowid set so
    // the parity claim does not rest on `compound_hanji_exists` passing
    // via its own positive rowid / defensive re-check.
    let mut tsaboo_rowids = prefix.lookup_exact("hanzi:查某");
    tsaboo_rowids.sort_unstable();
    assert_eq!(
        tsaboo_rowids,
        vec![1u32],
        "exact key must exclude 查某人 (rowid 3)"
    );
    assert_eq!(prefix.lookup_exact("hanzi:查某人"), vec![3u32]);
    assert!(
        prefix.lookup_exact("hanzi:查").is_empty(),
        "single-hanji prefix has no exact entry"
    );

    // Positive — exact 2-syllable hanji compounds with n=2.
    assert!(compound_hanji_exists("查某", 2, &prefix, &dict));
    assert!(compound_hanji_exists("麻煩", 2, &prefix, &dict));

    // syllable_count gate: a real 2-CJK entry that is NOT two TL
    // syllables must NOT auto-hyphenate (Codex pre-impl N2).
    assert!(!compound_hanji_exists("逐家", 2, &prefix, &dict));

    // 3-syllable word with n=3 → positive (v3.5.9 longest-match
    // extension). Same word with n=2 → negative: it is not a 2-syll
    // compound, and the parity pin above confirms its key does not
    // bleed into "hanzi:查某" either.
    assert!(compound_hanji_exists("查某人", 3, &prefix, &dict));
    assert!(!compound_hanji_exists("查某人", 2, &prefix, &dict));

    // 2-syll word with n=3 → negative (asymmetric: a record with
    // syllable_count==2 is not an n=3 compound).
    assert!(!compound_hanji_exists("查某", 3, &prefix, &dict));

    // n < 2 short-circuit: every input returns false at n=1 / n=0.
    assert!(!compound_hanji_exists("查某", 1, &prefix, &dict));
    assert!(!compound_hanji_exists("查某", 0, &prefix, &dict));

    // Absent key.
    assert!(!compound_hanji_exists("無彩", 2, &prefix, &dict));

    // Fn-level: a single-hanji prefix of a compound key resolves to
    // nothing (the raw-rowid parity is pinned directly above).
    assert!(!compound_hanji_exists("查", 2, &prefix, &dict));

    // Defensive re-check: key resolves to a record whose hanzi is
    // "好", not "壞" → rejected despite syllable_count == 2.
    assert!(!compound_hanji_exists("壞", 2, &prefix, &dict));
}
