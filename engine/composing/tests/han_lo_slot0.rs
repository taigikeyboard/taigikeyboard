//! A roman-only dictionary word inside a whole-buffer path keeps the sentence's
//! Hanji: slot 0 is Han-Lo mixed (`我má-tsiah`), the word in the mode's romanization.
//! Hermetic `LexiconHandle` install comes from `tests/common/mod.rs`.

use crate::common::{
    build_dictionary_fst, build_syllables_fst, build_tkdb_v4, config, empty_association_bin,
    fetch_at_pos_response, install_lexicon, Fetch, Row,
};
use test_support::{engine_install_lock, write_temp};

fn install_fixture() {
    let rows = vec![
        Row {
            toneless_key: "gua",
            hanji: "我",
            tl: "guá",
            syll: 1,
            freq: 9000,
        },
        Row {
            toneless_key: "matsiah",
            hanji: "",
            tl: "má-tsiah",
            syll: 2,
            freq: 25,
        },
    ];
    let dict_path = write_temp("dictionary.bin", &build_tkdb_v4(&rows));
    let fst_path = build_dictionary_fst(&rows);
    let association_path = write_temp("association.bin", &empty_association_bin());
    let syllables_path = build_syllables_fst(&["gua2", "ma2", "tsiah4", "ma1", "gua1"]);
    install_lexicon(&fst_path, &dict_path, &association_path, &syllables_path);
}

fn whole_buffer_hanji(raw: &str, input_mode: &str) -> Vec<Option<String>> {
    fetch_at_pos_response(&config(input_mode), raw, Fetch::default())
        .continuous
        .map(|continuous| continuous.candidates)
        .unwrap_or_default()
        .into_iter()
        .filter(|candidate| candidate.consumed_span_end as usize == raw.len())
        .filter(|candidate| candidate.roman != raw)
        .map(|candidate| candidate.hanji)
        .collect()
}

#[test]
fn tl_sentence_keeps_hanji_around_a_roman_only_word() {
    let _lock = engine_install_lock();
    install_fixture();
    let hanji = whole_buffer_hanji("guamatsiah", "tl");
    assert!(
        hanji.contains(&Some("我má-tsiah".to_owned())),
        "got {hanji:?}"
    );
}

#[test]
fn poj_sentence_renders_the_roman_only_word_in_poj() {
    let _lock = engine_install_lock();
    install_fixture();
    let hanji = whole_buffer_hanji("goamachiah", "poj");
    assert!(
        hanji.contains(&Some("我má-chiah".to_owned())),
        "got {hanji:?}"
    );
}

#[test]
fn roman_only_word_alone_has_no_hanji() {
    let _lock = engine_install_lock();
    install_fixture();
    let hanji = whole_buffer_hanji("matsiah", "tl");
    assert!(
        !hanji.is_empty() && hanji.iter().all(Option::is_none),
        "got {hanji:?}"
    );
}
