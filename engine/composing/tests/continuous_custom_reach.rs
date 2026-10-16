//! A custom-dictionary word answers the dictionary's syllable-reach rule
//! end to end (product owner 2026-10-12, all platforms through the shared
//! engine; `behavioral-invariants.md` §43).
//!
//! Reported symptom: custom 陳建中 `Tân Kiàn-tiong` listed from `t` (slot 1
//! once selected) and `ta` (slot 2) — every custom row the store's prefix
//! query returned was merged whole-buffer, exempt from the reach rule and
//! stamped FULL on the exact path. Now it surfaces once the typed input
//! reaches into `tiong` (`tankiant`), or on its exact abbreviation `tkt`.

use crate::common::{
    build_dictionary_fst, build_syllables_fst, build_tkdb_v4, config, empty_association_bin,
    fetch_cells, install_lexicon, selected, Fetch, Row, NOW_MS,
};
use lexicon::CustomEntry;
use test_support::{engine_install_lock, write_temp};

fn fixture_rows() -> Vec<Row> {
    vec![
        Row {
            toneless_key: "ta",
            hanji: "打",
            tl: "tá",
            syll: 1,
            freq: 5000,
        },
        Row {
            toneless_key: "tan",
            hanji: "陳",
            tl: "tân",
            syll: 1,
            freq: 4000,
        },
        Row {
            toneless_key: "kian",
            hanji: "建",
            tl: "kiàn",
            syll: 1,
            freq: 3000,
        },
        Row {
            toneless_key: "tiong",
            hanji: "中",
            tl: "tiong",
            syll: 1,
            freq: 3000,
        },
    ]
}

fn install() {
    install_with(&fixture_rows(), &["ta2", "tan5", "kian3", "tiong1"]);
}

fn install_with(rows: &[Row], syllables: &[&str]) {
    let dict_path = write_temp("dictionary.bin", &build_tkdb_v4(rows));
    let fst_path = build_dictionary_fst(rows);
    let association_path = write_temp("association.bin", &empty_association_bin());
    let syllables_path = build_syllables_fst(syllables);
    install_lexicon(&fst_path, &dict_path, &association_path, &syllables_path);
}

/// 陳建中 as the store hands it over, selected once a minute ago — the user
/// weight that lifted it to slot 1 in the report.
fn selected_custom() -> Fetch {
    Fetch {
        now_ms: NOW_MS,
        custom: vec![CustomEntry {
            roman: "Tân Kiàn-tiong".into(),
            hanji: Some("陳建中".into()),
        }],
        frequency: vec![selected("陳建中", "Tân Kiàn-tiong", 1, 60_000)],
        ..Default::default()
    }
}

fn lists_custom(input_mode: &str, raw: &str) -> (bool, Vec<Option<String>>) {
    let cells = fetch_cells(&config(input_mode), raw, selected_custom());
    let all: Vec<Option<String>> = cells.iter().map(|c| c.0.clone()).collect();
    let listed = all.iter().any(|h| h.as_deref() == Some("陳建中"));
    (listed, all)
}

#[test]
fn custom_word_waits_until_the_final_syllable_is_typed_into() {
    let _lock = engine_install_lock();
    install();
    // trace: face `tankiantiong`, ends [3, 7, 12] — the typed body must run
    // past 7. `ta` / `tan` / `tankian` are full-syllable buffers (exact
    // path), `t` / `tank` / `tanki` are not (partial-prefix path).
    for raw in ["t", "ta", "tan", "tank", "tanki", "tankian"] {
        let (listed, all) = lists_custom("tl", raw);
        assert!(!listed, "{raw}: 陳建中 not reached yet; got {all:?}");
    }
    for raw in ["tankiant", "tankianti", "tankiantiong"] {
        let (listed, all) = lists_custom("tl", raw);
        assert!(listed, "{raw}: 陳建中 reached; got {all:?}");
    }
}

#[test]
fn a_reached_custom_word_leads_the_front_segment_rows() {
    // USER 2026-10-12: tier first, then the user's own words — at
    // `tankiant` 陳建中 covers the whole input, so it leads 陳 / 打, which
    // cover only `tan` / `ta`.
    let _lock = engine_install_lock();
    install();
    for raw in ["tankiant", "tkt"] {
        let (_, all) = lists_custom("tl", raw);
        let first_word = all.iter().flatten().next().map(String::as_str);
        assert_eq!(first_word, Some("陳建中"), "{raw}: got {all:?}");
    }
}

#[test]
fn custom_word_answers_its_exact_abbreviation_only() {
    let _lock = engine_install_lock();
    install();
    let (listed, all) = lists_custom("tl", "tkt");
    assert!(listed, "tkt is 陳建中's abbreviation; got {all:?}");
    let (listed, all) = lists_custom("tl", "tk");
    assert!(
        !listed,
        "tk is no abbreviation of a 3-syllable word; got {all:?}"
    );
}

#[test]
fn typed_spaces_and_trailing_hyphen_keep_the_exact_word() {
    let _lock = engine_install_lock();
    install();
    for raw in [
        "tan kian tiong",
        "tan-kian-tiong",
        "tankiantiong-",
        "tan5kian3tiong1",
        "tan5kiantiong",
    ] {
        let (listed, all) = lists_custom("tl", raw);
        assert!(listed, "{raw:?}: 陳建中 typed in full; got {all:?}");
    }
}

#[test]
fn a_non_taigi_custom_word_is_one_syllable() {
    // A Latin word that parses as no Taigi syllable reads as one syllable,
    // so any prefix of it reaches it — as before.
    let _lock = engine_install_lock();
    install();
    let fetch = || Fetch {
        custom: vec![CustomEntry {
            roman: "iPhone".into(),
            hanji: None,
        }],
        ..Default::default()
    };
    for (input_mode, raw) in [
        ("tl", "iph"),
        ("tl", "iphone"),
        ("english", "iph"),
        ("english", "iphone"),
    ] {
        let cells = fetch_cells(&config(input_mode), raw, fetch());
        assert!(
            cells.iter().any(|c| c.1.eq_ignore_ascii_case("iphone")),
            "{input_mode} {raw}: custom iPhone listed; got {cells:?}"
        );
    }
}

#[test]
fn poj_follows_the_same_reach() {
    let _lock = engine_install_lock();
    install();
    let (listed, all) = lists_custom("poj", "tankian");
    assert!(!listed, "poj tankian: not reached; got {all:?}");
    let (listed, all) = lists_custom("poj", "tankiant");
    assert!(listed, "poj tankiant: reached; got {all:?}");
}

#[test]
fn a_custom_abbreviation_trails_the_full_syllable_rows() {
    // USER 2026-10-16: `thng` listed the literal, then custom bisyllables
    // reached through their initials (退伍 `thui-ńg`, 大人 `thai-ngìn`) ahead
    // of 湯 / 糖. A reading leads every abbreviation of its tier, custom or
    // not (`CandidateSortKey::abbreviation`); custom still leads the
    // abbreviations (`tkt` above).
    let _lock = engine_install_lock();
    let rows = [
        Row {
            toneless_key: "thng",
            hanji: "湯",
            tl: "thng",
            syll: 1,
            freq: 3000,
        },
        Row {
            toneless_key: "thng",
            hanji: "糖",
            tl: "thn̂g",
            syll: 1,
            freq: 2000,
        },
    ];
    install_with(&rows, &["thng1", "thng5", "thui3", "ng2", "thai7", "ngin5"]);
    let fetch = Fetch {
        now_ms: NOW_MS,
        custom: vec![
            CustomEntry {
                roman: "thui-ńg".into(),
                hanji: Some("退伍".into()),
            },
            CustomEntry {
                roman: "thai-ngìn".into(),
                hanji: Some("大人".into()),
            },
        ],
        ..Default::default()
    };
    for input_mode in ["tl", "poj"] {
        let cells = fetch_cells(&config(input_mode), "thng", fetch.clone());
        let words: Vec<&str> = cells.iter().filter_map(|c| c.0.as_deref()).collect();
        assert_eq!(
            words,
            ["湯", "糖", "退伍", "大人"],
            "{input_mode}: readings first, then the custom abbreviations; got {cells:?}"
        );
    }
}
