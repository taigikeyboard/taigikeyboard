//! The sidebar's Shortcuts section: the blocks of the desktops' Shortcuts pane
//! (`ShortcutSettingsView.swift`), in the page's language and the visitor's
//! key spelling. Read-only, so no recorder rows and no reset row.

use serde::Serialize;
use taigi_desktop_core::keys::shortcut_labels::{caret_chords_label, fixed_keys_label};
use taigi_desktop_core::keys::{ComposingAction, ComposingKeyBindings, ShortcutAction};
use taigi_desktop_core::platform::DesktopPlatform;
use taigi_desktop_core::settings::SettingsDocument;
use taigi_desktop_core::strings::{StringKey, StringResolver};

use crate::key::visitor_chord;

/// The two keys of one row, spaced as `caret_chords_label` spaces its own.
const KEY_GAP: &str = "  ";

#[derive(Debug, Serialize)]
pub(crate) struct WireShortcuts {
    groups: Vec<Group>,
}

#[derive(Debug, Serialize)]
struct Group {
    title: String,
    rows: Vec<Row>,
}

#[derive(Debug, Serialize)]
struct Row {
    label: String,
    keys: String,
    /// The fixed keys under the label (`fixed_keys_label`).
    #[serde(skip_serializing_if = "Option::is_none")]
    fixed_keys: Option<String>,
}

pub(crate) fn shortcuts(
    document: &SettingsDocument,
    strings: &StringResolver,
    platform: DesktopPlatform,
) -> WireShortcuts {
    let bindings = ComposingKeyBindings::from_document(document, platform);
    let composing_rows = |actions: &[ComposingAction]| -> Vec<Row> {
        let row = |action: &ComposingAction| {
            let keys: Vec<String> = action
                .fields()
                .iter()
                .filter_map(|&field| bindings.chord(*action, field))
                .map(|chord| chord.display(platform))
                .collect();
            Row {
                label: strings.resolve(action.label_key()).to_owned(),
                keys: keys.join(KEY_GAP),
                fixed_keys: fixed_keys_label(*action, bindings.slot_key_set(), platform),
            }
        };
        actions.iter().map(row).collect()
    };
    let global_rows = |actions: &[ShortcutAction]| -> Vec<Row> {
        let row = |action: &ShortcutAction| Row {
            label: strings.resolve(action.label_key()).to_owned(),
            keys: action
                .chord_in(document, platform)
                .map_or_else(String::new, |chord| {
                    visitor_chord(chord, platform).display(platform)
                }),
            fixed_keys: None,
        };
        actions.iter().map(row).collect()
    };

    let mut selection = composing_rows(ComposingAction::GROUPS[0]);
    selection.push(Row {
        label: strings
            .resolve(StringKey::DesktopShortcutMoveComposingCaret)
            .to_owned(),
        keys: caret_chords_label(platform),
        fixed_keys: None,
    });
    let mut groups = vec![
        Group {
            title: strings
                .resolve(StringKey::DesktopShortcutSectionCandidateSelection)
                .to_owned(),
            rows: selection,
        },
        Group {
            title: strings
                .resolve(StringKey::DesktopShortcutSectionOutput)
                .to_owned(),
            rows: composing_rows(ComposingAction::GROUPS[1]),
        },
    ];
    groups.extend(
        ShortcutAction::GROUP_TITLES
            .into_iter()
            .zip(ShortcutAction::GROUPS)
            .map(|(title, actions)| Group {
                title: strings.resolve(title).to_owned(),
                rows: global_rows(actions),
            }),
    );
    WireShortcuts { groups }
}

#[cfg(test)]
mod tests {
    use super::*;
    use taigi_desktop_core::settings::{keys, InputMode};
    use taigi_desktop_core::strings::DisplayLanguage;

    fn english() -> StringResolver {
        StringResolver::new(DisplayLanguage::English)
    }

    fn row<'a>(wire: &'a WireShortcuts, label: &str) -> &'a Row {
        wire.groups
            .iter()
            .flat_map(|group| &group.rows)
            .find(|row| row.label == label)
            .unwrap_or_else(|| panic!("no row {label}"))
    }

    #[test]
    fn the_blocks_follow_the_desktop_pane() {
        let wire = shortcuts(
            &SettingsDocument::default(),
            &english(),
            DesktopPlatform::Linux,
        );
        let sizes: Vec<_> = wire.groups.iter().map(|group| group.rows.len()).collect();
        // trace: GROUPS 4 + the caret row, 3, then 3, 2, 3.
        assert_eq!(sizes, [5, 3, 3, 2, 3]);
        assert_eq!(wire.groups[0].title, "Candidate Selection");
    }

    #[test]
    fn two_keys_share_a_row_and_an_empty_row_stays_empty() {
        let wire = shortcuts(
            &SettingsDocument::default(),
            &english(),
            DesktopPlatform::Linux,
        );
        // trace: `shipped_chord` ConfirmHighlighted = Enter, Space;
        // CommitAlternateScript has none; slot run qwdfzxvy; behind one Shift.
        assert_eq!(row(&wire, "Confirm Key").keys, "Enter  Space");
        let other = row(&wire, "Output the Other Script (Hanji / Romanization)");
        assert_eq!(other.keys, "");
        assert_eq!(other.fixed_keys.as_deref(), Some("Shift+QWDFZXVY;"));
    }

    #[test]
    fn the_keys_are_spelled_for_the_visitors_machine() {
        let document = SettingsDocument::default();
        // trace: `default_chord` ToggleTps = Ctrl+Alt+P; `visitor_chord` makes
        // it ⌃⌘ on the Mac, and `display` there takes glyphs and no separator.
        let label = "Switch Phonetic Symbols";
        let linux = shortcuts(&document, &english(), DesktopPlatform::Linux);
        assert_eq!(row(&linux, label).keys, "Ctrl+Alt+P");
        let mac = shortcuts(&document, &english(), DesktopPlatform::MacOS);
        assert_eq!(row(&mac, label).keys, "⌃⌘P");
    }

    #[test]
    fn phonetic_symbols_have_no_other_script_keys() {
        let mut document = SettingsDocument::default();
        document.set_choice(&keys::INPUT_MODE, InputMode::Tps);
        let wire = shortcuts(&document, &english(), DesktopPlatform::Linux);
        let other = row(&wire, "Output the Other Script (Hanji / Romanization)");
        assert_eq!(other.fixed_keys, None);
    }
}
