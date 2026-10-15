//! The key-path settings as the Swift side sends them with each request
//! (docs/architecture/macos-desktop-core-roadmap.md D5): the core's
//! `key_path_settings`, as typed entries, written into a `SettingsDocument`
//! the runtime reads through [`SnapshotSettings`]. The settings stay in
//! `UserDefaults`; Swift reads each name with the expression
//! `SettingsStore.swift` reads it with (`object(forKey:) as? Bool` for a
//! boolean — a stored 0 / 1 number reads as one, inventory S26 — and
//! `string(forKey:)` for text), so this side only checks the wire kind.
//! Chords keep the Mac spelling (`d` / `o`); the core reads them in the
//! `MacOS` grammar.

use std::collections::HashSet;
use std::sync::{Arc, LazyLock, Mutex};

use taigi_desktop_core::settings::key_path::{key_path_settings, KeyPathSetting};
use taigi_desktop_core::settings::{SettingsDocument, SettingsProvider};

use crate::proto::{setting_value, SettingDescriptor, SettingEntry, SettingKind, SettingValue};

/// Built once: every snapshot is checked against it.
static WHITELIST: LazyLock<Vec<KeyPathSetting>> = LazyLock::new(key_path_settings);

fn descriptor(setting: &KeyPathSetting) -> SettingDescriptor {
    let (kind, default) = match setting {
        KeyPathSetting::Switch(key) => (
            SettingKind::Boolean,
            Some(setting_value::Value::Boolean(key.default)),
        ),
        KeyPathSetting::Text { default, .. } => (
            SettingKind::Text,
            default.map(|text| setting_value::Value::Text(text.to_owned())),
        ),
    };
    SettingDescriptor {
        name: setting.name().to_owned(),
        kind: kind as i32,
        default_value: default.map(|value| SettingValue { value: Some(value) }),
    }
}

/// Writes `value` into `document`, or `None` when it is not `setting`'s
/// kind.
fn write(
    setting: &KeyPathSetting,
    value: &setting_value::Value,
    document: &mut SettingsDocument,
) -> Option<()> {
    match (setting, value) {
        (KeyPathSetting::Switch(key), setting_value::Value::Boolean(value)) => {
            document.set_bool(key, *value);
        }
        (KeyPathSetting::Text { name, .. }, setting_value::Value::Text(text)) => {
            document.set_raw_string(name, text);
        }
        _ => return None,
    }
    Some(())
}

/// The whitelist as `ConfigureReply` carries it.
pub(crate) fn descriptors() -> Vec<SettingDescriptor> {
    WHITELIST.iter().map(descriptor).collect()
}

/// Why a `SettingsSnapshot` was refused; the previous snapshot stays.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SettingsRefusal {
    UnknownName(String),
    RepeatedName(String),
    MissingValue(String),
    WrongKind(String),
}

/// The document a whole `SettingsSnapshot` describes, checked entry by entry
/// before anything is replaced.
pub(crate) fn document_from(entries: &[SettingEntry]) -> Result<SettingsDocument, SettingsRefusal> {
    let mut document = SettingsDocument::default();
    let mut seen = HashSet::new();
    for entry in entries {
        let name = &entry.name;
        let setting = WHITELIST
            .iter()
            .find(|setting| setting.name() == name)
            .ok_or_else(|| SettingsRefusal::UnknownName(name.clone()))?;
        if !seen.insert(name.as_str()) {
            return Err(SettingsRefusal::RepeatedName(name.clone()));
        }
        let value = entry
            .value
            .as_ref()
            .and_then(|value| value.value.as_ref())
            .ok_or_else(|| SettingsRefusal::MissingValue(name.clone()))?;
        write(setting, value, &mut document)
            .ok_or_else(|| SettingsRefusal::WrongKind(name.clone()))?;
    }
    Ok(document)
}

/// The last snapshot a request carried; the defaults until the first one.
#[derive(Default)]
pub(crate) struct SnapshotSettings {
    document: Mutex<Arc<SettingsDocument>>,
}

impl SnapshotSettings {
    /// Puts `document` in force and hands back the one it replaced — for
    /// a request that changed nothing to put back.
    pub(crate) fn replace(&self, document: Arc<SettingsDocument>) -> Arc<SettingsDocument> {
        std::mem::replace(&mut *self.lock(), document)
    }

    /// A poisoned lock is recovered: the value is a whole `Arc`, swapped in
    /// one assignment, so no panic can leave it half written.
    fn lock(&self) -> std::sync::MutexGuard<'_, Arc<SettingsDocument>> {
        self.document
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl SettingsProvider for SnapshotSettings {
    fn current(&self) -> Arc<SettingsDocument> {
        Arc::clone(&self.lock())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{boolean, text};
    use taigi_desktop_core::keys::{
        ChordField, ComposingAction, ComposingKeyBindings, ToneInputScheme,
    };
    use taigi_desktop_core::platform::DesktopPlatform;
    use taigi_desktop_core::settings::{
        CandidateDisplayMode, EngineSettings, InputMode, SettingChoice,
    };

    /// trace: EngineSettings::DEFAULT — inputMode "tl", candidateDisplayMode
    /// "sideBySide"; toneInputScheme "standard"; autoSpaceEnabled false;
    /// candidateWindowEnabled true; chords carry no default.
    #[test]
    fn descriptors_carry_the_core_defaults() {
        let descriptors = descriptors();
        let default_of = |name: &str| {
            descriptors
                .iter()
                .find(|d| d.name == name)
                .unwrap_or_else(|| panic!("{name} not described"))
                .default_value
                .clone()
                .and_then(|value| value.value)
        };
        let text_default = |raw: &str| Some(setting_value::Value::Text(raw.to_owned()));
        assert_eq!(
            default_of("inputMode"),
            text_default(InputMode::DEFAULT.raw())
        );
        assert_eq!(
            default_of("candidateDisplayMode"),
            text_default(CandidateDisplayMode::DEFAULT.raw())
        );
        assert_eq!(
            default_of("toneInputScheme"),
            text_default(ToneInputScheme::DEFAULT.raw())
        );
        assert_eq!(
            default_of("autoSpaceEnabled"),
            Some(setting_value::Value::Boolean(false))
        );
        assert_eq!(
            default_of("candidateWindowEnabled"),
            Some(setting_value::Value::Boolean(true))
        );
        assert_eq!(default_of("composingShortcut.nextCandidate"), None);
        for descriptor in &descriptors {
            let kind = SettingKind::try_from(descriptor.kind).unwrap();
            assert_ne!(kind, SettingKind::Unspecified, "{}", descriptor.name);
        }
    }

    #[test]
    fn no_entries_read_as_the_defaults() {
        let document = document_from(&[]).unwrap();
        assert_eq!(document.engine_settings(), EngineSettings::DEFAULT);
    }

    /// The snapshot's values reach what the key path reads; a chord in the Mac
    /// spelling (`o` = ⌥) parses in the MacOS grammar, and a stored `""`
    /// stays a cleared row rather than the default.
    #[test]
    fn entries_reach_the_key_path() {
        let document = document_from(&[
            text("inputMode", "poj"),
            boolean("isTranslateSwapped", true),
            boolean("moeDictEnabled", false),
            text("composingShortcut.pageForward", "o|005D"),
            text("composingShortcut.nextCandidate", ""),
            text("composingShortcut.confirmHighlighted.second", "|0020"),
        ])
        .unwrap();
        let engine = document.engine_settings();
        assert_eq!(engine.input_mode, InputMode::Poj);
        assert!(engine.is_hanji_first);
        assert!(!engine.dictionary_sources.kautian);
        let bindings = ComposingKeyBindings::from_document(&document, DesktopPlatform::MacOS);
        let page_forward = bindings
            .chord(ComposingAction::PageForward, ChordField::First)
            .expect("an ⌥] chord");
        assert_eq!(page_forward.raw_value(DesktopPlatform::MacOS), "o|005D");
        assert_eq!(
            bindings.chord(ComposingAction::NextCandidate, ChordField::First),
            None
        );
        let second = bindings
            .chord(ComposingAction::ConfirmHighlighted, ChordField::Second)
            .expect("a second-field Space");
        assert_eq!(second.raw_value(DesktopPlatform::MacOS), "|0020");
    }

    #[test]
    fn a_bad_entry_refuses_the_whole_request() {
        assert_eq!(
            document_from(&[
                boolean("isTranslateSwapped", true),
                boolean("displayLanguage", true)
            ])
            .unwrap_err(),
            SettingsRefusal::UnknownName("displayLanguage".to_owned())
        );
        assert_eq!(
            document_from(&[
                boolean("autoSpaceEnabled", true),
                boolean("autoSpaceEnabled", false)
            ])
            .unwrap_err(),
            SettingsRefusal::RepeatedName("autoSpaceEnabled".to_owned())
        );
        assert_eq!(
            document_from(&[SettingEntry {
                name: "autoSpaceEnabled".to_owned(),
                value: Some(SettingValue { value: None }),
            }])
            .unwrap_err(),
            SettingsRefusal::MissingValue("autoSpaceEnabled".to_owned())
        );
        assert_eq!(
            document_from(&[text("autoSpaceEnabled", "true")]).unwrap_err(),
            SettingsRefusal::WrongKind("autoSpaceEnabled".to_owned())
        );
        assert_eq!(
            document_from(&[boolean("inputMode", true)]).unwrap_err(),
            SettingsRefusal::WrongKind("inputMode".to_owned())
        );
    }

    /// A snapshot replaces the last one whole: a name left out reads as its
    /// default again.
    #[test]
    fn a_snapshot_replaces_the_last_one() {
        let settings = SnapshotSettings::default();
        settings.replace(Arc::new(
            document_from(&[text("inputMode", "poj")]).unwrap(),
        ));
        assert_eq!(
            settings.current().engine_settings().input_mode,
            InputMode::Poj
        );
        settings.replace(Arc::new(document_from(&[]).unwrap()));
        assert_eq!(
            settings.current().engine_settings(),
            EngineSettings::DEFAULT
        );
    }

    /// A stored `"tps"` — the picker's, the chord's or a restored mobile
    /// backup's — types TPS.
    #[test]
    fn a_stored_tps_reads_as_tps() {
        let document = document_from(&[text("inputMode", "tps")]).unwrap();
        assert_eq!(document.engine_settings().input_mode, InputMode::Tps);
    }
}
