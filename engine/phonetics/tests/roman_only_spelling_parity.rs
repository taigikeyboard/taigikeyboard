//! `phonetics::api::is_roman_only_spelling` against the dictionary pipeline's
//! `dictionary/common/cleanup.py` `is_roman_only_hanzi`: both answer the
//! shared `roman_only_spelling_cases.tsv` table (the pipeline's half is
//! `dictionary/tests/test_roman_only_spelling_cases.py`), and no shipped row
//! keeps a Hanji the port calls roman-only — the pipeline blanks those, so the
//! custom dictionary, which stores through the port, stores the same word the
//! same way.

use phonetics::api::is_roman_only_spelling;
use test_support::{dictionary_csv_or_skip, dictionary_roman_only_cases_path};

#[test]
fn the_port_answers_the_shared_case_table() {
    let path = dictionary_roman_only_cases_path();
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    let mut lines = source.lines();
    assert_eq!(lines.next(), Some("hanzi\troman_only"));
    let mut cases = 0;
    for line in lines {
        let (hanzi, expected) = line
            .split_once('\t')
            .unwrap_or_else(|| panic!("not a `hanzi<TAB>roman_only` row: {line:?}"));
        let expected = match expected {
            "true" => true,
            "false" => false,
            other => panic!("roman_only is `true` or `false`, not {other:?}"),
        };
        assert_eq!(is_roman_only_spelling(hanzi), expected, "{hanzi:?}");
        cases += 1;
    }
    assert!(cases > 0, "an empty case table proves nothing");
}

#[test]
fn no_shipped_row_keeps_a_roman_only_spelling_as_hanji() {
    let Some(csv) = dictionary_csv_or_skip("roman-only spelling") else {
        return;
    };
    let kept: Vec<&str> = csv
        .select(["hanzi"])
        .map(|[hanzi]| hanzi)
        .filter(|hanzi| is_roman_only_spelling(hanzi))
        .collect();
    assert!(kept.is_empty(), "the pipeline blanks these: {kept:?}");
}
