//! The typeface selection as it is STORED: four keys, read and written as one.
//!
//! `fontType` holds a bundled face's raw value, `CandidateFontSelection::
//! CUSTOM_RAW` with `customFontFile` naming the stored file, or
//! `CandidateFontSelection::INSTALLED_RAW` with `installedFontFamily` naming
//! the OS family and `installedFontFace` the weight of it ("" = its default
//! face). They are separate keys in one document, so every reader tolerates
//! one without the others — and one writer sets all four, in one update, so a
//! save can never leave `fontType` naming a custom or installed typeface with
//! nothing beside it, or a stale companion beside another kind — or a face of
//! one family beside another family.
//!
//! What is NOT here is turning a file name or a family into something
//! drawable: that needs DirectWrite, and lives with the code that draws.
//! macOS keeps a Swift twin: `StoredFontSelection` in
//! `macos/Sources/TaigiInputMethodCore/Settings/StoredFontSelection.swift`.

use super::choices::{CandidateFontChoice, CandidateFontSelection, SettingChoice};
use super::document::SettingsDocument;
use super::keys;

/// What the four keys say, before anything has tried to load a file or find
/// a family.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StoredFontSelection {
    BuiltIn(CandidateFontChoice),
    /// The library file name the user selected. Untrusted: it is read out of a
    /// document anything can write, so every consumer treats it as one path
    /// COMPONENT (`taigi_desktop_storage::remove_stored`).
    Custom(String),
    /// The OS-installed family the user selected, by the name the OS reports,
    /// and which of its faces by the face name the OS reports ("" = the
    /// family's default face). Untrusted the same way: both reach DirectWrite
    /// as names to look up, never as a path.
    Installed {
        family: String,
        face: String,
    },
}

impl StoredFontSelection {
    /// An installed family at its default face — what picking its row means.
    pub fn installed_family(family: impl Into<String>) -> Self {
        Self::Installed {
            family: family.into(),
            face: String::new(),
        }
    }

    /// Whether the two name the same typeface, whatever weight of it: what a
    /// picker row — one per family — is compared to the stored selection by.
    pub fn is_same_typeface(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Installed { family, .. }, Self::Installed { family: other, .. }) => {
                family == other
            }
            _ => self == other,
        }
    }
}

/// What `fontType` and its companions name together.
///
/// A `custom` or `installed` with nothing beside it reads as the default face
/// rather than as a half-written selection: the keys are separate writes, and
/// a reader must survive landing between them.
pub fn stored_font_selection(document: &SettingsDocument) -> StoredFontSelection {
    let default = || StoredFontSelection::BuiltIn(CandidateFontChoice::DEFAULT);
    match document.raw_string(keys::FONT_TYPE.name) {
        Some(CandidateFontSelection::CUSTOM_RAW) => {
            let file_name = document.string(&keys::CUSTOM_FONT_FILE);
            if file_name.is_empty() {
                default()
            } else {
                StoredFontSelection::Custom(file_name)
            }
        }
        Some(CandidateFontSelection::INSTALLED_RAW) => {
            let family = document.string(&keys::INSTALLED_FONT_FAMILY);
            if family.is_empty() {
                default()
            } else {
                StoredFontSelection::Installed {
                    family,
                    face: document.string(&keys::INSTALLED_FONT_FACE),
                }
            }
        }
        _ => StoredFontSelection::BuiltIn(document.choice(&keys::FONT_TYPE)),
    }
}

/// Writes all four keys for `selection`.
///
/// The one writer, so a pick of one kind always clears the other kinds'
/// companions: a stale one left beside `fontType` would come back the moment
/// the user picked that kind again, selecting a typeface they did not ask for.
pub fn set_stored_font_selection(document: &mut SettingsDocument, selection: &StoredFontSelection) {
    let (file_name, family, face) = match selection {
        StoredFontSelection::BuiltIn(choice) => {
            document.set_choice(&keys::FONT_TYPE, *choice);
            ("", "", "")
        }
        StoredFontSelection::Custom(file_name) => {
            document.set_raw_string(keys::FONT_TYPE.name, CandidateFontSelection::CUSTOM_RAW);
            (file_name.as_str(), "", "")
        }
        StoredFontSelection::Installed { family, face } => {
            document.set_raw_string(keys::FONT_TYPE.name, CandidateFontSelection::INSTALLED_RAW);
            ("", family.as_str(), face.as_str())
        }
    };
    document.set_string(&keys::CUSTOM_FONT_FILE, file_name);
    document.set_string(&keys::INSTALLED_FONT_FAMILY, family);
    document.set_string(&keys::INSTALLED_FONT_FACE, face);
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
        let selection = StoredFontSelection::Installed {
            family: "Microsoft JhengHei".to_owned(),
            face: "Bold".to_owned(),
        };

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
            &StoredFontSelection::installed_family("Arial"),
        );
        set_stored_font_selection(
            &mut document,
            &StoredFontSelection::Custom("mine.ttf".to_owned()),
        );
        assert_eq!(document.string(&keys::INSTALLED_FONT_FAMILY), "");

        set_stored_font_selection(
            &mut document,
            &StoredFontSelection::installed_family("Arial"),
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

    /// A settings file written before the face key existed: the family at its
    /// default face, which is what it drew then.
    #[test]
    fn an_installed_family_with_no_face_beside_it_reads_as_its_default_face() {
        let mut document = document();
        document.set_raw_string(keys::FONT_TYPE.name, CandidateFontSelection::INSTALLED_RAW);
        document.set_string(&keys::INSTALLED_FONT_FAMILY, "Arial");

        assert_eq!(
            stored_font_selection(&document),
            StoredFontSelection::installed_family("Arial"),
        );
    }

    /// A face belongs to the family it was picked in: another pick, of any
    /// kind, must not carry it over.
    #[test]
    fn picking_another_typeface_clears_the_face() {
        let mut document = document();
        let bold_arial = StoredFontSelection::Installed {
            family: "Arial".to_owned(),
            face: "Bold".to_owned(),
        };
        set_stored_font_selection(&mut document, &bold_arial);

        set_stored_font_selection(
            &mut document,
            &StoredFontSelection::installed_family("Verdana"),
        );
        assert_eq!(document.string(&keys::INSTALLED_FONT_FACE), "");

        set_stored_font_selection(&mut document, &bold_arial);
        set_stored_font_selection(
            &mut document,
            &StoredFontSelection::BuiltIn(CandidateFontChoice::Iansui),
        );
        assert_eq!(document.string(&keys::INSTALLED_FONT_FACE), "");
    }

    /// One picker row per family: its row is the stored selection's at any
    /// weight, and only for installed families.
    #[test]
    fn a_family_is_the_same_typeface_at_any_weight() {
        let bold_arial = StoredFontSelection::Installed {
            family: "Arial".to_owned(),
            face: "Bold".to_owned(),
        };

        assert!(StoredFontSelection::installed_family("Arial").is_same_typeface(&bold_arial));
        assert!(!StoredFontSelection::installed_family("Verdana").is_same_typeface(&bold_arial));
        assert!(!StoredFontSelection::Custom("Arial".to_owned()).is_same_typeface(&bold_arial));
        assert!(StoredFontSelection::Custom("a.ttf".to_owned())
            .is_same_typeface(&StoredFontSelection::Custom("a.ttf".to_owned())));
    }

    #[test]
    fn a_stale_face_beside_a_bundled_face_is_ignored() {
        let mut document = document();
        document.set_choice(&keys::FONT_TYPE, CandidateFontChoice::OpenHuninn);
        document.set_string(&keys::INSTALLED_FONT_FAMILY, "Arial");
        document.set_string(&keys::INSTALLED_FONT_FACE, "Bold");

        assert_eq!(
            stored_font_selection(&document),
            StoredFontSelection::BuiltIn(CandidateFontChoice::OpenHuninn),
        );
    }
}
