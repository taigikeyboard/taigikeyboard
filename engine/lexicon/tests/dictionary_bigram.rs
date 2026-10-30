//! `dictionary.bin` v6 bigram section — `DictionaryReader::bigram_continuations` +
//! `bigram_pair_count`
//! lookups by rowid, shared model words, and malformed-section bounds.

use lexicon::dictionary_reader::DictionaryReader;
use test_support::{
    build_tkdb_with_bigrams, write_temp, TkdbBigrams, TkdbModel, TkdbRow, TKDB_VERSION,
};

const HEADER_SIZE: usize = 56;

fn row<'a>(hanji: &'a str, tl: &'a str) -> TkdbRow<'a> {
    TkdbRow {
        bitmask: 1,
        frequency: 1,
        syllable_count: Some(1),
        kautian_subtag: Some(0),
        walker_cost: Some(1_000),
        hanji,
        tl,
    }
}

// rowid 1 有/ū, 2 的/ê, 3 鞋/ê, 4 出來/tshut--lâi, 5 出來/tshut-lâi (same model
// word as 4), 6 八/pueh (in no pair).
fn rows() -> Vec<TkdbRow<'static>> {
    vec![
        row("有", "ū"),
        row("的", "ê"),
        row("鞋", "ê"),
        row("出來", "tshut--lâi"),
        row("出來", "tshut-lâi"),
        row("八", "pueh"),
    ]
}

// words: 0 有, 1 的, 2 鞋, 3 出來. 有 → {的 411, 鞋 2, 出來 7}; 出來 → {的 5}.
fn bigrams() -> TkdbBigrams {
    TkdbBigrams {
        discount: 0.75,
        word_of_row: vec![Some(0), Some(1), Some(2), Some(3), Some(3), None],
        pairs: vec![(0, 3, 7), (0, 1, 411), (3, 1, 5), (0, 2, 2)],
    }
}

/// `(c, C, N)` for `previous → next`; `None` when `previous` has no continuation.
fn counts(reader: &DictionaryReader, previous: u32, next: u32) -> Option<(u32, u32, u32)> {
    let continuations = reader.bigram_continuations(previous)?;
    Some((
        reader.bigram_pair_count(&continuations, next),
        continuations.total,
        continuations.distinct,
    ))
}

fn open(name: &str, bytes: &[u8]) -> DictionaryReader {
    DictionaryReader::open(&write_temp(name, bytes)).expect("v6 fixture opens")
}

fn fixture() -> Vec<u8> {
    build_tkdb_with_bigrams(
        b"TKDB",
        TKDB_VERSION,
        &rows(),
        TkdbModel::default(),
        &bigrams(),
    )
}

#[test]
fn seen_and_unseen_pairs_share_the_previous_words_totals() {
    let reader = open("bigram-lookup.bin", &fixture());
    assert_eq!(reader.walker_model().bigram_discount(), 0.75);
    // trace: C(有) = 411 + 2 + 7 = 420, N(有) = 3
    let after_u = |pair| (pair, 420, 3);
    assert_eq!(counts(&reader, 1, 2), Some(after_u(411)));
    assert_eq!(counts(&reader, 1, 3), Some(after_u(2)));
    assert_eq!(counts(&reader, 1, 4), Some(after_u(7)));
    assert_eq!(
        counts(&reader, 1, 1),
        Some(after_u(0)),
        "seen word, unseen pair"
    );
    assert_eq!(
        counts(&reader, 1, 6),
        Some(after_u(0)),
        "next word in no pair"
    );
    assert_eq!(
        counts(&reader, 1, 99),
        Some(after_u(0)),
        "next rowid out of range"
    );
    // Records are still read past the section.
    assert_eq!(reader.record(6).expect("rowid 6").tl, "pueh");
}

#[test]
fn spellings_of_one_model_word_share_counts() {
    let reader = open("bigram-model-word.bin", &fixture());
    let after = |rowid| counts(&reader, rowid, 2);
    assert_eq!(after(4), after(5));
    assert_eq!(after(4), Some((5, 5, 1)));
    assert_eq!(counts(&reader, 1, 4), counts(&reader, 1, 5));
}

#[test]
fn a_word_without_continuations_takes_no_penalty() {
    let reader = open("bigram-no-continuation.bin", &fixture());
    assert_eq!(counts(&reader, 2, 1), None, "的 only ever follows");
    assert_eq!(counts(&reader, 6, 1), None, "八 is in no pair");
    assert_eq!(counts(&reader, 0, 1), None, "rowid 0");
    assert_eq!(counts(&reader, 7, 1), None, "rowid past the records");
}

#[test]
fn an_empty_section_answers_none_everywhere() {
    let bytes = build_tkdb_with_bigrams(
        b"TKDB",
        TKDB_VERSION,
        &rows(),
        TkdbModel::default(),
        &TkdbBigrams::default(),
    );
    let reader = open("bigram-empty.bin", &bytes);
    for previous in 1..=6 {
        for next in 1..=6 {
            assert_eq!(counts(&reader, previous, next), None);
        }
    }
}

/// The shipped artifact carries the `ue5` pair the walker penalty needs
/// (`docs/architecture/bigram-lm-roadmap.md` D6): an unseen `有→鞋` and a
/// frequent `有→的`.
#[test]
fn production_dictionary_bin_carries_the_ue5_pairs() {
    let path = test_support::production_artifact("dictionary.bin");
    if !path.exists() {
        eprintln!(
            "[sentinel] {} missing — production asset not bundled here; skipping",
            path.display()
        );
        return;
    }
    let reader = DictionaryReader::open(&path).expect("open production dictionary.bin");
    let rowid = |hanji: &str, tl: &str| {
        (1..=reader.record_count())
            .find(|&rowid| {
                reader
                    .record(rowid)
                    .is_some_and(|r| r.hanji.as_deref() == Some(hanji) && r.tl == tl)
            })
            .unwrap_or_else(|| panic!("{hanji}/{tl} not in production dictionary.bin"))
    };
    let (u, e, shoe) = (rowid("有", "ū"), rowid("的", "ê"), rowid("鞋", "ê"));
    // trace: walker_lm.load_bigrams over word_bigrams.tsv minus taigi_typing —
    // C(有/u7) = 48 740, N = 3 370, c(有, 的/e5) = 411, c(有, 鞋/e5) = 0.
    let after_u = |pair| (pair, 48_740, 3_370);
    assert_eq!(reader.walker_model().bigram_discount(), 0.75);
    assert_eq!(counts(&reader, u, e), Some(after_u(411)));
    assert_eq!(counts(&reader, u, shoe), Some(after_u(0)));
    // 出來 has two spellings (`tshut--lâi` / `tshut-lâi`) and one model word.
    assert_eq!(
        counts(&reader, u, rowid("出來", "tshut--lâi")),
        counts(&reader, u, rowid("出來", "tshut-lâi"))
    );
}

#[test]
fn malformed_section_entries_yield_none_not_a_panic() {
    // trace: word_of_record at 56 + 6×4 = 80; words at 80 + 6×4 = 104.
    let word_of_record = HEADER_SIZE + 6 * 4;
    let words = word_of_record + 6 * 4;

    let mut bytes = fixture();
    bytes[word_of_record..word_of_record + 4].copy_from_slice(&4u32.to_le_bytes());
    let reader = open("bigram-word-id-out-of-range.bin", &bytes);
    assert_eq!(counts(&reader, 1, 2), None, "word id ≥ word count");

    let mut bytes = fixture();
    // 有's pair_len (word 0, bytes 4..8) runs past the 4 pairs.
    bytes[words + 4..words + 8].copy_from_slice(&5u32.to_le_bytes());
    let reader = open("bigram-pairs-out-of-range.bin", &bytes);
    assert_eq!(counts(&reader, 1, 2), None, "pairs past the table");
}
