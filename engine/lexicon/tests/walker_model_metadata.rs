//! The model travels with each loaded dictionary, independently of engine builds.

use lexicon::dictionary_reader::DictionaryReader;
use test_support::{build_tkdb_with_model, write_temp, TkdbModel, TkdbRow, TKDB_VERSION};

fn dictionary(model: TkdbModel) -> Vec<u8> {
    build_tkdb_with_model(
        b"TKDB",
        TKDB_VERSION,
        &[TkdbRow {
            bitmask: 1,
            frequency: 10,
            syllable_count: Some(1),
            kautian_subtag: Some(0),
            walker_cost: Some(547),
            hanji: "好",
            tl: "hó",
        }],
        model,
    )
}

#[test]
fn each_reader_uses_its_own_model_and_record_offsets() {
    let small = TkdbModel {
        tokens: 15,
        vocabulary: 4,
        alpha: 1.0,
    };
    let large = TkdbModel {
        tokens: 3_627_713,
        vocabulary: 167_922,
        alpha: 10.0,
    };
    let a = DictionaryReader::open(&write_temp("model-small.bin", &dictionary(small))).unwrap();
    let b = DictionaryReader::open(&write_temp("model-large.bin", &dictionary(large))).unwrap();
    // trace: −ln((0 + 1) / (15 + 1 × 4)) × 1000 = 2,944; (10 + 1) / 19 → 547
    assert_eq!(a.walker_model().cost_of_count(0), 2_944);
    assert_eq!(a.walker_model().cost_of_count(10), 547);
    assert_eq!(b.walker_model().cost_of_count(2_000), 7_879);
    assert_eq!(
        a.walker_model().cost_of_count(10),
        547,
        "opening b did not replace a's model"
    );
    assert_eq!(a.record(1).unwrap().tl, "hó");
    assert_eq!(b.record(1).unwrap().walker_cost, 547);
}

#[test]
fn record_offsets_cannot_point_into_the_model_header_or_bigram_section() {
    // trace: header 56 B, offset table 56..60, word_of_record 60..64, record at 64.
    for target in [16u32, 60] {
        let mut bytes = dictionary(TkdbModel::default());
        bytes[56..60].copy_from_slice(&target.to_le_bytes());
        let reader = DictionaryReader::open(&write_temp("offset-in-model.bin", &bytes)).unwrap();
        assert!(reader.record(1).is_none(), "offset {target}");
    }
}

#[test]
fn truncated_or_invalid_metadata_fails_before_querying() {
    let valid = dictionary(TkdbModel::default());
    for length in 16..56 {
        let path = write_temp("truncated-model.bin", &valid[..length]);
        assert!(DictionaryReader::open(&path)
            .unwrap_err()
            .to_string()
            .contains("metadata"));
    }
    for (offset, invalid) in [
        (24, 0u64.to_le_bytes()),
        (32, f64::NAN.to_le_bytes()),
        (40, 0f64.to_le_bytes()),
        (40, 1.5f64.to_le_bytes()),
        (40, f64::NAN.to_le_bytes()),
    ] {
        let mut bytes = valid.clone();
        bytes[offset..offset + 8].copy_from_slice(&invalid);
        let path = write_temp("invalid-model.bin", &bytes);
        assert!(DictionaryReader::open(&path)
            .unwrap_err()
            .to_string()
            .contains("metadata"));
    }
    let mut bytes = valid.clone();
    bytes.truncate(59); // Full model header but an incomplete offset table.
    let path = write_temp("truncated-model-offsets.bin", &bytes);
    assert!(DictionaryReader::open(&path)
        .unwrap_err()
        .to_string()
        .contains("offset table"));
    let mut bytes = valid;
    bytes.truncate(63); // Offset table complete, word_of_record cut short.
    let path = write_temp("truncated-bigram.bin", &bytes);
    assert!(DictionaryReader::open(&path)
        .unwrap_err()
        .to_string()
        .contains("bigram section"));
}
