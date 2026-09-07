//! The typeface selection as it is STORED: two keys, read and written as one.
//!
//! `fontType` holds either a bundled face's raw value or
//! `CandidateFontSelection::CUSTOM_RAW`, and `customFontFile` names the stored
//! file while it does. The two are separate keys in one document, so every
//! reader tolerates one without the other — and one writer sets both, in one
//! update, so a save can never leave `fontType` naming a custom typeface with
//! no file beside it.
//!
//! Mirrors `SettingsStore.selectedCustomFontFile` /
//! `SettingsStore.candidateFontSelection`
//! (`macos/Sources/TaigiInputMethodCore/Settings/SettingsStore.swift`). What is
//! NOT here is turning a file name into something drawable: that needs
//! DirectWrite, and lives with the code that draws.

use super::choices::{CandidateFontChoice, CandidateFontSelection, SettingChoice};
use super::document::SettingsDocument;
use super::keys;

/// What the two keys say, before anything has tried to load a file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StoredFontSelection {
    BuiltIn(CandidateFontChoice),
    /// The library file name the user selected. Untrusted: it is read out of a
    /// document anything can write, so every consumer treats it as one path
    /// COMPONENT (`taigi_windows_storage::remove_stored`).
    Custom(String),
}

impl StoredFontSelection {
    /// The bundled face to fall back to while a custom selection cannot be
    /// drawn — the same answer `SettingsDocument::choice` gives on its own,
    /// since `CUSTOM_RAW` is not one of the roster's raw values.
    pub fn fallback(&self) -> CandidateFontChoice {
        match self {
            Self::BuiltIn(choice) => *choice,
            Self::Custom(_) => CandidateFontChoice::DEFAULT,
        }
    }
}

/// What `fontType` + `customFontFile` name together.
///
/// A `custom` with no file name beside it reads as the default face rather than
/// as a half-written selection: two keys are two writes, and a reader must
/// survive landing between them.
pub fn stored_font_selection(document: &SettingsDocument) -> StoredFontSelection {
    let is_custom =
        document.raw_string(keys::FONT_TYPE.name) == Some(CandidateFontSelection::CUSTOM_RAW);
    if !is_custom {
        return StoredFontSelection::BuiltIn(document.choice(&keys::FONT_TYPE));
    }
    let file_name = document.string(&keys::CUSTOM_FONT_FILE);
    if file_name.is_empty() {
        StoredFontSelection::BuiltIn(CandidateFontChoice::DEFAULT)
    } else {
        StoredFontSelection::Custom(file_name)
    }
}

/// Writes both halves of the selection into `document`.
///
/// The one writer, so a bundled face always clears the file name: a stale one
/// left beside `fontType` would come back the moment the user picked custom
/// again, selecting a typeface they did not ask for.
pub fn set_stored_font_selection(document: &mut SettingsDocument, selection: &StoredFontSelection) {
    match selection {
        StoredFontSelection::BuiltIn(choice) => {
            document.set_choice(&keys::FONT_TYPE, *choice);
            document.set_string(&keys::CUSTOM_FONT_FILE, "");
        }
        StoredFontSelection::Custom(file_name) => {
            document.set_raw_string(keys::FONT_TYPE.name, CandidateFontSelection::CUSTOM_RAW);
            document.set_string(&keys::CUSTOM_FONT_FILE, file_name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document() -> SettingsDocument {
        SettingsDocument::default()
    }

    #[test]
    fn nothing_stored_is_the_default_face() {
        assert_eq!(
            stored_font_selection(&document()),
            StoredFontSelection::BuiltIn(CandidateFontChoice::DEFAULT),
        );
    }

    #[test]
    fn a_bundled_face_round_trips() {
        let mut document = document();
        let selection = StoredFontSelection::BuiltIn(CandidateFontChoice::GenYoMin);

        set_stored_font_selection(&mut document, &selection);

        assert_eq!(stored_font_selection(&document), selection);
        assert_eq!(document.string(&keys::CUSTOM_FONT_FILE), "");
    }

    #[test]
    fn a_custom_face_round_trips_and_keeps_font_type_out_of_the_roster() {
        let mut document = document();
        let selection = StoredFontSelection::Custom("mine.ttf".to_owned());

        set_stored_font_selection(&mut document, &selection);

        assert_eq!(stored_font_selection(&document), selection);
        assert_eq!(
            document.raw_string(keys::FONT_TYPE.name),
            Some(CandidateFontSelection::CUSTOM_RAW),
        );
        // The roster's own unknown-value fallback is what an older build, or a
        // platform with no font library, reads out of that same key.
        assert_eq!(
            document.choice(&keys::FONT_TYPE),
            CandidateFontChoice::DEFAULT
        );
    }

    #[test]
    fn choosing_a_bundled_face_clears_the_file_name() {
        let mut document = document();
        set_stored_font_selection(
            &mut document,
            &StoredFontSelection::Custom("mine.ttf".to_owned()),
        );

        set_stored_font_selection(
            &mut document,
            &StoredFontSelection::BuiltIn(CandidateFontChoice::Iansui),
        );

        assert_eq!(document.string(&keys::CUSTOM_FONT_FILE), "");
    }

    /// Two keys are two writes; a reader landing between them must still have a
    /// typeface to draw in.
    #[test]
    fn custom_with_no_file_name_beside_it_reads_as_the_default_face() {
        let mut document = document();
        document.set_raw_string(keys::FONT_TYPE.name, CandidateFontSelection::CUSTOM_RAW);

        assert_eq!(
            stored_font_selection(&document),
            StoredFontSelection::BuiltIn(CandidateFontChoice::DEFAULT),
        );
    }

    /// A file name left over from a custom selection does not make a bundled
    /// one custom.
    #[test]
    fn a_stale_file_name_beside_a_bundled_face_is_ignored() {
        let mut document = document();
        document.set_choice(&keys::FONT_TYPE, CandidateFontChoice::OpenHuninn);
        document.set_string(&keys::CUSTOM_FONT_FILE, "left-over.ttf");

        assert_eq!(
            stored_font_selection(&document),
            StoredFontSelection::BuiltIn(CandidateFontChoice::OpenHuninn),
        );
    }
}
