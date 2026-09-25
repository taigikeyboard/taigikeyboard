//! The daily check with no window (`--check-updates`: the Windows scheduled
//! task), as one sequence over the settings file:
//! claim the due window, fetch, then record the outcome the way a window
//! would and claim the once-per-version announcement. Two locked updates
//! around the fetch, so two launches in the same window fetch once and
//! announce once.
//! What an announcement looks like (a toast, a desktop notification) and any
//! staging cleanup stay the platform's.

use crate::checker::{self, Outcome};
use crate::manifest::UpdateManifest;
use crate::transport::ManifestFetcher;
use taigi_desktop_core::settings::{update_schedule, SettingsDocument};

/// What a scheduled check that ran leaves for its caller.
#[derive(Clone, Debug)]
pub struct ScheduledCheck {
    pub outcome: Outcome,
    /// The settings as recorded — the display language the announcement is
    /// worded in.
    pub document: SettingsDocument,
    /// The update to announce, when this launch won the version's claim.
    pub announce: Option<UpdateManifest>,
}

/// `update` is one locked read-modify-write of `settings.json` — the
/// caller's `SettingsFileStore::update` — answering the document as written,
/// or `None` when the file refused; a closure rather than the store, so this
/// crate links no SQLite. `None` when the check did not run: not due,
/// another launch claimed the window, or the file refused the write.
pub fn run_scheduled_check(
    mut update: impl FnMut(&mut dyn FnMut(&mut SettingsDocument)) -> Option<SettingsDocument>,
    fetcher: &dyn ManifestFetcher,
    installed_version: &str,
    now_ms: i64,
) -> Option<ScheduledCheck> {
    let mut claimed = false;
    update(&mut |document| claimed = update_schedule::claim_due_check(document, now_ms))?;
    if !claimed {
        return None;
    }
    let outcome = checker::check(fetcher, installed_version);
    // The record and the announcement claim in one locked update: a
    // recorded update is never left unclaimed by a failed second write.
    let mut won = false;
    let document = update(&mut |document| {
        checker::record(document, &outcome);
        if let Outcome::UpdateAvailable(manifest) = &outcome {
            won = checker::claim_announcement(document, &manifest.version);
        }
    })?;
    let announce = match &outcome {
        Outcome::UpdateAvailable(manifest) if won => Some(manifest.clone()),
        _ => None,
    };
    Some(ScheduledCheck {
        outcome,
        document,
        announce,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::FetchError;
    use std::cell::RefCell;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// The settings file, in memory.
    fn file(
        document: &RefCell<SettingsDocument>,
    ) -> impl FnMut(&mut dyn FnMut(&mut SettingsDocument)) -> Option<SettingsDocument> + '_ {
        move |mutate| {
            let mut document = document.borrow_mut();
            mutate(&mut document);
            Some(document.clone())
        }
    }

    struct Counting {
        answer: Result<UpdateManifest, FetchError>,
        fetches: AtomicUsize,
    }

    impl ManifestFetcher for Counting {
        fn fetch_published(&self) -> Result<UpdateManifest, FetchError> {
            self.fetches.fetch_add(1, Ordering::SeqCst);
            self.answer.clone()
        }
    }

    fn newer() -> Counting {
        Counting {
            answer: Ok(UpdateManifest {
                version: "9.9.9".to_owned(),
                download_page_url: "https://taigikeyboard.tw/".to_owned(),
                package: None,
            }),
            fetches: AtomicUsize::new(0),
        }
    }

    #[test]
    fn two_launches_in_one_window_fetch_once_and_announce_once() {
        // trace: claim_due_check stamps under the lock, so the second launch
        // at the same instant finds the window taken and never fetches.
        let settings = RefCell::new(SettingsDocument::default());
        let mut store = file(&settings);
        let fetcher = newer();
        let first = run_scheduled_check(&mut store, &fetcher, "3.6.10", 1_000).expect("due");
        assert!(matches!(first.outcome, Outcome::UpdateAvailable(_)));
        assert_eq!(
            first.announce.map(|manifest| manifest.version).as_deref(),
            Some("9.9.9")
        );
        assert!(run_scheduled_check(&mut store, &fetcher, "3.6.10", 1_000).is_none());
        assert_eq!(fetcher.fetches.load(Ordering::SeqCst), 1);

        // The next day's window fetches again and finds the same version
        // already announced.
        let next_day = 1_000 + update_schedule::CHECK_INTERVAL_MS;
        let second =
            run_scheduled_check(&mut store, &fetcher, "3.6.10", next_day).expect("due again");
        assert!(second.announce.is_none());
        assert!(checker::has_stored_pending(&second.document));
    }

    #[test]
    fn a_failed_fetch_is_recorded_as_nothing_and_announces_nothing() {
        let settings = RefCell::new(SettingsDocument::default());
        let mut store = file(&settings);
        let fetcher = Counting {
            answer: Err(FetchError::Rejected),
            fetches: AtomicUsize::new(0),
        };
        let ran = run_scheduled_check(&mut store, &fetcher, "3.6.10", 1_000).expect("due");
        assert_eq!(ran.outcome, Outcome::Failed);
        assert!(ran.announce.is_none());
        assert!(
            !update_schedule::is_due(&ran.document, 1_000),
            "stamped anyway"
        );
    }
}
