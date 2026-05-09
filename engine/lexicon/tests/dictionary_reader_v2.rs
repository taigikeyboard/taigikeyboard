//! `dictionary.bin` v2 — round-trip and bounds tests for the
//! `syllable_count` byte added between `tl_len` and the `hanzi` payload.

use lexicon::dictionary_reader::DictionaryReader;

mod common;
use common::{build_tkdb_v2, write_temp};

const HEADER_SIZE: usize = 16;

#[test]
fn invariant_lex_v2_round_trip_syllable_count() {
    let rows: &[(u16, u32, u8, &str, &str)] = &[
        (0x0001, 100, 1, "紙", "tsuá"),
        (0x0001, 200, 2, "珠仔", "tsu-á"),
        (0x0010, 50, 3, "", "tai-uan-ue"),
    ];
    let bytes = build_tkdb_v2(b"TKDB", rows);
    let path = write_temp("dictionary-reader-v2-roundtrip.bin", &bytes);
    let reader = DictionaryReader::open(&path).expect("v2 binary opens");
    assert_eq!(reader.record_count(), 3, "header record_count");

    let r1 = reader.record(1).expect("rowid 1");
    assert_eq!(r1.syllable_count, 1, "single-syllable");
    assert_eq!(r1.tl, "tsuá");
    assert_eq!(r1.hanzi.as_deref(), Some("紙"));

    let r2 = reader.record(2).expect("rowid 2");
    assert_eq!(r2.syllable_count, 2, "multi-syllable");
    assert_eq!(r2.tl, "tsu-á");
    assert_eq!(r2.hanzi.as_deref(), Some("珠仔"));

    let r3 = reader.record(3).expect("rowid 3");
    assert_eq!(r3.syllable_count, 3);
    assert!(r3.hanzi.is_none(), "no hanzi → None");
    assert_eq!(r3.tl, "tai-uan-ue");
}

#[test]
fn invariant_lex_v2_truncated_record_returns_none() {
    let rows: &[(u16, u32, u8, &str, &str)] = &[(0, 0, 1, "好", "ho2")];
    let mut bytes = build_tkdb_v2(b"TKDB", rows);
    bytes.pop();
    let path = write_temp("dictionary-reader-v2-truncated.bin", &bytes);
    let reader = DictionaryReader::open(&path).expect("opens with valid header");
    assert!(reader.record(1).is_none(), "truncated payload → None");
}

#[test]
fn invariant_lex_v2_min_record_size_guard() {
    // Forge a header + single-entry offset table whose offset points to the
    // first byte after the offset table itself, with no payload bytes
    // (record_end == record_offset, below the 9-byte fixed prefix).
    // `record(1)` must return `None` rather than panic.
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"TKDB");
    bytes.extend_from_slice(&2u32.to_le_bytes()); // version
    bytes.extend_from_slice(&1u32.to_le_bytes()); // count
    bytes.extend_from_slice(&0u32.to_le_bytes()); // build_ts
    bytes.extend_from_slice(&((HEADER_SIZE + 4) as u32).to_le_bytes()); // offset[0]

    let path = write_temp("dictionary-reader-v2-min-guard.bin", &bytes);
    let reader = DictionaryReader::open(&path).expect("header + offset table only");
    assert!(
        reader.record(1).is_none(),
        "0-byte payload below 9-byte prefix"
    );
}
