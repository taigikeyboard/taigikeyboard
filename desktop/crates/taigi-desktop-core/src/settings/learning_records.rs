//! The Learning Records pane's model, shared by both settings windows:
//! which kinds and orders there are to pick from, the list (`listing.rs`,
//! shared with Custom Dictionary), what each job answers, and how a row
//! reads. The engine owns the rows and their rules
//! (`engine/userdata/src/learning_records.rs`); the shells own the widgets
//! and which kind and order are on screen.
//!
//! Design: `docs/architecture/learning-records-page-roadmap.md`.

use super::listing::{self, JobOutcome, ListedRow, LoadRequest, PAGE_SIZE};
use super::presentation::{Confirmation, PageMessage};
use crate::engine::user_data::{
    self, LearningRecord, LearningRecordKind, LearningRecordOrder, LearningRecordPage,
    UserDataError,
};
use crate::strings::StringKey;

/// The learned rows on screen.
pub type Listing = listing::Listing<LearningRecord>;

impl ListedRow for LearningRecord {
    type Id = i64;
    const EMPTY: StringKey = StringKey::DictionaryLearningRecordsEmpty;
    const READ_FAILED: StringKey = StringKey::DictionaryLearningRecordsReadFailed;
    const WRITE_FAILED: StringKey = StringKey::DictionaryLearningRecordsWriteFailed;

    fn id(&self) -> &i64 {
        &self.id
    }
}

/// The kinds the desktop lists with their labels, in picker order.
pub const KINDS: [(LearningRecordKind, StringKey); 2] = [
    (
        LearningRecordKind::Frequency,
        StringKey::DictionaryLearningRecordsFrequency,
    ),
    (
        LearningRecordKind::LearnedPhrase,
        StringKey::DictionaryLearningRecordsPhrases,
    ),
];

/// The orders the picker offers; the first is the default.
pub const ORDERS: [LearningRecordOrder; 2] = [
    LearningRecordOrder::MostUsed,
    LearningRecordOrder::MostRecent,
];

/// The largest count the edit field offers; the engine clamps to the same
/// (`engine/userdata` `set_count`).
pub const MAX_COUNT: i64 = 1_000_000;

/// A count field's value as a count the engine takes: a whole number in
/// `1..=MAX_COUNT`. `None` for no number at all (a cleared field). A typed
/// fraction rounds.
pub fn whole_count(value: f64) -> Option<i64> {
    value
        .is_finite()
        .then(|| value.round().clamp(1.0, MAX_COUNT as f64) as i64)
}

pub fn order_label(order: LearningRecordOrder) -> StringKey {
    match order {
        LearningRecordOrder::MostUsed => StringKey::DictionaryLearningRecordsOrderMostUsed,
        LearningRecordOrder::MostRecent => StringKey::DictionaryLearningRecordsOrderMostRecent,
    }
}

/// The note under the count field. Word frequency's ranking boost stops
/// growing at count 40 (`engine/ranking/src/score.rs` `MAX_BOOST`); the
/// other kinds make no such promise, so they say nothing.
pub fn count_note(kind: LearningRecordKind) -> Option<StringKey> {
    (kind == LearningRecordKind::Frequency)
        .then_some(StringKey::DictionaryLearningRecordsCountCapInfo)
}

/// The page asked for, of `kind` in `order`.
pub fn fetch(
    request: &LoadRequest,
    kind: LearningRecordKind,
    order: LearningRecordOrder,
) -> Result<LearningRecordPage, String> {
    user_data::list_learning_page(kind, order, &request.filter, request.page, PAGE_SIZE)
        .map_err(|error| error.to_string())
}

/// Sets `record`'s count.
pub fn set_count_job(record: LearningRecord, count: i64) -> JobOutcome {
    applied(
        user_data::set_learning_record_count(record, count).map(|stored| stored.is_some()),
        None,
        LearningRecord::WRITE_FAILED,
    )
}

/// Forgets `record`; the keyboard learns it again on the next pick.
pub fn delete_job(record: LearningRecord) -> JobOutcome {
    applied(
        user_data::delete_learning_record(record),
        None,
        LearningRecord::WRITE_FAILED,
    )
}

/// Makes `record`'s word a custom word; the receipt says where it went — the
/// reload takes a learned phrase off its list, a frequency row stays. The page
/// offers this on a row whose `can_add_to_custom_dictionary` is set.
pub fn add_to_custom_dictionary_job(record: LearningRecord) -> JobOutcome {
    applied(
        user_data::add_learning_record_to_custom_dictionary(record).map(|()| true),
        Some(StringKey::DictionaryLearningRecordsAddedToCustomDictionary),
        LearningRecord::WRITE_FAILED,
    )
}

/// Delete Learning Records asks first (`Confirmation` says why); the line
/// under the title says the custom words stay.
pub const CLEAR_ALL: Confirmation = Confirmation {
    title: StringKey::DictionaryClearLearningRecords,
    message: StringKey::DictionaryClearLearningRecordsMessage,
};

/// Empties every learning table — word frequency, learned phrases, and the
/// next-word association no page lists — whichever kind is on screen. Three
/// files, no transaction that could span them; the engine attempts each even
/// when an earlier one fails, and the notice reports rather than claims. The
/// list reloads either way: what it shows is what the stores still hold.
pub fn clear_all_job() -> JobOutcome {
    applied(
        user_data::clear_learning_records().map(|()| true),
        Some(StringKey::DictionaryClearLearningRecordsDone),
        StringKey::DictionaryClearLearningRecordsFailed,
    )
}

/// Every write reloads: the row was added, went, or was never there. A write
/// that landed says `receipt`, when it has one, and one that failed is
/// titled `failure`; a row already gone (deleted elsewhere, evicted, its id
/// taken by another word) is said, not reported as a failure.
fn applied(
    result: Result<bool, UserDataError>,
    receipt: Option<StringKey>,
    failure: StringKey,
) -> JobOutcome {
    let message = match result {
        Ok(true) => receipt.map(PageMessage::Done),
        Ok(false) => Some(PageMessage::Done(StringKey::DictionaryLearningRecordGone)),
        Err(error) => Some(PageMessage::failure(failure, error)),
    };
    JobOutcome {
        message,
        is_reload_wanted: true,
    }
}

/// The day a row was last used, `YYYY-MM-DD` in the viewer's calendar —
/// `utc_offset_seconds` east of UTC, which each shell reads from its own
/// clock. Empty when the store held no readable time (`last_used_ms` 0).
pub fn last_used_label(last_used_ms: i64, utc_offset_seconds: i64) -> String {
    if last_used_ms <= 0 {
        return String::new();
    }
    let local_seconds = last_used_ms.div_euclid(1000) + utc_offset_seconds;
    let (year, month, day) = civil_date(local_seconds.div_euclid(86_400));
    format!("{year:04}-{month:02}-{day:02}")
}

/// The proleptic Gregorian date `days` after 1970-01-01 (Howard Hinnant's
/// `civil_from_days`).
fn civil_date(days: i64) -> (i64, u32, u32) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_index + 2) / 5 + 1) as u32;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    } as u32;
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::user_data::{CustomDictionaryRefusal, UserDataPage};
    use crate::settings::listing::LoadLanded;

    fn record(id: i64, text: &str) -> LearningRecord {
        LearningRecord {
            id,
            text: text.to_owned(),
            tl: "tâi-uân".to_owned(),
            count: 3,
            ..LearningRecord::default()
        }
    }

    #[test]
    fn the_desktop_lists_frequency_and_phrases_and_names_both_orders() {
        assert_eq!(
            KINDS.map(|(kind, _)| kind),
            [
                LearningRecordKind::Frequency,
                LearningRecordKind::LearnedPhrase
            ],
            "no association on the desktop"
        );
        assert_eq!(ORDERS[0], LearningRecordOrder::MostUsed, "the default");
        assert_eq!(
            order_label(LearningRecordOrder::MostRecent),
            StringKey::DictionaryLearningRecordsOrderMostRecent
        );
    }

    #[test]
    fn the_count_field_answers_a_whole_number_inside_the_engines_range() {
        // trace: round, then clamp to 1..=1_000_000 (`set_count` clamps the
        // same, `engine/userdata`).
        assert_eq!(whole_count(25.0), Some(25));
        assert_eq!(whole_count(2.5), Some(3), "round half away from zero");
        assert_eq!(whole_count(0.2), Some(1), "never below one");
        assert_eq!(whole_count(5_000_000.0), Some(1_000_000));
        assert_eq!(whole_count(f64::NAN), None, "a cleared field");
    }

    #[test]
    fn only_word_frequency_says_its_count_stops_mattering_at_forty() {
        assert_eq!(
            count_note(LearningRecordKind::Frequency),
            Some(StringKey::DictionaryLearningRecordsCountCapInfo)
        );
        assert_eq!(count_note(LearningRecordKind::LearnedPhrase), None);
    }

    #[test]
    fn an_add_says_it_landed_or_why_not_and_reloads() {
        assert_eq!(
            applied(
                Ok(true),
                Some(StringKey::DictionaryLearningRecordsAddedToCustomDictionary),
                LearningRecord::WRITE_FAILED,
            ),
            JobOutcome {
                message: Some(PageMessage::Done(
                    StringKey::DictionaryLearningRecordsAddedToCustomDictionary
                )),
                is_reload_wanted: true
            }
        );
        let refused = applied(
            Err(UserDataError::Refused {
                refusal: CustomDictionaryRefusal::Full,
                detail: "custom dictionary is full (max 30000 entries)".to_owned(),
            }),
            Some(StringKey::DictionaryLearningRecordsAddedToCustomDictionary),
            LearningRecord::WRITE_FAILED,
        );
        assert!(refused.is_reload_wanted);
        assert_eq!(
            refused.message,
            Some(PageMessage::Failure {
                title: StringKey::DictionaryLearningRecordsWriteFailed,
                detail: "custom dictionary is full (max 30000 entries)".to_owned(),
            })
        );
    }

    #[test]
    fn delete_learning_records_reloads_whether_or_not_every_store_emptied() {
        let done = applied(
            Ok(true),
            Some(StringKey::DictionaryClearLearningRecordsDone),
            StringKey::DictionaryClearLearningRecordsFailed,
        );
        assert_eq!(
            done,
            JobOutcome {
                message: Some(PageMessage::Done(
                    StringKey::DictionaryClearLearningRecordsDone
                )),
                is_reload_wanted: true,
            }
        );
        // A store that could not be emptied is reported; the list still
        // reloads to what the others now hold.
        let failed = applied(
            Err(UserDataError::EngineUnavailable("resetUserData")),
            Some(StringKey::DictionaryClearLearningRecordsDone),
            StringKey::DictionaryClearLearningRecordsFailed,
        );
        assert!(failed.is_reload_wanted);
        assert!(matches!(
            failed.message,
            Some(PageMessage::Failure {
                title: StringKey::DictionaryClearLearningRecordsFailed,
                ..
            })
        ));
    }

    #[test]
    fn a_write_reloads_and_says_a_missing_row_without_calling_it_a_failure() {
        assert_eq!(
            applied(Ok(true), None, LearningRecord::WRITE_FAILED),
            JobOutcome {
                message: None,
                is_reload_wanted: true
            }
        );
        assert_eq!(
            applied(Ok(false), None, LearningRecord::WRITE_FAILED).message,
            Some(PageMessage::Done(StringKey::DictionaryLearningRecordGone))
        );
        let failed = applied(
            Err(UserDataError::EngineUnavailable("learningRecordDelete")),
            None,
            LearningRecord::WRITE_FAILED,
        );
        assert!(failed.is_reload_wanted);
        assert!(matches!(
            failed.message,
            Some(PageMessage::Failure {
                title: StringKey::DictionaryLearningRecordsWriteFailed,
                ..
            })
        ));
        assert_eq!(
            JobOutcome::did_not_finish::<LearningRecord>().message,
            Some(PageMessage::failure(
                StringKey::DictionaryLearningRecordsWriteFailed,
                "the operation did not finish"
            ))
        );
    }

    #[test]
    fn the_list_selects_by_row_id_and_names_its_own_empty_state() {
        let mut listing = Listing::default();
        assert_eq!(
            listing.empty_state_key(),
            Some(StringKey::DictionaryLearningRecordsEmpty)
        );
        let request = listing.begin_load();
        let loaded = UserDataPage {
            page: 0,
            rows: vec![record(7, "台灣"), record(9, "食飯")],
            match_count: 2,
            total_count: 2,
        };
        assert_eq!(
            listing.land(request.generation, Ok(loaded)),
            LoadLanded::Adopted
        );
        listing.selected_id = Some(9);
        assert_eq!(
            listing.selected_row().map(|row| row.text.as_str()),
            Some("食飯")
        );
        listing.page = 1;
        listing.rewind();
        assert_eq!((listing.page, listing.selected_id), (0, None));
        let request = listing.begin_load();
        assert_eq!(
            listing.land(request.generation, Err("disk".to_owned())),
            LoadLanded::Failed(PageMessage::failure(
                StringKey::DictionaryLearningRecordsReadFailed,
                "disk"
            ))
        );
    }

    #[test]
    fn the_last_used_day_is_the_viewers_calendar_day() {
        // trace: 2026-01-01T00:00:00Z = 1767225600 s = day 20454.
        let new_year = 1_767_225_600_000;
        assert_eq!(last_used_label(new_year, 0), "2026-01-01");
        assert_eq!(last_used_label(new_year, 8 * 3600), "2026-01-01", "Taipei");
        assert_eq!(
            last_used_label(new_year, -3600),
            "2025-12-31",
            "west of UTC"
        );
        // trace: 2024-02-29T12:00:00Z = 1709208000 s.
        assert_eq!(last_used_label(1_709_208_000_000, 0), "2024-02-29");
        assert_eq!(last_used_label(0, 0), "", "no readable time");
    }
}
