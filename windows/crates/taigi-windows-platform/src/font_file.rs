//! What DirectWrite can say about a font file: whether it is one, what its
//! first family is called, and a collection holding just that file.
//!
//! Wanted by both binaries, which is why it lives here: the settings window
//! validates an imported file with it, and the DLL draws with it. Neither can
//! hold it — the settings crate is UI the DLL must never link, and the DLL is
//! not on the settings window's side of the install.
//!
//! ONE FAMILY PER FILE. A `.ttc` contributes its first family and no other,
//! mirroring `CustomFontLibrary.swift`'s "a `.ttc` contributes its first
//! face": the stored selection is a file name with no room for a face index,
//! so offering the rest would be offering rows nothing could name.
//!
//! A custom typeface gets a collection OF ITS OWN, never one shared with the
//! bundled roster. DirectWrite has no process-wide font registration — the
//! collection is handed to `CreateTextFormat` explicitly — so a family name
//! this Windows already carries cannot win the lookup, and the macOS library's
//! name-collision refusal has nothing to guard against here.
//!
//! THE COLLECTION MAY NOT OUTLIVE THE FACTORY THAT BUILT IT. An
//! `IDWriteFontCollection1` does not keep its factory alive, and DirectWrite
//! reads factory-owned state when a layout resolves the family — so a
//! collection whose factory has been released faults inside `DWrite.dll`, not
//! here, and takes the host process with it (2026-09-11: `load` used to make
//! an isolated factory of its own and drop it, and typing in any host with a
//! custom typeface selected killed the host with `0xc0000005`; measured
//! headlessly — keeping that factory alive, or building against the caller's,
//! both remove it). `load` therefore takes the factory the caller will draw
//! with; `inspect` owns a factory for the length of one call and lets nothing
//! built from it escape.
//!
//! On a non-Windows host every function answers "not a font", so the callers
//! compile and their pure parts test natively (`make check` on macOS).

use std::path::Path;

/// A typeface file this Windows can draw with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontFaceInfo {
    /// The family name the file declares. What a text format is asked for, and
    /// what the picker shows. Untrusted text out of a file the user chose:
    /// display it, never log it or build a path from it.
    pub family_name: String,
}

#[derive(Debug, thiserror::Error)]
pub enum FontFileError {
    #[error("the file carries no typeface this Windows can read")]
    NotAFont,
    #[error("the typeface has no family name")]
    NoFamilyName,
    #[error("DirectWrite refused the file: {0}")]
    DirectWrite(String),
}

/// Whether `path` is a typeface this Windows can draw with, and what its first
/// family is called. What an import validates with.
///
/// Owns a factory for the length of the call: only the NAME comes back, and
/// the collection is dropped before the factory that made it.
pub fn inspect(path: &Path) -> Result<FontFaceInfo, FontFileError> {
    #[cfg(windows)]
    {
        imp::inspect(path)
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        Err(FontFileError::NotAFont)
    }
}

/// Every family the OS has installed, by the name `family_name_of` picks, in
/// DirectWrite's order. Read fresh each call — `check_for_updates` is on, so a
/// font installed or removed since the last call is reflected.
///
/// What the Manage Typefaces pane lists after the bundled and imported rows (and sorts,
/// by the same fold it searches with); nothing is loaded for a family until
/// the candidate window asks the system collection for it.
pub fn system_families() -> Result<Vec<String>, FontFileError> {
    #[cfg(windows)]
    {
        imp::system_families()
    }
    #[cfg(not(windows))]
    {
        Err(FontFileError::NotAFont)
    }
}

/// One weight of an installed family, as the Manage Typefaces pane offers it
/// and the candidate window draws it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FamilyFace {
    /// The face name the family declares for it (`FAMILY_NAME_LOCALE`, else
    /// its first) — what the pane shows and `installedFontFace` stores.
    pub name: String,
    /// `DWRITE_FONT_WEIGHT`, 1–999: what a text format asks for.
    pub weight: i32,
}

/// The weights of one installed family, and which of them it draws in when no
/// weight was picked.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FamilyFaces {
    /// Upright, normal-width and real (not simulated) faces only, so every one
    /// is a WEIGHT of the family rather than an italic or a condensed cut —
    /// lightest first, one per name (`ordered_faces`).
    pub faces: Vec<FamilyFace>,
    /// The name of the face DirectWrite matches for `DWRITE_FONT_WEIGHT_NORMAL`
    /// — what a family with no weight picked has always drawn in, because that
    /// is what the candidate window asks for then. Not necessarily one of
    /// `faces` (a family whose normal match is condensed, say).
    pub default_face: Option<String>,
}

impl FamilyFaces {
    /// The face called `name`, when the family has it. "" names none.
    pub fn named(&self, name: &str) -> Option<&FamilyFace> {
        self.faces
            .iter()
            .find(|face| !name.is_empty() && face.name == name)
    }

    /// What the pane shows selected for the stored `name`: that face, or —
    /// for "" (no weight picked) or a face the family no longer has — the one
    /// the candidate window then draws (`default_face`). `None` when that is
    /// not one of `faces`.
    pub fn shown(&self, name: &str) -> Option<&FamilyFace> {
        self.named(name).or_else(|| {
            let default = self.default_face.as_deref()?;
            self.faces.iter().find(|face| face.name == default)
        })
    }
}

/// The weights of `family` the OS has installed (`FamilyFaces`); empty when
/// the OS has no such family.
///
/// What the Manage Typefaces pane offers for the selected installed family;
/// the candidate window reads the same list out of its own collection
/// (`faces_in`), so the two agree on what a stored name means.
pub fn system_family_faces(family: &str) -> Result<FamilyFaces, FontFileError> {
    #[cfg(windows)]
    {
        imp::system_family_faces(family)
    }
    #[cfg(not(windows))]
    {
        let _ = family;
        Err(FontFileError::NotAFont)
    }
}

/// `faces` in weight order, one per name.
///
/// A family that names two upright, normal-width faces alike is malformed
/// (OpenType requires subfamily names to be unique in a family); the lighter
/// one is kept so a stored name always means the same face.
pub fn ordered_faces(mut faces: Vec<FamilyFace>) -> Vec<FamilyFace> {
    faces.sort_by_key(|face| face.weight);
    let mut seen = std::collections::HashSet::new();
    faces.retain(|face| seen.insert(face.name.clone()));
    faces
}

/// The locale a family is named in for storage, display and lookup alike.
///
/// One rule for every reader: the name `inspect` reads out of a file, the
/// name the pane lists and stores, and the name the renderer looks up have to
/// be the SAME string for the same family, or an import would not find the
/// installed row it duplicates and a stored selection would not resolve.
/// `en-us` first because nearly every family carries it; the first name the
/// family declares otherwise.
pub const FAMILY_NAME_LOCALE: &str = "en-us";

#[cfg(windows)]
pub use imp::{faces_in, load, system_collection};

#[cfg(windows)]
mod imp {
    use super::{FamilyFace, FamilyFaces, FontFaceInfo, FontFileError};
    use std::path::Path;
    use windows::core::{Interface, BOOL, PCWSTR, PWSTR};
    use windows::Win32::Graphics::DirectWrite::{
        DWriteCreateFactory, IDWriteFactory3, IDWriteFontCollection1, IDWriteFontFamily,
        IDWriteFontFile, IDWriteFontSetBuilder1, IDWriteLocalizedStrings,
        DWRITE_FACTORY_TYPE_ISOLATED, DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_FACE_TYPE_UNKNOWN,
        DWRITE_FONT_FILE_TYPE_UNKNOWN, DWRITE_FONT_SIMULATIONS_NONE, DWRITE_FONT_STRETCH_NORMAL,
        DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_NORMAL,
    };

    /// The file's family as a collection of its own, plus its name.
    ///
    /// `factory` is the caller's, and the collection it answers with is only
    /// usable while that factory lives — see the module doc for what happens
    /// when it does not. Draw with the SAME factory the text formats are made
    /// on, which is what the bundled roster already does
    /// (`ui::render::load_private_fonts`).
    pub fn load(
        factory: &IDWriteFactory3,
        path: &Path,
    ) -> Result<(IDWriteFontCollection1, FontFaceInfo), FontFileError> {
        let file = file_reference(factory, path)?;
        require_supported(&file)?;
        let collection = collection_of(factory, &file)?;
        let family_name = first_family_name(&collection)?;
        Ok((collection, FontFaceInfo { family_name }))
    }

    /// The family name alone, read under a factory of this call's own.
    ///
    /// Isolated: a validation pass in the settings window has no reason to
    /// share caches with anything. Nothing built from that factory leaves —
    /// the collection is dropped here, while the factory that made it is
    /// still alive.
    pub fn inspect(path: &Path) -> Result<FontFaceInfo, FontFileError> {
        // SAFETY: plain factory creation on the calling thread; the type
        // parameter names the interface asked for.
        let factory: IDWriteFactory3 =
            unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_ISOLATED) }.map_err(refused)?;
        let (collection, info) = load(&factory, path)?;
        drop(collection);
        Ok(info)
    }

    /// The OS's font collection, as this factory sees it. `check_for_updates`
    /// makes DirectWrite look for fonts installed or removed since the factory
    /// last built the collection; without it, the answer may predate them.
    pub fn system_collection(
        factory: &IDWriteFactory3,
        check_for_updates: bool,
    ) -> Result<IDWriteFontCollection1, FontFileError> {
        let mut collection: Option<IDWriteFontCollection1> = None;
        // SAFETY: the out-parameter is a live local; the call fills it or
        // fails. Downloadable (cloud) fonts are left out: a family the OS has
        // not fetched yet is not one the candidate window can draw in now.
        unsafe { factory.GetSystemFontCollection(false, &mut collection, check_for_updates) }
            .map_err(refused)?;
        collection.ok_or_else(|| FontFileError::DirectWrite("no system font collection".to_owned()))
    }

    pub fn system_families() -> Result<Vec<String>, FontFileError> {
        // SHARED, unlike `inspect`: enumerating the OS's fonts is exactly the
        // work DirectWrite's process-wide cache exists for, and nothing built
        // here escapes to be drawn with.
        // SAFETY: plain factory creation on the calling thread.
        let factory: IDWriteFactory3 =
            unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED) }.map_err(refused)?;
        let collection = system_collection(&factory, true)?;
        // SAFETY: the collection is live; the call only reads its count.
        let count = unsafe { collection.GetFontFamilyCount() };
        let mut families = Vec::with_capacity(count as usize);
        for index in 0..count {
            // SAFETY: `index` is inside the count read above.
            let Ok(family) = (unsafe { collection.GetFontFamily(index) }) else {
                continue;
            };
            // A family with no readable name is skipped rather than failing
            // the list: one odd font must not empty the pane.
            if let Ok(name) = family_name_of(&family) {
                families.push(name);
            }
        }
        Ok(families)
    }

    pub fn system_family_faces(family: &str) -> Result<FamilyFaces, FontFileError> {
        // SHARED for the reason `system_families` is; nothing built here
        // escapes either.
        // SAFETY: plain factory creation on the calling thread.
        let factory: IDWriteFactory3 =
            unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED) }.map_err(refused)?;
        let collection = system_collection(&factory, true)?;
        Ok(faces_in(&collection, family).unwrap_or_default())
    }

    /// `family`'s weights in `collection` (`super::system_family_faces`), or
    /// `None` because the collection has no such family — the one lookup that
    /// both checks the family and reads it.
    /// A name with an embedded NUL is not one DirectWrite can be asked for —
    /// it would be truncated into some other, valid name — so it is no family.
    pub fn faces_in(collection: &IDWriteFontCollection1, family: &str) -> Option<FamilyFaces> {
        if family.is_empty() || family.contains('\0') {
            return None;
        }
        let name = super::to_wide_nul(family);
        let mut index = 0_u32;
        let mut exists = BOOL(0);
        // SAFETY: a live NUL-terminated name and two live out-parameters.
        let found =
            unsafe { collection.FindFamilyName(PCWSTR(name.as_ptr()), &mut index, &mut exists) }
                .is_ok()
                && exists.as_bool();
        if !found {
            return None;
        }
        // SAFETY: `index` is the one `FindFamilyName` just answered with.
        let Ok(family) = (unsafe { collection.GetFontFamily(index) }) else {
            return None;
        };
        let family: &IDWriteFontFamily = &family;
        // SAFETY: the family is live; the call only reads its count.
        let count = unsafe { family.GetFontCount() };
        let mut faces = Vec::with_capacity(count as usize);
        for index in 0..count {
            // SAFETY: `index` is inside the count read above.
            let Ok(font) = (unsafe { family.GetFont(index) }) else {
                continue;
            };
            // SAFETY: plain property reads on a live font.
            let is_weight_of_family = unsafe {
                font.GetStyle() == DWRITE_FONT_STYLE_NORMAL
                    && font.GetStretch() == DWRITE_FONT_STRETCH_NORMAL
                    && font.GetSimulations() == DWRITE_FONT_SIMULATIONS_NONE
            };
            if !is_weight_of_family {
                continue;
            }
            // A face with no readable name is skipped rather than failing the
            // list: nothing could store or show it.
            // SAFETY: the font is live and owns the strings it hands back.
            let Ok(names) = (unsafe { font.GetFaceNames() }) else {
                continue;
            };
            if let Ok(name) = name_in_locale(&names) {
                // SAFETY: a plain property read on a live font.
                let weight = unsafe { font.GetWeight() }.0;
                faces.push(FamilyFace { name, weight });
            }
        }
        // The very request the candidate window makes with no weight picked
        // (`ui::render::format_with`), so the face named here is the one it
        // draws — DirectWrite's own weight/stretch/style distance, not a rule
        // of ours that could pick a neighbour.
        // SAFETY: the family is live; the call only matches among its fonts.
        let default_face = unsafe {
            family.GetFirstMatchingFont(
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
            )
        }
        .ok()
        // SAFETY: the font is live and owns the strings it hands back.
        .and_then(|font| unsafe { font.GetFaceNames() }.ok())
        .and_then(|names| name_in_locale(&names).ok());
        Some(FamilyFaces {
            faces: super::ordered_faces(faces),
            default_face,
        })
    }

    /// The name a family goes by everywhere in this program
    /// (`FAMILY_NAME_LOCALE`, else its first).
    pub(super) fn family_name_of(family: &IDWriteFontFamily) -> Result<String, FontFileError> {
        // SAFETY: the family is live and owns the strings it hands back.
        let names = unsafe { family.GetFamilyNames() }.map_err(refused)?;
        name_in_locale(&names)
    }

    /// `names` in `FAMILY_NAME_LOCALE`, else its first — the one rule for a
    /// family's name and a face's name alike.
    fn name_in_locale(names: &IDWriteLocalizedStrings) -> Result<String, FontFileError> {
        let locale = super::to_wide_nul(super::FAMILY_NAME_LOCALE);
        let mut index = 0_u32;
        let mut exists = BOOL(0);
        // SAFETY: a live NUL-terminated locale name and two live out-parameters.
        let found =
            unsafe { names.FindLocaleName(PCWSTR(locale.as_ptr()), &mut index, &mut exists) }
                .is_ok()
                && exists.as_bool();
        localized_string(names, if found { index } else { 0 })
    }

    fn file_reference(
        factory: &IDWriteFactory3,
        path: &Path,
    ) -> Result<IDWriteFontFile, FontFileError> {
        let wide = super::to_wide_nul(&path.to_string_lossy());
        // SAFETY: a live NUL-terminated path for the duration of the call. The
        // write-time argument is `None` on purpose: DirectWrite then reads the
        // file's own, and a fabricated one makes later operations fail
        // (`CreateFontFileReference`).
        unsafe { factory.CreateFontFileReference(PCWSTR(wide.as_ptr()), None) }.map_err(refused)
    }

    /// Recognized is not supported: `Analyze` answers those in two different
    /// out-parameters, and a file with no faces is neither.
    fn require_supported(file: &IDWriteFontFile) -> Result<(), FontFileError> {
        let mut is_supported = BOOL(0);
        let mut file_type = DWRITE_FONT_FILE_TYPE_UNKNOWN;
        let mut face_type = DWRITE_FONT_FACE_TYPE_UNKNOWN;
        let mut face_count = 0_u32;
        // SAFETY: four out-parameters, each a live local of the documented
        // type; the call fills them or fails.
        unsafe {
            file.Analyze(
                &mut is_supported,
                &mut file_type,
                Some(&mut face_type),
                &mut face_count,
            )
        }
        .map_err(refused)?;
        if !is_supported.as_bool() || face_count == 0 {
            return Err(FontFileError::NotAFont);
        }
        Ok(())
    }

    fn collection_of(
        factory: &IDWriteFactory3,
        file: &IDWriteFontFile,
    ) -> Result<IDWriteFontCollection1, FontFileError> {
        // SAFETY: a DWrite object on the calling thread, asked for a builder.
        let builder = unsafe { factory.CreateFontSetBuilder() }.map_err(refused)?;
        let builder: IDWriteFontSetBuilder1 = builder.cast().map_err(refused)?;
        // SAFETY: the builder and the file reference are both live; the
        // builder copies what it needs out of the reference.
        unsafe { builder.AddFontFile(file) }.map_err(refused)?;
        // SAFETY: the builder is live and holds the one file added above.
        let set = unsafe { builder.CreateFontSet() }.map_err(refused)?;
        // SAFETY: the set is live and was made by this factory.
        unsafe { factory.CreateFontCollectionFromFontSet(&set) }.map_err(refused)
    }

    /// The name of the collection's first family — the one a text format will
    /// be asked for.
    fn first_family_name(collection: &IDWriteFontCollection1) -> Result<String, FontFileError> {
        // SAFETY: the collection is live; the call only reads its count.
        if unsafe { collection.GetFontFamilyCount() } == 0 {
            return Err(FontFileError::NoFamilyName);
        }
        // SAFETY: index 0 is inside the count checked above.
        let family = unsafe { collection.GetFontFamily(0) }.map_err(refused)?;
        family_name_of(&family)
    }

    /// One string out of a `IDWriteLocalizedStrings`, read through the vtable.
    ///
    /// NOT the generated `GetString(&mut [u16])` wrapper: it hands the OS
    /// `transmute(slice.as_ptr())`, a read-only provenance, and an optimized
    /// build may fold our reads of the buffer back to its initializer — the
    /// defect `os_out_buffer` carries the measurement for. `windows/clippy.toml`
    /// denies the wrapper.
    fn localized_string(
        names: &IDWriteLocalizedStrings,
        index: u32,
    ) -> Result<String, FontFileError> {
        // SAFETY: `names` is live; `index` is the caller's, and the length
        // query fails rather than writing when it is out of range.
        let length = unsafe { names.GetStringLength(index) }.map_err(refused)? as usize;
        if length == 0 {
            return Err(FontFileError::NoFamilyName);
        }
        // The length excludes the terminating NUL, which the call writes.
        let mut buffer = vec![0_u16; length + 1];
        // SAFETY: the same vtable slot the wrapper uses, with `as_mut_ptr` in
        // place of `as_ptr` so the pointer the OS writes through carries
        // write provenance; the buffer is live for the call and DirectWrite
        // keeps neither it nor the pointer.
        unsafe {
            (Interface::vtable(names).GetString)(
                Interface::as_raw(names),
                index,
                PWSTR(buffer.as_mut_ptr()),
                buffer.len() as u32,
            )
            .ok()
            .map_err(refused)?;
        }
        let name = String::from_utf16_lossy(&buffer[..length]);
        if name.is_empty() {
            Err(FontFileError::NoFamilyName)
        } else {
            Ok(name)
        }
    }

    fn refused(error: windows::core::Error) -> FontFileError {
        FontFileError::DirectWrite(error.to_string())
    }
}

#[cfg(windows)]
fn to_wide_nul(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod face_tests {
    use super::*;

    fn face(name: &str, weight: i32) -> FamilyFace {
        FamilyFace {
            name: name.to_owned(),
            weight,
        }
    }

    fn light_regular_bold(default_face: Option<&str>) -> FamilyFaces {
        FamilyFaces {
            faces: vec![face("Light", 300), face("Regular", 400), face("Bold", 700)],
            default_face: default_face.map(str::to_owned),
        }
    }

    #[test]
    fn faces_are_ordered_lightest_first_one_per_name() {
        let faces = ordered_faces(vec![
            face("Bold", 700),
            face("Light", 300),
            face("Regular", 400),
            face("Bold", 800),
        ]);

        assert_eq!(
            faces,
            vec![face("Light", 300), face("Regular", 400), face("Bold", 700)],
        );
    }

    #[test]
    fn a_named_face_is_shown_as_itself() {
        let faces = light_regular_bold(Some("Regular"));

        assert_eq!(faces.named("Bold"), Some(&face("Bold", 700)));
        assert_eq!(faces.shown("Bold"), Some(&face("Bold", 700)));
    }

    /// No weight picked, or one the family no longer has, shows the face
    /// DirectWrite matches for normal — whichever weight that is.
    #[test]
    fn no_name_or_a_missing_one_shows_the_default_face() {
        let faces = light_regular_bold(Some("Light"));

        assert_eq!(faces.named(""), None);
        assert_eq!(faces.shown(""), Some(&face("Light", 300)));
        assert_eq!(faces.named("Black"), None);
        assert_eq!(faces.shown("Black"), Some(&face("Light", 300)));
    }

    /// A default match that is not one of the offered weights shows none
    /// selected rather than a neighbour that does not draw.
    #[test]
    fn a_default_face_outside_the_offered_weights_shows_none() {
        assert_eq!(light_regular_bold(Some("Condensed")).shown(""), None);
        assert_eq!(light_regular_bold(None).shown(""), None);
    }
}

/// The collection `load` answers with has to survive being drawn: the caller's
/// factory builds it, a text format asks that factory for the family, and a
/// layout resolves it. That last step is where a collection whose factory had
/// been released faulted inside `DWrite.dll` and took the host process with it
/// — an access violation, so this test does not fail with a message: the test
/// process dies and cargo reports the signal.
///
/// Windows-only, and it needs the bundled typefaces: `make check-box` runs it
/// on the box, out of a checkout that has them.
#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use windows::core::{Interface, PCWSTR};
    use windows::Win32::Graphics::DirectWrite::{
        DWriteCreateFactory, IDWriteFactory3, DWRITE_FACTORY_TYPE_SHARED,
        DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_NORMAL,
        DWRITE_TEXT_METRICS,
    };

    /// A typeface every checkout has: the repo-root folder all four platforms
    /// package (`assets/fonts/font/`).
    fn bundled_typeface() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../assets/fonts/font/iansui_regular.ttf")
    }

    #[test]
    fn a_loaded_collection_still_draws_after_the_call_that_built_it() {
        let path = bundled_typeface();
        assert!(path.is_file(), "no bundled typeface at {}", path.display());
        // SAFETY: DirectWrite calls on the test thread, each out-parameter a
        // live local; the factory outlives everything built from it.
        unsafe {
            let factory: IDWriteFactory3 =
                DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED).expect("a shared factory");

            let (collection, info) = load(&factory, &path).expect("the typeface loads");
            assert!(!info.family_name.is_empty());

            let family = to_wide_nul(&info.family_name);
            let locale = to_wide_nul("zh-TW");
            let format = factory
                .CreateTextFormat(
                    PCWSTR(family.as_ptr()),
                    &collection
                        .cast::<windows::Win32::Graphics::DirectWrite::IDWriteFontCollection>()
                        .expect("a font collection"),
                    DWRITE_FONT_WEIGHT_NORMAL,
                    DWRITE_FONT_STYLE_NORMAL,
                    DWRITE_FONT_STRETCH_NORMAL,
                    18.0,
                    PCWSTR(locale.as_ptr()),
                )
                .expect("a text format");

            let text: Vec<u16> = "台語 tâi-gí".encode_utf16().collect();
            let layout = factory
                .CreateTextLayout(&text, &format, 400.0, 100.0)
                .expect("a layout");
            let mut metrics = DWRITE_TEXT_METRICS::default();
            layout.GetMetrics(&mut metrics).expect("metrics");

            assert!(
                metrics.width > 0.0 && metrics.height > 0.0,
                "the text measured to nothing: {} x {}",
                metrics.width,
                metrics.height,
            );
        }
    }

    /// Every name `system_families` lists resolves back through
    /// `FindFamilyName` in the same collection — the round trip the stored
    /// selection depends on (`ui::render::installed_font_id`). The docs
    /// promise a case-insensitive exact match and say nothing about localized
    /// aliases, so this is what proves the `FAMILY_NAME_LOCALE` rule holds on
    /// a real system, including families that carry no `en-us` name.
    #[test]
    fn every_listed_family_is_found_again_by_the_name_it_was_listed_under() {
        let families = system_families().expect("the system collection lists");
        assert!(
            families.len() > 10,
            "suspiciously few families: {families:?}"
        );
        // SAFETY: DirectWrite calls on the test thread with live locals.
        unsafe {
            let factory: IDWriteFactory3 =
                DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED).expect("a shared factory");
            let collection = system_collection(&factory, false).expect("the system collection");
            for family in &families {
                let name = to_wide_nul(family);
                let mut index = 0_u32;
                let mut exists = windows::core::BOOL(0);
                collection
                    .FindFamilyName(PCWSTR(name.as_ptr()), &mut index, &mut exists)
                    .expect("FindFamilyName");
                assert!(exists.as_bool(), "listed family not found again: {family}");
            }
        }
    }
}
