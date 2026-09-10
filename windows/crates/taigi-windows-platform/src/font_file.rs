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

#[cfg(windows)]
pub use imp::load;

#[cfg(windows)]
mod imp {
    use super::{FontFaceInfo, FontFileError};
    use std::path::Path;
    use windows::core::{Interface, BOOL, PCWSTR, PWSTR};
    use windows::Win32::Graphics::DirectWrite::{
        DWriteCreateFactory, IDWriteFactory3, IDWriteFontCollection1, IDWriteFontFile,
        IDWriteFontSetBuilder1, IDWriteLocalizedStrings, DWRITE_FACTORY_TYPE_ISOLATED,
        DWRITE_FONT_FACE_TYPE_UNKNOWN, DWRITE_FONT_FILE_TYPE_UNKNOWN,
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

    fn file_reference(
        factory: &IDWriteFactory3,
        path: &Path,
    ) -> Result<IDWriteFontFile, FontFileError> {
        let wide = to_wide_nul(&path.to_string_lossy());
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
        // SAFETY: the family is live and owns the strings it hands back.
        let names = unsafe { family.GetFamilyNames() }.map_err(refused)?;
        localized_string(&names, 0)
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

    fn to_wide_nul(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
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
    /// package (`fonts/font/`).
    fn bundled_typeface() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fonts/font/iansui_regular.ttf")
    }

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
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

            let family = wide(&info.family_name);
            let locale = wide("zh-TW");
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
}
