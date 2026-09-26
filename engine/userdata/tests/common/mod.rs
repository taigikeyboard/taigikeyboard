//! Helpers both integration suites use.

use userdata::{AssociationPair, UserDataPaths};

pub fn scratch() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

pub fn paths(directory: &tempfile::TempDir) -> UserDataPaths {
    UserDataPaths::in_directory(directory.path())
}

pub fn pair(previous: &str, previous_tl: &str, next: &str, next_tl: &str) -> AssociationPair {
    AssociationPair {
        previous: previous.into(),
        previous_tl: previous_tl.into(),
        next: next.into(),
        next_tl: next_tl.into(),
    }
}
