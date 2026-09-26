//! No Hyphens (`AppConfig.hyphenless_roman`, USER 2026-09-20) integration test
//! (`INVARIANT_HYPHENLESS_ROMAN_DISPLAY_ONLY`). With the flag on, every
//! candidate's rendered `roman` drops the dictionary's inter-syllable `-`
//! and writes the neutral-tone marker `--` as `·` — dictionary and custom rows
//! alike — while the identity sidechannels the platform round-trips on
//! commit (`display_text`, `canonical_tl`) keep the dictionary form, and the
//! §34 literal keeps whatever the user typed. Off, nothing changes.
//!
//! Hermetic `LexiconHandle` install comes from `tests/common/mod.rs`.

use protos::engine::AppConfig;

mod common;
use common::Fetch;
use common::{
    build_dictionary_fst, build_syllables_fst, build_tkdb_v3, cell_with_hanji, config,
    empty_association_bin, engine_install_lock, fetch_cells, install_lexicon, write_temp, Cell,
    Row,
};
use lexicon::CustomEntry;

fn fixture_rows() -> Vec<Row> {
    vec![
        Row {
            toneless_key: "taiuan",
            hanzi: "台灣",
            tl: "tâi-uân",
            syll: 2,
            freq: 9000,
        },
        Row {
            toneless_key: "hoogua",
            hanzi: "予我",
            tl: "hōo--guá",
            syll: 2,
            freq: 9000,
        },
        Row {
            toneless_key: "tai",
            hanzi: "台",
            tl: "tâi",
            syll: 1,
            freq: 100,
        },
        Row {
            toneless_key: "uan",
            hanzi: "灣",
            tl: "uân",
            syll: 1,
            freq: 100,
        },
        Row {
            toneless_key: "hoo",
            hanzi: "予",
            tl: "hōo",
            syll: 1,
            freq: 100,
        },
        Row {
            toneless_key: "gua",
            hanzi: "我",
            tl: "guá",
            syll: 1,
            freq: 100,
        },
        Row {
            toneless_key: "so",
            hanzi: "鎖",
            tl: "só",
            syll: 1,
            freq: 100,
        },
        Row {
            toneless_key: "si",
            hanzi: "匙",
            tl: "sî",
            syll: 1,
            freq: 100,
        },
    ]
}

fn install_fixture() {
    let rows = fixture_rows();
    let dict_path = write_temp("dictionary.bin", &build_tkdb_v3(&rows));
    let fst_path = build_dictionary_fst(&rows);
    let assoc_path = write_temp("association.bin", &empty_association_bin());
    let syllables_path = build_syllables_fst(&["tai5", "uan5", "hoo7", "gua2", "so2", "si5"]);
    install_lexicon(&fst_path, &dict_path, &assoc_path, &syllables_path);
}

fn fetch(raw: &str, input_mode: &str, hyphenless: bool, custom: Vec<CustomEntry>) -> Vec<Cell> {
    let cfg = AppConfig {
        hyphenless_roman: hyphenless,
        ..config(input_mode)
    };
    let fetch = Fetch {
        custom,
        ..Default::default()
    };
    fetch_cells(&cfg, raw, fetch)
}

#[test]
fn hyphenless_roman_strips_the_dictionary_hyphen_but_keeps_the_identity_keys() {
    let _lock = engine_install_lock();
    install_fixture();
    let cells = fetch("taiuan", "tl", true, vec![]);
    let taiuan = cell_with_hanji(&cells, "台灣");
    assert_eq!(taiuan.1, "tâiuân", "rendered roman");
    assert_eq!(taiuan.2, "台灣", "display_text (詞頻 key) untouched");
    assert_eq!(
        taiuan.3, "tâi-uân",
        "canonical_tl (association key) untouched"
    );
    assert_eq!(cells[0].1, "taiuan", "the §34 literal is what was typed");
}

#[test]
fn hyphenless_roman_writes_the_khinsiann_marker_as_a_middle_dot() {
    let _lock = engine_install_lock();
    install_fixture();
    let cells = fetch("hoogua", "tl", true, vec![]);
    let hoogua = cell_with_hanji(&cells, "予我");
    assert_eq!(hoogua.1, "hōo\u{00b7}guá");
    assert_eq!(hoogua.3, "hōo--guá");
}

#[test]
fn hyphenless_roman_applies_after_the_poj_presentation_pass() {
    let _lock = engine_install_lock();
    install_fixture();
    let cells = fetch("taioan", "poj", true, vec![]);
    let taiuan = cell_with_hanji(&cells, "台灣");
    assert_eq!(taiuan.1, "tâioân", "POJ spelling, no hyphen");
    assert_eq!(taiuan.3, "tâi-uân", "canonical TL keeps the hyphen");
}

#[test]
fn hyphenless_roman_covers_custom_dictionary_rows() {
    let _lock = engine_install_lock();
    install_fixture();
    let custom = vec![CustomEntry {
        roman: "só-sî".into(),
        hanji: Some("鎖匙".into()),
    }];
    let cells = fetch("sosi", "tl", true, custom);
    let sosi = cell_with_hanji(&cells, "鎖匙");
    assert_eq!(sosi.1, "sósî");
    assert_eq!(sosi.3, "só-sî", "custom identity keeps the stored hyphen");
}

#[test]
fn hyphenless_roman_leaves_a_typed_hyphen_in_the_literal_alone() {
    let _lock = engine_install_lock();
    install_fixture();
    let cells = fetch("tai-uan", "tl", true, vec![]);
    assert_eq!(cells[0].1, "tai-uan", "literal renders the typed hyphen");
    assert_eq!(cells[0].0, None);
    let taiuan = cell_with_hanji(&cells, "台灣");
    assert_eq!(taiuan.1, "tâiuân", "dictionary row is still hyphenless");
}

#[test]
fn hyphenless_roman_off_renders_the_dictionary_form() {
    let _lock = engine_install_lock();
    install_fixture();
    let on = fetch("hoogua", "tl", true, vec![]);
    let off = fetch("hoogua", "tl", false, vec![]);
    assert_eq!(cell_with_hanji(&off, "予我").1, "hōo--guá");
    // Same rows, same order, same identity — only `roman` differs.
    assert_eq!(on.len(), off.len());
    for (a, b) in on.iter().zip(&off) {
        assert_eq!((&a.0, &a.2, &a.3), (&b.0, &b.2, &b.3), "{a:?} vs {b:?}");
    }
}
