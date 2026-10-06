//! The user-data stores, opened once per process on the platform's word
//! (`UserDataRequest.open`) and reset on its word (`.reset`). What they hold
//! is the engine's (`docs/contributing/rust-migration-policy.md` §6); where the
//! files live is the platform's. Plan: `docs/architecture/user-data-engine-roadmap.md`.

use std::path::PathBuf;
use std::sync::{Arc, Once, OnceLock};

use protos::engine::{
    user_data_request, user_data_response, OpenUserData, UsageRecorded, UserDataJournal,
    UserDataOpened, UserDataRequest, UserDataResponse,
};

use crate::{JournalMode, RequestError, UserDataPaths, UserDataStores};

/// Where and how the stores were opened, and the stores.
struct Opened {
    paths: UserDataPaths,
    journal: JournalMode,
    stores: Arc<UserDataStores>,
    /// The open finishing — files open, custom dictionary taken over — run
    /// once however many callers wait on it, on whichever
    /// thread gets there first.
    initialized: Arc<Once>,
}

/// The stores this process opened. Set once, then read without a lock: from
/// roadmap P3b every keystroke reads through [`UserDataHandle::stores`], and
/// must never wait behind an open (its takeover can take seconds).
pub struct UserDataHandle {
    opened: OnceLock<Opened>,
}

impl UserDataHandle {
    pub(crate) const fn new() -> Self {
        Self {
            opened: OnceLock::new(),
        }
    }

    /// The process's one handle.
    pub fn instance() -> &'static Self {
        static HANDLE: UserDataHandle = UserDataHandle::new();
        &HANDLE
    }

    /// Answers one user-data request: `Open`, `RecordUsage`, or a page request.
    pub fn handle(&self, request: &UserDataRequest) -> Result<UserDataResponse, RequestError> {
        let method = request
            .method
            .as_ref()
            .ok_or(RequestError::Invalid("user-data request has no method"))?;
        let result = match method {
            user_data_request::Method::Open(open) => {
                user_data_response::Result::Opened(self.open(open)?)
            }
            user_data_request::Method::RecordUsage(usage) => {
                self.record_usage(usage)?;
                user_data_response::Result::UsageRecorded(UsageRecorded {})
            }
            page => Self::handle_page(self.settled_stores()?, page)?,
        };
        Ok(UserDataResponse {
            result: Some(result),
        })
    }

    /// The open stores, for the engine's own reads and writes; `None` until
    /// the platform opens them. Never blocks.
    pub fn stores(&self) -> Option<&UserDataStores> {
        self.opened.get().map(|opened| &*opened.stores)
    }

    fn open(&self, open: &OpenUserData) -> Result<UserDataOpened, RequestError> {
        let paths = requested_paths(open)?;
        let journal = journal(open.journal());
        // The stores are published before they finish opening: from here on
        // a write queues behind the open on its store's worker and a read
        // answers neutral until the file is ready, so once this call has
        // been handled nothing a caller reports is lost.
        let opened = self.opened.get_or_init(|| {
            let stores = UserDataStores::at(paths.clone(), journal);
            stores.open();
            Opened {
                paths: paths.clone(),
                journal,
                stores: Arc::new(stores),
                initialized: Arc::new(Once::new()),
            }
        });
        if opened.paths != paths || opened.journal != journal {
            return Err(RequestError::Invalid(
                "user data is already open at other paths or with another journal",
            ));
        }
        if open.in_background {
            let (stores, initialized) =
                (Arc::clone(&opened.stores), Arc::clone(&opened.initialized));
            let spawned = std::thread::Builder::new()
                .name("taigi-user-data-open".into())
                .spawn(move || finish_open(&stores, &initialized));
            if let Err(error) = spawned {
                // No thread to spare: finish here rather than never.
                log::error!("user_data.open_thread_failed error={error}");
                finish_open(&opened.stores, &opened.initialized);
            }
        } else {
            finish_open(&opened.stores, &opened.initialized);
        }
        Ok(UserDataOpened {})
    }

    /// The open stores, or the refusal a request before the open gets. Never
    /// blocks: for `RecordUsage`, sent from the key path, whose write queues
    /// behind the open.
    pub(crate) fn opened_stores(&self) -> Result<&UserDataStores, RequestError> {
        self.stores()
            .ok_or(RequestError::Invalid("user data is not open yet"))
    }

    /// The open stores once they have finished opening — for the pages'
    /// requests (`requests.rs::handle_page`), which run off the key path and must
    /// not read or edit a custom dictionary still being taken over or re-derived
    /// by a background open. Waits for that open, or finishes it here.
    fn settled_stores(&self) -> Result<&UserDataStores, RequestError> {
        let opened = self
            .opened
            .get()
            .ok_or(RequestError::Invalid("user data is not open yet"))?;
        finish_open(&opened.stores, &opened.initialized);
        Ok(&opened.stores)
    }
}

/// Waits for every store to open and finishes the custom dictionary's
/// takeover, once per process.
fn finish_open(stores: &UserDataStores, initialized: &Once) {
    initialized.call_once(|| {
        stores.open_blocking();
        log::info!(
            "user_data.open frequency={} association={} custom_dictionary={} learned_phrases={}",
            stores.frequency.is_ready(),
            stores.association.is_ready(),
            stores.custom_dictionary.is_ready(),
            stores.learned_phrases.is_ready()
        );
    });
}

/// The files `open` names: its directory under the shared names, with a
/// non-empty `association_path` overriding that one file (Android).
fn requested_paths(open: &OpenUserData) -> Result<UserDataPaths, RequestError> {
    let mut paths = UserDataPaths::in_directory(&absolute(&open.directory)?);
    if !open.association_path.is_empty() {
        paths.association = absolute(&open.association_path)?;
    }
    Ok(paths)
}

fn absolute(path: &str) -> Result<PathBuf, RequestError> {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        Ok(path)
    } else {
        Err(RequestError::Invalid(
            "user-data paths must be absolute and non-empty",
        ))
    }
}

fn journal(journal: UserDataJournal) -> JournalMode {
    match journal {
        UserDataJournal::Wal => JournalMode::Wal,
        UserDataJournal::Delete => JournalMode::Delete,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn open_request(directory: &std::path::Path) -> UserDataRequest {
        UserDataRequest {
            method: Some(user_data_request::Method::Open(OpenUserData {
                directory: directory.display().to_string(),
                journal: UserDataJournal::Delete as i32,
                ..OpenUserData::default()
            })),
        }
    }

    pub(crate) fn opened(response: UserDataResponse) -> UserDataOpened {
        match response.result {
            Some(user_data_response::Result::Opened(opened)) => opened,
            other => panic!("expected Opened, got {other:?}"),
        }
    }

    #[test]
    fn open_readies_every_store_and_leaves_the_custom_dictionary_empty() {
        let directory = tempfile::tempdir().unwrap();
        let handle = UserDataHandle::new();

        opened(handle.handle(&open_request(directory.path())).unwrap());

        let stores = handle.stores().unwrap();
        assert!(stores.frequency.is_ready());
        assert!(stores.association.is_ready());
        assert!(stores.custom_dictionary.is_ready());
        assert!(stores.learned_phrases.is_ready());
        assert_eq!(
            stores.custom_dictionary.count().unwrap(),
            0,
            "a fresh install starts with no custom words"
        );
    }

    #[test]
    fn a_repeat_open_at_the_same_paths_answers_and_other_paths_are_refused() {
        let directory = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        let handle = UserDataHandle::new();
        handle.handle(&open_request(directory.path())).unwrap();

        assert!(handle.handle(&open_request(directory.path())).is_ok());
        assert_eq!(
            handle.handle(&open_request(elsewhere.path())).unwrap_err(),
            RequestError::Invalid(
                "user data is already open at other paths or with another journal"
            )
        );
    }

    #[test]
    fn a_repeat_open_with_another_journal_is_refused() {
        let directory = tempfile::tempdir().unwrap();
        let handle = UserDataHandle::new();
        handle.handle(&open_request(directory.path())).unwrap();
        let mut wal = open_request(directory.path());
        if let Some(user_data_request::Method::Open(open)) = wal.method.as_mut() {
            open.journal = UserDataJournal::Wal as i32;
        }

        assert!(matches!(handle.handle(&wal), Err(RequestError::Invalid(_))));
    }

    #[test]
    fn a_relative_or_empty_directory_is_refused_before_anything_opens() {
        for directory in ["", "relative/dir"] {
            let handle = UserDataHandle::new();
            let request = open_request(std::path::Path::new(directory));

            assert!(matches!(
                handle.handle(&request),
                Err(RequestError::Invalid(_))
            ));
            assert!(handle.stores().is_none());
        }
    }

    #[test]
    fn a_relative_association_override_is_refused_before_anything_opens() {
        let handle = UserDataHandle::new();
        let mut request = open_request(std::path::Path::new("/tmp"));
        if let Some(user_data_request::Method::Open(open)) = request.method.as_mut() {
            open.association_path = "user_association.db".into();
        }

        assert!(matches!(
            handle.handle(&request),
            Err(RequestError::Invalid(_))
        ));
        assert!(handle.stores().is_none());
    }

    #[test]
    fn an_association_override_replaces_that_one_file() {
        // trace: Android keeps `user_association.db` in `filesDir`, the other
        // three stores in `directory`.
        let directory = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        let association = elsewhere.path().join("user_association.db");
        let handle = UserDataHandle::new();
        let mut request = open_request(directory.path());
        if let Some(user_data_request::Method::Open(open)) = request.method.as_mut() {
            open.association_path = association.display().to_string();
        }

        opened(handle.handle(&request).unwrap());

        let stores = handle.stores().unwrap();
        assert!(stores.association.is_ready());
        assert!(association.exists(), "the override names the file");
        assert!(
            !directory.path().join("user_association.db").exists(),
            "and the directory's own slot stays empty"
        );
    }
}
