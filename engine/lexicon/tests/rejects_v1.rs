//! Pin the `dictionary.bin` v1–v5 → v6 incompatibility error message so a
//! future error-text refactor can't quietly drop the rebuild diagnosis.

use lexicon::dictionary_reader::DictionaryReader;
use lexicon::LexiconError;

use crate::common::build_tkdb_legacy;
use test_support::{build_tkdb, write_temp, TkdbRow};

/// Opens `bytes` and asserts the reader rejects them with the rebuild marker.
fn assert_rejected_with_rebuild_marker(name: &str, bytes: &[u8]) {
    let path = write_temp(name, bytes);
    let err = DictionaryReader::open(&path).expect_err("older layout must be rejected");

    let LexiconError::InvalidBinary(msg) = &err else {
        panic!("expected InvalidBinary, got {err:?}");
    };
    assert!(
        msg.contains("v1/v2/v3/v4/v5→v6"),
        "v1/v2/v3/v4/v5→v6 marker missing from message: {msg}"
    );
    assert!(
        msg.contains("rebuild"),
        "operational guidance missing from message: {msg}"
    );
}

#[test]
fn invariant_lex_v1_rejected_with_legacy_to_v6_marker() {
    let rows = [TkdbRow {
        bitmask: 0x0001,
        frequency: 1,
        syllable_count: None, // v1 layout: no syllable_count byte
        kautian_subtag: None, // v1 layout: no subtag bytes
        walker_cost: None,    // v1 layout: no walker-cost bytes
        hanji: "好",
        tl: "ho2",
    }];
    assert_rejected_with_rebuild_marker("rejects-v1.bin", &build_tkdb(b"TKDB", 1, &rows));
}

#[test]
fn invariant_lex_v2_through_v4_rejected_with_legacy_to_v6_marker() {
    // The older record layouts and v4's unchanged records all lack the v5
    // model header. None can be loaded by the v6 reader.
    for version in [2, 3, 4] {
        let bytes = build_tkdb_legacy(b"TKDB", version, &[(0x0001, 1, 1, "好", "ho2")]);
        assert_rejected_with_rebuild_marker(&format!("rejects-v{version}.bin"), &bytes);
    }
}

#[test]
fn invariant_lex_v5_rejected_with_legacy_to_v6_marker() {
    // v5 has the model header but no bigram section.
    let bytes = build_tkdb(b"TKDB", 5, &[]);
    assert_rejected_with_rebuild_marker("rejects-v5.bin", &bytes);
}

#[test]
fn invariant_lex_unrelated_version_uses_generic_message() {
    let bytes = build_tkdb(b"TKDB", 99, &[]);
    let path = write_temp("rejects-v99.bin", &bytes);
    let err = DictionaryReader::open(&path).expect_err("v99 must be rejected");

    let LexiconError::InvalidBinary(msg) = &err else {
        panic!("expected InvalidBinary, got {err:?}");
    };
    assert!(
        !msg.contains("v1/v2/v3/v4/v5→v6"),
        "v1/v2/v3/v4/v5→v6 marker leaked onto unrelated-version error: {msg}"
    );
    assert!(
        msg.contains("unsupported version 99"),
        "raw version missing from generic message: {msg}"
    );
}
