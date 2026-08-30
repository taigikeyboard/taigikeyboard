//! 自訂詞庫: the words the user added themselves. Port of
//! `CustomDictionaryPage.swift`: rows fetched one PAGE at a time (10, so a
//! page never needs a scroller of its own), a filter that reloads once it
//! settles, a `ListView` whose selection drives the add / edit / delete
//! trio, the pager under it, CSV import/export, delete all, and the one
//! destructive verb for the learning records.
//!
//! Every store call — the loads included — runs off the UI thread: the
//! newest load wins by generation, and the writes share the page's one
//! work slot (refused, not queued — a queued delete would name a row the
//! list may no longer show).
//!
//! Named divergence: the Mac's double-click-to-edit and right-click menu
//! become an explicit ✎ button over the selection. Reactor's `ListView`
//! exposes neither a double-click nor a per-item flyout, and a button over
//! the selection is reachable from the keyboard, which neither was.

// 中文: 自訂詞庫頁 — 分頁表格、篩選、新增/編輯/刪除、CSV 匯入匯出、清除學習紀錄;所有資料庫呼叫都在背景執行緒。⚠ 無雙擊/右鍵選單(Reactor 沒有),改用選取 + ✎ 按鈕。

use crate::presentation::PageMessage;
use crate::winui::cards;
use crate::winui::window::{Message as WindowMessage, SettingsWindow};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use taigi_windows_core::settings::keys;
use taigi_windows_core::strings::{StringKey, StringResolver};
use taigi_windows_storage::{
    utc_timestamp_now, CustomDictionaryCSV, CustomDictionaryCSVError, CustomDictionaryRow,
    CustomDictionaryStore, UserDataStores,
};
use windows_reactor::*;

/// `CustomDictionaryPageModel.pageSize`.
const PAGE_SIZE: usize = 10;
/// `reloadWhenFilterSettles`.
const FILTER_SETTLE: Duration = Duration::from_millis(200);
/// How long a job may run before the page says so (`overlayDelay`): a
/// millisecond-long write must not flash a spinner.
const OVERLAY_DELAY: Duration = Duration::from_millis(400);
/// A definite height, not a floor (`Metrics.tableHeight`): the page is
/// sized from the page size, so a short page keeps the controls under the
/// table where they were.
const TABLE_HEIGHT: f64 = 300.0;
const DIALOG_FIELD_WIDTH: f64 = 320.0;
/// Between the two columns, and between the small controls under the list.
const CONTROL_GAP: f64 = 8.0;
const TABLE_COLUMN_GAP: f64 = 12.0;
/// The header sits over the list's own item inset.
const TABLE_HEADER_INSET: f64 = 12.0;
const TABLE_HEADER_GAP: f64 = 8.0;
const OVERLAY_RING_SIZE: f64 = 20.0;
/// WinUI's secondary text, as opacity, so it follows the theme.
const SECONDARY_OPACITY: f64 = 0.65;

/// Which field of the entry dialog changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryField {
    Roman,
    Hanzi,
}

#[derive(Clone)]
pub enum Message {
    FilterChanged(String),
    /// The filter stopped changing: reload from the first page.
    FilterSettled(u64),
    /// A page the user stepped to, already clamped by the caller.
    ShowPage(usize),
    Loaded(u64, Box<LoadOutcome>),
    Select(Option<usize>),
    Add,
    Edit,
    Delete,
    EntryFieldChanged(EntryField, String),
    EntryDialogClosed(ContentDialogResult),
    Export,
    Import,
    DeleteAll,
    ClearLearningRecords,
    JobFinished(u64, Box<JobOutcome>),
    /// The job at this generation has run long enough to say so.
    ShowBusy(u64),
}

/// What a load hands back: the page it fetched, or why it could not.
#[derive(Clone)]
pub enum LoadOutcome {
    Loaded {
        page: usize,
        rows: Vec<CustomDictionaryRow>,
        match_count: usize,
        total_count: usize,
    },
    Failed(String),
}

/// What a write hands back.
#[derive(Clone)]
pub struct JobOutcome {
    message: Option<PageMessage>,
    /// Whether the list changed and must be reloaded.
    is_reload_wanted: bool,
}

/// The row being added or edited in the dialog.
struct EditingRow {
    original: CustomDictionaryRow,
    roman: String,
    hanzi: String,
}

#[derive(Default)]
pub struct CustomDictionaryModel {
    rows: Vec<CustomDictionaryRow>,
    /// Every entry, for the section header — what the dictionary HOLDS,
    /// which is not what the current filter matches.
    total_count: usize,
    /// How many entries the current filter matches; what the pager divides.
    match_count: usize,
    /// Which page is on screen, zero-based.
    page: usize,
    filter: String,
    /// Which load the rows on screen came from: a load started under an
    /// older filter can come back after a newer one has, and would put rows
    /// on screen that do not match the box. The newest wins by number.
    load_generation: u64,
    is_first_load_requested: bool,
    /// The row the list has selected, by ID — never by index, which moves
    /// under a reload.
    selected_id: Option<String>,
    editing: Option<EditingRow>,
    /// The page's one work slot: `Some` while a write runs. A file
    /// dialog does not need it — it is modal and runs on the UI thread,
    /// so no second command can arrive while it is up (the egui page had
    /// to reserve the slot across it because its dialog did not block the
    /// frame loop).
    job_generation: Option<u64>,
    next_job_generation: u64,
    /// What the running job is called, for the overlay.
    job_label: Option<StringKey>,
    /// Whether the running job has lasted long enough to say so.
    is_busy_shown: bool,
}

impl CustomDictionaryModel {
    fn page_count_for(match_count: usize) -> usize {
        match_count.div_ceil(PAGE_SIZE).max(1)
    }

    fn page_count(&self) -> usize {
        Self::page_count_for(self.match_count)
    }

    fn is_working(&self) -> bool {
        self.job_generation.is_some()
    }

    fn selected_row(&self) -> Option<&CustomDictionaryRow> {
        let id = self.selected_id.as_deref()?;
        self.rows.iter().find(|row| row.id == id)
    }

    fn selected_index(&self) -> Option<usize> {
        let id = self.selected_id.as_deref()?;
        self.rows.iter().position(|row| row.id == id)
    }

    /// What the dictionary holds — and, while a filter narrows it, how much
    /// of that the filter matches.
    fn count_label(&self) -> String {
        if self.filter.trim().is_empty() {
            self.total_count.to_string()
        } else {
            format!("{} / {}", self.match_count, self.total_count)
        }
    }
}

/// What the page needs from the window to run a job. Disjoint fields, so
/// the page can write settings and raise an alert while it owns its model.
pub struct PageEnvironment<'a> {
    pub stores: &'a UserDataStores,
    pub message: &'a mut Option<PageMessage>,
}

/// Starts the first load, once, when the page is first shown.
pub fn ensure_loaded(
    model: &mut CustomDictionaryModel,
    stores: &UserDataStores,
    context: &ComponentContext<SettingsWindow>,
) {
    if model.is_first_load_requested {
        return;
    }
    model.is_first_load_requested = true;
    load(model, stores, context);
}

pub fn update(
    model: &mut CustomDictionaryModel,
    message: Message,
    environment: PageEnvironment<'_>,
    context: &ComponentContext<SettingsWindow>,
) {
    let PageEnvironment {
        stores,
        message: alert,
    } = environment;
    match message {
        Message::FilterChanged(filter) => {
            if filter == model.filter {
                return;
            }
            model.filter = filter;
            // The reload waits for the box to settle, so a word typed
            // letter by letter is one query, not six. A wait the runtime
            // will not start applies the filter AT ONCE instead — a
            // filter that never arrives would leave the list showing
            // rows the box no longer describes, and "applied without
            // waiting" is not a failure worth a banner.
            model.load_generation = model.load_generation.wrapping_add(1);
            let generation = model.load_generation;
            _ = context.spawn_background_with_rejection(
                move |_| {
                    std::thread::sleep(FILTER_SETTLE);
                    WindowMessage::CustomDictionary(Message::FilterSettled(generation))
                },
                WindowMessage::CustomDictionary(Message::FilterSettled(generation)),
            );
        }
        Message::FilterSettled(generation) => {
            if generation != model.load_generation {
                return;
            }
            model.page = 0;
            load(model, stores, context);
        }
        Message::ShowPage(page) => {
            model.page = page;
            load(model, stores, context);
        }
        Message::Loaded(generation, outcome) => {
            if generation != model.load_generation {
                return;
            }
            match *outcome {
                LoadOutcome::Loaded {
                    page,
                    rows,
                    match_count,
                    total_count,
                } => {
                    model.page = page;
                    model.rows = rows;
                    model.match_count = match_count;
                    model.total_count = total_count;
                    // A selection the new page does not hold is no
                    // selection: the ✎ and − buttons must not act on a row
                    // that is not on screen.
                    if model.selected_index().is_none() {
                        model.selected_id = None;
                    }
                }
                // Not an empty list: "empty" and "could not be read" look
                // the same on screen, and only one is worth doing
                // something about. The rows already shown stay.
                LoadOutcome::Failed(error) => {
                    *alert = Some(PageMessage::failure(
                        StringKey::DesktopCustomDictReadFailed,
                        error,
                    ));
                }
            }
        }
        Message::Select(index) => {
            model.selected_id = index
                .and_then(|index| model.rows.get(index))
                .map(|row| row.id.clone());
        }
        Message::Add => {
            model.editing = Some(EditingRow {
                original: CustomDictionaryRow::new("", ""),
                roman: String::new(),
                hanzi: String::new(),
            });
        }
        Message::Edit => {
            if let Some(row) = model.selected_row() {
                model.editing = Some(EditingRow {
                    roman: row.roman.clone(),
                    hanzi: row.hanzi.clone(),
                    original: row.clone(),
                });
            }
        }
        Message::Delete => {
            let Some(id) = model.selected_id.clone() else {
                return;
            };
            let store = Arc::clone(&stores.custom_dictionary);
            write(
                model,
                context,
                StringKey::DesktopProgressDeleting,
                move || {
                    store
                        .delete(&id)
                        .map(|_| ())
                        .map_err(|error| error.to_string())
                },
            );
        }
        Message::EntryFieldChanged(field, text) => {
            if let Some(editing) = model.editing.as_mut() {
                match field {
                    EntryField::Roman => editing.roman = text,
                    EntryField::Hanzi => editing.hanzi = text,
                }
            }
        }
        Message::EntryDialogClosed(result) => {
            let Some(editing) = model.editing.take() else {
                return;
            };
            if result != ContentDialogResult::Primary {
                return;
            }
            // A romanization is what the entry is found by; without one
            // there is nothing to store it under. The dialog's primary
            // button is disabled in that case, so this is the belt.
            if editing.roman.trim().is_empty() {
                return;
            }
            // The ID is the original's: an edit is an edit, not a new
            // entry that happens to replace one.
            let mut row = editing.original;
            row.roman = editing.roman.trim().to_owned();
            row.hanzi = editing.hanzi.trim().to_owned();
            row.updated_at = utc_timestamp_now();
            let store = Arc::clone(&stores.custom_dictionary);
            write(
                model,
                context,
                StringKey::DesktopProgressSaving,
                move || store.upsert(&row).map_err(|error| error.to_string()),
            );
        }
        Message::Export => export(model, stores, context),
        Message::Import => import(model, stores, context),
        Message::DeleteAll => {
            let store = Arc::clone(&stores.custom_dictionary);
            write(
                model,
                context,
                StringKey::DesktopProgressDeleting,
                move || {
                    store
                        .delete_all()
                        .map(|_| ())
                        .map_err(|error| error.to_string())
                },
            );
        }
        Message::ClearLearningRecords => clear_learning_records(model, stores, context),
        Message::JobFinished(generation, outcome) => {
            if model.job_generation != Some(generation) {
                return;
            }
            model.job_generation = None;
            model.job_label = None;
            model.is_busy_shown = false;
            let JobOutcome {
                message,
                is_reload_wanted,
            } = *outcome;
            if message.is_some() {
                *alert = message;
            }
            if is_reload_wanted {
                load(model, stores, context);
            }
        }
        Message::ShowBusy(generation) => {
            if model.job_generation == Some(generation) {
                model.is_busy_shown = true;
            }
        }
    }
}

/// Starts a load of the page on screen, pulling it back inside the list if
/// the list shrank under it — a delete on the last page, or a filter that
/// now matches less. A load has no overlay: the rows already on screen
/// stay put while it runs.
fn load(
    model: &mut CustomDictionaryModel,
    stores: &UserDataStores,
    context: &ComponentContext<SettingsWindow>,
) {
    model.load_generation = model.load_generation.wrapping_add(1);
    let generation = model.load_generation;
    let filter = model.filter.trim().to_owned();
    let wanted_page = model.page;
    let store = Arc::clone(&stores.custom_dictionary);
    _ = context.spawn_background_with_rejection(
        move |_| {
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let match_count = store.count_matching(&filter)?;
                let page = wanted_page.min(CustomDictionaryModel::page_count_for(match_count) - 1);
                let rows = store.rows(&filter, PAGE_SIZE, page * PAGE_SIZE)?;
                // With no filter the two counts ask the same question.
                let total_count = if filter.is_empty() {
                    match_count
                } else {
                    store.count()?
                };
                Ok::<_, taigi_windows_storage::CustomDictionaryError>(LoadOutcome::Loaded {
                    page,
                    rows,
                    match_count,
                    total_count,
                })
            }));
            let outcome = match outcome {
                Ok(Ok(loaded)) => loaded,
                Ok(Err(error)) => LoadOutcome::Failed(error.to_string()),
                Err(_) => LoadOutcome::Failed("the load did not finish".to_owned()),
            };
            WindowMessage::CustomDictionary(Message::Loaded(generation, Box::new(outcome)))
        },
        // A thread the runtime would not start is a failure the user sees,
        // never a load that quietly never lands.
        WindowMessage::CustomDictionary(Message::Loaded(
            generation,
            Box::new(LoadOutcome::Failed(
                "the load could not be started".to_owned(),
            )),
        )),
    );
}

/// Takes the page's one work slot for `job`, or does nothing because
/// something else holds it (the macOS model: refused, not queued).
fn begin_job(
    model: &mut CustomDictionaryModel,
    context: &ComponentContext<SettingsWindow>,
    label: StringKey,
    job: impl FnOnce() -> JobOutcome + Send + 'static,
) {
    if model.is_working() {
        return;
    }
    model.next_job_generation = model.next_job_generation.wrapping_add(1);
    let generation = model.next_job_generation;
    model.job_generation = Some(generation);
    model.job_label = Some(label);
    model.is_busy_shown = false;
    _ = context.spawn_background_with_rejection(
        move |_| {
            // A panicking store call must not leave the slot held for the
            // life of the window.
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(job))
                .unwrap_or_else(|_| JobOutcome {
                    message: Some(PageMessage::failure(
                        StringKey::DesktopCustomDictWriteFailed,
                        "the operation did not finish",
                    )),
                    is_reload_wanted: true,
                });
            WindowMessage::CustomDictionary(Message::JobFinished(generation, Box::new(outcome)))
        },
        WindowMessage::CustomDictionary(Message::JobFinished(
            generation,
            Box::new(JobOutcome {
                message: Some(PageMessage::failure(
                    StringKey::DesktopCustomDictWriteFailed,
                    "the operation could not be started",
                )),
                is_reload_wanted: false,
            }),
        )),
    );
    // The overlay waits, so a millisecond-long write does not flash it.
    _ = context.spawn_background(move |_| {
        std::thread::sleep(OVERLAY_DELAY);
        WindowMessage::CustomDictionary(Message::ShowBusy(generation))
    });
}

/// A write that answers with nothing but its failure, and asks for a
/// reload either way.
fn write(
    model: &mut CustomDictionaryModel,
    context: &ComponentContext<SettingsWindow>,
    label: StringKey,
    body: impl FnOnce() -> Result<(), String> + Send + 'static,
) {
    begin_job(model, context, label, move || JobOutcome {
        message: body()
            .err()
            .map(|error| PageMessage::failure(StringKey::DesktopCustomDictWriteFailed, error)),
        is_reload_wanted: true,
    });
}

fn export(
    model: &mut CustomDictionaryModel,
    stores: &UserDataStores,
    context: &ComponentContext<SettingsWindow>,
) {
    if model.is_working() {
        return;
    }
    let suggested = format!(
        "taigi_custom_dictionary_{}.csv",
        taigi_windows_platform::local_date()
    );
    let Some(path) = crate::winui::file_dialog::save(&suggested) else {
        return;
    };
    // The WHOLE dictionary, not the page or the filter's matches.
    let store = Arc::clone(&stores.custom_dictionary);
    begin_job(
        model,
        context,
        StringKey::DesktopProgressExporting,
        move || {
            let outcome = store
                .all_rows()
                .map_err(|error| error.to_string())
                .and_then(|rows| write_atomically(&path, CustomDictionaryCSV::encode(&rows)));
            JobOutcome {
                message: outcome
                    .err()
                    .map(|error| PageMessage::failure(StringKey::CommonExportFailed, error)),
                is_reload_wanted: false,
            }
        },
    );
}

fn import(
    model: &mut CustomDictionaryModel,
    stores: &UserDataStores,
    context: &ComponentContext<SettingsWindow>,
) {
    if model.is_working() {
        return;
    }
    let Some(path) = crate::winui::file_dialog::open() else {
        return;
    };
    let store = Arc::clone(&stores.custom_dictionary);
    begin_job(
        model,
        context,
        StringKey::DesktopProgressImporting,
        move || {
            let decoded =
                CustomDictionaryCSV::decode_file(&path, CustomDictionaryStore::MAX_ENTRIES);
            let message = match decoded {
                Err(CustomDictionaryCSVError::NotUtf8) => PageMessage::NotUtf8,
                Err(error) => PageMessage::failure(StringKey::CommonImportFailed, error),
                Ok(rows) => match store.batch_import(&rows) {
                    Ok(result) => PageMessage::Imported {
                        imported: result.imported,
                        skipped: result.skipped,
                    },
                    Err(error) => PageMessage::failure(StringKey::CommonImportFailed, error),
                },
            };
            JobOutcome {
                message: Some(message),
                is_reload_wanted: true,
            }
        },
    );
}

/// Deletes both learning tables — two calls, two files, no transaction
/// that could span them; the second is attempted even when the first
/// fails, and the alert reports rather than claims.
fn clear_learning_records(
    model: &mut CustomDictionaryModel,
    stores: &UserDataStores,
    context: &ComponentContext<SettingsWindow>,
) {
    let frequency = Arc::clone(&stores.frequency);
    let association = Arc::clone(&stores.association);
    begin_job(
        model,
        context,
        StringKey::DesktopProgressDeleting,
        move || {
            let mut failures = Vec::new();
            if let Err(error) = frequency.delete_all() {
                failures.push(format!("user_frequency: {error}"));
            }
            if let Err(error) = association.delete_all() {
                failures.push(format!("user_association: {error}"));
            }
            let message = if failures.is_empty() {
                PageMessage::Done(StringKey::DesktopClearLearningRecordsDone)
            } else {
                PageMessage::Failure {
                    title: StringKey::DesktopClearLearningRecordsFailed,
                    detail: failures.join("\n"),
                }
            };
            JobOutcome {
                message: Some(message),
                // The custom dictionary is untouched by this.
                is_reload_wanted: false,
            }
        },
    );
}

/// `Data.write(to:options:.atomic)`: the file appears whole or not at all,
/// and a failed export never damages the one it would have replaced.
fn write_atomically(path: &PathBuf, contents: String) -> Result<(), String> {
    let temporary = path.with_extension(format!("csv.{}.tmp", std::process::id()));
    std::fs::write(&temporary, contents).map_err(|error| error.to_string())?;
    std::fs::rename(&temporary, path).map_err(|error| {
        // The rename's error is the one worth reporting; a temporary file
        // that will not go is not what the user asked about.
        std::fs::remove_file(&temporary).ok();
        error.to_string()
    })
}

/// Segoe Fluent Icons: Add, Edit, Remove, and the two chevrons.
const ADD_GLYPH: &str = "\u{E710}";
const EDIT_GLYPH: &str = "\u{E70F}";
const REMOVE_GLYPH: &str = "\u{E738}";
const PREVIOUS_GLYPH: &str = "\u{E76B}";
const NEXT_GLYPH: &str = "\u{E76C}";

pub fn view(
    window: &SettingsWindow,
    strings: &StringResolver,
    context: &mut ViewContext<SettingsWindow>,
) -> View {
    let model = window.custom_dictionary();
    let enabled_row = cards::switch_row(
        strings.resolve(StringKey::DictionaryCustomDictEnabled),
        window.document().bool(&keys::IS_CUSTOM_DICT_ENABLED),
        true,
        context.callback(|is_on| WindowMessage::SetSwitch(keys::IS_CUSTOM_DICT_ENABLED, is_on)),
    );
    // No user-data directory: the stores never opened, and the banner at
    // the top of the window says so — nothing to list, nothing to write.
    if window.is_read_only() {
        return enabled_row;
    }
    // Mutual exclusion is the slot's; the greyed look waits the same
    // 400 ms as the overlay so a millisecond-long write does not flash it.
    let is_enabled = !model.is_busy_shown;
    View::fragment((
        enabled_row,
        cards::section_title_with_count(
            strings.resolve(StringKey::DesktopEntriesSection),
            &model.count_label(),
        ),
        TextBox::new()
            .text(model.filter.clone())
            .is_enabled(is_enabled)
            .placeholder_text(strings.resolve(StringKey::DictionarySearchPlaceholder))
            .on_text_changed(
                context
                    .callback(|text| WindowMessage::CustomDictionary(Message::FilterChanged(text))),
            ),
        entry_table(model, strings, context, is_enabled),
        cards::section_gap(),
        csv_row(strings, context, is_enabled),
        cards::action_enabled(
            strings.resolve(StringKey::DictionaryDeleteAll),
            true,
            is_enabled,
            context.callback(|()| WindowMessage::CustomDictionary(Message::DeleteAll)),
        ),
        cards::section_gap(),
        cards::action_enabled(
            strings.resolve(StringKey::DesktopClearLearningRecords),
            true,
            is_enabled,
            context.callback(|()| WindowMessage::CustomDictionary(Message::ClearLearningRecords)),
        ),
        busy_overlay(model, strings),
        entry_dialog(model, strings, context),
    ))
}

/// The entries in one card: the list, then the add / edit / delete trio
/// and the pager under it — as a list sits in one `ListView` surface on
/// Windows.
fn entry_table(
    model: &CustomDictionaryModel,
    strings: &StringResolver,
    context: &mut ViewContext<SettingsWindow>,
    is_enabled: bool,
) -> View {
    let items = model
        .rows
        .iter()
        .map(|row| {
            (
                row.id.clone(),
                ListViewItem::new().tag(row.id.clone()).content(
                    Grid::new()
                        .columns([GridLength::Star(1.0), GridLength::Star(1.0)])
                        .column_spacing(TABLE_COLUMN_GAP)
                        .children((
                            TextBlock::new()
                                .text(row.roman.clone())
                                .opacity(SECONDARY_OPACITY)
                                .grid_column(0),
                            TextBlock::new().text(row.hanzi.clone()).grid_column(1),
                        )),
                ),
            )
        })
        .collect::<Vec<_>>();
    let has_selection = model.selected_row().is_some();
    // The list itself stays live while a job runs: selecting a row writes
    // nothing, and the verbs over it are what a job turns off.
    let list = ListView::new()
        .selection_mode(ListViewSelectionMode::Single)
        .selected_index(model.selected_index())
        .on_selection_changed(
            context.callback(|index| WindowMessage::CustomDictionary(Message::Select(index))),
        )
        .height(TABLE_HEIGHT)
        .collection_slot(ListViewSlot::Items, items);
    cards::frame(View::fragment((
        // The column names, above the list rather than inside it: a
        // `ListView` has no header of its own.
        Grid::new()
            .columns([GridLength::Star(1.0), GridLength::Star(1.0)])
            .column_spacing(TABLE_COLUMN_GAP)
            .margin(Thickness::new(
                TABLE_HEADER_INSET,
                0.0,
                0.0,
                TABLE_HEADER_GAP,
            ))
            .children((
                TextBlock::new()
                    .text(strings.resolve(StringKey::DictionaryRomanLabel))
                    .font_weight(FontWeight::SEMI_BOLD)
                    .grid_column(0),
                TextBlock::new()
                    .text(strings.resolve(StringKey::DictionaryHanziLabel))
                    .font_weight(FontWeight::SEMI_BOLD)
                    .grid_column(1),
            )),
        list,
        table_controls(model, strings, context, is_enabled, has_selection),
    )))
}

/// The add / edit / delete trio and the pager (`entryTableControls`).
fn table_controls(
    model: &CustomDictionaryModel,
    strings: &StringResolver,
    context: &mut ViewContext<SettingsWindow>,
    is_enabled: bool,
    has_selection: bool,
) -> View {
    let can_forward = is_enabled && model.page + 1 < model.page_count();
    let can_backward = is_enabled && model.page > 0;
    let page = model.page;
    Grid::new()
        .columns([GridLength::Auto, GridLength::STAR, GridLength::Auto])
        .margin(Thickness::new(0.0, TABLE_HEADER_GAP, 0.0, 0.0))
        .children((
            StackPanel::new()
                .orientation(Orientation::Horizontal)
                .spacing(CONTROL_GAP)
                .grid_column(0)
                .children((
                    icon_button(
                        ADD_GLYPH,
                        strings.resolve(StringKey::DictionaryAddEntry),
                        is_enabled,
                        context.callback(|()| WindowMessage::CustomDictionary(Message::Add)),
                    ),
                    icon_button(
                        EDIT_GLYPH,
                        strings.resolve(StringKey::DictionaryEditEntry),
                        is_enabled && has_selection,
                        context.callback(|()| WindowMessage::CustomDictionary(Message::Edit)),
                    ),
                    icon_button(
                        REMOVE_GLYPH,
                        strings.resolve(StringKey::CommonDelete),
                        is_enabled && has_selection,
                        context.callback(|()| WindowMessage::CustomDictionary(Message::Delete)),
                    ),
                )),
            // Digits only: the pager needs no wording in five languages.
            TextBlock::new()
                .text(format!("{} / {}", model.page + 1, model.page_count()))
                .opacity(SECONDARY_OPACITY)
                .horizontal_alignment(HorizontalAlignment::Right)
                .vertical_alignment(VerticalAlignment::Center)
                .margin(Thickness::xy(CONTROL_GAP, 0.0))
                .grid_column(1),
            StackPanel::new()
                .orientation(Orientation::Horizontal)
                .spacing(CONTROL_GAP)
                .grid_column(2)
                .children((
                    icon_button(
                        PREVIOUS_GLYPH,
                        strings.resolve(StringKey::DesktopActionPageBackward),
                        can_backward,
                        context.callback(move |()| {
                            WindowMessage::CustomDictionary(Message::ShowPage(
                                page.saturating_sub(1),
                            ))
                        }),
                    ),
                    icon_button(
                        NEXT_GLYPH,
                        strings.resolve(StringKey::DesktopActionPageForward),
                        can_forward,
                        context.callback(move |()| {
                            WindowMessage::CustomDictionary(Message::ShowPage(page + 1))
                        }),
                    ),
                )),
        ))
}

fn icon_button(glyph: &str, tooltip: &str, is_enabled: bool, on_click: Callback<()>) -> View {
    Button::new()
        .is_enabled(is_enabled)
        .on_click(on_click)
        .content(FontIcon::new().glyph(glyph))
        .tooltip(tooltip)
}

/// Import and export, side by side under the table.
fn csv_row(
    strings: &StringResolver,
    context: &mut ViewContext<SettingsWindow>,
    is_enabled: bool,
) -> View {
    StackPanel::new()
        .orientation(Orientation::Horizontal)
        .spacing(CONTROL_GAP)
        .horizontal_alignment(HorizontalAlignment::Right)
        .margin(Thickness::new(0.0, 0.0, 0.0, CONTROL_GAP))
        .children((
            Button::new()
                .is_enabled(is_enabled)
                .on_click(context.callback(|()| WindowMessage::CustomDictionary(Message::Import)))
                .content(strings.resolve(StringKey::DictionaryImportCSV)),
            Button::new()
                .is_enabled(is_enabled)
                .on_click(context.callback(|()| WindowMessage::CustomDictionary(Message::Export)))
                .content(strings.resolve(StringKey::DictionaryExportCSV)),
        ))
}

/// The job's name over a ring, once it has run long enough to say so.
fn busy_overlay(model: &CustomDictionaryModel, strings: &StringResolver) -> View {
    let Some(label) = model.job_label.filter(|_| model.is_busy_shown) else {
        return View::empty();
    };
    cards::frame(
        StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(CONTROL_GAP)
            .horizontal_alignment(HorizontalAlignment::Center)
            .children((
                ProgressRing::new()
                    .is_active(true)
                    .width(OVERLAY_RING_SIZE)
                    .height(OVERLAY_RING_SIZE),
                TextBlock::new()
                    .text(strings.resolve(label))
                    .vertical_alignment(VerticalAlignment::Center),
            )),
    )
}

/// Add or edit one entry (`CustomDictionaryEntrySheet`). Enter and Escape
/// are the dialog's own: `ContentDialog` gives the primary and close
/// buttons those keys, which is what the egui sheet hand-rolled.
fn entry_dialog(
    model: &CustomDictionaryModel,
    strings: &StringResolver,
    context: &mut ViewContext<SettingsWindow>,
) -> View {
    let Some(editing) = model.editing.as_ref() else {
        return View::empty();
    };
    let is_new = editing.original.roman.is_empty();
    ContentDialog::new()
        .title(strings.resolve(if is_new {
            StringKey::DictionaryAddEntry
        } else {
            StringKey::DictionaryEditEntry
        }))
        .primary_button_text(strings.resolve(StringKey::DictionarySave))
        .close_button_text(strings.resolve(StringKey::CommonCancel))
        // A romanization is what the entry is found by; without one there
        // is nothing to store it under.
        .is_primary_button_enabled(!editing.roman.trim().is_empty())
        .is_open(true)
        .on_closed(
            context.callback(|result| {
                WindowMessage::CustomDictionary(Message::EntryDialogClosed(result))
            }),
        )
        .content(
            StackPanel::new().spacing(CONTROL_GAP).children((
                TextBox::new()
                    .text(editing.roman.clone())
                    .width(DIALOG_FIELD_WIDTH)
                    .on_text_changed(context.callback(|text| {
                        WindowMessage::CustomDictionary(Message::EntryFieldChanged(
                            EntryField::Roman,
                            text,
                        ))
                    }))
                    .slot(
                        TextBoxSlot::Header,
                        strings.resolve(StringKey::DictionaryRomanLabel),
                    ),
                TextBox::new()
                    .text(editing.hanzi.clone())
                    .width(DIALOG_FIELD_WIDTH)
                    .on_text_changed(context.callback(|text| {
                        WindowMessage::CustomDictionary(Message::EntryFieldChanged(
                            EntryField::Hanzi,
                            text,
                        ))
                    }))
                    .slot(
                        TextBoxSlot::Header,
                        strings.resolve(StringKey::DictionaryHanziLabel),
                    ),
            )),
        )
}
