//! Pin the `dictionary.bin` v1 → v2 incompatibility error message so a
//! future error-text refactor can't quietly drop the rebuild diagnosis.

use lexicon::dictionary_reader::DictionaryReader;
use lexicon::LexiconError;

mod common;
use common::{build_tkdb_bin, write_temp, DictRow};

#[test]
fn invariant_lex_v1_rejected_with_v1_to_v2_marker() {
    let rows = [DictRow {
        bitmask: 0x0001,
        frequency: 1,
        syllable_count: None, // v1 layout: no syllable_count byte
        hanzi: "好",
        tl: "ho2",
    }];
    let bytes = build_tkdb_bin(b"TKDB", 1, &rows);
    let path = write_temp("rejects-v1.bin", &bytes);
    let err = DictionaryReader::open(&path).expect_err("v1 must be rejected");

    let LexiconError::InvalidBinary(msg) = &err else {
        panic!("expected InvalidBinary, got {err:?}");
    };
    assert!(
        msg.contains("v1→v2"),
        "v1→v2 marker missing from message: {msg}"
    );
    assert!(
        msg.contains("rebuild"),
        "operational guidance missing from message: {msg}"
    );
}

#[test]
fn invariant_lex_unrelated_version_uses_generic_message() {
    let bytes = build_tkdb_bin(b"TKDB", 99, &[]);
    let path = write_temp("rejects-v99.bin", &bytes);
    let err = DictionaryReader::open(&path).expect_err("v99 must be rejected");

    let LexiconError::InvalidBinary(msg) = &err else {
        panic!("expected InvalidBinary, got {err:?}");
    };
    assert!(
        !msg.contains("v1→v2"),
        "v1→v2 marker leaked onto unrelated-version error: {msg}"
    );
    assert!(
        msg.contains("unsupported version 99"),
        "raw version missing from generic message: {msg}"
    );
}
