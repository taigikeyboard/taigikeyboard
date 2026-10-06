//! The Custom Dictionary pane's model, shared by both settings windows: the
//! list (`listing.rs`, shared with Learning Records), the destructive
//! command that asks first, and what each job answers. The shells own the widgets, the timers, and the work slot a job
//! runs in — Windows holds one per page, Linux one per window so an outcome
//! outlives a page rebuilt under it. macOS keeps a Swift twin:
//! `CustomDictionaryPageModel` in `CustomDictionaryPage.swift`.

use super::listing::{self, JobOutcome, ListedRow, LoadRequest, PAGE_SIZE};
use super::presentation::{Confirmation, PageMessage};
use crate::engine::user_data::{
    self, CustomDictionaryEntry, CustomDictionaryPage, CustomDictionaryRefusal, UserDataError,
};
use crate::strings::StringKey;
use std::path::Path;

/// Delete All asks first (`Confirmation` says why).
pub const DELETE_ALL: Confirmation = Confirmation {
    title: StringKey::DictionaryDeleteAll,
    message: StringKey::DictionaryDeleteAllMessage,
};

/// A write answers with nothing but its failure, and asks for a reload
/// either way.
fn write_outcome(result: Result<(), UserDataError>) -> JobOutcome {
    JobOutcome {
        message: result
            .err()
            .map(|error| PageMessage::failure(CustomDictionaryEntry::WRITE_FAILED, error)),
        is_reload_wanted: true,
    }
}

/// Adds a word (`id` empty — the engine mints one) or edits the one with
/// `id`: an edit is an edit, not a new entry that happens to replace one.
/// Both fields are stored trimmed.
pub fn save_entry_job(id: &str, roman: &str, hanji: &str) -> JobOutcome {
    write_outcome(user_data::save_custom_entry(id, roman.trim(), hanji.trim()).map(|_| ()))
}

pub fn delete_entry_job(id: &str) -> JobOutcome {
    write_outcome(user_data::delete_custom_entry(id))
}

pub fn delete_all_job() -> JobOutcome {
    write_outcome(user_data::delete_all_custom_entries())
}

/// Imports a CSV file into the custom dictionary.
pub fn import_job(path: &Path) -> JobOutcome {
    let message = match user_data::import_custom_csv_file(path) {
        Ok(imported) => PageMessage::Imported {
            imported: imported.imported as usize,
            skipped: imported.skipped as usize,
        },
        Err(UserDataError::Refused {
            refusal: CustomDictionaryRefusal::NotUtf8,
            ..
        }) => PageMessage::NotUtf8,
        Err(error) => PageMessage::failure(StringKey::CommonImportFailed, error),
    };
    JobOutcome {
        message: Some(message),
        is_reload_wanted: true,
    }
}

/// Exports the WHOLE dictionary — not the page or the filter's matches —
/// through `write_file`, each window's own atomic write.
pub fn export_job(
    path: &Path,
    write_file: impl FnOnce(&Path, &[u8]) -> Result<(), String>,
) -> JobOutcome {
    let outcome = user_data::export_custom_csv()
        .map_err(|error| error.to_string())
        .and_then(|csv| write_file(path, &csv));
    JobOutcome {
        message: outcome
            .err()
            .map(|error| PageMessage::failure(StringKey::CommonExportFailed, error)),
        is_reload_wanted: false,
    }
}

/// The name the export's save dialog suggests.
pub fn export_file_name(local_date: &str) -> String {
    format!("taigi_custom_dictionary_{local_date}.csv")
}

/// The dictionary's page on screen (`listing.rs`).
pub type Listing = listing::Listing<CustomDictionaryEntry>;

impl ListedRow for CustomDictionaryEntry {
    type Id = String;
    const EMPTY: StringKey = StringKey::DictionaryCustomDictEmpty;
    const READ_FAILED: StringKey = StringKey::DesktopCustomDictReadFailed;
    const WRITE_FAILED: StringKey = StringKey::DesktopCustomDictWriteFailed;

    fn id(&self) -> &String {
        &self.id
    }
}

/// The dictionary page `request` asks for.
pub fn fetch(request: &LoadRequest) -> Result<CustomDictionaryPage, String> {
    user_data::list_custom_page(&request.filter, request.page, PAGE_SIZE)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::listing::{JobState, LoadLanded};

    fn listing(match_count: usize, total_count: usize, filter: &str) -> Listing {
        let mut listing = Listing::default();
        listing.set_filter(filter.to_owned());
        listing.match_count = match_count;
        listing.total_count = total_count;
        listing
    }

    fn entry(id: &str) -> CustomDictionaryEntry {
        CustomDictionaryEntry {
            id: id.to_owned(),
            roman: "tsia̍h".to_owned(),
            hanji: "食".to_owned(),
            ..CustomDictionaryEntry::default()
        }
    }

    /// Page `number` of a dictionary holding `total_count` entries, of which
    /// the filter matches `match_count`.
    fn page(
        number: usize,
        ids: &[&str],
        match_count: usize,
        total_count: usize,
    ) -> CustomDictionaryPage {
        CustomDictionaryPage {
            page: number,
            rows: ids.iter().map(|id| entry(id)).collect(),
            match_count,
            total_count,
        }
    }

    /// Everything a load may change, to compare before and after.
    fn on_screen(listing: &Listing) -> (Vec<String>, usize, usize, usize, Option<String>) {
        (
            listing.rows.iter().map(|row| row.id.clone()).collect(),
            listing.page,
            listing.match_count,
            listing.total_count,
            listing.selected_id.clone(),
        )
    }

    /// A listing showing page 1 (`a`, `b`) of 12 matches out of 20, `a` selected.
    fn shown() -> Listing {
        let mut listing = Listing::default();
        let request = listing.begin_load();
        listing.land(request.generation, Ok(page(1, &["a", "b"], 12, 20)));
        listing.selected_id = Some("a".to_owned());
        listing
    }

    #[test]
    fn the_count_reads_as_one_number_until_a_filter_actually_narrows_it() {
        assert_eq!(listing(2, 2, "").count_label(), "2");
        assert_eq!(listing(2, 2, "tsia").count_label(), "2");
        assert_eq!(listing(1, 2, "tsia").count_label(), "1 / 2");
    }

    #[test]
    fn an_empty_list_says_which_kind_of_empty_it_is() {
        assert_eq!(
            listing(0, 0, "").empty_state_key(),
            Some(StringKey::DictionaryCustomDictEmpty)
        );
        assert_eq!(
            listing(0, 2, "zzz").empty_state_key(),
            Some(StringKey::DictionaryNoResults)
        );
        let mut listed = listing(1, 1, "");
        listed.rows.push(entry("a"));
        assert_eq!(listed.empty_state_key(), None, "rows say it themselves");
    }

    #[test]
    fn pages_are_ten_rows_and_never_fewer_than_one() {
        // trace: 0 → 1 page, 10 → 1, 11 → 2, 25 → 3.
        for (match_count, pages) in [(0, 1), (10, 1), (11, 2), (25, 3)] {
            assert_eq!(listing(match_count, match_count, "").page_count(), pages);
        }
    }

    #[test]
    fn a_step_stays_inside_the_pages_that_exist() {
        // trace: 25 matches → pages 0, 1, 2.
        let mut listing = listing(25, 25, "");
        assert!(!listing.step_page(-1), "before the first page");
        assert!(listing.step_page(2));
        assert_eq!(listing.page, 2);
        assert!(!listing.step_page(1), "past the last page");
        assert_eq!(listing.page, 2);
    }

    #[test]
    fn only_the_pages_own_job_settles_or_shows_busy() {
        let mut job = JobState::default();
        assert!(!job.is_running());
        job.start(4);
        assert!(job.is_running());
        assert!(!job.show_busy(3), "another job's overlay timer");
        assert!(!job.is_busy_shown());
        assert!(job.show_busy(4));
        assert!(job.is_busy_shown());
        assert!(!job.finish(3), "a job a rebuilt page inherited");
        assert!(job.finish(4));
        assert!(!job.is_busy_shown());
        assert!(!job.is_running());
        assert!(!job.finish(4), "settled once");
    }

    #[test]
    fn a_page_owning_its_slot_counts_its_own_generations() {
        let mut job = JobState::default();
        let first = job.start_next();
        assert!(job.is_running());
        assert!(job.finish(first));
        let second = job.start_next();
        assert_ne!(
            first, second,
            "a late answer to the first job is not the second's"
        );
        assert!(!job.finish(first));
        assert!(job.finish(second));
        assert_eq!(
            JobOutcome::could_not_start::<CustomDictionaryEntry>(),
            JobOutcome {
                message: Some(PageMessage::failure(
                    StringKey::DesktopCustomDictWriteFailed,
                    "the operation could not be started"
                )),
                is_reload_wanted: false,
            }
        );
    }

    #[test]
    fn a_filter_that_did_not_change_starts_nothing_and_only_the_newest_settles() {
        let mut listing = Listing::default();
        assert_eq!(listing.set_filter(String::new()), None);
        let first = listing.set_filter("ts".to_owned()).unwrap();
        let second = listing.set_filter("tsia".to_owned()).unwrap();
        listing.page = 3;
        assert!(!listing.settle(first), "an older keystroke's timer");
        assert_eq!(listing.page, 3);
        assert!(listing.settle(second));
        assert_eq!(listing.page, 0, "a settled filter reloads from page one");
    }

    #[test]
    fn a_stale_load_changes_nothing() {
        let mut listing = shown();
        let before = on_screen(&listing);
        let old = listing.begin_load();
        let new = listing.begin_load();
        assert_eq!(
            listing.land(old.generation, Ok(page(0, &["c"], 1, 1))),
            LoadLanded::Stale
        );
        assert_eq!(on_screen(&listing), before);
        assert_eq!(
            listing.land(old.generation, Err("gone".to_owned())),
            LoadLanded::Stale,
            "a stale failure is not reported either"
        );
        // A keystroke after the load started makes it stale too.
        listing.set_filter("x".to_owned());
        assert_eq!(
            listing.land(new.generation, Ok(page(0, &["c"], 1, 1))),
            LoadLanded::Stale
        );
        assert_eq!(on_screen(&listing), before);
    }

    // INVARIANT_USER_DATA_LIST_FILTER_RELOAD_SELECTION (§58): the filter
    // as typed, a reload after every write, a selection only on screen.
    #[test]
    fn a_load_sends_the_filter_as_typed_and_asks_for_the_page_on_screen() {
        let mut listing = Listing::default();
        listing.set_filter(" tsia ".to_owned());
        listing.page = 2;
        let request = listing.begin_load();
        assert_eq!(request.filter, " tsia ", "the engine trims it");
        assert_eq!(request.page, 2);
    }

    #[test]
    fn an_adopted_page_is_the_engines_and_drops_an_off_page_selection() {
        let mut listing = shown();
        // The user asked for page 3; the engine pulled it back to page 0.
        listing.page = 3;
        let request = listing.begin_load();
        assert_eq!(
            listing.land(request.generation, Ok(page(0, &["b", "a"], 11, 19))),
            LoadLanded::Adopted
        );
        assert_eq!(
            on_screen(&listing),
            (
                vec!["b".to_owned(), "a".to_owned()],
                0,
                11,
                19,
                Some("a".to_owned())
            ),
            "an on-page selection stays, at its new index"
        );
        assert_eq!(listing.selected_index(), Some(1));
        let request = listing.begin_load();
        listing.land(request.generation, Ok(page(1, &["c", "d"], 11, 19)));
        assert_eq!(listing.selected_id, None);
        // Back on the page that held it, nothing comes back selected.
        let request = listing.begin_load();
        listing.land(request.generation, Ok(page(0, &["b", "a"], 11, 19)));
        assert_eq!(listing.selected_id, None);
    }

    #[test]
    fn a_failed_load_changes_nothing_on_screen() {
        let mut listing = shown();
        // The user stepped on before the failing load started.
        listing.page = 2;
        let before = on_screen(&listing);
        let request = listing.begin_load();
        assert_eq!(
            listing.land(request.generation, Err("disk".to_owned())),
            LoadLanded::Failed(PageMessage::failure(
                StringKey::DesktopCustomDictReadFailed,
                "disk"
            ))
        );
        assert_eq!(
            on_screen(&listing),
            before,
            "rows, counts, the page asked for and the selection all stay"
        );
    }

    #[test]
    fn a_blank_filter_is_kept_as_typed_and_reads_as_a_filter() {
        let mut listing = Listing::default();
        listing.set_filter(" ".to_owned());
        assert_eq!(listing.filter(), " ");
        assert_eq!(listing.begin_load().filter, " ");
        assert_eq!(
            listing.empty_state_key(),
            Some(StringKey::DictionaryNoResults)
        );
    }

    #[test]
    fn a_write_reports_only_its_failure_and_reloads_either_way() {
        assert_eq!(
            write_outcome(Ok(())),
            JobOutcome {
                message: None,
                is_reload_wanted: true,
            }
        );
        let refused = UserDataError::EngineUnavailable("customDictionarySave");
        assert_eq!(
            write_outcome(Err(refused.clone())),
            JobOutcome {
                message: Some(PageMessage::failure(
                    StringKey::DesktopCustomDictWriteFailed,
                    refused
                )),
                is_reload_wanted: true,
            }
        );
        let died = JobOutcome::did_not_finish::<CustomDictionaryEntry>();
        assert!(died.is_reload_wanted);
        assert_eq!(
            died.message,
            Some(PageMessage::failure(
                StringKey::DesktopCustomDictWriteFailed,
                "the operation did not finish"
            ))
        );
    }

    #[test]
    fn the_export_suggests_a_dated_file_name() {
        assert_eq!(
            export_file_name("2026-09-30"),
            "taigi_custom_dictionary_2026-09-30.csv"
        );
    }
}
