//! The chord-rule table (`fixtures/chord_rules.tsv`): every row read under the
//! Mac's rules. The Swift Shortcuts pane reads these rules through the seam
//! since P14; `ChordRulesCrossCheckTests.swift` holds its `default` and
//! `resolve` rows to the same hand-written answers.

use std::collections::BTreeMap;

use taigi_desktop_core::keys::{
    ChordField, ComposingAction, ComposingKeyBindings, ComposingKeyChord, KeyEventSnapshot,
    KeyModifiers,
};
use taigi_desktop_core::settings::SettingsDocument;

use crate::runtime::DESKTOP_PLATFORM;

const TABLE: &str = include_str!("../fixtures/chord_rules.tsv");

/// The rows, without comments and blank lines, each split on tabs.
fn rows() -> impl Iterator<Item = Vec<&'static str>> {
    TABLE
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.split('\t').collect())
}

fn action(raw: &str) -> ComposingAction {
    ComposingAction::from_raw(raw).unwrap_or_else(|| panic!("no action {raw}"))
}

fn chord(raw: &str) -> String {
    ComposingKeyChord::from_raw(raw, DESKTOP_PLATFORM).map_or_else(
        || "refused".to_owned(),
        |chord| chord.raw_value(DESKTOP_PLATFORM),
    )
}

fn modifiers(letters: &str) -> KeyModifiers {
    let has = |letter| letters.contains(letter);
    KeyModifiers {
        shift: has('s'),
        control: has('c'),
        alt: has('o'),
        win: has('d'),
    }
}

fn scalars(hex: &str) -> String {
    hex.split(',')
        .map(|field| char::from_u32(u32::from_str_radix(field, 16).unwrap()).unwrap())
        .collect()
}

fn bindings(stored: &str) -> String {
    let mut document = SettingsDocument::default();
    if stored != "-" {
        for assignment in stored.split(';') {
            let (name, value) = assignment.split_once('=').unwrap();
            document.set_raw_string(&action(name).settings_key_name(ChordField::First), value);
        }
    }
    let bindings = ComposingKeyBindings::from_document(&document, DESKTOP_PLATFORM);
    ComposingAction::ALL
        .map(|action| {
            let chord = bindings
                .chord(action, ChordField::First)
                .map_or_else(|| "-".to_owned(), |chord| chord.raw_value(DESKTOP_PLATFORM));
            format!("{}={chord}", action.raw())
        })
        .join(";")
}

#[test]
fn the_core_answers_every_row_of_the_mac_chord_table() {
    let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
    for row in rows() {
        *kinds.entry(row[0]).or_default() += 1;
        match row[0] {
            "default" => assert_eq!(
                action(row[1]).default_chord().raw_value(DESKTOP_PLATFORM),
                row[2],
                "{row:?}"
            ),
            "parse" => assert_eq!(chord(row[1]), row[2], "{row:?}"),
            "match" => {
                let stored = ComposingKeyChord::from_raw(row[1], DESKTOP_PLATFORM).unwrap();
                let characters = scalars(row[2]);
                let event = KeyEventSnapshot {
                    characters: Some(characters.clone()),
                    characters_ignoring_modifiers: Some(characters),
                    modifiers: modifiers(row[3]),
                    ..KeyEventSnapshot::default()
                };
                let matches = if stored.matches(&event, DESKTOP_PLATFORM) {
                    "yes"
                } else {
                    "no"
                };
                assert_eq!(matches, row[4], "{row:?}");
            }
            "resolve" => assert_eq!(bindings(row[1]), row[2], "{row:?}"),
            kind => panic!("unknown row kind {kind}"),
        }
    }
    // Every action has its default row, and no kind went missing.
    assert_eq!(kinds.get("default"), Some(&ComposingAction::ALL.len()));
    assert_eq!(kinds.len(), 4);
}
