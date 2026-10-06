//! Learning Records: what the keyboard learned from the user's picks. The
//! Custom Dictionary page's shape (`custom_dictionary.rs`) over the engine's
//! learning stores: a kind picker (word frequency, phrases) and an order
//! picker over a filter, rows fetched one PAGE at a time (10), a list whose
//! selection drives the edit-count / delete pair — plus Add to Custom
//! Dictionary — and the pager under it, then Delete Learning Records under
//! the card: it empties every learning store, whichever kind is on screen,
//! and asks first. No add — a word the user wants is a custom word.
//!
//! The listing rules and every job body are
//! `taigi_desktop_core::settings::learning_records`'s, shared with the
//! Linux pane; this file draws them and runs them off the UI thread, in the
//! page's one work slot (refused, not queued) — as Custom Dictionary does.
//! Deleting one row asks nothing, as deleting one custom word does not: the
//! keyboard learns the row again on the next pick.

use crate::winui::cards;
use crate::winui::list_pager::{
    self, icon_button, ADD_GLYPH, CONTROL_GAP, EDIT_GLYPH, REMOVE_GLYPH, SECONDARY_OPACITY,
    TABLE_COLUMN_GAP, TABLE_HEADER_GAP, TABLE_HEADER_INSET, TABLE_HEIGHT,
};
use crate::winui::list_selection::{selectable_list, SettledRows};
use crate::winui::pages::choice_row;
use crate::winui::window::{Message as WindowMessage, SettingsWindow};
use taigi_desktop_core::engine::user_data::{
    LearningRecord, LearningRecordKind, LearningRecordOrder, LearningRecordPage,
};
use taigi_desktop_core::settings::learning_records::{
    add_to_custom_dictionary_job, clear_all_job, count_note, delete_job, fetch, last_used_label,
    order_label, set_count_job, whole_count, Listing, CLEAR_ALL, KINDS, MAX_COUNT, ORDERS,
};
use taigi_desktop_core::settings::listing::{JobOutcome, JobState, LoadLanded, FILTER_SETTLE};
use taigi_desktop_core::settings::presentation::PageMessage;
use taigi_desktop_core::strings::{StringKey, StringResolver};
use windows_reactor::*;

const DIALOG_FIELD_WIDTH: f64 = 320.0;
/// Reading, word, count, last used: the two texts share most of the line,
/// the count and the day take what a number and `YYYY-MM-DD` need.
const TABLE_COLUMNS: [GridLength; 4] = [
    GridLength::Star(1.0),
    GridLength::Star(1.0),
    GridLength::Star(0.5),
    GridLength::Star(0.8),
];
/// A fetched page and each row's last-used day, read off the UI thread:
/// the day asks the zone database once per row, not once per render.
type LoadedPage = (LearningRecordPage, Vec<String>);

#[derive(Clone)]
pub enum Message {
    /// `None` when a pop-up cleared its selection: nothing changes.
    ChooseKind(Option<(LearningRecordKind, StringKey)>),
    ChooseOrder(Option<LearningRecordOrder>),
    FilterChanged(String),
    /// The filter stopped changing: reload from the first page.
    FilterSettled(u64),
    /// A page the user stepped to, already clamped by the caller.
    ShowPage(usize),
    Loaded(u64, Box<Result<LoadedPage, String>>),
    Select(Option<usize>),
    Edit,
    Delete,
    /// The selected row's word becomes a custom word: a learned phrase then
    /// leaves this list, a frequency row stays.
    AddToCustomDictionary,
    /// The count field's value; `None` while it is cleared.
    CountChanged(Option<f64>),
    CountDialogClosed(ContentDialogResult),
    /// Delete Learning Records asking for its confirmation (`Confirmation`
    /// says why it asks).
    AskClearAll,
    ConfirmClosed(ContentDialogResult),
    JobFinished(u64, Box<JobOutcome>),
    /// The job at this generation has run long enough to say so.
    ShowBusy(u64),
    /// What the list on screen holds, as `list_selection` reports it.
    RowsApplied(Option<Vec<String>>),
}

/// The row whose count is being edited in the dialog.
struct EditingCount {
    record: LearningRecord,
    /// What the field holds; `None` while it is cleared.
    count: Option<f64>,
}

pub struct LearningRecordsModel {
    listing: Listing,
    /// `listing.rows`' last-used days, index for index; replaced with them.
    last_used: Vec<String>,
    kind: LearningRecordKind,
    order: LearningRecordOrder,
    is_first_load_requested: bool,
    /// Which rows the list on screen holds, so the selection index reaches
    /// XAML a render after the rows it counts do (`list_selection`).
    settled: SettledRows,
    editing: Option<EditingCount>,
    /// Delete Learning Records is waiting on its dialog.
    is_confirming_clear_all: bool,
    /// The page's one work slot: the job it waits on, and whether the
    /// overlay shows.
    job: JobState,
}

impl Default for LearningRecordsModel {
    fn default() -> Self {
        Self {
            listing: Listing::default(),
            last_used: Vec::new(),
            kind: KINDS[0].0,
            order: ORDERS[0],
            is_first_load_requested: false,
            settled: SettledRows::default(),
            editing: None,
            is_confirming_clear_all: false,
            job: JobState::default(),
        }
    }
}

/// Starts the first load, once, when the page is first shown.
pub fn ensure_loaded(model: &mut LearningRecordsModel, context: &ComponentContext<SettingsWindow>) {
    if model.is_first_load_requested {
        return;
    }
    model.is_first_load_requested = true;
    load(model, context);
}

/// `alert` is the window's one notice: what a finished job has to say.
pub fn update(
    model: &mut LearningRecordsModel,
    message: Message,
    alert: &mut Option<PageMessage>,
    context: &ComponentContext<SettingsWindow>,
) {
    match message {
        Message::ChooseKind(Some((kind, _))) => {
            if kind != model.kind {
                model.kind = kind;
                show_from_first_page(model, context);
            }
        }
        Message::ChooseOrder(Some(order)) => {
            if order != model.order {
                model.order = order;
                show_from_first_page(model, context);
            }
        }
        Message::ChooseKind(None) | Message::ChooseOrder(None) => {}
        Message::FilterChanged(filter) => {
            let Some(generation) = model.listing.set_filter(filter) else {
                return;
            };
            // A wait the runtime will not start applies the filter at once
            // instead (Custom Dictionary's reasoning).
            _ = context.spawn_background_with_rejection(
                move |_| {
                    std::thread::sleep(FILTER_SETTLE);
                    WindowMessage::LearningRecords(Message::FilterSettled(generation))
                },
                WindowMessage::LearningRecords(Message::FilterSettled(generation)),
            );
        }
        Message::FilterSettled(generation) => {
            if model.listing.settle(generation) {
                load(model, context);
            }
        }
        Message::ShowPage(page) => {
            model.listing.page = page;
            load(model, context);
        }
        Message::Loaded(generation, outcome) => {
            let (outcome, last_used) = match *outcome {
                Ok((page, last_used)) => (Ok(page), last_used),
                Err(detail) => (Err(detail), Vec::new()),
            };
            match model.listing.land(generation, outcome) {
                LoadLanded::Adopted => model.last_used = last_used,
                LoadLanded::Failed(notice) => *alert = Some(notice),
                LoadLanded::Stale => {}
            }
        }
        Message::Select(index) => {
            // Resolved through the rows XAML holds, as on Custom Dictionary;
            // the key names the kind too, so a row of the kind just left is
            // never taken for this kind's row with the same id.
            let Some(key) = index.and_then(|index| model.settled.key_at(index)) else {
                return;
            };
            if let Some(row) = model.listing.rows.iter().find(|row| row_key(row) == key) {
                model.listing.selected_id = Some(row.id);
            }
        }
        Message::Edit => {
            // One dialog at a time (`dialog`).
            if model.is_confirming_clear_all {
                return;
            }
            if let Some(row) = model.listing.selected_row() {
                model.editing = Some(EditingCount {
                    count: Some(row.count.max(1) as f64),
                    record: row.clone(),
                });
            }
        }
        Message::Delete => {
            let Some(row) = model.listing.selected_row().cloned() else {
                return;
            };
            begin_job(model, context, move || delete_job(row));
        }
        Message::AddToCustomDictionary => {
            let Some(row) = model.listing.selected_row().cloned() else {
                return;
            };
            begin_job(model, context, move || add_to_custom_dictionary_job(row));
        }
        Message::CountChanged(count) => {
            if let Some(editing) = model.editing.as_mut() {
                editing.count = count;
            }
        }
        Message::CountDialogClosed(result) => {
            let Some(EditingCount { record, count }) = model.editing.take() else {
                return;
            };
            if result != ContentDialogResult::Primary {
                return;
            }
            // The primary button is disabled while the field is cleared, so
            // this is the belt.
            let Some(count) = count.and_then(whole_count) else {
                return;
            };
            begin_job(model, context, move || set_count_job(record, count));
        }
        Message::AskClearAll => {
            // Nothing to ask while a count is open or a job holds the slot
            // — the clear could not start anyway.
            if model.editing.is_none() && !model.job.is_running() {
                model.is_confirming_clear_all = true;
            }
        }
        Message::ConfirmClosed(result) => {
            let was_confirming = std::mem::take(&mut model.is_confirming_clear_all);
            // The primary button is the destructive one; Escape, the close
            // button and a dismissal all leave the stores alone.
            if was_confirming && result == ContentDialogResult::Primary {
                begin_job(model, context, clear_all_job);
            }
        }
        Message::JobFinished(generation, outcome) => {
            if !model.job.finish(generation) {
                return;
            }
            let JobOutcome {
                message,
                is_reload_wanted,
            } = *outcome;
            if message.is_some() {
                *alert = message;
            }
            if is_reload_wanted {
                load(model, context);
            }
        }
        Message::ShowBusy(generation) => {
            model.job.show_busy(generation);
        }
        Message::RowsApplied(rows) => model.settled.report(rows),
    }
}

/// Another kind or order: the first page of it, nothing selected, the
/// filter kept. The load it starts makes any older one stale.
fn show_from_first_page(
    model: &mut LearningRecordsModel,
    context: &ComponentContext<SettingsWindow>,
) {
    model.listing.rewind();
    load(model, context);
}

/// The list item's key: the kind and the store's row id — ids are per
/// store, so the two kinds share them.
fn row_key(record: &LearningRecord) -> String {
    format!("{}:{}", record.kind, record.id)
}

/// Starts a load of the page on screen (`list_pager::spawn_load`), each
/// row's last-used day read beside it.
fn load(model: &mut LearningRecordsModel, context: &ComponentContext<SettingsWindow>) {
    let request = model.listing.begin_load();
    let (kind, order) = (model.kind, model.order);
    list_pager::spawn_load(
        context,
        request.generation,
        move || {
            let page = fetch(&request, kind, order)?;
            // The day in the user's calendar at the time it was used.
            let last_used = page
                .rows
                .iter()
                .map(|row| {
                    last_used_label(
                        row.last_used_ms,
                        taigi_windows_platform::utc_offset_seconds_at(row.last_used_ms),
                    )
                })
                .collect();
            Ok((page, last_used))
        },
        |generation, outcome| WindowMessage::LearningRecords(Message::Loaded(generation, outcome)),
    );
}

/// Takes the page's one work slot for `job`, or does nothing because
/// something else holds it (`list_pager::spawn_job`).
fn begin_job(
    model: &mut LearningRecordsModel,
    context: &ComponentContext<SettingsWindow>,
    job: impl FnOnce() -> JobOutcome + Send + 'static,
) {
    list_pager::spawn_job::<LearningRecord>(
        &mut model.job,
        context,
        job,
        |generation, outcome| {
            WindowMessage::LearningRecords(Message::JobFinished(generation, outcome))
        },
        |generation| WindowMessage::LearningRecords(Message::ShowBusy(generation)),
    );
}

pub fn view(
    window: &SettingsWindow,
    strings: &StringResolver,
    context: &mut ViewContext<SettingsWindow>,
) -> View {
    // No user-data directory: the stores never opened, and the banner at
    // the top of the window says so — nothing to list, nothing to write.
    if window.is_read_only() {
        return View::empty();
    }
    let model = window.learning_records();
    // Mutual exclusion is the slot's; the greyed look waits the same
    // 400 ms as the overlay so a millisecond-long write does not flash it.
    let is_enabled = !model.job.is_busy_shown();
    let current_kind = KINDS
        .iter()
        .copied()
        .find(|(kind, _)| *kind == model.kind)
        .unwrap_or(KINDS[0]);
    View::fragment((
        choice_row(
            strings.resolve(StringKey::DictionaryLearningRecords),
            &KINDS,
            current_kind,
            is_enabled,
            |(_, label)| strings.resolve(label).to_owned(),
            |kind| WindowMessage::LearningRecords(Message::ChooseKind(kind)),
            context,
        ),
        choice_row(
            strings.resolve(StringKey::DictionaryLearningRecordsOrder),
            &ORDERS,
            model.order,
            is_enabled,
            |order| strings.resolve(order_label(order)).to_owned(),
            |order| WindowMessage::LearningRecords(Message::ChooseOrder(order)),
            context,
        ),
        cards::section_title_with_count(
            strings.resolve(StringKey::DesktopEntriesSection),
            &model.listing.count_label(),
        ),
        TextBox::new()
            .text(model.listing.filter().to_owned())
            .is_enabled(is_enabled)
            .placeholder_text(strings.resolve(StringKey::DictionarySearchPlaceholder))
            .on_text_changed(
                context
                    .callback(|text| WindowMessage::LearningRecords(Message::FilterChanged(text))),
            ),
        record_table(model, strings, context, is_enabled),
        cards::section_gap(),
        cards::action_row(
            strings.resolve(StringKey::DictionaryClearLearningRecords),
            strings.resolve(StringKey::CommonDelete),
            true,
            is_enabled,
            context.callback(|()| WindowMessage::LearningRecords(Message::AskClearAll)),
        ),
        list_pager::busy_overlay(
            model
                .job
                .is_busy_shown()
                .then_some(StringKey::DesktopProgressWorking),
            strings,
        ),
        dialog(model, strings, context),
    ))
}

/// The one dialog the page can have up. A count is being edited or Delete
/// Learning Records is being confirmed — never both: each verb refuses while
/// the other's dialog is up (`update`).
fn dialog(
    model: &LearningRecordsModel,
    strings: &StringResolver,
    context: &mut ViewContext<SettingsWindow>,
) -> View {
    if model.editing.is_some() {
        return count_dialog(model, strings, context);
    }
    if !model.is_confirming_clear_all {
        return View::empty();
    }
    list_pager::confirm_dialog(
        CLEAR_ALL,
        strings,
        context.callback(|result| WindowMessage::LearningRecords(Message::ConfirmClosed(result))),
    )
}

/// One row's cells, or the header's, on the table's four columns.
fn table_line(cells: [View; 4]) -> View {
    let [reading, word, count, last_used] = cells;
    Grid::new()
        .columns(TABLE_COLUMNS)
        .column_spacing(TABLE_COLUMN_GAP)
        .children((
            Border::new().grid_column(0).content(reading),
            Border::new().grid_column(1).content(word),
            Border::new().grid_column(2).content(count),
            Border::new().grid_column(3).content(last_used),
        ))
}

/// The records in one card: the column names, the list, then the edit /
/// delete pair — with Add to Custom Dictionary between them — and the pager
/// under it (Custom Dictionary's `entry_table`).
fn record_table(
    model: &LearningRecordsModel,
    strings: &StringResolver,
    context: &mut ViewContext<SettingsWindow>,
    is_enabled: bool,
) -> View {
    let items = model
        .listing
        .rows
        .iter()
        .zip(&model.last_used)
        .map(|(row, last_used)| {
            let key = row_key(row);
            (
                key.clone(),
                ListViewItem::new().tag(key).content(table_line([
                    TextBlock::new()
                        .text(row.tl.clone())
                        .opacity(SECONDARY_OPACITY)
                        .into(),
                    TextBlock::new().text(row.text.clone()).into(),
                    TextBlock::new().text(row.count.to_string()).into(),
                    TextBlock::new()
                        .text(last_used.clone())
                        .opacity(SECONDARY_OPACITY)
                        .into(),
                ])),
            )
        })
        .collect::<Vec<_>>();
    let has_selection = model.listing.selected_row().is_some();
    // On for a selected row the engine says can be added: one syllable or no
    // Hanji greys it out on either kind.
    let can_add_selection = model
        .listing
        .selected_row()
        .is_some_and(|row| row.can_add_to_custom_dictionary);
    let add_button = icon_button(
        ADD_GLYPH,
        strings.resolve(StringKey::DictionaryLearningRecordsAddToCustomDictionary),
        is_enabled && can_add_selection,
        context.callback(|()| WindowMessage::LearningRecords(Message::AddToCustomDictionary)),
    );
    // The list stays live while a job runs; the verbs are what a job turns
    // off (Custom Dictionary's reasoning, `list_selection`).
    let list = selectable_list(
        "learningRecords.rows",
        &model.settled,
        items,
        model.listing.selected_index(),
        ListView::new()
            .selection_mode(ListViewSelectionMode::Single)
            .on_selection_changed(
                context.callback(|index| WindowMessage::LearningRecords(Message::Select(index))),
            )
            .height(TABLE_HEIGHT),
        context,
        |rows| WindowMessage::LearningRecords(Message::RowsApplied(rows)),
    );
    // OVER the list, not in place of it, as on Custom Dictionary.
    let list = Grid::new().children((
        list,
        Border::new().content(list_pager::empty_state(
            model.listing.empty_state_key(),
            strings,
        )),
    ));
    let header = |key| -> View {
        TextBlock::new()
            .text(strings.resolve(key))
            .font_weight(FontWeight::SEMI_BOLD)
            .into()
    };
    cards::frame(View::fragment((
        // The column names, above the list: a `ListView` has no header.
        Border::new()
            .margin(Thickness::new(
                TABLE_HEADER_INSET,
                0.0,
                0.0,
                TABLE_HEADER_GAP,
            ))
            .content(table_line([
                header(StringKey::DictionaryRomanLabel),
                header(StringKey::DictionaryHanziLabel),
                header(StringKey::DictionaryLearningRecordsCount),
                header(StringKey::DictionaryLearningRecordsLastUsed),
            ])),
        list,
        list_pager::bar(
            (
                icon_button(
                    EDIT_GLYPH,
                    strings.resolve(StringKey::DictionaryLearningRecordsEditCount),
                    is_enabled && has_selection,
                    context.callback(|()| WindowMessage::LearningRecords(Message::Edit)),
                ),
                add_button,
                icon_button(
                    REMOVE_GLYPH,
                    strings.resolve(StringKey::CommonDelete),
                    is_enabled && has_selection,
                    context.callback(|()| WindowMessage::LearningRecords(Message::Delete)),
                ),
            ),
            model.listing.page,
            model.listing.page_count(),
            is_enabled,
            strings,
            context,
            |page| WindowMessage::LearningRecords(Message::ShowPage(page)),
        ),
    )))
}

/// Edit one row's count: the word and its reading as the body, a
/// `NumberBox` for the count, and — for word frequency — the note that
/// counts past 40 rank the same. Escape is the dialog's own close key.
fn count_dialog(
    model: &LearningRecordsModel,
    strings: &StringResolver,
    context: &mut ViewContext<SettingsWindow>,
) -> View {
    let Some(editing) = model.editing.as_ref() else {
        return View::empty();
    };
    let record = &editing.record;
    let note = match LearningRecordKind::try_from(record.kind)
        .ok()
        .and_then(count_note)
    {
        Some(key) => TextBlock::new()
            .text(strings.resolve(key))
            .text_wrapping(TextWrapping::Wrap)
            .opacity(SECONDARY_OPACITY)
            .width(DIALOG_FIELD_WIDTH)
            .into(),
        None => View::empty(),
    };
    ContentDialog::new()
        .title(strings.resolve(StringKey::DictionaryLearningRecordsEditCount))
        .primary_button_text(strings.resolve(StringKey::CommonSave))
        .close_button_text(strings.resolve(StringKey::CommonCancel))
        .is_primary_button_enabled(editing.count.and_then(whole_count).is_some())
        .is_open(true)
        .on_closed(
            context.callback(|result| {
                WindowMessage::LearningRecords(Message::CountDialogClosed(result))
            }),
        )
        .content(
            StackPanel::new().spacing(CONTROL_GAP).children((
                TextBlock::new()
                    .text(format!("{}  {}", record.text, record.tl).trim().to_owned())
                    .text_wrapping(TextWrapping::Wrap)
                    .width(DIALOG_FIELD_WIDTH),
                NumberBox::new()
                    .minimum(1.0)
                    .maximum(MAX_COUNT as f64)
                    .value(editing.count)
                    .width(DIALOG_FIELD_WIDTH)
                    .on_value_changed(context.callback(|count| {
                        WindowMessage::LearningRecords(Message::CountChanged(count))
                    }))
                    .slot(
                        NumberBoxSlot::Header,
                        strings.resolve(StringKey::DictionaryLearningRecordsCount),
                    ),
                note,
            )),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_key_tells_the_two_kinds_apart_at_the_same_id() {
        let frequency = LearningRecord {
            kind: LearningRecordKind::Frequency as i32,
            id: 7,
            ..LearningRecord::default()
        };
        let phrase = LearningRecord {
            kind: LearningRecordKind::LearnedPhrase as i32,
            ..frequency.clone()
        };
        assert_ne!(row_key(&frequency), row_key(&phrase));
    }
}
