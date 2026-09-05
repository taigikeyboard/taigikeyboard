//! What a check concludes and what it leaves in `settings.json`, as pure
//! decisions over the document. Port of `UpdateChecker` — the schedule
//! (`updateNextCheckMs`, stamped BEFORE the fetch so a hanging server does
//! not re-check every launch), the pending manifest, the once-per-version
//! announcement gate.

use crate::manifest::{DottedVersion, UpdateManifest};
use crate::transport::ManifestFetcher;
use taigi_windows_core::settings::{keys, SettingsDocument};

/// `UpdateChecker.checkInterval`: daily.
pub const CHECK_INTERVAL_MS: i64 = 24 * 60 * 60 * 1000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    UpdateAvailable(UpdateManifest),
    UpToDate,
    Failed,
}

/// Whether the daily check is due (`checkAutomatically`'s guard).
pub fn is_due(document: &SettingsDocument, now_ms: i64) -> bool {
    now_ms >= document.i64(&keys::UPDATE_NEXT_CHECK_MS)
}

/// Stamped before the fetch, whatever it answers.
pub fn stamp_next_check(document: &mut SettingsDocument, now_ms: i64) {
    document.set_i64(&keys::UPDATE_NEXT_CHECK_MS, now_ms + CHECK_INTERVAL_MS);
}

/// Fetches and classifies. A manifest that cannot be read is a failed
/// check, never an "up to date".
pub fn check(fetcher: &dyn ManifestFetcher, installed_version: &str) -> Outcome {
    match fetcher.fetch_published() {
        Ok(manifest) => classify(&manifest, installed_version),
        Err(error) => {
            log::debug!("update.fetch_failed error={error}");
            Outcome::Failed
        }
    }
}

/// Newer than the running build ⇒ available; unparsable on either side
/// reads as up to date (a development build with no version).
pub fn classify(manifest: &UpdateManifest, installed_version: &str) -> Outcome {
    match (
        DottedVersion::parse(installed_version),
        DottedVersion::parse(&manifest.version),
    ) {
        (Some(installed), Some(remote)) if remote > installed => {
            Outcome::UpdateAvailable(manifest.clone())
        }
        _ => Outcome::UpToDate,
    }
}

/// The stored pending update, if it is still newer than what is running
/// (an upgrade since makes it stale — cleared by the caller).
pub fn pending_update(
    document: &SettingsDocument,
    installed_version: &str,
) -> Option<UpdateManifest> {
    let stored = document.string(&keys::UPDATE_PENDING_MANIFEST);
    if stored.is_empty() {
        return None;
    }
    let manifest = UpdateManifest::decode(stored.as_bytes()).ok()?;
    match classify(&manifest, installed_version) {
        Outcome::UpdateAvailable(manifest) => Some(manifest),
        _ => None,
    }
}

pub fn has_stored_pending(document: &SettingsDocument) -> bool {
    !document.string(&keys::UPDATE_PENDING_MANIFEST).is_empty()
}

/// What the outcome leaves behind (`deliver`): a found update is kept, an
/// up-to-date answer clears one, a failure changes nothing.
pub fn record(document: &mut SettingsDocument, outcome: &Outcome) {
    match outcome {
        Outcome::UpdateAvailable(manifest) => {
            document.set_string(&keys::UPDATE_PENDING_MANIFEST, &manifest.encode());
        }
        Outcome::UpToDate => clear_pending(document),
        Outcome::Failed => {}
    }
}

pub fn clear_pending(document: &mut SettingsDocument) {
    document.set_string(&keys::UPDATE_PENDING_MANIFEST, "");
}

/// The automatic check announces a version once
/// (`updateLastNotifiedVersion`); the manual check always answers.
pub fn has_already_announced(document: &SettingsDocument, version: &str) -> bool {
    let announced = document.string(&keys::UPDATE_LAST_NOTIFIED_VERSION);
    match (
        DottedVersion::parse(&announced),
        DottedVersion::parse(version),
    ) {
        (Some(announced), Some(candidate)) => announced == candidate,
        _ => false,
    }
}

pub fn record_announced(document: &mut SettingsDocument, version: &str) {
    document.set_string(&keys::UPDATE_LAST_NOTIFIED_VERSION, version);
}

/// Records the announcement if nobody has: run INSIDE one locked
/// `settings.json` update, it is the claim that keeps the scheduled task
/// and an open window from toasting the same version twice. Answers
/// whether the caller won the claim (and so posts the toast).
pub fn claim_announcement(document: &mut SettingsDocument, version: &str) -> bool {
    if has_already_announced(document, version) {
        return false;
    }
    record_announced(document, version);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::FetchError;

    struct Canned(Result<UpdateManifest, FetchError>);

    impl ManifestFetcher for Canned {
        fn fetch_published(&self) -> Result<UpdateManifest, FetchError> {
            self.0.clone()
        }
    }

    fn manifest(version: &str) -> UpdateManifest {
        UpdateManifest {
            version: version.to_owned(),
            download_page_url: "https://taigikeyboard.tw/".to_owned(),
            package: None,
        }
    }

    #[test]
    fn newer_is_available_same_or_older_is_up_to_date_unreadable_is_failed() {
        // trace: UpdateChecker.classify + the fetch error path.
        assert_eq!(
            check(&Canned(Ok(manifest("3.7.0"))), "3.6.6"),
            Outcome::UpdateAvailable(manifest("3.7.0"))
        );
        assert_eq!(
            check(&Canned(Ok(manifest("3.6.6"))), "3.6.6"),
            Outcome::UpToDate
        );
        assert_eq!(
            check(&Canned(Ok(manifest("0.0.0"))), "3.6.6"),
            Outcome::UpToDate
        );
        assert_eq!(
            check(&Canned(Err(FetchError::Rejected)), "3.6.6"),
            Outcome::Failed
        );
        // A development build with no version never sees an update.
        assert_eq!(check(&Canned(Ok(manifest("9.9.9"))), ""), Outcome::UpToDate);
    }

    #[test]
    fn the_schedule_is_stamped_before_the_fetch_and_the_pending_manifest_round_trips() {
        let mut document = SettingsDocument::default();
        assert!(is_due(&document, 0));
        stamp_next_check(&mut document, 1_000);
        assert!(!is_due(&document, 1_000 + CHECK_INTERVAL_MS - 1));
        assert!(is_due(&document, 1_000 + CHECK_INTERVAL_MS));
        record(&mut document, &Outcome::UpdateAvailable(manifest("3.7.0")));
        assert_eq!(pending_update(&document, "3.6.6"), Some(manifest("3.7.0")));
        // Upgraded past it: stale, and reported as none.
        assert_eq!(pending_update(&document, "3.7.0"), None);
        assert!(has_stored_pending(&document));
        record(&mut document, &Outcome::Failed);
        assert!(has_stored_pending(&document), "a failure changes nothing");
        record(&mut document, &Outcome::UpToDate);
        assert!(!has_stored_pending(&document));
    }

    #[test]
    fn an_announcement_is_made_once_per_version() {
        let mut document = SettingsDocument::default();
        assert!(!has_already_announced(&document, "3.7.0"));
        record_announced(&mut document, "3.7.0");
        assert!(has_already_announced(&document, "3.7.0"));
        assert!(
            has_already_announced(&document, "3.7"),
            "zero-padded equality"
        );
        assert!(!has_already_announced(&document, "3.7.1"));
        assert!(claim_announcement(&mut document, "3.7.1"));
        assert!(!claim_announcement(&mut document, "3.7.1"), "one claim");
    }
}
