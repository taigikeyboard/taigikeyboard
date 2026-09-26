//! The user-data stores, opened once per process on the platform's word
//! (`UserDataRequest.open`) and reset on its word (`.reset`). What they hold
//! is the engine's (`.claude/rules/rust-migration-policy.md` §6); where the
//! files live is the platform's. Plan: `docs/architecture/user-data-engine-roadmap.md`.

use std::fmt::Display;
use std::path::PathBuf;
use std::sync::OnceLock;

use protos::engine::{
    response, user_data_request, user_data_response, ErrorCode, OpenUserData, ResetUserData,
    Response, UserDataJournal, UserDataOpened, UserDataRequest, UserDataReset, UserDataResponse,
};
use userdata::{JournalMode, UserDataPaths, UserDataStores};

/// Why a user-data request did nothing.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum UserDataError {
    /// The request itself is wrong: no method, a relative or empty path, a
    /// second open at other paths or journal, a reset of nothing, a reset
    /// before open.
    Invalid(&'static str),
    /// A store could not do what was asked — its message, logged.
    Store(String),
}

/// Answers one user-data request — the `user-data` arm of `crate::run`.
pub(crate) fn respond(id: u32, generation: u64, request: &UserDataRequest) -> Response {
    let (error, payload) = match UserDataHandle::instance().handle(request) {
        Ok(answer) => (ErrorCode::Ok, Some(response::Payload::UserData(answer))),
        Err(UserDataError::Invalid(reason)) => {
            log::warn!("user-data request refused (id={id}): {reason}");
            (ErrorCode::FailInvariant, None)
        }
        Err(UserDataError::Store(message)) => {
            log::error!("user-data store failed (id={id}): {message}");
            (ErrorCode::FailIo, None)
        }
    };
    Response {
        id,
        error: error as i32,
        generation,
        payload,
    }
}

/// Where and how the stores were opened, and the stores.
struct Opened {
    paths: UserDataPaths,
    journal: JournalMode,
    stores: UserDataStores,
}

/// The stores this process opened. Set once, then read without a lock: from
/// roadmap P3b every keystroke reads through [`UserDataHandle::stores`], and
/// must never wait behind an open (its takeover can take seconds).
pub(crate) struct UserDataHandle {
    opened: OnceLock<Opened>,
}

impl UserDataHandle {
    pub(crate) const fn new() -> Self {
        Self {
            opened: OnceLock::new(),
        }
    }

    /// The process's one handle.
    pub(crate) fn instance() -> &'static Self {
        static HANDLE: UserDataHandle = UserDataHandle::new();
        &HANDLE
    }

    pub(crate) fn handle(
        &self,
        request: &UserDataRequest,
    ) -> Result<UserDataResponse, UserDataError> {
        let result = match request.method.as_ref() {
            Some(user_data_request::Method::Open(open)) => {
                user_data_response::Result::Opened(self.open(open)?)
            }
            Some(user_data_request::Method::Reset(reset)) => {
                user_data_response::Result::Reset(self.reset(reset)?)
            }
            None => return Err(UserDataError::Invalid("user-data request has no method")),
        };
        Ok(UserDataResponse {
            result: Some(result),
        })
    }

    /// The open stores, for the engine's own reads and writes; `None` until
    /// the platform opens them. Never blocks.
    pub(crate) fn stores(&self) -> Option<&UserDataStores> {
        self.opened.get().map(|opened| &opened.stores)
    }

    fn open(&self, open: &OpenUserData) -> Result<UserDataOpened, UserDataError> {
        let paths = UserDataPaths {
            frequency: absolute(&open.frequency_path)?,
            association: absolute(&open.association_path)?,
            custom_dictionary: absolute(&open.custom_dictionary_path)?,
            learned_phrases: absolute(&open.learned_phrases_path)?,
        };
        let journal = journal(open.journal());
        // A second open waits for the first to finish rather than opening
        // the same files twice.
        let opened = self.opened.get_or_init(|| {
            let stores = UserDataStores::at(paths.clone(), journal);
            stores.open_blocking();
            let ready = readiness(&stores);
            log::info!(
                "user_data.open frequency={} association={} custom_dictionary={} learned_phrases={}",
                ready.frequency_ready,
                ready.association_ready,
                ready.custom_dictionary_ready,
                ready.learned_phrases_ready
            );
            Opened {
                paths: paths.clone(),
                journal,
                stores,
            }
        });
        if opened.paths != paths || opened.journal != journal {
            return Err(UserDataError::Invalid(
                "user data is already open at other paths or with another journal",
            ));
        }
        Ok(readiness(&opened.stores))
    }

    fn reset(&self, reset: &ResetUserData) -> Result<UserDataReset, UserDataError> {
        if !(reset.frequency
            || reset.association
            || reset.custom_dictionary
            || reset.learned_phrases)
        {
            return Err(UserDataError::Invalid("reset selects no store"));
        }
        let stores = self
            .stores()
            .ok_or(UserDataError::Invalid("reset before user data was opened"))?;
        let mut removed = UserDataReset::default();
        if reset.frequency {
            removed.frequency_removed = stores.frequency.delete_all().map_err(store_error)?;
        }
        if reset.association {
            removed.association_removed = stores.association.delete_all().map_err(store_error)?;
        }
        if reset.custom_dictionary {
            let count = stores.custom_dictionary.delete_all().map_err(store_error)?;
            removed.custom_dictionary_removed = i64::try_from(count).unwrap_or(i64::MAX);
        }
        if reset.learned_phrases {
            removed.learned_phrases_removed =
                stores.learned_phrases.delete_all().map_err(store_error)?;
        }
        Ok(removed)
    }
}

fn store_error(error: impl Display) -> UserDataError {
    UserDataError::Store(error.to_string())
}

fn absolute(path: &str) -> Result<PathBuf, UserDataError> {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        Ok(path)
    } else {
        Err(UserDataError::Invalid(
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

fn readiness(stores: &UserDataStores) -> UserDataOpened {
    UserDataOpened {
        frequency_ready: stores.frequency.is_ready(),
        association_ready: stores.association.is_ready(),
        custom_dictionary_ready: stores.custom_dictionary.is_ready(),
        learned_phrases_ready: stores.learned_phrases.is_ready(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_request(directory: &std::path::Path) -> UserDataRequest {
        let paths = UserDataPaths::in_directory(directory);
        UserDataRequest {
            method: Some(user_data_request::Method::Open(OpenUserData {
                frequency_path: paths.frequency.display().to_string(),
                association_path: paths.association.display().to_string(),
                custom_dictionary_path: paths.custom_dictionary.display().to_string(),
                learned_phrases_path: paths.learned_phrases.display().to_string(),
                journal: UserDataJournal::Delete as i32,
            })),
        }
    }

    fn reset_request(reset: ResetUserData) -> UserDataRequest {
        UserDataRequest {
            method: Some(user_data_request::Method::Reset(reset)),
        }
    }

    fn opened(response: UserDataResponse) -> UserDataOpened {
        match response.result {
            Some(user_data_response::Result::Opened(opened)) => opened,
            other => panic!("expected Opened, got {other:?}"),
        }
    }

    #[test]
    fn open_readies_every_store_and_seeds_the_custom_dictionary() {
        let directory = tempfile::tempdir().unwrap();
        let handle = UserDataHandle::new();

        let answer = opened(handle.handle(&open_request(directory.path())).unwrap());

        assert!(answer.frequency_ready);
        assert!(answer.association_ready);
        assert!(answer.custom_dictionary_ready);
        assert!(answer.learned_phrases_ready);
        let stores = handle.stores().unwrap();
        assert_eq!(
            stores.custom_dictionary.count().unwrap(),
            2,
            "seeded before the answer"
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
            UserDataError::Invalid(
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

        assert!(matches!(
            handle.handle(&wal),
            Err(UserDataError::Invalid(_))
        ));
    }

    #[test]
    fn a_relative_path_is_refused_before_anything_opens() {
        let handle = UserDataHandle::new();
        let mut request = open_request(std::path::Path::new("/tmp"));
        if let Some(user_data_request::Method::Open(open)) = request.method.as_mut() {
            open.frequency_path = "user_frequency.db".into();
        }

        assert!(matches!(
            handle.handle(&request),
            Err(UserDataError::Invalid(_))
        ));
        assert!(handle.stores().is_none());
    }

    #[test]
    fn reset_empties_only_the_selected_stores() {
        let directory = tempfile::tempdir().unwrap();
        let handle = UserDataHandle::new();
        handle.handle(&open_request(directory.path())).unwrap();
        let stores = handle.stores().unwrap();
        stores.frequency.record("台", "tâi");

        let response = handle
            .handle(&reset_request(ResetUserData {
                frequency: true,
                ..ResetUserData::default()
            }))
            .unwrap();

        match response.result {
            Some(user_data_response::Result::Reset(reset)) => {
                assert_eq!(reset.frequency_removed, 1);
                assert_eq!(reset.custom_dictionary_removed, 0);
            }
            other => panic!("expected Reset, got {other:?}"),
        }
        assert_eq!(
            stores.custom_dictionary.count().unwrap(),
            2,
            "not selected, kept"
        );
    }

    #[test]
    fn a_reset_of_nothing_or_before_open_is_refused() {
        let handle = UserDataHandle::new();
        assert_eq!(
            handle
                .handle(&reset_request(ResetUserData {
                    frequency: true,
                    ..ResetUserData::default()
                }))
                .unwrap_err(),
            UserDataError::Invalid("reset before user data was opened")
        );
        assert_eq!(
            handle
                .handle(&reset_request(ResetUserData::default()))
                .unwrap_err(),
            UserDataError::Invalid("reset selects no store")
        );
    }
}
