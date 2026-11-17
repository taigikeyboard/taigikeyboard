//! Whole-file replace, the one write discipline for files a desktop keeps
//! or hands the user: `settings.json` and the custom-dictionary export.
//! `Data.write(to:options:.atomic)` on macOS: the file appears whole or not
//! at all, and a failed write never damages the one it would have replaced.

use std::io::{self, Write};
use std::path::Path;
use std::thread;
use std::time::Duration;

/// A rename onto a file another process is reading fails with a sharing
/// violation on Windows; a few short retries outlast any read.
const REPLACE_ATTEMPTS: u32 = 5;
const REPLACE_RETRY_DELAY: Duration = Duration::from_millis(20);

/// Writes `contents` to a temporary file beside `path`, then renames it over
/// `path`. The temporary file is created exclusively in the target's
/// directory (no name to collide with — two saves at once included — no
/// symlink to follow) and removed if the write or the rename fails.
pub fn replace_file(path: &Path, contents: &[u8]) -> io::Result<()> {
    let directory = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no parent directory"))?;
    let mut temporary = tempfile::Builder::new()
        .prefix(".taigi-")
        .suffix(".tmp")
        .tempfile_in(directory)?;
    temporary.write_all(contents)?;
    // Closed before the rename, and renamed with std's `rename` rather than
    // `persist`: on Windows std replaces with POSIX semantics, which succeed
    // over a reader that opened the target with `FILE_SHARE_DELETE` (every
    // std read does — the TIP's reload included), where `persist`'s
    // `MoveFileExW` fails. Dropped on return, the path removes the
    // temporary file if the rename never moved it.
    let temporary = temporary.into_temp_path();
    for attempt in 1..REPLACE_ATTEMPTS {
        match std::fs::rename(&temporary, path) {
            Ok(()) => return Ok(()),
            Err(error) => {
                log::warn!("replace_file.retry attempt={attempt} error={error}");
                thread::sleep(REPLACE_RETRY_DELAY);
            }
        }
    }
    // The last rename's error is the one worth reporting.
    std::fs::rename(&temporary, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn replaces_the_file_and_leaves_no_temporary_behind() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("export.csv");
        fs::write(&path, b"old").unwrap();
        replace_file(&path, b"new").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new");
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[test]
    fn a_failed_rename_keeps_the_old_file_and_removes_the_temporary() {
        let directory = tempfile::tempdir().unwrap();
        // A directory in the target's place: the rename fails on every
        // platform, after the temporary file was written.
        let path = directory.path().join("export.csv");
        fs::create_dir(&path).unwrap();
        fs::write(path.join("kept"), b"old").unwrap();
        assert!(replace_file(&path, b"new").is_err());
        assert_eq!(fs::read(path.join("kept")).unwrap(), b"old");
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[test]
    fn a_missing_directory_is_an_error_and_writes_nothing() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("missing").join("export.csv");
        assert!(replace_file(&path, b"new").is_err());
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    }
}
