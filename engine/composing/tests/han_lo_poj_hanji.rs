//! A Han-Lo dictionary word (`親tsiânn`: the Hanji column spells part of the
//! word in TL) shows its romanization in POJ in POJ mode (`親chiâⁿ`), as the
//! candidate's `roman` does; `display_text`, the identity key, keeps the stored
//! spelling. A custom word's Hanji stays as the user saved it.
//! Hermetic `LexiconHandle` install comes from `tests/common/mod.rs`.

use crate::common::{
    build_dictionary_fst, build_syllables_fst, build_tkdb_v4, config, empty_association_bin,
    fetch_at_pos_response, install_lexicon, Fetch, Row,
};
use lexicon::{CustomEntry, LearnedEntry};
use protos::engine::CandidateMessage;
use test_support::{engine_install_lock, write_temp};

const STORED: &str = "親tsiânn";
// trace: `tsiânn` → strip tone `tsiann` (tone 5), normalize→`tsiann`,
// split→("ts","iann"), to_poj → `chi` + `â` + `ⁿ` = `chiâⁿ` — the production
// `dictionary.csv` `poj` column of this row reads `chhin-chiâⁿ`.
const SHOWN_POJ: &str = "親chi\u{e2}\u{207f}";

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
            toneless_key: "tshintsiann",
            hanji: STORED,
            tl: "tshin-tsiânn",
            syll: 2,
            freq: 50,
        },
    ];
    let dict_path = write_temp("dictionary.bin", &build_tkdb_v4(&rows));
    let fst_path = build_dictionary_fst(&rows);
    let association_path = write_temp("association.bin", &empty_association_bin());
    // Every production syllable that is a strict prefix of a fixture syllable
    // (`gu`, `tshi`, `tsi`, `tsia`, `tsian`) is in the inventory too, so the
    // walker could cut there; the assertions name the word it must pick.
    let syllables_path = build_syllables_fst(&[
        "gua2", "gua1", "gu5", "gu1", "tshin1", "tshi1", "tsiann5", "tsiann1", "tsi1", "tsia1",
        "tsian1",
    ]);
    install_lexicon(&fst_path, &dict_path, &association_path, &syllables_path);
}

fn whole_buffer(raw: &str, input_mode: &str, fetch: Fetch) -> Vec<CandidateMessage> {
    fetch_at_pos_response(&config(input_mode), raw, fetch)
        .continuous
        .map(|continuous| continuous.candidates)
        .unwrap_or_default()
        .into_iter()
        .filter(|candidate| candidate.consumed_span_end as usize == raw.len())
        .filter(|candidate| candidate.hanji.is_some())
        .collect()
}

fn hanji_and_key(candidates: &[CandidateMessage]) -> Vec<(String, String)> {
    candidates
        .iter()
        .map(|c| (c.hanji.clone().unwrap_or_default(), c.display_text.clone()))
        .collect()
}

#[test]
fn poj_mode_shows_the_han_lo_romanization_in_poj_keyed_by_the_stored_spelling() {
    let _lock = engine_install_lock();
    install_fixture();
    let listed = hanji_and_key(&whole_buffer("chhinchiann", "poj", Fetch::default()));
    assert!(
        listed.contains(&(SHOWN_POJ.to_owned(), STORED.to_owned())),
        "got {listed:?}"
    );
}

#[test]
fn tl_mode_shows_the_stored_spelling() {
    let _lock = engine_install_lock();
    install_fixture();
    let listed = hanji_and_key(&whole_buffer("tshintsiann", "tl", Fetch::default()));
    assert!(
        listed.contains(&(STORED.to_owned(), STORED.to_owned())),
        "got {listed:?}"
    );
}

#[test]
fn poj_sentence_renders_its_han_lo_word_in_poj() {
    let _lock = engine_install_lock();
    install_fixture();
    let listed = hanji_and_key(&whole_buffer("goachhinchiann", "poj", Fetch::default()));
    let sentence = (format!("我{SHOWN_POJ}"), format!("我{STORED}"));
    assert!(listed.contains(&sentence), "got {listed:?}");
}

#[test]
fn a_learned_han_lo_phrase_shows_in_poj_too() {
    // `阮` is no dictionary word here, so only the learned row can list it.
    let _lock = engine_install_lock();
    install_fixture();
    let fetch = Fetch {
        learned: vec![LearnedEntry {
            hanji: format!("阮{STORED}"),
            canonical_tl: "guá tshin-tsiânn".into(),
        }],
        ..Fetch::default()
    };
    let listed = hanji_and_key(&whole_buffer("goachhinchiann", "poj", fetch));
    let phrase = (format!("阮{SHOWN_POJ}"), format!("阮{STORED}"));
    assert!(listed.contains(&phrase), "got {listed:?}");
}

#[test]
fn a_custom_han_lo_word_keeps_its_saved_spelling_in_poj_mode() {
    let _lock = engine_install_lock();
    install_fixture();
    let fetch = Fetch {
        custom: vec![CustomEntry {
            roman: "tshin-tsiânn".into(),
            hanji: Some("親tsiânn仔".into()),
        }],
        ..Fetch::default()
    };
    let listed = hanji_and_key(&whole_buffer("chhinchiann", "poj", fetch));
    assert!(
        listed.contains(&("親tsiânn仔".to_owned(), "親tsiânn仔".to_owned())),
        "got {listed:?}"
    );
}

// A Han-Lo dictionary word's Taigi romanization follows the typed case in
// every block — the span-local row, a partial-prefix extension (`TSHINTSIA`)
// and an abbreviation (`TSHTS`) — after its POJ rewrite in POJ mode; its
// identity keeps the stored spelling.
#[test]
fn a_han_lo_word_follows_the_typed_case_in_every_block() {
    let _lock = engine_install_lock();
    install_fixture();
    let cases = [
        ("TSHINTSIANN", "tl", "親TSIÂNN"),
        ("Tshintsiann", "tl", STORED),
        ("TSHINTSIA", "tl", "親TSIÂNN"),
        ("TSHTS", "tl", "親TSIÂNN"),
        ("CHHINCHIANN", "poj", "親CHI\u{c2}\u{1d3a}"),
    ];
    for (raw, mode, shown) in cases {
        let listed = hanji_and_key(&whole_buffer(raw, mode, Fetch::default()));
        assert!(
            listed.contains(&(shown.to_owned(), STORED.to_owned())),
            "{raw}: got {listed:?}"
        );
    }
}

// A custom word's Hanji stays as the user saved it under any typed case, a
// Taigi run (`親tsiânn仔`) and a non-Taigi word (`iPhone`) alike.
#[test]
fn a_custom_han_lo_word_keeps_its_saved_case() {
    let _lock = engine_install_lock();
    install_fixture();
    for saved in ["親tsiânn仔", "iPhone"] {
        let fetch = || Fetch {
            custom: vec![CustomEntry {
                roman: "tshin-tsiânn".into(),
                hanji: Some(saved.into()),
            }],
            ..Fetch::default()
        };
        for (raw, mode) in [
            ("TSHINTSIANN", "tl"),
            ("Tshintsiann", "tl"),
            ("CHHINCHIANN", "poj"),
            ("Chhinchiann", "poj"),
        ] {
            let listed = hanji_and_key(&whole_buffer(raw, mode, fetch()));
            assert!(
                listed.contains(&(saved.to_owned(), saved.to_owned())),
                "{saved} under {raw}: got {listed:?}"
            );
        }
    }
}
