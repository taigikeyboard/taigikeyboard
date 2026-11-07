//! The sidebar's Learning Records section over the desktop panes' model
//! (`taigi_desktop_core::settings::learning_records`, the Linux and Windows
//! twin): one call per change carries the whole view the page keeps (kind,
//! order, filter, page) and, at most, one write; the answer is the page of
//! rows that view now shows. The engine's ops answer on this thread, so a
//! write has landed before the reload reads. Text arrives translated, as
//! the Shortcuts section's does. No count edit and no Add to Custom
//! Dictionary on the web, and nothing at all in a tab whose stores are not
//! open (maintainer 2026-11-07).

use serde::{Deserialize, Serialize};
use taigi_desktop_core::engine::user_data::LearningRecord;
use taigi_desktop_core::settings::learning_records::{
    clear_all_job, delete_job, fetch, last_used_label, Listing, KINDS, ORDERS,
};
use taigi_desktop_core::settings::listing::LoadLanded;
use taigi_desktop_core::settings::presentation::PageMessage;
use taigi_desktop_core::strings::{StringKey, StringResolver};

/// What the page asks for. `kind` and `order` index `KINDS` / `ORDERS`;
/// one out of range reads as the first.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LearningRecordsCall {
    kind: usize,
    order: usize,
    filter: String,
    page: usize,
    /// East of UTC, the page's clock now: what the last-used day reads in.
    utc_offset_seconds: i64,
    #[serde(default)]
    write: Option<Write>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum Write {
    /// The row with `id` among those last answered.
    Delete { id: i64 },
    /// Every learning store, whichever kind is on screen.
    Clear,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WireRow<'a> {
    id: i64,
    tl: &'a str,
    text: &'a str,
    count: i64,
    last_used: String,
}

/// A page notice: its title, and the English diagnostic line, which the
/// page logs.
#[derive(Debug, PartialEq, Serialize)]
struct WireMessage {
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WireView<'a> {
    rows: Vec<WireRow<'a>>,
    /// What the store holds, `matches / total` while a filter narrows it.
    count: String,
    /// The page served, zero-based: pulled back when the list shrank.
    page: usize,
    page_label: String,
    has_previous: bool,
    has_next: bool,
    /// The empty list's sentence, `None` while there are rows.
    #[serde(skip_serializing_if = "Option::is_none")]
    empty: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<WireMessage>,
    /// A write ran: the stores changed, so the page saves them.
    wrote: bool,
}

#[derive(Default)]
pub(crate) struct LearningRecordsPage {
    /// The rows last answered, which a delete names by id.
    listing: Listing,
    /// The stores are open. Before that a call is refused: the engine has
    /// nothing to list, and the files are still to be handed in.
    is_open: bool,
}

impl LearningRecordsPage {
    pub(crate) fn opened(&mut self) {
        self.is_open = true;
    }

    /// `call_json`: a [`LearningRecordsCall`]. Refused before the stores
    /// open, or for a malformed call.
    pub(crate) fn answer(
        &mut self,
        call_json: &str,
        strings: &StringResolver,
    ) -> Result<String, String> {
        if !self.is_open {
            return Err("learning records: the stores are not open".to_owned());
        }
        let call: LearningRecordsCall = serde_json::from_str(call_json)
            .map_err(|error| format!("learning records: {error}"))?;
        let (wrote, mut message) = match call.write {
            None => (false, None),
            Some(write) => self.write(write),
        };
        let (kind, _) = KINDS.get(call.kind).copied().unwrap_or(KINDS[0]);
        let order = ORDERS.get(call.order).copied().unwrap_or(ORDERS[0]);
        self.listing.set_filter(call.filter);
        self.listing.page = call.page;
        let request = self.listing.begin_load();
        let fetched = fetch(&request, kind, order);
        if let LoadLanded::Failed(failure) = self.listing.land(request.generation, fetched) {
            // A failed read keeps the rows it could not replace; the write's
            // own notice, when there is one, says more.
            message.get_or_insert(failure);
        }
        let view = WireView {
            message: message.map(|message| wire_message(&message, strings)),
            wrote,
            ..self.view(call.utc_offset_seconds, strings)
        };
        Ok(serde_json::to_string(&view).expect("plain strings and numbers"))
    }

    /// `(ran, notice)`: a delete of a row no longer on screen runs nothing.
    fn write(&self, write: Write) -> (bool, Option<PageMessage>) {
        let outcome = match write {
            Write::Delete { id } => {
                let Some(record) = self.listing.rows.iter().find(|row| row.id == id) else {
                    return (
                        false,
                        Some(PageMessage::Done(StringKey::DictionaryLearningRecordGone)),
                    );
                };
                delete_job(record.clone())
            }
            Write::Clear => clear_all_job(),
        };
        (true, outcome.message)
    }

    /// The listing on screen, no notice and no write.
    fn view(&self, utc_offset_seconds: i64, strings: &StringResolver) -> WireView<'_> {
        let listing = &self.listing;
        WireView {
            rows: listing
                .rows
                .iter()
                .map(|record| wire_row(record, utc_offset_seconds))
                .collect(),
            count: listing.count_label(),
            page: listing.page,
            page_label: format!("{} / {}", listing.page + 1, listing.page_count()),
            has_previous: listing.page > 0,
            has_next: listing.page + 1 < listing.page_count(),
            empty: listing.empty_state_key().map(|key| strings.resolve(key)),
            message: None,
            wrote: false,
        }
    }
}

fn wire_row(record: &LearningRecord, utc_offset_seconds: i64) -> WireRow<'_> {
    WireRow {
        id: record.id,
        tl: &record.tl,
        text: &record.text,
        count: record.count,
        last_used: last_used_label(record.last_used_ms, utc_offset_seconds),
    }
}

fn wire_message(message: &PageMessage, strings: &StringResolver) -> WireMessage {
    WireMessage {
        title: message.title(strings),
        detail: message.detail(strings),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use taigi_desktop_core::engine::user_data::UserDataPage;
    use taigi_desktop_core::strings::DisplayLanguage;

    fn english() -> StringResolver {
        StringResolver::new(DisplayLanguage::English)
    }

    fn call(value: Value) -> LearningRecordsCall {
        serde_json::from_value(value).unwrap()
    }

    fn record(id: i64, text: &str) -> LearningRecord {
        LearningRecord {
            id,
            text: text.to_owned(),
            tl: "tâi-uân".to_owned(),
            count: 3,
            // trace: 2026-01-01T00:00:00Z.
            last_used_ms: 1_767_225_600_000,
            ..LearningRecord::default()
        }
    }

    fn shown(rows: Vec<LearningRecord>, total: usize) -> LearningRecordsPage {
        let mut page = LearningRecordsPage::default();
        let request = page.listing.begin_load();
        let loaded = UserDataPage {
            page: 0,
            match_count: total,
            total_count: total,
            rows,
        };
        page.listing.land(request.generation, Ok(loaded));
        page
    }

    #[test]
    fn nothing_is_answered_before_the_stores_open() {
        let mut page = LearningRecordsPage::default();
        let read = r#"{"kind":0,"order":0,"filter":"","page":0,"utcOffsetSeconds":0}"#;
        assert!(page.answer(read, &english()).is_err());
        page.opened();
        assert!(
            page.answer("{", &english()).is_err(),
            "a malformed call is refused once open too"
        );
    }

    #[test]
    fn a_call_reads_the_view_and_an_optional_write() {
        let read = call(json!({
            "kind": 1, "order": 0, "filter": "tai", "page": 2, "utcOffsetSeconds": 28800
        }));
        assert!(read.write.is_none());
        let delete = call(json!({
            "kind": 0, "order": 1, "filter": "", "page": 0, "utcOffsetSeconds": 0,
            "write": { "type": "delete", "id": 7 }
        }));
        assert!(matches!(delete.write, Some(Write::Delete { id: 7 })));
        let clear = call(json!({
            "kind": 0, "order": 0, "filter": "", "page": 0, "utcOffsetSeconds": 0,
            "write": { "type": "clear" }
        }));
        assert!(matches!(clear.write, Some(Write::Clear)));
    }

    #[test]
    fn the_view_is_translated_and_dates_in_the_viewers_day() {
        let page = shown(vec![record(7, "台灣")], 12);
        let view = serde_json::to_value(page.view(-3600, &english())).unwrap();
        assert_eq!(
            view,
            json!({
                "rows": [{
                    "id": 7, "tl": "tâi-uân", "text": "台灣", "count": 3,
                    "lastUsed": "2025-12-31"
                }],
                "count": "12",
                "page": 0,
                // trace: 12 rows / PAGE_SIZE 10 = 2 pages.
                "pageLabel": "1 / 2",
                "hasPrevious": false,
                "hasNext": true,
                "wrote": false
            })
        );
        let empty = shown(vec![], 0);
        let empty = serde_json::to_value(empty.view(0, &english())).unwrap();
        // trace: dictionary.learningRecordsEmpty en.
        assert_eq!(empty["empty"], "Nothing Learned Yet");
        assert_eq!(empty["pageLabel"], "1 / 1");
    }

    #[test]
    fn a_delete_of_a_row_not_on_screen_runs_nothing_and_says_it_is_gone() {
        // After a trap the new instance has listed nothing yet.
        let page = shown(vec![record(7, "台灣")], 1);
        let (ran, message) = page.write(Write::Delete { id: 9 });
        assert!(!ran);
        assert_eq!(
            wire_message(&message.unwrap(), &english()),
            WireMessage {
                // trace: dictionary.learningRecordGone en.
                title: "This Record Is No Longer Stored".to_owned(),
                detail: None
            }
        );
    }

    #[test]
    fn a_failure_keeps_its_english_detail_for_the_console() {
        let failure = PageMessage::failure(
            StringKey::DictionaryClearLearningRecordsFailed,
            "user_frequency: disk",
        );
        assert_eq!(
            wire_message(&failure, &english()),
            WireMessage {
                // trace: dictionary.clearLearningRecordsFailed en.
                title: "Could Not Clear Learning Records".to_owned(),
                detail: Some("user_frequency: disk".to_owned())
            }
        );
    }
}
