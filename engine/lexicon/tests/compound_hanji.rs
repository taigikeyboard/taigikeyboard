//! §10.2 — `compound_hanji_exists` hermetic regression.
//!
//! Locks the dictionary-compound oracle the composing render/commit join
//! (`composing::api::nailed_prefix`) uses to decide whether a contiguous
//! run of manually-nailed single-syllable segments reconstructs a known
//! n-syllable compound (查某 → `tsa-bóo`, 紅尾冬 → `Âng-bóe-tang`) and
//! therefore renders its internal boundaries as hyphens. Synthetic FST +
//! dict.bin so the contract does not depend on shipped dictionary
//! data.
//!
//! v3.5.9 extended the v3.5.8 §10.2 Option A bigram-only oracle to
//! parametric `syllable_count` so the caller's longest-match loop can
//! ask "is `hanji` an n-syllable compound?" for any `n >= 2`.

use crate::common::{build_tkdb_v4, write_synthetic_fst};
use lexicon::compound_hanji_exists;
use lexicon::dictionary_reader::{DictionaryReader, Filter};
use lexicon::prefix_index::PrefixIndex;
use test_support::write_temp;

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
            // Intentionally points at rowid 5 whose record.hanji == "好":
            // exercises the defensive `record.hanji == Some(hanji)`
            // re-check (wrong-rowid / future index drift).
            ("hanzi:壞", 5),
        ],
    );
    let dict_bytes = build_tkdb_v4(
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

    // Every source on: this matrix pins the Hanji / syllable-count gates.
    let exists = |hanji: &str, reading: &str, syllable_count: u8| {
        let every_source = Filter::from_enabled_bitmask(u32::MAX);
        compound_hanji_exists(
            hanji,
            reading,
            syllable_count,
            &every_source,
            &prefix,
            &dict,
        )
    };

    // Positive — exact 2-syllable hanji compounds with n=2.
    assert!(exists("查某", "tsa-bóo", 2));
    assert!(exists("麻煩", "mâ-huân", 2));

    // syllable_count gate: a real 2-CJK entry that is NOT two TL
    // syllables must NOT auto-hyphenate (Codex pre-impl N2).
    assert!(!exists("逐家", "ta̍k-ke", 2));

    // 3-syllable word with n=3 → positive (v3.5.9 longest-match
    // extension). Same word with n=2 → negative: it is not a 2-syll
    // compound, and the parity pin above confirms its key does not
    // bleed into "hanzi:查某" either.
    assert!(exists("查某人", "tsa-bóo-lâng", 3));
    assert!(!exists("查某人", "tsa-bóo-lâng", 2));

    // 2-syll word with n=3 → negative (asymmetric: a record with
    // syllable_count==2 is not an n=3 compound).
    assert!(!exists("查某", "tsa-bóo", 3));

    // n < 2 short-circuit: every input returns false at n=1 / n=0.
    assert!(!exists("查某", "tsa-bóo", 1));
    assert!(!exists("查某", "tsa-bóo", 0));

    // Absent key.
    assert!(!exists("無彩", "bô-tshái", 2));

    // Fn-level: a single-hanji prefix of a compound key resolves to
    // nothing (the raw-rowid parity is pinned directly above).
    assert!(!exists("查", "tsa", 2));

    // Defensive re-check: key resolves to a record whose hanji is
    // "好", not "壞" → rejected despite syllable_count == 2.
    assert!(!exists("壞", "hó", 2));
}

#[test]
fn compound_hanji_exists_needs_the_picked_reading_under_an_enabled_source() {
    // #772: 俺娘 lives only in two sources a default user has off — one
    // row per source, each with its own reading — and 查某 in an enabled one.
    const SOURCE_ON: u16 = 1 << 1;
    const SOURCE_A: u16 = 1 << 3;
    const SOURCE_B: u16 = 1 << 4;
    let fst_path = write_synthetic_fst(
        "compound-hanji-sources.fst",
        &[("hanzi:俺娘", 1), ("hanzi:俺娘", 2), ("hanzi:查某", 3)],
    );
    let dict_bytes = build_tkdb_v4(
        b"TKDB",
        &[
            (SOURCE_A, 300u32, 2u8, "俺娘", "án-nia"),
            (SOURCE_B, 200u32, 2u8, "俺娘", "án-niâ"),
            (SOURCE_ON, 500u32, 2u8, "查某", "tsa-bóo"),
        ],
    );
    let dict_path = write_temp("compound-hanji-sources-dict.bin", &dict_bytes);
    let prefix = PrefixIndex::open(&fst_path).expect("open synthetic fst");
    let dict = DictionaryReader::open(&dict_path).expect("open synthetic dict");
    let exists = |hanji: &str, reading: &str, enabled_sources_bitmask: u32| {
        let filter = Filter::from_enabled_bitmask(enabled_sources_bitmask);
        compound_hanji_exists(hanji, reading, 2, &filter, &prefix, &dict)
    };

    // Every source on: the picks' reading matches source B's row.
    assert!(exists("俺娘", "ánniâ", u32::MAX));
    // Both 俺娘 sources off: no compound, the run joins with a space.
    assert!(!exists("俺娘", "ánniâ", u32::from(SOURCE_ON)));
    assert!(exists("查某", "tsabóo", u32::from(SOURCE_ON)));
    // Source B on: its reading matches; source A's reading is a word only
    // a disabled source carries.
    assert!(exists("俺娘", "ánniâ", u32::from(SOURCE_B)));
    assert!(!exists("俺娘", "ánnia", u32::from(SOURCE_B)));
    assert!(exists("俺娘", "ánnia", u32::from(SOURCE_A)));

    // Word identity (Core Principle #6): the same Hanji under another
    // reading is another word — 查 `tshâ` + 某 `bóo` is not 查某 `tsa-bóo`.
    assert!(!exists("查某", "tshâbóo", u32::MAX));
    // Tone-preserving: a different tone is a different reading.
    assert!(!exists("查某", "tsābóo", u32::MAX));
    // Separators in the reading do not matter.
    assert!(exists("查某", "tsa bóo", u32::MAX));
    // A run whose picks carry no reading matches nothing.
    assert!(!exists("查某", "", u32::MAX));
}
