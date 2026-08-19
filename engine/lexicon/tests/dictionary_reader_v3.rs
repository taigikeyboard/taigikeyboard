//! `dictionary.bin` v3 — round-trip and bounds tests for the record layout:
//! `syllable_count` u8 (v2) + `kautian_subtag` u16 (v3) between `tl_len` and
//! the `hanzi` payload.

use lexicon::dictionary_reader::DictionaryReader;

mod common;
use common::{build_tkdb_v3, build_tkdb_v3_subtag, write_temp};

const HEADER_SIZE: usize = 16;

#[test]
fn invariant_lex_v3_round_trip_syllable_count() {
    let rows: &[(u16, u32, u8, &str, &str)] = &[
        (0x0001, 100, 1, "紙", "tsuá"),
        (0x0001, 200, 2, "珠仔", "tsu-á"),
        (0x0010, 50, 3, "", "tai-uan-ue"),
    ];
    let bytes = build_tkdb_v3(b"TKDB", rows);
    let path = write_temp("dictionary-reader-v3-roundtrip.bin", &bytes);
    let reader = DictionaryReader::open(&path).expect("v3 binary opens");
    assert_eq!(reader.record_count(), 3, "header record_count");

    let r1 = reader.record(1).expect("rowid 1");
    assert_eq!(r1.syllable_count, 1, "single-syllable");
    assert_eq!(r1.kautian_subtag, 0, "subtag default 0");
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
fn invariant_lex_v3_round_trip_kautian_subtag() {
    // (bitmask, freq, syllable_count, kautian_subtag, hanzi, tl)
    // subtag layout: bit 0 = main, bits 1..=10 = accent, bit 11 = name.
    let rows: &[(u16, u32, u8, u16, &str, &str)] = &[
        (0x0001, 100, 1, 0b0000_0000_0001, "詞", "su"), // main only
        (0x0001, 90, 1, 0b1000_0000_0000, "姓", "senn"), // name only
        (0x0001, 80, 1, 0b0000_0000_0110, "八", "pueh"), // accent bits 0,1
        (0x0001, 70, 1, 0b1000_0000_0001, "王", "ong"), // main + name
    ];
    let bytes = build_tkdb_v3_subtag(b"TKDB", rows);
    let path = write_temp("dictionary-reader-v3-subtag.bin", &bytes);
    let reader = DictionaryReader::open(&path).expect("v3 binary opens");

    assert_eq!(reader.record(1).unwrap().kautian_subtag, 0b0000_0000_0001);
    assert_eq!(reader.record(2).unwrap().kautian_subtag, 0b1000_0000_0000);
    assert_eq!(reader.record(3).unwrap().kautian_subtag, 0b0000_0000_0110);
    assert_eq!(reader.record(4).unwrap().kautian_subtag, 0b1000_0000_0001);
}

#[test]
fn invariant_lex_v3_masks_reserved_subtag_bits_on_read() {
    // Reserved bits 12-15 must be masked off on read so a future writer
    // setting them can never corrupt the subcollection filter AND.
    let rows: &[(u16, u32, u8, u16, &str, &str)] = &[(0x0001, 100, 1, 0xF001, "詞", "su")]; // reserved bits + main
    let bytes = build_tkdb_v3_subtag(b"TKDB", rows);
    let path = write_temp("dictionary-reader-v3-reserved.bin", &bytes);
    let reader = DictionaryReader::open(&path).expect("v3 binary opens");
    assert_eq!(
        reader.record(1).unwrap().kautian_subtag,
        0x0001,
        "reserved bits 12-15 masked off; only the main bit survives"
    );
}

#[test]
fn invariant_lex_v3_truncated_record_returns_none() {
    let rows: &[(u16, u32, u8, &str, &str)] = &[(0, 0, 1, "好", "ho2")];
    let mut bytes = build_tkdb_v3(b"TKDB", rows);
    bytes.pop();
    let path = write_temp("dictionary-reader-v3-truncated.bin", &bytes);
    let reader = DictionaryReader::open(&path).expect("opens with valid header");
    assert!(reader.record(1).is_none(), "truncated payload → None");
}

#[test]
fn invariant_lex_v3_min_record_size_guard() {
    // Forge a v3 header + single-entry offset table whose offset points to the
    // first byte after the offset table itself, with no payload bytes
    // (record_end == record_offset, below the 11-byte fixed prefix).
    // `record(1)` must return `None` rather than panic.
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"TKDB");
    bytes.extend_from_slice(&3u32.to_le_bytes()); // version
    bytes.extend_from_slice(&1u32.to_le_bytes()); // count
    bytes.extend_from_slice(&0u32.to_le_bytes()); // build_ts
    bytes.extend_from_slice(&((HEADER_SIZE + 4) as u32).to_le_bytes()); // offset[0]

    let path = write_temp("dictionary-reader-v3-min-guard.bin", &bytes);
    let reader = DictionaryReader::open(&path).expect("header + offset table only");
    assert!(
        reader.record(1).is_none(),
        "0-byte payload below 11-byte prefix"
    );
}
