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
pub fn inspect(path: &Path) -> Result<FontFaceInfo, FontFileError> {
    #[cfg(windows)]
    {
        imp::load(path).map(|(_, info)| info)
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
    /// Isolated factory: a validation pass in the settings window, and the
    /// TIP's own drawing factory, have no reason to share caches — and this
    /// one is dropped as soon as the collection it made is held.
    pub fn load(path: &Path) -> Result<(IDWriteFontCollection1, FontFaceInfo), FontFileError> {
        // SAFETY: plain factory creation on the calling thread; the type
        // parameter names the interface asked for.
        let factory: IDWriteFactory3 =
            unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_ISOLATED) }.map_err(refused)?;
        let file = file_reference(&factory, path)?;
        require_supported(&file)?;
        let collection = collection_of(&factory, &file)?;
        let family_name = first_family_name(&collection)?;
        Ok((collection, FontFaceInfo { family_name }))
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
        // SAFETY: DWrite objects on the calling thread; the builder copies
        // what it needs out of the file reference it is given.
        unsafe {
            let builder: IDWriteFontSetBuilder1 = factory
                .CreateFontSetBuilder()
                .and_then(|builder| builder.cast())
                .map_err(refused)?;
            builder.AddFontFile(file).map_err(refused)?;
            let set = builder.CreateFontSet().map_err(refused)?;
            factory
                .CreateFontCollectionFromFontSet(&set)
                .map_err(refused)
        }
    }

    /// The name of the collection's first family — the one a text format will
    /// be asked for.
    fn first_family_name(collection: &IDWriteFontCollection1) -> Result<String, FontFileError> {
        // SAFETY: the collection is live; index 0 is inside a count checked
        // right above it.
        let names = unsafe {
            if collection.GetFontFamilyCount() == 0 {
                return Err(FontFileError::NoFamilyName);
            }
            collection
                .GetFontFamily(0)
                .map_err(refused)?
                .GetFamilyNames()
                .map_err(refused)?
        };
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
