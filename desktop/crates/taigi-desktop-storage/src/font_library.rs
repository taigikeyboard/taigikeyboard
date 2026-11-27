//! The typefaces the user added themselves: where they are kept, what may join
//! them, and the copy that puts one there. Shared by macOS (over
//! `taigi-macos-ffi` / `DesktopCoreBridge`), Windows, and Linux.
//!
//! The FILE half only. Whether a copied file is really a typeface — and what
//! its family or PostScript name is — belongs to each platform's font stack
//! (Core Text on macOS, DirectWrite on Windows); this module is pure so it
//! holds one authority for storage rules across all desktop platforms.
//!
//! No index file beside the copies: the directory IS the list, and the face
//! names are read back out of the files. One authority, nothing to reconcile.

use crate::directory::{created, user_data_directory, DirectoryError};
use std::io::Write;
use std::path::{Component, Path, PathBuf};

/// The folder under `%APPDATA%\TaigiKeyboard` (Windows) or the per-bundle
/// Application Support directory (macOS) the user's typefaces are copied into.
/// Kept in its own folder rather than loose beside the databases, and NOT the
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

/// The longest a stored file's name may be, before its extension, counted in
/// Unicode scalars — matching Rust `char` and Swift `Unicode.Scalar` so one
/// rule gives one answer on every desktop platform.
const MAX_STORED_STEM_LENGTH: usize = 64;

/// How many attempts one stem may try (`stem.ext`, then `stem-2.ext` …
/// `stem-100.ext`) before the import gives up. A library with a hundred copies
/// of one name is a mistake, not a use.
pub const MAX_NAME_ATTEMPTS: usize = 100;

/// Whether `character` would make the name more than one path component, or is
/// one Windows refuses in a file name at all.
fn is_structural(character: char) -> bool {
    matches!(
        character,
        '/' | '\\' | '<' | '>' | ':' | '"' | '|' | '?' | '*'
    )
}

/// Whether `character` draws nothing, or draws the rest of the name somewhere
/// other than where it is written.
///
/// ZWNJ and ZWJ are deliberately absent: they join letters and emoji, and
/// dropping them rewrites text the user meant.
fn is_invisible(character: char) -> bool {
    matches!(character,
        '\u{0}'..='\u{1F}' | '\u{7F}'..='\u{9F}'      // C0, DEL and C1
        | '\u{61C}' | '\u{200B}' | '\u{200E}' | '\u{200F}' // Arabic letter mark, ZWSP, LRM, RLM
        | '\u{2028}' | '\u{2029}'                      // line and paragraph separators
        | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' // the bidi overrides and isolates
        | '\u{FEFF}')
}

/// Whether `character` puts a mark on screen. The complement of Unicode's
/// `Default_Ignorable_Code_Point` and `White_Space` (excluding controls and
/// newlines already dropped by `is_invisible`), matching Foundation's
/// `!scalar.properties.isDefaultIgnorableCodePoint && !CharacterSet.whitespaces.contains(scalar)`
/// without pulling in a Unicode property crate for one predicate. Only the
/// ignorables `is_invisible` does NOT already drop can reach this.
fn is_visible(character: char) -> bool {
    !character.is_whitespace()
        && !matches!(character,
            '\u{AD}' | '\u{34F}' | '\u{115F}'..='\u{1160}' | '\u{17B4}'..='\u{17B5}'
            | '\u{180B}'..='\u{180F}' | '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{206F}' | '\u{3164}' | '\u{FE00}'..='\u{FE0F}' | '\u{FEFF}'
            | '\u{FFA0}' | '\u{FFF0}'..='\u{FFF8}' | '\u{1BCA0}'..='\u{1BCA3}'
            | '\u{1D173}'..='\u{1D17A}' | '\u{E0000}'..='\u{E0FFF}')
}

/// The names Windows reserves for a device, matched on the part before the
/// first dot with trailing spaces ignored and case folded — which is how
/// Windows itself resolves one, so `CON.foo` is `CON` to it.
fn is_windows_device_name(stem: &str) -> bool {
    const DEVICES: [&str; 4] = ["CON", "PRN", "AUX", "NUL"];
    let head = stem
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end_matches(' ');
    let folded = head.to_uppercase();
    if DEVICES.contains(&folded.as_str()) {
        return true;
    }
    // `COM` and `LPT` take the superscript digits too, which Windows folds to
    // their ASCII forms.
    let Some(port) = folded
        .strip_prefix("COM")
        .or_else(|| folded.strip_prefix("LPT"))
    else {
        return false;
    };
    matches!(
        port,
        "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
    )
}

/// Trimmed from both ends, before and after the length cap. Windows drops a
/// trailing dot or space of its own accord, and a name that starts with one
/// reads as an accident rather than as a name.
const TRIMMED_FROM_THE_ENDS: [char; 3] = ['-', '.', ' '];

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("{0} is not a font file")]
    UnsupportedFileType(String),
    #[error("not a regular file")]
    NotARegularFile,
    #[error("the file is {} MB, over the {} MB limit", .0 / (1024 * 1024), MAX_FILE_SIZE / (1024 * 1024))]
    TooLarge(u64),
    #[error("could not read the file: {0}")]
    Unreadable(#[source] std::io::Error),
    #[error("could not copy the file into the library: {0}")]
    NotCopied(#[source] std::io::Error),
    #[error("every name this typeface could be stored under is taken")]
    NoAvailableName,
    #[error(transparent)]
    Directory(#[from] DirectoryError),
}

/// `%APPDATA%\TaigiKeyboard\Fonts`, created if absent.
pub fn fonts_directory() -> Result<PathBuf, DirectoryError> {
    created(user_data_directory()?.join(FONTS_FOLDER_NAME))
}

/// The stored file names, sorted, skipping anything that is not a valid font
/// file component.
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
        .filter(|name| is_single_path_component(name) && has_allowed_extension(Path::new(name)))
        .collect();
    names.sort();
    names
}

/// Copies `source` into `directory` and answers the name it was stored under.
///
/// Validates what a file system can answer — the extension, that it is a
/// regular file, and its size — then copies. Whether the bytes are really a
/// typeface is settled afterwards, against the COPY, by the caller's font
/// stack (Core Text or DirectWrite); a copy that fails that check is removed
/// again (`remove_stored`).
///
/// The destination is CREATED, exclusively, before a byte is written: an
/// existence check followed by `std::fs::copy` would let two imports pick the
/// same name and overwrite one another, and would also inherit the source
/// file's permissions — a read-only original then makes a library file the
/// user cannot delete. Creating it here gives the copy this process's own
/// default permissions.
///
/// The open descriptor's metadata is re-checked (`fstat`, and `(dev, ino)` on
/// Unix) and the copy is bounded, rather than trusting the initial `lstat`
/// alone: a source replaced by a symlink or grown between `lstat` and `read`
/// cannot bypass the regular-file check or the size ceiling.
pub fn copy_in(directory: &Path, source: &Path) -> Result<String, ImportError> {
    let extension = extension_of(source);
    if !ALLOWED_EXTENSIONS.contains(&extension.as_str()) {
        return Err(ImportError::UnsupportedFileType(extension));
    }
    let lstat = std::fs::symlink_metadata(source).map_err(ImportError::Unreadable)?;
    if !lstat.is_file() {
        return Err(ImportError::NotARegularFile);
    }
    if lstat.len() > MAX_FILE_SIZE {
        return Err(ImportError::TooLarge(lstat.len()));
    }
    let mut reader = std::fs::File::open(source).map_err(ImportError::Unreadable)?;
    let fstat = reader.metadata().map_err(ImportError::Unreadable)?;
    if !fstat.is_file() {
        return Err(ImportError::NotARegularFile);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if (lstat.dev(), lstat.ino()) != (fstat.dev(), fstat.ino()) {
            return Err(ImportError::NotARegularFile);
        }
    }
    if fstat.len() > MAX_FILE_SIZE {
        return Err(ImportError::TooLarge(fstat.len()));
    }
    let (stored, mut writer) = create_unused_file(directory, source, &extension)?;
    let copy_result = copy_bounded(&mut reader, &mut writer, MAX_FILE_SIZE)
        .and_then(|_| writer.flush().map_err(ImportError::NotCopied));
    // Drop the handle before any rollback unlink so Windows never hits a
    // sharing violation while removing a half-written copy.
    drop(writer);
    if let Err(error) = copy_result {
        return Err(discarding(directory, &stored, error));
    }
    Ok(stored)
}

/// Streams `reader` into `writer`, refusing any growth past `max_bytes` before
/// writing the excess bytes.
pub(crate) fn copy_bounded<R: std::io::Read, W: std::io::Write>(
    reader: &mut R,
    writer: &mut W,
    max_bytes: u64,
) -> Result<u64, ImportError> {
    const CHUNK_SIZE: usize = 64 * 1024;
    let mut buffer = vec![0_u8; CHUNK_SIZE];
    let mut total_written: u64 = 0;
    loop {
        let remaining = max_bytes.saturating_sub(total_written);
        let request_size = usize::try_from(remaining.saturating_add(1))
            .unwrap_or(CHUNK_SIZE)
            .min(CHUNK_SIZE);
        let bytes_read = loop {
            match reader.read(&mut buffer[..request_size]) {
                Ok(count) => break count,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(ImportError::Unreadable(error)),
            }
        };
        if bytes_read == 0 {
            return Ok(total_written);
        }
        let next_total = total_written.saturating_add(bytes_read as u64);
        if next_total > max_bytes {
            return Err(ImportError::TooLarge(next_total));
        }
        let mut offset = 0;
        while offset < bytes_read {
            match writer.write(&buffer[offset..bytes_read]) {
                Ok(0) => {
                    return Err(ImportError::NotCopied(std::io::Error::new(
                        std::io::ErrorKind::WriteZero,
                        "failed to write whole buffer",
                    )));
                }
                Ok(written) => offset += written,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(ImportError::NotCopied(error)),
            }
        }
        total_written = next_total;
    }
}

/// Takes a half-written copy back out on the way to reporting `error`. Best
/// effort by construction: this runs because something already failed.
fn discarding(directory: &Path, stored: &str, error: ImportError) -> ImportError {
    let _ = remove_stored(directory, stored);
    error
}

/// Deletes one stored typeface. The caller unregisters it first — a file
/// deleted under a live registration leaves the process drawing from nothing.
pub fn remove_stored(directory: &Path, file_name: &str) -> std::io::Result<()> {
    let Some(path) = stored_path(directory, file_name) else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "invalid typeface file name",
        ));
    };
    std::fs::remove_file(path)
}

/// Creates the file `source` will be stored as: a sanitized name, suffixed
/// until one is free, and CLAIMED by the creation itself.
///
/// `create_new` is the reservation: it fails when the name is taken, so two
/// imports racing for the same name each end up with their own — an import
/// never overwrites, and one stored name is one set of bytes for as long as
/// the file exists. That is what lets a selection, a text format and a cached
/// candidate window agree on WHICH typeface is meant.
fn create_unused_file(
    directory: &Path,
    source: &Path,
    extension: &str,
) -> Result<(String, std::fs::File), ImportError> {
    let stem = sanitized_stem(
        &source
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default(),
    );
    for attempt in 1..=MAX_NAME_ATTEMPTS {
        let candidate = if attempt == 1 {
            format!("{stem}.{extension}")
        } else {
            format!("{stem}-{attempt}.{extension}")
        };
        match std::fs::File::create_new(directory.join(&candidate)) {
            Ok(file) => return Ok((candidate, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(ImportError::NotCopied(error)),
        }
    }
    Err(ImportError::NoAvailableName)
}

/// `name` reduced to what one path component may hold, keeping as much of the
/// picked file's own name as is safe to keep, or `typeface` when nothing
/// showable survives.
///
/// The stored name is also the name the pane LISTS (`font_management`,
/// `FontManagementPage`), so this keeps the user's spelling: their capitals,
/// their spaces, their script. Everything it removes it removes for a reason:
///
/// - Structural characters become `-` (path separators and Windows-reserved
///   characters).
/// - Invisible characters (C0/DEL/C1, bidi overrides/isolates, line/paragraph
///   separators, ZWSP, BOM) are dropped outright, while ZWNJ and ZWJ stay so
///   joining scripts and emoji are not rewritten.
/// - Edges are trimmed of `-`, `.`, and spaces before and after the 64-scalar
///   cap so truncation cannot leave a leading/trailing separator behind.
/// - Windows reserved device names (`CON`, `PRN`, `AUX`, `NUL`, `COM1`..`9`,
///   `LPT1`..`9`, superscript `¹²³` ports, matched before the first dot,
///   case-insensitive with trailing spaces ignored) and names without any
///   visible scalar collapse to `"typeface"`.
pub fn sanitized_stem(name: &str) -> String {
    let reduced: String = name
        .chars()
        .filter(|character| !is_invisible(*character))
        .map(|character| {
            if is_structural(character) {
                '-'
            } else {
                character
            }
        })
        .collect();
    let capped: String = reduced
        .trim_matches(|character| TRIMMED_FROM_THE_ENDS.contains(&character))
        .chars()
        .take(MAX_STORED_STEM_LENGTH)
        .collect();
    let trimmed = capped.trim_matches(|character| TRIMMED_FROM_THE_ENDS.contains(&character));
    if !trimmed.chars().any(is_visible) || is_windows_device_name(trimmed) {
        "typeface".to_owned()
    } else {
        trimmed.to_owned()
    }
}

/// Whether `file_name` names a single path component inside the library rather
/// than a relative traversal (`..`), empty name, NUL byte, or separator path.
pub fn is_single_path_component(file_name: &str) -> bool {
    if file_name.is_empty()
        || file_name == "."
        || file_name == ".."
        || file_name.contains(['/', '\\', '\0'])
    {
        return false;
    }
    let mut components = Path::new(file_name).components();
    matches!(
        (components.next(), components.next()),
        (Some(Component::Normal(_)), None)
    )
}

/// Validates that `file_name` (which may come from untrusted settings storage)
/// is a single path component inside the library rather than a traversal path.
pub fn sanitized_component(file_name: &str) -> Option<&str> {
    is_single_path_component(file_name).then_some(file_name)
}

/// Resolves `file_name` inside `directory`, returning `None` when `file_name`
/// could escape or fail to name a single entry in the library directory.
pub fn stored_path(directory: &Path, file_name: &str) -> Option<PathBuf> {
    sanitized_component(file_name).map(|name| directory.join(name))
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
    use std::sync::atomic::{AtomicU64, Ordering};

    /// A directory of this test's own.
    ///
    /// The counter, not the clock alone: `SystemTime::now` is coarse enough on
    /// Windows that two tests starting together read the same nanoseconds, and
    /// two tests sharing a scratch directory see each other's files — which is
    /// how `a_second_import_of_the_same_name_is_stored_beside_the_first` came
    /// to find a `locked.ttf` the read-only-source test had written.
    fn scratch() -> PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let directory = std::env::temp_dir().join(format!(
            "taigi-fonts-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
        ));
        std::fs::create_dir_all(&directory).unwrap();
        directory
    }

    fn write(directory: &Path, name: &str, bytes: usize) -> PathBuf {
        let path = directory.join(name);
        std::fs::write(&path, vec![0_u8; bytes]).unwrap();
        path
    }

    /// The stored name is ALSO the name the pane shows, so the user's own
    /// spelling survives — their capitals, their spaces, their script.
    #[test]
    fn a_stored_name_keeps_the_picked_name_as_the_user_spelled_it() {
        assert_eq!(sanitized_stem("My Font"), "My Font");
        assert_eq!(sanitized_stem("源樣明體"), "源樣明體");
        assert_eq!(sanitized_stem("SnailFont-Pomacea"), "SnailFont-Pomacea");
        assert_eq!(sanitized_stem("jf-openhuninn-2.1"), "jf-openhuninn-2.1");
    }

    /// What it does not keep: anything that would make the name more than one
    /// path component, anything Windows refuses, and anything that draws
    /// nothing.
    #[test]
    fn a_stored_name_is_one_path_component_and_nothing_else() {
        assert_eq!(sanitized_stem("../../windows/system32"), "windows-system32");
        assert_eq!(sanitized_stem("a:b|c?d*e\"f<g>h"), "a-b-c-d-e-f-g-h");
        assert_eq!(sanitized_stem(""), "typeface");
        assert_eq!(sanitized_stem("..."), "typeface");
        assert_eq!(sanitized_stem("  . - "), "typeface");
    }

    /// Windows resolves a device name on the part before the first dot,
    /// ignoring case and trailing spaces.
    #[test]
    fn a_windows_device_name_is_refused() {
        assert_eq!(sanitized_stem("CON"), "typeface");
        assert_eq!(sanitized_stem("nul"), "typeface");
        assert_eq!(sanitized_stem("CON.foo"), "typeface");
        assert_eq!(sanitized_stem("Com1"), "typeface");
        assert_eq!(sanitized_stem("COM¹"), "typeface");
        assert_eq!(sanitized_stem("CONSOLE"), "CONSOLE");
    }

    /// A name that draws nothing, or draws the rest of itself somewhere else.
    /// ZWNJ and ZWJ stay: they join letters and emoji.
    #[test]
    fn the_invisible_characters_go_and_the_joiners_stay() {
        assert_eq!(sanitized_stem("Fo\u{202E}nt"), "Font");
        assert_eq!(sanitized_stem("Fo\u{200B}nt"), "Font");
        assert_eq!(sanitized_stem("Fo\u{FEFF}nt"), "Font");
        assert_eq!(sanitized_stem("Fo\u{9}nt"), "Font");
        assert_eq!(sanitized_stem("\u{200D}"), "typeface");
        assert_eq!(sanitized_stem("क\u{200D}ष"), "क\u{200D}ष");
    }

    /// The cap counts Unicode scalars, the unit the macOS port counts, and the
    /// edges are trimmed again afterwards so a cut cannot leave a dot or a
    /// space behind.
    #[test]
    fn a_stored_name_is_capped_in_scalars_and_retrimmed_after_the_cut() {
        assert_eq!(sanitized_stem(&"a".repeat(200)).chars().count(), 64);
        assert_eq!(sanitized_stem(&"字".repeat(200)).chars().count(), 64);
        assert_eq!(
            sanitized_stem(&format!("{} tail", "a".repeat(63))),
            "a".repeat(63),
        );
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

        let error = copy_in(&library, &source).unwrap_err();
        assert!(matches!(error, ImportError::TooLarge(_)));
        assert_eq!(error.to_string(), "the file is 64 MB, over the 64 MB limit");
        assert_eq!(
            ImportError::TooLarge(65 * 1024 * 1024).to_string(),
            "the file is 65 MB, over the 64 MB limit"
        );
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

        assert_eq!(copy_in(&library, &source).unwrap(), "Iansui Regular.ttf");
        assert_eq!(copy_in(&library, &source).unwrap(), "Iansui Regular-2.ttf");
        assert_eq!(
            stored_file_names(&library),
            ["Iansui Regular-2.ttf", "Iansui Regular.ttf"],
        );
    }

    /// The copy is this library's file, whatever the original was. A
    /// read-only source used to hand its permissions to the copy, which then
    /// could not be removed — neither by a failed import's rollback nor by the
    /// user.
    #[cfg(unix)]
    #[test]
    fn a_read_only_source_still_produces_a_removable_copy() {
        use std::os::unix::fs::PermissionsExt;
        let library = scratch();
        let elsewhere = scratch();
        let source = write(&elsewhere, "locked.ttf", 16);
        std::fs::set_permissions(&source, std::fs::Permissions::from_mode(0o444)).unwrap();

        let stored = copy_in(&library, &source).unwrap();

        assert!(!std::fs::metadata(library.join(&stored))
            .unwrap()
            .permissions()
            .readonly());
        remove_stored(&library, &stored).expect("the library's own copy is removable");
    }

    /// The destination is claimed by creating it, so a name already taken
    /// cannot be written over — the property every id, cached text format and
    /// stored selection leans on.
    #[test]
    fn an_import_never_writes_over_a_name_already_taken() {
        let library = scratch();
        let elsewhere = scratch();
        write(&library, "mine.ttf", 1);
        let source = write(&elsewhere, "mine.ttf", 32);

        let stored = copy_in(&library, &source).unwrap();

        assert_eq!(stored, "mine-2.ttf");
        assert_eq!(
            std::fs::metadata(library.join("mine.ttf")).unwrap().len(),
            1
        );
        assert_eq!(std::fs::metadata(library.join(&stored)).unwrap().len(), 32);
    }

    /// Explicit parity coverage for Unicode `Default_Ignorable_Code_Point` and
    /// non-ASCII `White_Space` boundaries with macOS Foundation's
    /// `isDefaultIgnorableCodePoint` / `CharacterSet.whitespaces`.
    #[test]
    fn default_ignorable_and_unicode_whitespace_parity() {
        for ignorable_or_space in [
            "\u{00AD}",
            "\u{034F}",
            "\u{115F}",
            "\u{1160}",
            "\u{17B4}",
            "\u{180E}",
            "\u{200C}",
            "\u{200D}",
            "\u{2060}",
            "\u{3164}",
            "\u{FE0F}",
            "\u{FFA0}",
            "\u{E0100}",
            "\u{00A0}",
            "\u{3000}",
            "\u{00A0}\u{200D}\u{FE0F}\u{3000}\u{00AD}",
        ] {
            assert_eq!(
                sanitized_stem(ignorable_or_space),
                "typeface",
                "expected {ignorable_or_space:?} to collapse"
            );
        }
        assert_eq!(sanitized_stem("क\u{200D}ष"), "क\u{200D}ष");
        assert_eq!(sanitized_stem("a\u{FE0F}"), "a\u{FE0F}");
        assert_eq!(sanitized_stem("a\u{200C}b"), "a\u{200C}b");
    }

    #[test]
    fn copy_bounded_allows_up_to_cap_and_refuses_overflow_before_writing_past_it() {
        let mut out = Vec::new();
        let exact = vec![0x41_u8; 16];
        let written = copy_bounded(&mut exact.as_slice(), &mut out, 16).unwrap();
        assert_eq!(written, 16);
        assert_eq!(out, exact);

        let mut overflow_out = Vec::new();
        let one_over = vec![0x42_u8; 17];
        let err = copy_bounded(&mut one_over.as_slice(), &mut overflow_out, 16).unwrap_err();
        assert!(matches!(err, ImportError::TooLarge(17)));
        assert!(overflow_out.is_empty());

        let multi_chunk_limit: u64 = 64 * 1024;
        let multi_chunk_over = vec![0x43_u8; (multi_chunk_limit as usize) + 1];
        let mut multi_out = Vec::new();
        let err = copy_bounded(
            &mut multi_chunk_over.as_slice(),
            &mut multi_out,
            multi_chunk_limit,
        )
        .unwrap_err();
        assert!(matches!(err, ImportError::TooLarge(65537)));
        assert!(multi_out.len() <= multi_chunk_limit as usize);
    }

    #[test]
    fn copy_in_refuses_missing_source_and_caps_at_100_attempts() {
        let library = scratch();
        let elsewhere = scratch();
        let missing = elsewhere.join("missing.ttf");
        assert!(matches!(
            copy_in(&library, &missing).unwrap_err(),
            ImportError::Unreadable(_)
        ));

        let source = write(&elsewhere, "taken.ttf", 8);
        for attempt in 1..MAX_NAME_ATTEMPTS {
            let name = if attempt == 1 {
                "taken.ttf".to_owned()
            } else {
                format!("taken-{attempt}.ttf")
            };
            write(&library, &name, 1);
        }
        assert_eq!(copy_in(&library, &source).unwrap(), "taken-100.ttf");
        assert!(matches!(
            copy_in(&library, &source).unwrap_err(),
            ImportError::NoAvailableName
        ));
    }

    #[cfg(unix)]
    #[test]
    fn copy_in_refuses_symlink_source() {
        let library = scratch();
        let elsewhere = scratch();
        let target = write(&elsewhere, "real.ttf", 16);
        let link = elsewhere.join("linked.ttf");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        assert!(matches!(
            copy_in(&library, &link).unwrap_err(),
            ImportError::NotARegularFile
        ));
        assert_eq!(stored_file_names(&library), Vec::<String>::new());
    }

    #[test]
    fn the_roster_holds_font_files_only() {
        let library = scratch();
        write(&library, "one.ttf", 4);
        write(&library, "notes.csv", 4);
        std::fs::create_dir(library.join("two.otf")).unwrap();
        #[cfg(unix)]
        write(&library, "bad\\name.ttf", 4);

        assert_eq!(stored_file_names(&library), ["one.ttf"]);
    }

    #[test]
    fn a_stored_name_from_the_settings_file_cannot_reach_another_directory() {
        // `settings.json` / `UserDefaults` are writable outside the app, and
        // `customFontFile` is read out of them — so the name is untrusted input
        // to a path.
        let parent = scratch();
        let library = parent.join("Fonts");
        std::fs::create_dir(&library).unwrap();
        let outside_victim = write(&parent, "keep.ttf", 4);
        let inside_victim = write(&library, "keep.ttf", 4);

        for unsafe_name in [
            "../keep.ttf",
            "./keep.ttf",
            "sub/keep.ttf",
            "..\\keep.ttf",
            "",
            ".",
            "..",
            "a\0b.ttf",
        ] {
            assert!(!is_single_path_component(unsafe_name), "{unsafe_name:?}");
            assert_eq!(sanitized_component(unsafe_name), None, "{unsafe_name:?}");
            assert_eq!(stored_path(&library, unsafe_name), None, "{unsafe_name:?}");
            let err = remove_stored(&library, unsafe_name).unwrap_err();
            assert_eq!(
                err.kind(),
                std::io::ErrorKind::InvalidInput,
                "{unsafe_name:?}"
            );
        }

        assert!(
            outside_victim.exists(),
            "a name with .. deleted a file outside the library"
        );
        assert!(
            inside_victim.exists(),
            "a stripped traversal name deleted a file inside the library"
        );
    }
}
