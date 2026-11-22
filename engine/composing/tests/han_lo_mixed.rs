//! Hàn-Lô lām (`AppConfig.han_lo_mixed`): every dictionary word also offers its
//! romanization as a word of its own — identity `(canonical-TL,
//! canonical-TL)`, its own selection weight, a prior from its best Hanji
//! homophone — in the list under any display but Romanization Only, and in the
//! walker's whole-sentence path (`lexicon::continuous::han_lo`).
//!
//! Fixture costs are `ln(T / (1 + freq))` milli-nats: 食 9000 → 隻 5000 is
//! +588, → 𤆬 3000 is +1099, so the `tsia̍h` romanization (食 + 1000) sorts
//! between 隻 and 𤆬. Hanji rows carry the kautian bit; the LKK `kah` row the
//! LKK bit, so only it is a recommendation.
//!
//! Hermetic `LexiconHandle` install comes from `tests/common/mod.rs`.

use protos::engine::{AppConfig, CandidateDisplayMode, CellScript};

use lexicon::LearnedEntry;

use crate::common::{
    build_dictionary_fst, build_syllables_fst, build_tkdb_v4_sourced, config_with_display_mode,
    empty_association_bin, fetch_at_pos_response, install_lexicon, selected, Fetch, Row, NOW_MS,
};
use test_support::{engine_install_lock, write_temp};

const KAUTIAN: u16 = 1 << 0;
const LKK: u16 = 1 << 11;

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
            toneless_key: "tsiah",
            hanji: "食",
            tl: "tsia̍h",
            syll: 1,
            freq: 9000,
        },
        Row {
            toneless_key: "tsiah",
            hanji: "𤆬",
            tl: "tsia̍h",
            syll: 1,
            freq: 3000,
        },
        Row {
            toneless_key: "tsiah",
            hanji: "隻",
            tl: "tsiah",
            syll: 1,
            freq: 5000,
        },
        Row {
            toneless_key: "kah",
            hanji: "甲",
            tl: "kah",
            syll: 1,
            freq: 9000,
        },
        Row {
            toneless_key: "kah",
            hanji: "佮",
            tl: "kah",
            syll: 1,
            freq: 8000,
        },
        Row {
            toneless_key: "kah",
            hanji: "",
            tl: "kah",
            syll: 1,
            freq: 10,
        },
    ];
    let source = |row: &Row| if row.hanji.is_empty() { LKK } else { KAUTIAN };
    let dict_path = write_temp("dictionary.bin", &build_tkdb_v4_sourced(&rows, source));
    let fst_path = build_dictionary_fst(&rows);
    let association_path = write_temp("association.bin", &empty_association_bin());
    let syllables_path = build_syllables_fst(&["gua2", "tsiah8", "tsiah4", "kah4"]);
    install_lexicon(&fst_path, &dict_path, &association_path, &syllables_path);
}

fn config(display: CandidateDisplayMode, han_lo_mixed: bool) -> AppConfig {
    AppConfig {
        han_lo_mixed,
        ..config_with_display_mode("tl", display as i32)
    }
}

/// `(hanji, roman, display_text)` per row, list order.
fn rows(raw: &str, config: &AppConfig, fetch: Fetch) -> Vec<(Option<String>, String, String)> {
    fetch_at_pos_response(config, raw, fetch)
        .continuous
        .map(|c| c.candidates)
        .unwrap_or_default()
        .into_iter()
        .map(|c| (c.hanji, c.roman, c.display_text))
        .collect()
}

fn hanji_and_roman(rows: &[(Option<String>, String, String)]) -> Vec<(Option<&str>, &str)> {
    rows.iter()
        .map(|(hanji, roman, _)| (hanji.as_deref(), roman.as_str()))
        .collect()
}

fn picked(word: &str, canonical_tl: &str) -> Fetch {
    Fetch {
        now_ms: NOW_MS,
        frequency: vec![selected(word, canonical_tl, 3, 1_000)],
        ..Fetch::default()
    }
}

/// The whole-buffer path's text: the first row past the literal that covers
/// the buffer.
fn sentence(raw: &str, config: &AppConfig, fetch: Fetch) -> Option<String> {
    fetch_at_pos_response(config, raw, fetch)
        .continuous
        .map(|c| c.candidates)
        .unwrap_or_default()
        .into_iter()
        .skip(1)
        .find(|c| c.consumed_span_end as usize == raw.len())
        .map(|c| c.hanji.unwrap_or(c.roman))
}

#[test]
fn off_lists_no_romanization_rows() {
    let _lock = engine_install_lock();
    install_fixture();
    let listed = rows(
        "tsiah",
        &config(CandidateDisplayMode::SideBySide, false),
        Fetch::default(),
    );
    assert!(
        listed.iter().skip(1).all(|(hanji, _, _)| hanji.is_some()),
        "{listed:?}"
    );
}

#[test]
fn pairing_lists_each_reading_as_a_word_of_its_own() {
    let _lock = engine_install_lock();
    install_fixture();
    let listed = rows(
        "tsiah",
        &config(CandidateDisplayMode::SideBySide, true),
        Fetch::default(),
    );
    assert_eq!(
        hanji_and_roman(&listed),
        [
            (None, "tsiah"),
            (Some("食"), "tsia̍h"),
            (Some("隻"), "tsiah"),
            (None, "tsia̍h"),
            (Some("𤆬"), "tsia̍h"),
        ]
    );
    // The romanization is its own word, and the literal that absorbs the
    // `tsiah` one learns the romanization, not 隻.
    assert_eq!(listed[3].2, "tsia̍h");
    assert_eq!(listed[0].2, "tsiah");
}

#[test]
fn a_picked_romanization_leads_its_hanji() {
    let _lock = engine_install_lock();
    install_fixture();
    let listed = rows(
        "tsiah",
        &config(CandidateDisplayMode::SideBySide, true),
        picked("tsia̍h", "tsia̍h"),
    );
    assert_eq!(
        hanji_and_roman(&listed)[..3],
        [(None, "tsiah"), (None, "tsia̍h"), (Some("食"), "tsia̍h")]
    );
}

#[test]
fn a_picked_hanji_does_not_lift_its_romanization() {
    let _lock = engine_install_lock();
    install_fixture();
    let listed = rows(
        "tsiah",
        &config(CandidateDisplayMode::SideBySide, true),
        picked("𤆬", "tsia̍h"),
    );
    assert_eq!(
        hanji_and_roman(&listed),
        [
            (None, "tsiah"),
            (Some("𤆬"), "tsia̍h"),
            (Some("食"), "tsia̍h"),
            (Some("隻"), "tsiah"),
            (None, "tsia̍h"),
        ]
    );
}

#[test]
fn an_lkk_romanization_leads_its_hanji_homophones() {
    let _lock = engine_install_lock();
    install_fixture();
    // Without the literal, so the LKK row shows where it ranks.
    let listed = rows(
        "kah",
        &config(CandidateDisplayMode::SideBySide, true),
        Fetch {
            literal_roman_candidate_disabled: true,
            ..Fetch::default()
        },
    );
    assert_eq!(
        hanji_and_roman(&listed),
        [(None, "kah"), (Some("甲"), "kah"), (Some("佮"), "kah")]
    );
}

#[test]
fn hanji_with_romanization_draws_hanji_rows_as_hanji_cells_only() {
    let _lock = engine_install_lock();
    install_fixture();
    let cells: Vec<(CellScript, Option<String>, String)> = fetch_at_pos_response(
        &config(CandidateDisplayMode::Combined, true),
        "tsiah",
        Fetch::default(),
    )
    .continuous
    .map(|c| c.candidates)
    .unwrap_or_default()
    .into_iter()
    .map(|c| (c.cell_script(), c.hanji, c.roman))
    .collect();
    let scripts: Vec<CellScript> = cells.iter().map(|(script, _, _)| *script).collect();
    assert_eq!(
        scripts,
        [
            CellScript::Roman,
            CellScript::Hanji,
            CellScript::Hanji,
            CellScript::Roman,
            CellScript::Hanji,
        ],
        "{cells:?}"
    );
    assert!(cells
        .iter()
        .all(|(script, hanji, _)| (*script == CellScript::Roman) == hanji.is_none()));
}

#[test]
fn romanization_only_ignores_the_switch() {
    let _lock = engine_install_lock();
    install_fixture();
    assert_eq!(
        rows(
            "tsiah",
            &config(CandidateDisplayMode::RomanOnly, true),
            Fetch::default()
        ),
        rows(
            "tsiah",
            &config(CandidateDisplayMode::RomanOnly, false),
            Fetch::default()
        ),
    );
}

#[test]
fn the_sentence_takes_a_picked_romanization() {
    let _lock = engine_install_lock();
    install_fixture();
    let on = config(CandidateDisplayMode::SideBySide, true);
    let off = config(CandidateDisplayMode::SideBySide, false);
    assert_eq!(
        sentence("guatsiah", &on, Fetch::default()).as_deref(),
        Some("我食")
    );
    assert_eq!(
        sentence("guatsiah", &on, picked("tsia̍h", "tsia̍h")).as_deref(),
        Some("我tsia̍h")
    );
    assert_eq!(
        sentence("guatsiah", &off, picked("tsia̍h", "tsia̍h")).as_deref(),
        Some("我食")
    );
}

#[test]
fn the_sentence_takes_an_lkk_romanization() {
    let _lock = engine_install_lock();
    install_fixture();
    let on = config(CandidateDisplayMode::SideBySide, true);
    let off = config(CandidateDisplayMode::SideBySide, false);
    assert_eq!(
        sentence("guakah", &on, Fetch::default()).as_deref(),
        Some("我kah")
    );
    assert_eq!(
        sentence("guakah", &off, Fetch::default()).as_deref(),
        Some("我甲")
    );
}

// Two romanized words meeting in the sentence keep a word space.
#[test]
fn the_sentence_spaces_two_romanized_words() {
    let _lock = engine_install_lock();
    install_fixture();
    let on = config(CandidateDisplayMode::SideBySide, true);
    assert_eq!(
        sentence("guatsiahkah", &on, picked("tsia̍h", "tsia̍h")).as_deref(),
        Some("我tsia̍h kah")
    );
}

// A learned Han-Lo phrase is stored in TL and shown in the input mode's
// spelling; its identity (`display_text`) stays the stored form.
#[test]
fn a_learned_han_lo_phrase_shows_in_poj_under_poj() {
    let _lock = engine_install_lock();
    install_fixture();
    let fetch = Fetch {
        learned: vec![LearnedEntry {
            hanji: "我tsia̍h".into(),
            canonical_tl: "guá tsia̍h".into(),
        }],
        ..Fetch::default()
    };
    let poj = AppConfig {
        han_lo_mixed: true,
        ..config_with_display_mode("poj", CandidateDisplayMode::SideBySide as i32)
    };
    let listed = fetch_at_pos_response(&poj, "goachiah", fetch.clone())
        .continuous
        .map(|c| c.candidates)
        .unwrap_or_default();
    let row = listed
        .iter()
        .find(|c| c.display_text == "我tsia̍h")
        .unwrap_or_else(|| panic!("the learned row: {listed:?}"));
    assert_eq!(row.hanji.as_deref(), Some("我chia̍h"));
    let tl = rows(
        "guatsiah",
        &config(CandidateDisplayMode::SideBySide, true),
        fetch,
    );
    assert!(
        tl.iter()
            .any(|(hanji, _, _)| hanji.as_deref() == Some("我tsia̍h")),
        "{tl:?}"
    );
}

// Under POJ input the sentence writes a romanization word in POJ, keyed on
// its TL identity.
#[test]
fn the_sentence_writes_a_romanization_in_poj_under_poj() {
    let _lock = engine_install_lock();
    install_fixture();
    let poj = AppConfig {
        han_lo_mixed: true,
        ..config_with_display_mode("poj", CandidateDisplayMode::SideBySide as i32)
    };
    let row = fetch_at_pos_response(&poj, "goachiah", picked("tsia̍h", "tsia̍h"))
        .continuous
        .map(|c| c.candidates)
        .unwrap_or_default()
        .into_iter()
        .skip(1)
        .find(|c| c.consumed_span_end as usize == "goachiah".len())
        .expect("the sentence");
    assert_eq!(row.hanji.as_deref(), Some("我chia̍h"));
    assert_eq!(row.display_text, "我tsia̍h");
}
