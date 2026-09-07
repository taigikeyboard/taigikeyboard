//! The typefaces the user added themselves: where they are kept, what may join
//! them, and the copy that puts one there. Port of the file half of
//! `macos/Sources/TaigiInputMethodCore/Storage/CustomFontLibrary.swift`.
//!
//! The FILE half only. Whether a copied file is really a typeface — and what
//! its family is called — is DirectWrite's answer, and lives beside the code
//! that draws with it; this module is pure so it can be tested on the machine
//! the crate is developed on rather than only on the Windows box.
//!
//! No index file beside the copies: the directory IS the list, and the family
//! names are read back out of the files. One authority, nothing to reconcile.

use crate::directory::{created, user_data_directory, DirectoryError};
use std::path::{Path, PathBuf};

/// The folder under `%APPDATA%\TaigiKeyboard` the user's typefaces are copied
/// into. Their own folder rather than loose beside the databases, and NOT the
/// install directory's `Fonts\` — that one ships with the app and is replaced
/// wholesale by every upgrade (`TaigiKeyboard.iss:150,193`).
pub const FONTS_FOLDER_NAME: &str = "Fonts";

/// What a font file may be named. Collections are taken in as well as single
/// faces; a `.ttc` contributes its first face to the picker.
pub const ALLOWED_EXTENSIONS: [&str; 3] = ["ttf", "otf", "ttc"];

/// The most one file may weigh. The largest bundled face is already over
/// 20 MB, so this is generous rather than tight — a screening limit against a
/// mistaken pick, not a security boundary.
pub const MAX_FILE_SIZE: u64 = 64 * 1024 * 1024;

/// The longest a stored file's name may be, before its extension.
const MAX_STORED_STEM_LENGTH: usize = 64;

/// What a stored name is built from. Everything else — separators, dots, the
/// picked name's own script — becomes `-`, so the result can neither escape
/// the directory nor hide an extension.
fn is_allowed_in_stored_name(character: char) -> bool {
    character.is_ascii_lowercase()
        || character.is_ascii_digit()
        || character == '-'
        || character == '_'
}

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("{0} is not a font file")]
    UnsupportedFileType(String),
    #[error("not a regular file")]
    NotARegularFile,
    #[error("the file is {}, over the {} limit", .0 / (1024 * 1024), MAX_FILE_SIZE / (1024 * 1024))]
    TooLarge(u64),
    #[error("could not read the file: {0}")]
    Unreadable(#[source] std::io::Error),
    #[error("could not copy the file into the library: {0}")]
    NotCopied(#[source] std::io::Error),
    #[error(transparent)]
    Directory(#[from] DirectoryError),
}

/// `%APPDATA%\TaigiKeyboard\Fonts`, created if absent.
pub fn fonts_directory() -> Result<PathBuf, DirectoryError> {
    created(user_data_directory()?.join(FONTS_FOLDER_NAME))
}

/// The stored file names, sorted, skipping anything that is not a font file.
///
/// A directory that cannot be read answers with nothing rather than an error:
/// one unreadable folder must not take the bundled roster off the picker too.
pub fn stored_file_names(directory: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| has_allowed_extension(Path::new(name)))
        .collect();
    names.sort();
    names
}

/// Copies `source` into `directory` and answers the name it was stored under.
///
/// Validates what a file system can answer — the extension, that it is a
/// regular file, and its size — then copies. Whether the bytes are really a
/// typeface is settled afterwards, against the COPY, by the caller that can
/// ask DirectWrite; a copy that fails that check is removed again
/// (`remove_stored`).
pub fn copy_in(directory: &Path, source: &Path) -> Result<String, ImportError> {
    let extension = extension_of(source);
    if !ALLOWED_EXTENSIONS.contains(&extension.as_str()) {
        return Err(ImportError::UnsupportedFileType(extension));
    }
    let metadata = std::fs::symlink_metadata(source).map_err(ImportError::Unreadable)?;
    if !metadata.is_file() {
        return Err(ImportError::NotARegularFile);
    }
    if metadata.len() > MAX_FILE_SIZE {
        return Err(ImportError::TooLarge(metadata.len()));
    }
    let stored = unused_file_name(directory, source, &extension);
    std::fs::copy(source, directory.join(&stored)).map_err(ImportError::NotCopied)?;
    Ok(stored)
}

/// Deletes one stored typeface. The caller unregisters it first — a file
/// deleted under a live registration leaves the process drawing from nothing.
pub fn remove_stored(directory: &Path, file_name: &str) -> std::io::Result<()> {
    std::fs::remove_file(directory.join(sanitized_component(file_name)))
}

/// The name `source` will be stored under: sanitized, and suffixed until it is
/// one no file in `directory` has.
///
/// An import never overwrites, so one stored name is one set of bytes for as
/// long as the file exists — which is what lets a selection, a text format and
/// a cached candidate window all agree on WHICH typeface is meant.
fn unused_file_name(directory: &Path, source: &Path, extension: &str) -> String {
    let stem = sanitized_stem(
        &source
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default(),
    );
    let mut candidate = format!("{stem}.{extension}");
    let mut suffix = 2;
    while directory.join(&candidate).exists() {
        candidate = format!("{stem}-{suffix}.{extension}");
        suffix += 1;
    }
    candidate
}

/// `name` reduced to what a stored file's name may hold, or `typeface` when
/// nothing of it survives. The stored name is built from characters this
/// function chose, never from the picked file's own name verbatim — a name is
/// untrusted text, and this one becomes a path.
pub fn sanitized_stem(name: &str) -> String {
    let reduced: String = name
        .to_lowercase()
        .chars()
        .map(|character| {
            if is_allowed_in_stored_name(character) {
                character
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = reduced.trim_matches('-');
    if trimmed.is_empty() {
        "typeface".to_owned()
    } else {
        trimmed.chars().take(MAX_STORED_STEM_LENGTH).collect()
    }
}

/// A stored name as a single path component, so a value read back out of
/// `settings.json` — which anything can write — cannot reach another directory.
fn sanitized_component(file_name: &str) -> String {
    let path = Path::new(file_name);
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn extension_of(path: &Path) -> String {
    path.extension()
        .map(|extension| extension.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

fn has_allowed_extension(path: &Path) -> bool {
    ALLOWED_EXTENSIONS.contains(&extension_of(path).as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch() -> PathBuf {
        let directory = std::env::temp_dir().join(format!("taigi-fonts-{}", uuid()));
        std::fs::create_dir_all(&directory).unwrap();
        directory
    }

    fn uuid() -> u128 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    }

    fn write(directory: &Path, name: &str, bytes: usize) -> PathBuf {
        let path = directory.join(name);
        std::fs::write(&path, vec![0_u8; bytes]).unwrap();
        path
    }

    #[test]
    fn stored_names_are_built_here_not_taken_from_the_picked_file() {
        assert_eq!(sanitized_stem("My Font"), "my-font");
        assert_eq!(sanitized_stem("../../windows/system32"), "windows-system32");
        assert_eq!(sanitized_stem("源樣明體"), "typeface");
        assert_eq!(sanitized_stem(""), "typeface");
        assert_eq!(sanitized_stem("..."), "typeface");
        assert_eq!(sanitized_stem(&"a".repeat(200)).len(), 64);
    }

    #[test]
    fn a_file_that_is_not_a_font_is_refused_and_copies_nothing() {
        let library = scratch();
        let source = write(&library, "notes.txt", 8);

        let error = copy_in(&library, &source).unwrap_err();

        assert!(matches!(error, ImportError::UnsupportedFileType(extension) if extension == "txt"));
        assert_eq!(stored_file_names(&library), Vec::<String>::new());
    }

    #[test]
    fn a_font_file_over_the_ceiling_is_refused() {
        let library = scratch();
        let source = library.join("huge.ttf");
        let file = std::fs::File::create(&source).unwrap();
        // Sparse: the ceiling is about the length, and writing 64 MB of zeroes
        // to assert on it would be a slow test for the same answer.
        file.set_len(MAX_FILE_SIZE + 1).unwrap();
        drop(file);

        assert!(matches!(
            copy_in(&library, &source).unwrap_err(),
            ImportError::TooLarge(_),
        ));
    }

    #[test]
    fn a_directory_named_like_a_font_is_refused() {
        let library = scratch();
        let source = library.join("pretend.otf");
        std::fs::create_dir(&source).unwrap();

        assert!(matches!(
            copy_in(&library, &source).unwrap_err(),
            ImportError::NotARegularFile,
        ));
    }

    #[test]
    fn a_second_import_of_the_same_name_is_stored_beside_the_first() {
        let library = scratch();
        let elsewhere = scratch();
        let source = write(&elsewhere, "Iansui Regular.ttf", 16);

        assert_eq!(copy_in(&library, &source).unwrap(), "iansui-regular.ttf");
        assert_eq!(copy_in(&library, &source).unwrap(), "iansui-regular-2.ttf");
        assert_eq!(
            stored_file_names(&library),
            ["iansui-regular-2.ttf", "iansui-regular.ttf"],
        );
    }

    #[test]
    fn the_roster_holds_font_files_only() {
        let library = scratch();
        write(&library, "one.ttf", 4);
        write(&library, "notes.csv", 4);
        std::fs::create_dir(library.join("two.otf")).unwrap();

        assert_eq!(stored_file_names(&library), ["one.ttf"]);
    }

    #[test]
    fn a_stored_name_from_the_settings_file_cannot_reach_another_directory() {
        // `settings.json` is a file anything can write, and `customFontFile` is
        // read out of it — so the name it holds is untrusted input to a path.
        let parent = scratch();
        let library = parent.join("Fonts");
        std::fs::create_dir(&library).unwrap();
        let victim = write(&parent, "keep.ttf", 4);

        let _ = remove_stored(&library, "../keep.ttf");

        assert!(
            victim.exists(),
            "a name with .. deleted a file outside the library"
        );
    }
}
