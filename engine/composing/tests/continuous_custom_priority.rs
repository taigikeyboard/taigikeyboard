//! A custom-dictionary word leads the never-picked words but ranks below the
//! words the user picks — in the list and at the walker's slot 0 alike (USER
//! 2026-10-16, all platforms through the shared engine;
//! `continuous-input-ranking.md` `preference_class`).
//!
//! Reported symptom: a bulk import filed 屢 / 李 / 俚 / 荔 under `lí`, and
//! `li2` listed the never-picked custom 屢 first, ahead of 你 (picked) and of
//! the custom 李 (picked 7×) — `custom_lead` sat above `user_weight`, and the
//! walker's slot 0 took the first custom row under the key unconditionally.

use crate::common::{
    build_dictionary_fst, build_syllables_fst, build_tkdb_v4, empty_association_bin, fetch_hanji,
    install_lexicon, selected, Fetch, Row, Selected, NOW_MS,
};
use lexicon::CustomEntry;
use test_support::{engine_install_lock, write_temp};

fn row(hanji: &'static str, freq: u32) -> Row {
    Row {
        toneless_key: "li",
        hanji,
        tl: "lí",
        syll: 1,
        freq,
    }
}

fn install() {
    let rows = [row("你", 5000), row("李", 1918), row("屢", 100)];
    let dict_path = write_temp("dictionary.bin", &build_tkdb_v4(&rows));
    let fst_path = build_dictionary_fst(&rows);
    let association_path = write_temp("association.bin", &empty_association_bin());
    let syllables_path = build_syllables_fst(&["li2"]);
    install_lexicon(&fst_path, &dict_path, &association_path, &syllables_path);
}

fn custom(hanji: &str) -> CustomEntry {
    CustomEntry {
        roman: "lí".into(),
        hanji: Some(hanji.into()),
    }
}

/// The `li` list (walker slot 0 included) for `custom` rows in store order
/// and `frequency` picks.
fn list(custom: &[&str], frequency: Vec<Selected>) -> Vec<String> {
    let _lock = engine_install_lock();
    install();
    fetch_hanji(
        "li",
        "tl",
        Fetch {
            now_ms: NOW_MS,
            custom: custom.iter().copied().map(self::custom).collect(),
            frequency,
            ..Default::default()
        },
    )
}

#[test]
fn never_picked_custom_words_lead_in_store_order() {
    // 屢 and 李 repeat dictionary words: the custom copy replaces each, and
    // every never-picked custom word leads the never-picked rest.
    let hanji = list(&["屢", "荔", "李"], vec![]);
    assert_eq!(hanji, ["屢", "荔", "李", "你"]);
}

#[test]
fn a_picked_word_leads_the_never_picked_custom_words() {
    let hanji = list(&["屢", "荔", "李"], vec![selected("你", "lí", 2, 60_000)]);
    assert_eq!(hanji, ["你", "屢", "荔", "李"]);
}

#[test]
fn the_picked_custom_word_takes_slot_0_over_the_one_filed_first() {
    // Two words the dictionary lacks, both custom: the one the user picks
    // wins the walker's edge, whatever the store order.
    let hanji = list(&["荔", "蒞"], vec![selected("蒞", "lí", 7, 60_000)]);
    assert_eq!(hanji[0], "蒞", "{hanji:?}");
    assert_eq!(hanji[1], "荔", "{hanji:?}");
}

#[test]
fn a_picked_custom_twin_leads_once() {
    // The picked custom 李 and its dictionary twin share one weight; the
    // custom copy replaces the twin at slot 0 and in the list alike, and the
    // heavier pick leads the lighter one.
    let hanji = list(
        &["屢", "李"],
        vec![
            selected("李", "lí", 7, 60_000),
            selected("你", "lí", 2, 60_000),
        ],
    );
    assert_eq!(hanji, ["李", "你", "屢"]);
}
