//! The settings the composing key path reads — `SettingsDocument::
//! engine_settings`, `ComposingKeyBindings::from_document`, the intent
//! executor and the auto-space policy — and nothing else. Windows and Linux
//! read the whole `settings.json`; a shell that pushes a snapshot instead
//! (macOS, `macos/crates/taigi-macos-ffi`) pushes exactly these, so a new
//! key-path read adds its key here.

use super::choices::SettingChoice;
use super::dictionary_sources::{KAUTIAN_SUBCOLLECTIONS, MOE_OTHERS, OTHERS, SUPPLEMENTS};
use super::{keys, SettingsKey};
use crate::keys::ComposingAction;

/// One key-path setting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyPathSetting {
    Switch(SettingsKey<bool>),
    /// A string: a choice, stored as its persisted spelling, or a composing
    /// chord field, which has no stored default — absent is what the field
    /// ships with, `""` a cleared field.
    Text {
        name: String,
        default: Option<&'static str>,
    },
}

impl KeyPathSetting {
    fn choice<T: SettingChoice>(key: SettingsKey<T>) -> Self {
        Self::Text {
            name: key.name.to_owned(),
            default: Some(key.default.raw()),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Switch(key) => key.name,
            Self::Text { name, .. } => name,
        }
    }
}

/// Every key-path setting: the four choices, the switches (the General
/// pane's, the custom dictionary's, every dictionary source), one chord
/// per field of every composing action.
pub fn key_path_settings() -> Vec<KeyPathSetting> {
    let switches = [
        keys::IS_HANJI_FIRST,
        keys::IS_AUTO_SPACE_ENABLED,
        keys::IS_CANDIDATE_WINDOW_ENABLED,
        keys::IS_LITERAL_ROMAN_CANDIDATE_ENABLED,
        keys::IS_NASAL_MARKER_UPPERCASE_ENABLED,
        keys::IS_CUSTOM_DICT_ENABLED,
        keys::IS_KAUTIAN_ENABLED,
    ]
    .into_iter()
    .chain(
        KAUTIAN_SUBCOLLECTIONS
            .iter()
            .chain(&MOE_OTHERS)
            .chain(&OTHERS)
            .chain(&SUPPLEMENTS)
            .map(|(key, _)| *key),
    )
    .map(KeyPathSetting::Switch);
    let chords = ComposingAction::all_fields().map(|(action, field)| KeyPathSetting::Text {
        name: action.settings_key_name(field),
        default: None,
    });
    [
        KeyPathSetting::choice(keys::INPUT_MODE),
        KeyPathSetting::choice(keys::TONE_INPUT_SCHEME),
        KeyPathSetting::choice(keys::CANDIDATE_DISPLAY_MODE),
        KeyPathSetting::choice(keys::SYLLABLE_SEPARATOR),
    ]
    .into_iter()
    .chain(switches)
    .chain(chords)
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The General pane's keys, the dictionary sources, the display mode,
    /// the custom dictionary switch, one row per composing action — each
    /// once. Not the romanization TPS returns to: only a mode switch reads
    /// it, never a key. Nor the TPS key panel, which no key reads. Nor the
    /// keyboard layout: the Windows key translation reads it before a key
    /// reaches the shared path, and macOS sets it on IMK, not in a snapshot.
    #[test]
    fn the_key_path_settings_are_the_ones_the_key_path_reads() {
        let mut expected: Vec<String> = keys::GENERAL_KEYS
            .iter()
            .filter(|name| {
                **name != keys::LAST_ROMANIZATION_MODE.name
                    && **name != keys::TPS_KEYBOARD_SHOWN.name
                    && **name != keys::KEYBOARD_LAYOUT.name
            })
            .chain(&keys::DICTIONARY_SOURCE_KEYS)
            .chain(&[
                keys::CANDIDATE_DISPLAY_MODE.name,
                keys::IS_CUSTOM_DICT_ENABLED.name,
            ])
            .map(|name| (*name).to_owned())
            .chain(
                ComposingAction::all_fields()
                    .map(|(action, field)| action.settings_key_name(field)),
            )
            .collect();
        let mut names: Vec<String> = key_path_settings()
            .iter()
            .map(|setting| setting.name().to_owned())
            .collect();
        expected.sort();
        names.sort();
        assert_eq!(names, expected);
    }
}
