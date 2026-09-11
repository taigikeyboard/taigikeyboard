//! The typeface selection as it is STORED: three keys, read and written as one.
//!
//! `fontType` holds a bundled face's raw value, `CandidateFontSelection::
//! CUSTOM_RAW` with `customFontFile` naming the stored file, or
//! `CandidateFontSelection::INSTALLED_RAW` with `installedFontFamily` naming
//! the OS family. They are separate keys in one document, so every reader
//! tolerates one without the others — and one writer sets all three, in one
//! update, so a save can never leave `fontType` naming a custom or installed
//! typeface with nothing beside it, or a stale companion beside a bundled face.
//!
//! Mirrors `StoredFontSelection` in
//! `macos/Sources/TaigiInputMethodCore/Settings/StoredFontSelection.swift`. What
//! is NOT here is turning a file name or a family into something drawable:
//! that needs DirectWrite, and lives with the code that draws.

use super::choices::{CandidateFontChoice, CandidateFontSelection, SettingChoice};
use super::document::SettingsDocument;
use super::keys;

/// What the three keys say, before anything has tried to load a file or find
/// a family.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StoredFontSelection {
    BuiltIn(CandidateFontChoice),
    /// The library file name the user selected. Untrusted: it is read out of a
    /// document anything can write, so every consumer treats it as one path
    /// COMPONENT (`taigi_windows_storage::remove_stored`).
    Custom(String),
    /// The OS-installed family the user selected, by the name the OS reports.
    /// Untrusted the same way: it reaches DirectWrite as a name to look up,
    /// never as a path.
    Installed(String),
}

/// What `fontType` and its companions name together.
///
/// A `custom` or `installed` with nothing beside it reads as the default face
/// rather than as a half-written selection: three keys are three writes, and a
/// reader must survive landing between them.
pub fn stored_font_selection(document: &SettingsDocument) -> StoredFontSelection {
    // A kind outside the roster reads its companion key, and is the default
    // face while that key is empty.
    let companion = |key, wrap: fn(String) -> StoredFontSelection| {
        let value = document.string(key);
        if value.is_empty() {
            StoredFontSelection::BuiltIn(CandidateFontChoice::DEFAULT)
        } else {
            wrap(value)
        }
    };
    match document.raw_string(keys::FONT_TYPE.name) {
        Some(CandidateFontSelection::CUSTOM_RAW) => {
            companion(&keys::CUSTOM_FONT_FILE, StoredFontSelection::Custom)
        }
        Some(CandidateFontSelection::INSTALLED_RAW) => {
            companion(&keys::INSTALLED_FONT_FAMILY, StoredFontSelection::Installed)
        }
        _ => StoredFontSelection::BuiltIn(document.choice(&keys::FONT_TYPE)),
    }
}

/// Writes all three keys for `selection`.
///
/// The one writer, so a pick of one kind always clears the other kinds'
/// companions: a stale one left beside `fontType` would come back the moment
/// the user picked that kind again, selecting a typeface they did not ask for.
pub fn set_stored_font_selection(document: &mut SettingsDocument, selection: &StoredFontSelection) {
    let (file_name, family) = match selection {
        StoredFontSelection::BuiltIn(choice) => {
            document.set_choice(&keys::FONT_TYPE, *choice);
            ("", "")
        }
        StoredFontSelection::Custom(file_name) => {
            document.set_raw_string(keys::FONT_TYPE.name, CandidateFontSelection::CUSTOM_RAW);
            (file_name.as_str(), "")
        }
        StoredFontSelection::Installed(family) => {
            document.set_raw_string(keys::FONT_TYPE.name, CandidateFontSelection::INSTALLED_RAW);
            ("", family.as_str())
        }
    };
    document.set_string(&keys::CUSTOM_FONT_FILE, file_name);
    document.set_string(&keys::INSTALLED_FONT_FAMILY, family);
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

    #[test]
    fn an_installed_family_round_trips_and_keeps_font_type_out_of_the_roster() {
        let mut document = document();
        let selection = StoredFontSelection::Installed("Microsoft JhengHei".to_owned());

        set_stored_font_selection(&mut document, &selection);

        assert_eq!(stored_font_selection(&document), selection);
        assert_eq!(
            document.raw_string(keys::FONT_TYPE.name),
            Some(CandidateFontSelection::INSTALLED_RAW),
        );
        assert_eq!(document.string(&keys::CUSTOM_FONT_FILE), "");
        assert_eq!(
            document.choice(&keys::FONT_TYPE),
            CandidateFontChoice::DEFAULT
        );
    }

    /// Each kind clears the other kinds' companions.
    #[test]
    fn picking_one_kind_clears_the_other_kinds_companions() {
        let mut document = document();
        set_stored_font_selection(
            &mut document,
            &StoredFontSelection::Installed("Arial".to_owned()),
        );
        set_stored_font_selection(
            &mut document,
            &StoredFontSelection::Custom("mine.ttf".to_owned()),
        );
        assert_eq!(document.string(&keys::INSTALLED_FONT_FAMILY), "");

        set_stored_font_selection(
            &mut document,
            &StoredFontSelection::Installed("Arial".to_owned()),
        );
        assert_eq!(document.string(&keys::CUSTOM_FONT_FILE), "");
    }

    #[test]
    fn installed_with_no_family_beside_it_reads_as_the_default_face() {
        let mut document = document();
        document.set_raw_string(keys::FONT_TYPE.name, CandidateFontSelection::INSTALLED_RAW);

        assert_eq!(
            stored_font_selection(&document),
            StoredFontSelection::BuiltIn(CandidateFontChoice::DEFAULT),
        );
    }

    #[test]
    fn a_stale_family_beside_a_bundled_face_is_ignored() {
        let mut document = document();
        document.set_choice(&keys::FONT_TYPE, CandidateFontChoice::OpenHuninn);
        document.set_string(&keys::INSTALLED_FONT_FAMILY, "Arial");

        assert_eq!(
            stored_font_selection(&document),
            StoredFontSelection::BuiltIn(CandidateFontChoice::OpenHuninn),
        );
    }
}
