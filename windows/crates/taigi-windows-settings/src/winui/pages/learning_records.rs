//! Learning Records: what the keyboard learned from the user's picks. The
//! Custom Dictionary page's shape (`custom_dictionary.rs`) over the engine's
//! learning stores: a kind picker (word frequency, phrases) and an order
//! picker over a filter, rows fetched one PAGE at a time (10), a list whose
//! selection drives the edit-count / delete pair (✎ opens the record
//! dialog: the count and, for a row the engine says can be added, an Add to
//! Custom Dictionary tab beside it) — plus, on learned phrases, Add to
//! Custom Dictionary, which opens Custom Dictionary's Add Word dialog over
//! the row's word — and the pager under it, then the `.taigi` backup (every
//! store, so a restore reloads Custom Dictionary too) and Delete Learning
//! Records under the card: it empties every learning store, whichever kind
//! is on screen, on the press. No add — a word the user wants is a custom
//! word.
//!
//! The listing rules and every job body are
//! `taigi_desktop_core::settings::learning_records`'s, shared with the
//! Linux pane; this file draws them and runs them off the UI thread, in the
//! page's one work slot (refused, not queued) — as Custom Dictionary does.
//! Deleting one row asks nothing, as deleting one custom word does not: the
//! keyboard learns the row again on the next pick.

use crate::winui::cards;
use crate::winui::file_dialog;
use crate::winui::list_pager::{
    self, icon_button, ADD_GLYPH, CONTROL_GAP, EDIT_GLYPH, REMOVE_GLYPH, SECONDARY_OPACITY,
    TABLE_COLUMN_GAP, TABLE_HEADER_GAP, TABLE_HEADER_INSET, TABLE_HEIGHT,
};
use crate::winui::list_selection::{selectable_list, SettledRows};
use crate::winui::pages::custom_dictionary::{self, word_dialog, EntryField};
use crate::winui::pages::{choice_row, RowLabel};
use crate::winui::window::{Message as WindowMessage, SettingsWindow};
use taigi_desktop_core::engine::user_data::{
    LearningRecord, LearningRecordKind, LearningRecordOrder, LearningRecordPage,
};
use taigi_desktop_core::settings::learning_records::{
    add_to_custom_dictionary_job, backup_file_name, clear_all_job, count_note, delete_job,
    export_backup_job, fetch, import_backup_job, last_used_label, offers_add_to_custom_dictionary,
    order_label, set_count_job, whole_count, Listing, KINDS, MAX_COUNT, ORDERS,
};
use taigi_desktop_core::settings::listing::{JobOutcome, JobState, LoadLanded, FILTER_SETTLE};
use taigi_desktop_core::settings::presentation::PageMessage;
use taigi_desktop_core::settings::{keys, SettingsDocument};
use taigi_desktop_core::strings::{StringKey, StringResolver};
use taigi_desktop_storage::SettingsWriter;
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
    /// Opens Custom Dictionary's Add Word dialog over the selected row's
    /// typed form and word, for the user to tweak before Save.
    AddToCustomDictionary,
    AddWordFieldChanged(EntryField, String),
    /// Save files the word as edited: the learned phrase then leaves this
    /// list. Cancel does nothing.
    AddWordDialogClosed(ContentDialogResult),
    /// The count field's value; `None` while it is cleared.
    CountChanged(Option<f64>),
    /// The record dialog's word fields, for a row the engine says can be
    /// added.
    RecordWordFieldChanged(EntryField, String),
    /// The record dialog's tab, by the selector item's text.
    RecordTabChanged(Option<String>),
    /// Primary follows the tab: Save writes the count, Add to Custom
    /// Dictionary files the word as edited. Cancel does nothing.
    CountDialogClosed(ContentDialogResult),
    /// Delete Learning Records runs on the press, no confirmation (USER
    /// 2026-10-16).
    ClearAll,
    ExportBackup,
    /// Restores into every store, so Custom Dictionary reloads too.
    ImportBackup,
    JobFinished(u64, Box<JobOutcome>),
    /// The job at this generation has run long enough to say so.
    ShowBusy(u64),
    /// What the list on screen holds, as `list_selection` reports it.
    RowsApplied(Option<Vec<String>>),
}

/// The row open in the record dialog: its count, and — for a row the
/// engine says can be added — its word as the user has edited it so far.
struct EditingCount {
    record: LearningRecord,
    /// What the field holds; `None` while it is cleared.
    count: Option<f64>,
    /// `(roman, hanji)`; `None` leaves the add tab out.
    word: Option<(String, String)>,
    /// Which tab is on screen; only a row with `word` has the add tab.
    is_add_tab: bool,
}

/// The row whose word is being filed in the Add Word dialog, and the word
/// as the user has edited it so far.
struct AddingWord {
    record: LearningRecord,
    roman: String,
    hanji: String,
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
    adding: Option<AddingWord>,
    /// The page's one work slot: the job it waits on, and whether the
    /// overlay shows.
    job: JobState,
    /// Whether the job `job` waits on writes the custom dictionary (an
    /// added word, a restored backup), so its landing reloads that page too.
    is_job_custom_dictionary_write: bool,
}

impl LearningRecordsModel {
    /// The page as the window opens it: on the kind and order last picked
    /// (#773), nothing fetched yet.
    pub fn restored(document: &SettingsDocument) -> Self {
        Self {
            listing: Listing::default(),
            last_used: Vec::new(),
            kind: document.choice(&keys::LEARNING_RECORDS_KIND),
            order: document.choice(&keys::LEARNING_RECORDS_ORDER),
            is_first_load_requested: false,
            settled: SettledRows::default(),
            editing: None,
            adding: None,
            job: JobState::default(),
            is_job_custom_dictionary_write: false,
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

/// `alert` is the window's one notice: what a finished job has to say;
/// `settings` keeps the kind and order picked, for the next open.
pub fn update(
    model: &mut LearningRecordsModel,
    message: Message,
    settings: &mut SettingsWriter,
    alert: &mut Option<PageMessage>,
    context: &ComponentContext<SettingsWindow>,
) {
    match message {
        Message::ChooseKind(Some((kind, _))) => {
            if kind != model.kind {
                model.kind = kind;
                settings.update(|document| document.set_choice(&keys::LEARNING_RECORDS_KIND, kind));
                show_from_first_page(model, context);
            }
        }
        Message::ChooseOrder(Some(order)) => {
            if order != model.order {
                model.order = order;
                settings
                    .update(|document| document.set_choice(&keys::LEARNING_RECORDS_ORDER, order));
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
            if let Some(row) = model.listing.selected_row() {
                model.editing = Some(EditingCount {
                    count: Some(row.count.max(1) as f64),
                    word: row
                        .can_add_to_custom_dictionary
                        .then(|| (row.tl.clone(), row.text.clone())),
                    is_add_tab: false,
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
            if let Some(row) = model.listing.selected_row() {
                model.adding = Some(AddingWord {
                    roman: row.tl.clone(),
                    hanji: row.text.clone(),
                    record: row.clone(),
                });
            }
        }
        Message::AddWordFieldChanged(field, text) => {
            if let Some(adding) = model.adding.as_mut() {
                match field {
                    EntryField::Roman => adding.roman = text,
                    EntryField::Hanji => adding.hanji = text,
                }
            }
        }
        Message::AddWordDialogClosed(result) => {
            let Some(AddingWord {
                record,
                roman,
                hanji,
            }) = model.adding.take()
            else {
                return;
            };
            // The primary button is disabled without a romanization, so
            // this is the belt.
            if result != ContentDialogResult::Primary || roman.trim().is_empty() {
                return;
            }
            begin_job_then(model, context, true, move || {
                add_to_custom_dictionary_job(record, &roman, &hanji)
            });
        }
        Message::CountChanged(count) => {
            if let Some(editing) = model.editing.as_mut() {
                editing.count = count;
            }
        }
        Message::RecordWordFieldChanged(field, text) => {
            if let Some((roman, hanji)) = model.editing.as_mut().and_then(|e| e.word.as_mut()) {
                match field {
                    EntryField::Roman => *roman = text,
                    EntryField::Hanji => *hanji = text,
                }
            }
        }
        Message::RecordTabChanged(text) => {
            // The selector answers with the item's text: the add tab's is
            // the label the view resolved from the same document.
            let add_label = crate::presentation::strings_for(settings.document())
                .resolve(StringKey::DictionaryLearningRecordsAddToCustomDictionary);
            if let (Some(editing), Some(text)) = (model.editing.as_mut(), text) {
                editing.is_add_tab = editing.word.is_some() && text == add_label;
            }
        }
        Message::CountDialogClosed(result) => {
            let Some(EditingCount {
                record,
                count,
                word,
                is_add_tab,
            }) = model.editing.take()
            else {
                return;
            };
            if result != ContentDialogResult::Primary {
                return;
            }
            if is_add_tab {
                // Disabled without a romanization, so this is the belt.
                let Some((roman, hanji)) = word.filter(|(roman, _)| !roman.trim().is_empty())
                else {
                    return;
                };
                begin_job_then(model, context, true, move || {
                    add_to_custom_dictionary_job(record, &roman, &hanji)
                });
            } else {
                // Disabled while the field is cleared, so this is the belt.
                let Some(count) = count.and_then(whole_count) else {
                    return;
                };
                begin_job(model, context, move || set_count_job(record, count));
            }
        }
        Message::ClearAll => begin_job(model, context, clear_all_job),
        Message::ExportBackup => export_backup(model, context),
        Message::ImportBackup => import_backup(model, context),
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
            // The added word or restored rows land on Custom Dictionary
            // without leaving for it: that page loads once, on its first
            // show, and would otherwise keep the rows it read before.
            if std::mem::take(&mut model.is_job_custom_dictionary_write) {
                context.sender().send(WindowMessage::CustomDictionary(
                    custom_dictionary::Message::Reload,
                ));
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
    begin_job_then(model, context, false, job);
}

/// `begin_job`; `writes_custom_dictionary` also reloads Custom Dictionary
/// once the job lands. Set only for a job that started — one refused
/// because another holds the slot never lands.
fn begin_job_then(
    model: &mut LearningRecordsModel,
    context: &ComponentContext<SettingsWindow>,
    writes_custom_dictionary: bool,
    job: impl FnOnce() -> JobOutcome + Send + 'static,
) {
    let has_started = list_pager::spawn_job::<LearningRecord>(
        &mut model.job,
        context,
        job,
        |generation, outcome| {
            WindowMessage::LearningRecords(Message::JobFinished(generation, outcome))
        },
        |generation| WindowMessage::LearningRecords(Message::ShowBusy(generation)),
    );
    if has_started {
        model.is_job_custom_dictionary_write = writes_custom_dictionary;
    }
}

fn export_backup(model: &mut LearningRecordsModel, context: &ComponentContext<SettingsWindow>) {
    if model.job.is_running() {
        return;
    }
    let suggested = backup_file_name(&taigi_windows_platform::local_date());
    let Some(path) = file_dialog::save(&file_dialog::BACKUP, &suggested) else {
        return;
    };
    begin_job(model, context, move || {
        export_backup_job(
            &path,
            taigi_windows_platform::DESKTOP_PLATFORM,
            crate::updates::INSTALLED_VERSION,
            taigi_desktop_storage::replace_file,
        )
    });
}

/// A restore writes the custom dictionary too, so its landing reloads that
/// page, as Add to Custom Dictionary does.
fn import_backup(model: &mut LearningRecordsModel, context: &ComponentContext<SettingsWindow>) {
    if model.job.is_running() {
        return;
    }
    let Some(path) = file_dialog::open(&file_dialog::BACKUP) else {
        return;
    };
    begin_job_then(model, context, true, move || import_backup_job(&path));
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
            RowLabel::plain(strings, StringKey::DictionaryLearningRecords),
            &KINDS,
            current_kind,
            is_enabled,
            |(_, label)| strings.resolve(label).to_owned(),
            |kind| WindowMessage::LearningRecords(Message::ChooseKind(kind)),
            context,
        ),
        choice_row(
            RowLabel::plain(strings, StringKey::DictionaryLearningRecordsOrder),
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
        backup_row(strings, context, is_enabled),
        cards::action_row(
            strings.resolve(StringKey::DictionaryClearLearningRecords),
            strings.resolve(StringKey::CommonDelete),
            true,
            is_enabled,
            context.callback(|()| WindowMessage::LearningRecords(Message::ClearAll)),
        ),
        list_pager::busy_overlay(
            model
                .job
                .is_busy_shown()
                .then_some(StringKey::DesktopProgressWorking),
            strings,
        ),
        count_dialog(model, strings, context),
        add_word_dialog(model, strings, context),
    ))
}

/// The `.taigi` backup as one card: its title and import / export —
/// every store, not only this page's records.
fn backup_row(
    strings: &StringResolver,
    context: &mut ViewContext<SettingsWindow>,
    is_enabled: bool,
) -> View {
    cards::row(
        strings.resolve(StringKey::DictionaryBackupRestore),
        StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(CONTROL_GAP)
            .children((
                Button::new()
                    .is_enabled(is_enabled)
                    .on_click(
                        context
                            .callback(|()| WindowMessage::LearningRecords(Message::ImportBackup)),
                    )
                    .content(strings.resolve(StringKey::DictionaryImportBackup)),
                Button::new()
                    .is_enabled(is_enabled)
                    .on_click(
                        context
                            .callback(|()| WindowMessage::LearningRecords(Message::ExportBackup)),
                    )
                    .content(strings.resolve(StringKey::DictionaryExportBackup)),
            )),
    )
}

/// Add to Custom Dictionary: Custom Dictionary's Add Word dialog over the
/// row's typed form and word, for the user to tweak (hyphens, capitals)
/// before Save files it.
fn add_word_dialog(
    model: &LearningRecordsModel,
    strings: &StringResolver,
    context: &mut ViewContext<SettingsWindow>,
) -> View {
    let Some(adding) = model.adding.as_ref() else {
        return View::empty();
    };
    word_dialog(
        true,
        &adding.roman,
        &adding.hanji,
        strings,
        context.callback(|result| {
            WindowMessage::LearningRecords(Message::AddWordDialogClosed(result))
        }),
        context.callback(|text| {
            WindowMessage::LearningRecords(Message::AddWordFieldChanged(EntryField::Roman, text))
        }),
        context.callback(|text| {
            WindowMessage::LearningRecords(Message::AddWordFieldChanged(EntryField::Hanji, text))
        }),
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
    // Left out on word frequency (its words are dictionary words already);
    // on learned phrases, on for a selected row the engine says can be
    // added — no Hanji greys it out.
    let can_add_selection = model
        .listing
        .selected_row()
        .is_some_and(|row| row.can_add_to_custom_dictionary);
    let add_button = if offers_add_to_custom_dictionary(model.kind) {
        icon_button(
            ADD_GLYPH,
            strings.resolve(StringKey::DictionaryLearningRecordsAddToCustomDictionary),
            is_enabled && can_add_selection,
            context.callback(|()| WindowMessage::LearningRecords(Message::AddToCustomDictionary)),
        )
    } else {
        View::empty()
    };
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
    let count_field = NumberBox::new()
        .minimum(1.0)
        .maximum(MAX_COUNT as f64)
        .value(editing.count)
        .width(DIALOG_FIELD_WIDTH)
        .on_value_changed(
            context.callback(|count| WindowMessage::LearningRecords(Message::CountChanged(count))),
        )
        .slot(
            NumberBoxSlot::Header,
            strings.resolve(StringKey::DictionaryLearningRecordsCount),
        );
    let heading = TextBlock::new()
        .text(format!("{}  {}", record.text, record.tl).trim().to_owned())
        .text_wrapping(TextWrapping::Wrap)
        .width(DIALOG_FIELD_WIDTH);
    let count_label = strings.resolve(StringKey::DictionaryLearningRecordsEditCount);
    let add_label = strings.resolve(StringKey::DictionaryLearningRecordsAddToCustomDictionary);
    // A row the engine says can be added gets two tabs — the count, and its
    // word as the user edits it (hyphens, capitals) before Add files it —
    // one task on screen at a time; the primary button follows the tab.
    // Any other row shows the count alone.
    let (content, primary_text, is_primary_enabled): (View, &str, bool) = match &editing.word {
        Some((roman, hanji)) => {
            let tab = |text: &'static str, is_selected: bool| {
                KeyedView::new(
                    text,
                    SelectorBarItem::new().text(text).is_selected(is_selected),
                )
            };
            let selector = SelectorBar::new()
                .on_selected_text_changed(context.callback(|text| {
                    WindowMessage::LearningRecords(Message::RecordTabChanged(text))
                }))
                .collection_slot(
                    SelectorBarSlot::Items,
                    [
                        tab(count_label, !editing.is_add_tab),
                        tab(add_label, editing.is_add_tab),
                    ],
                );
            let page: View = if editing.is_add_tab {
                let field = |text: &str, field: EntryField, header: StringKey| {
                    TextBox::new()
                        .text(text.to_owned())
                        .width(DIALOG_FIELD_WIDTH)
                        .on_text_changed(context.callback(move |text| {
                            WindowMessage::LearningRecords(Message::RecordWordFieldChanged(
                                field, text,
                            ))
                        }))
                        .slot(TextBoxSlot::Header, strings.resolve(header))
                };
                StackPanel::new().spacing(CONTROL_GAP).children((
                    field(roman, EntryField::Roman, StringKey::DictionaryRomanLabel),
                    field(hanji, EntryField::Hanji, StringKey::DictionaryHanziLabel),
                ))
            } else {
                StackPanel::new()
                    .spacing(CONTROL_GAP)
                    .children((count_field, note))
            };
            let (text, is_enabled) = if editing.is_add_tab {
                (add_label, !roman.trim().is_empty())
            } else {
                (
                    strings.resolve(StringKey::CommonSave),
                    editing.count.and_then(whole_count).is_some(),
                )
            };
            (
                StackPanel::new()
                    .spacing(CONTROL_GAP)
                    .children((heading, selector, page)),
                text,
                is_enabled,
            )
        }
        None => (
            StackPanel::new()
                .spacing(CONTROL_GAP)
                .children((heading, count_field, note)),
            strings.resolve(StringKey::CommonSave),
            editing.count.and_then(whole_count).is_some(),
        ),
    };
    ContentDialog::new()
        .title(if editing.word.is_some() {
            strings.resolve(StringKey::CommonEdit)
        } else {
            count_label
        })
        .primary_button_text(primary_text)
        .close_button_text(strings.resolve(StringKey::CommonCancel))
        .is_primary_button_enabled(is_primary_enabled)
        .is_open(true)
        .on_closed(
            context.callback(|result| {
                WindowMessage::LearningRecords(Message::CountDialogClosed(result))
            }),
        )
        .content(content)
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
