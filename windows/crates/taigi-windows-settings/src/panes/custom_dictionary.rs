//! 自訂詞庫: the words the user added themselves. Port of
//! `CustomDictionaryPage.swift`: rows fetched one PAGE at a time (10, so a
//! page never needs a scroller of its own), a filter that reloads once it
//! settles, a table whose click selects and double-click edits, the +/−
//! pair and the pager under it, CSV import/export, delete all, and the one
//! destructive verb for the learning records. Every store call — the loads
//! included — runs off the UI thread (`work::PendingWork`): the newest load
//! wins by generation, the writes share the page's one work slot.

// 中文: 自訂詞庫頁 — 分頁表格、篩選、新增/編輯/刪除、CSV 匯入匯出、清除學習紀錄;所有資料庫呼叫都在背景執行緒。

use super::section_gap;
use crate::app::SettingsApp;
use crate::presentation::PageMessage;
use crate::widgets::settings_card;
use crate::work::PendingWork;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use taigi_windows_core::settings::keys;
use taigi_windows_core::strings::{StringKey, StringResolver};
use taigi_windows_storage::{
    utc_timestamp_now, CustomDictionaryCSV, CustomDictionaryCSVError, CustomDictionaryRow,
    CustomDictionaryStore, UserDataStores,
};

/// `CustomDictionaryPageModel.pageSize`.
const PAGE_SIZE: usize = 10;
/// `reloadWhenFilterSettles`.
const FILTER_SETTLE: Duration = Duration::from_millis(200);
const TABLE_ROW_HEIGHT: f32 = 24.0;
const TABLE_HEADER_HEIGHT: f32 = 28.0;
/// A definite height, not a floor (`Metrics.tableHeight`): the page is
/// sized from the page size, so a short page keeps the controls under the
/// table where they were.
const TABLE_HEIGHT: f32 = TABLE_HEADER_HEIGHT + PAGE_SIZE as f32 * TABLE_ROW_HEIGHT;
const EMPTY_STATE_SYMBOL_SIZE: f32 = 34.0;
/// The card's padding around the table: tighter than a setting's, so the
/// columns keep their width.
const TABLE_CARD_PADDING: i8 = 8;
const ENTRY_SHEET_WIDTH: f32 = 360.0;

/// The row being added or edited in the sheet.
struct EditingRow {
    original: CustomDictionaryRow,
    roman: String,
    hanzi: String,
}

/// What a write job hands back.
struct WriteResult {
    message: Option<PageMessage>,
    /// Whether the list changed and must be reloaded.
    reload: bool,
}

/// What a load job hands back: the page it fetched, or why it could not.
struct LoadResult {
    generation: u64,
    page: usize,
    outcome: Result<LoadedPage, String>,
}

struct LoadedPage {
    rows: Vec<CustomDictionaryRow>,
    match_count: usize,
    total_count: usize,
}

#[derive(Default)]
pub struct CustomDictionaryPageModel {
    rows: Vec<CustomDictionaryRow>,
    /// Every entry, for the section header — what the dictionary HOLDS,
    /// which is not what the current filter matches.
    total_count: usize,
    /// How many entries the current filter matches; what the pager divides.
    match_count: usize,
    /// Which page is on screen, zero-based.
    page: usize,
    filter: String,
    /// When the filter last changed: the reload waits for it to settle.
    filter_changed_at: Option<Instant>,
    /// Which load the rows on screen came from: a load started under an
    /// older filter can come back after a newer one has, and would put rows
    /// on screen that do not match the box. The newest wins by number.
    load_generation: u64,
    load_work: Option<PendingWork<LoadResult>>,
    is_load_requested: bool,
    selected_id: Option<String>,
    /// A delete asked for by the table's menu or the − button, run once the
    /// frame's borrows are over.
    pending_delete: Option<String>,
    editing: Option<EditingRow>,
    /// The page's one work slot for writes; `is_slot_reserved` holds it
    /// across a file dialog, which opens before the job exists.
    work: Option<PendingWork<WriteResult>>,
    is_slot_reserved: bool,
    pub message: Option<PageMessage>,
}

impl CustomDictionaryPageModel {
    fn page_count_for(match_count: usize) -> usize {
        match_count.div_ceil(PAGE_SIZE).max(1)
    }

    fn page_count(&self) -> usize {
        Self::page_count_for(self.match_count)
    }

    fn is_working(&self) -> bool {
        self.work.is_some() || self.is_slot_reserved
    }

    /// Starts a load of the page on screen (off the UI thread), pulling it
    /// back inside the list if the list shrank under it — a delete on the
    /// last page, or a filter that now matches less.
    fn request_load(&mut self, store: &Arc<CustomDictionaryStore>) {
        self.load_generation += 1;
        let generation = self.load_generation;
        let filter = self.filter.clone();
        let wanted_page = self.page;
        let store = Arc::clone(store);
        self.load_work = Some(PendingWork::spawn_quiet(move || {
            let outcome = (|| {
                let match_count = store.count_matching(&filter)?;
                let page = wanted_page.min(Self::page_count_for(match_count) - 1);
                let rows = store.rows(&filter, PAGE_SIZE, page * PAGE_SIZE)?;
                // With no filter the two counts ask the same question.
                let total_count = if filter.trim().is_empty() {
                    match_count
                } else {
                    store.count()?
                };
                Ok((
                    page,
                    LoadedPage {
                        rows,
                        match_count,
                        total_count,
                    },
                ))
            })()
            .map_err(|error: taigi_windows_storage::CustomDictionaryError| error.to_string());
            match outcome {
                Ok((page, loaded)) => LoadResult {
                    generation,
                    page,
                    outcome: Ok(loaded),
                },
                Err(error) => LoadResult {
                    generation,
                    page: wanted_page,
                    outcome: Err(error),
                },
            }
        }));
    }

    fn request_first_page(&mut self, store: &Arc<CustomDictionaryStore>) {
        self.page = 0;
        self.request_load(store);
    }

    /// Collects a finished load, if it is still the newest one asked for.
    fn poll_load(&mut self) {
        let Some(finished) = self.load_work.as_ref().and_then(PendingWork::poll) else {
            return;
        };
        self.load_work = None;
        let Some(result) = finished else {
            self.message = Some(PageMessage::failure(
                StringKey::DesktopCustomDictReadFailed,
                "the load did not finish",
            ));
            return;
        };
        if result.generation != self.load_generation {
            return;
        }
        match result.outcome {
            Ok(loaded) => {
                self.page = result.page;
                self.rows = loaded.rows;
                self.match_count = loaded.match_count;
                self.total_count = loaded.total_count;
            }
            // Not an empty list: "empty" and "could not be read" look the
            // same on screen, and only one is worth doing something about.
            Err(error) => {
                self.message = Some(PageMessage::failure(
                    StringKey::DesktopCustomDictReadFailed,
                    error,
                ))
            }
        }
    }

    /// Collects a finished write: its message, and the reload it asked for.
    /// A job whose thread died is a failure the user sees, not a quiet end.
    fn poll_write(&mut self, store: &Arc<CustomDictionaryStore>) {
        let Some(finished) = self.work.as_ref().and_then(PendingWork::poll) else {
            return;
        };
        self.work = None;
        match finished {
            Some(result) => {
                if result.message.is_some() {
                    self.message = result.message;
                }
                if result.reload {
                    self.request_load(store);
                }
            }
            None => {
                self.message = Some(PageMessage::failure(
                    StringKey::DesktopCustomDictWriteFailed,
                    "the operation did not finish",
                ));
                self.request_load(store);
            }
        }
    }

    /// Takes the page's one work slot for `job`, or answers false because
    /// something else holds it (the macOS model: refused, not queued — a
    /// queued delete would name a row the list may no longer show).
    fn begin_work(
        &mut self,
        label: StringKey,
        job: impl FnOnce() -> WriteResult + Send + 'static,
    ) -> bool {
        if self.work.is_some() {
            return false;
        }
        self.is_slot_reserved = false;
        self.work = Some(PendingWork::spawn(label, job));
        true
    }

    fn write(
        &mut self,
        label: StringKey,
        store: Arc<CustomDictionaryStore>,
        body: impl FnOnce(&CustomDictionaryStore) -> Result<(), String> + Send + 'static,
    ) {
        self.begin_work(label, move || WriteResult {
            message: body(&store)
                .err()
                .map(|error| PageMessage::failure(StringKey::DesktopCustomDictWriteFailed, error)),
            reload: true,
        });
    }

    /// The slot is taken BEFORE the file panel, not after it: the moment
    /// the panel closes the job is still to start, and that is exactly the
    /// window a second export could start in.
    fn reserve_slot(&mut self) -> bool {
        if self.is_working() {
            return false;
        }
        self.is_slot_reserved = true;
        true
    }

    fn export_csv(&mut self, store: Arc<CustomDictionaryStore>, frame: &eframe::Frame) {
        if !self.reserve_slot() {
            return;
        }
        let suggested = format!(
            "taigi_custom_dictionary_{}.csv",
            taigi_windows_platform::local_date()
        );
        let chosen = rfd::FileDialog::new()
            .set_parent(frame)
            .set_file_name(suggested)
            .add_filter("CSV", &["csv"])
            .save_file();
        let Some(path) = chosen else {
            self.is_slot_reserved = false;
            return;
        };
        self.begin_work(StringKey::DesktopProgressExporting, move || {
            let outcome = store
                .all_rows()
                .map_err(|error| error.to_string())
                .and_then(|rows| write_atomically(&path, CustomDictionaryCSV::encode(&rows)));
            WriteResult {
                message: outcome
                    .err()
                    .map(|error| PageMessage::failure(StringKey::CommonExportFailed, error)),
                reload: false,
            }
        });
    }

    fn import_csv(&mut self, store: Arc<CustomDictionaryStore>, frame: &eframe::Frame) {
        if !self.reserve_slot() {
            return;
        }
        let chosen = rfd::FileDialog::new()
            .set_parent(frame)
            .add_filter("CSV", &["csv", "txt"])
            .pick_file();
        let Some(path) = chosen else {
            self.is_slot_reserved = false;
            return;
        };
        self.begin_work(StringKey::DesktopProgressImporting, move || {
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
            WriteResult {
                message: Some(message),
                reload: true,
            }
        });
    }

    /// Deletes both learning tables — two calls, two files, no transaction
    /// that could span them; the second is attempted even when the first
    /// fails, and the alert reports rather than claims.
    fn delete_learning_records(&mut self, stores: &UserDataStores) {
        let frequency = Arc::clone(&stores.frequency);
        let association = Arc::clone(&stores.association);
        self.begin_work(StringKey::DesktopProgressDeleting, move || {
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
            WriteResult {
                message: Some(message),
                reload: false,
            }
        });
    }

    fn selected_row(&self) -> Option<&CustomDictionaryRow> {
        let id = self.selected_id.as_deref()?;
        self.rows.iter().find(|row| row.id == id)
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

/// `Data.write(to:options:.atomic)`: the file appears whole or not at all,
/// and a failed export never damages the one it would have replaced.
fn write_atomically(path: &PathBuf, contents: String) -> Result<(), String> {
    let temporary = path.with_extension(format!("csv.{}.tmp", std::process::id()));
    std::fs::write(&temporary, contents).map_err(|error| error.to_string())?;
    std::fs::rename(&temporary, path).map_err(|error| {
        std::fs::remove_file(&temporary).ok();
        error.to_string()
    })
}

pub fn show(ui: &mut egui::Ui, app: &mut SettingsApp, frame: &eframe::Frame) {
    let strings = app.strings();
    let store = Arc::clone(&app.stores().custom_dictionary);
    let mut model = std::mem::take(&mut app.custom_dictionary);

    let mut is_enabled = app.document().bool(&keys::IS_CUSTOM_DICT_ENABLED);
    if settings_card::switch_row(
        ui,
        strings.resolve(StringKey::DictionaryCustomDictEnabled),
        &mut is_enabled,
    ) {
        app.update_document(|document| {
            document.set_bool(&keys::IS_CUSTOM_DICT_ENABLED, is_enabled)
        });
    }
    // No user-data directory: the stores never opened, and the banner at
    // the top of the window says so — nothing to list, nothing to write.
    if app.is_read_only() {
        app.custom_dictionary = model;
        return;
    }

    model.poll_write(&store);
    model.poll_load();
    if !model.is_load_requested {
        model.is_load_requested = true;
        model.request_first_page(&store);
    }
    if model
        .filter_changed_at
        .is_some_and(|changed| changed.elapsed() >= FILTER_SETTLE)
    {
        model.filter_changed_at = None;
        model.request_first_page(&store);
    }
    if model.load_work.is_some() {
        ui.ctx().request_repaint_after(Duration::from_millis(50));
    }
    // Mutual exclusion is the slot's; the greyed look waits the same
    // 400 ms as the spinner so a millisecond-long write does not flash it.
    let looks_busy = model.work.as_ref().is_some_and(PendingWork::is_slow);

    ui.add_enabled_ui(!looks_busy, |ui| {
        settings_card::section_title(
            ui,
            strings.resolve(StringKey::DesktopEntriesSection),
            Some(&model.count_label()),
        );
        let filter = ui.add(
            egui::TextEdit::singleline(&mut model.filter)
                .hint_text(strings.resolve(StringKey::DictionarySearchPlaceholder))
                .desired_width(f32::INFINITY),
        );
        if filter.changed() {
            model.filter_changed_at = Some(Instant::now());
            ui.ctx().request_repaint_after(FILTER_SETTLE);
        }
        ui.add_space(6.0);
        // The table and its controls in one card, as a list sits in one
        // `ListView` surface on Windows.
        settings_card::frame(ui, egui::Margin::same(TABLE_CARD_PADDING), |ui| {
            entry_table(ui, &mut model, &strings);
            entry_table_controls(ui, &mut model, &strings, &store);
        });

        section_gap(ui);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .button(strings.resolve(StringKey::DictionaryExportCSV))
                .clicked()
            {
                model.export_csv(Arc::clone(&store), frame);
            }
            if ui
                .button(strings.resolve(StringKey::DictionaryImportCSV))
                .clicked()
            {
                model.import_csv(Arc::clone(&store), frame);
            }
        });
        ui.add_space(6.0);
        if settings_card::action(ui, strings.resolve(StringKey::DictionaryDeleteAll), true) {
            model.write(
                StringKey::DesktopProgressDeleting,
                Arc::clone(&store),
                |store| {
                    store
                        .delete_all()
                        .map(|_| ())
                        .map_err(|error| error.to_string())
                },
            );
        }

        section_gap(ui);
        if settings_card::action(
            ui,
            strings.resolve(StringKey::DesktopClearLearningRecords),
            true,
        ) {
            model.delete_learning_records(app.stores());
        }
    });

    entry_sheet(ui.ctx(), &mut model, &strings, &store);
    crate::work::show_overlay(ui.ctx(), &strings, model.work.as_ref());
    app.custom_dictionary = model;
}

/// The entries: click selects, double-click edits, the context menu keeps
/// a one-click route to both verbs. A definite height whatever the page
/// holds.
fn entry_table(ui: &mut egui::Ui, model: &mut CustomDictionaryPageModel, strings: &StringResolver) {
    let mut to_edit = None;
    let mut to_delete = None;
    let table = ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), TABLE_HEIGHT),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.set_min_height(TABLE_HEIGHT);
            egui_extras::TableBuilder::new(ui)
                .id_salt("custom_dictionary")
                .striped(false)
                .vscroll(false)
                .sense(egui::Sense::click())
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(
                    egui_extras::Column::initial(200.0)
                        .at_least(80.0)
                        .resizable(true),
                )
                .column(egui_extras::Column::remainder())
                .header(TABLE_HEADER_HEIGHT, |mut header| {
                    header.col(|ui| {
                        ui.strong(strings.resolve(StringKey::DictionaryRomanLabel));
                    });
                    header.col(|ui| {
                        ui.strong(strings.resolve(StringKey::DictionaryHanziLabel));
                    });
                })
                .body(|body| {
                    body.rows(TABLE_ROW_HEIGHT, model.rows.len(), |mut row| {
                        let entry = &model.rows[row.index()];
                        row.set_selected(model.selected_id.as_deref() == Some(entry.id.as_str()));
                        row.col(|ui| {
                            ui.weak(&entry.roman);
                        });
                        row.col(|ui| {
                            ui.label(&entry.hanzi);
                        });
                        let response = row.response();
                        if response.clicked() {
                            model.selected_id = Some(entry.id.clone());
                        }
                        if response.double_clicked() {
                            to_edit = Some(entry.clone());
                        }
                        response.context_menu(|ui| {
                            if ui
                                .button(strings.resolve(StringKey::DictionaryEditEntry))
                                .clicked()
                            {
                                to_edit = Some(entry.clone());
                                ui.close_menu();
                            }
                            if ui
                                .button(strings.resolve(StringKey::CommonDelete))
                                .clicked()
                            {
                                to_delete = Some(entry.id.clone());
                                ui.close_menu();
                            }
                        });
                    });
                });
        },
    );
    // The empty case as an overlay: the filter box stays reachable and the
    // columns stay put while a filter is narrowed to nothing and back.
    if model.rows.is_empty() {
        let mut body = table.response.rect;
        body.min.y += TABLE_HEADER_HEIGHT;
        let centre = body.center();
        if model.filter.trim().is_empty() {
            // A symbol rather than a sentence (USER 2026-08-24): an empty
            // dictionary needs no explaining. An empty tray, not a book.
            paint_tray(ui.painter(), centre, ui.visuals().weak_text_color());
        } else {
            ui.painter().text(
                centre,
                egui::Align2::CENTER_CENTER,
                strings.resolve(StringKey::DictionaryNoResults),
                egui::TextStyle::Body.resolve(ui.style()),
                ui.visuals().weak_text_color(),
            );
        }
    }
    if let Some(row) = to_edit {
        model.editing = Some(EditingRow {
            roman: row.roman.clone(),
            hanzi: row.hanzi.clone(),
            original: row,
        });
    }
    if let Some(id) = to_delete {
        model.pending_delete = Some(id);
    }
}

/// An empty tray: an open box with a lip, the shape of `tray`.
fn paint_tray(painter: &egui::Painter, centre: egui::Pos2, color: egui::Color32) {
    let size = EMPTY_STATE_SYMBOL_SIZE;
    let rect = egui::Rect::from_center_size(centre, egui::vec2(size, size * 0.75));
    let stroke = egui::Stroke::new(2.0_f32, color);
    painter.rect_stroke(rect, 4.0, stroke, egui::StrokeKind::Middle);
    let lip_y = rect.bottom() - size * 0.28;
    let dip_y = lip_y + size * 0.12;
    let points = [
        egui::pos2(rect.left(), lip_y),
        egui::pos2(rect.left() + size * 0.25, lip_y),
        egui::pos2(rect.left() + size * 0.33, dip_y),
        egui::pos2(rect.right() - size * 0.33, dip_y),
        egui::pos2(rect.right() - size * 0.25, lip_y),
        egui::pos2(rect.right(), lip_y),
    ];
    for pair in points.windows(2) {
        painter.line_segment([pair[0], pair[1]], stroke);
    }
}

/// The `+` / `−` pair and the pager (`entryTableControls`).
fn entry_table_controls(
    ui: &mut egui::Ui,
    model: &mut CustomDictionaryPageModel,
    strings: &StringResolver,
    store: &Arc<CustomDictionaryStore>,
) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui
            .small_button("+")
            .on_hover_text(strings.resolve(StringKey::DictionaryAddEntry))
            .clicked()
        {
            model.editing = Some(EditingRow {
                original: CustomDictionaryRow::new("", ""),
                roman: String::new(),
                hanzi: String::new(),
            });
        }
        let has_selection = model.selected_row().is_some();
        if ui
            .add_enabled(has_selection, egui::Button::new("−").small())
            .on_hover_text(strings.resolve(StringKey::CommonDelete))
            .clicked()
        {
            model.pending_delete = model.selected_id.clone();
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let can_forward = model.page + 1 < model.page_count();
            let can_backward = model.page > 0;
            if ui
                .add_enabled(can_forward, egui::Button::new("›").small())
                .on_hover_text(strings.resolve(StringKey::DesktopActionPageForward))
                .clicked()
            {
                model.page += 1;
                model.request_load(store);
            }
            if ui
                .add_enabled(can_backward, egui::Button::new("‹").small())
                .on_hover_text(strings.resolve(StringKey::DesktopActionPageBackward))
                .clicked()
            {
                model.page -= 1;
                model.request_load(store);
            }
            // Digits only: the pager needs no wording in five languages.
            ui.weak(
                egui::RichText::new(format!("{} / {}", model.page + 1, model.page_count()))
                    .monospace(),
            );
        });
    });
    if let Some(id) = model.pending_delete.take() {
        model.write(
            StringKey::DesktopProgressDeleting,
            Arc::clone(store),
            move |store| {
                store
                    .delete(&id)
                    .map(|_| ())
                    .map_err(|error| error.to_string())
            },
        );
    }
}

/// Add or edit one entry: a sheet the user finishes and dismisses
/// (`CustomDictionaryEntrySheet`).
fn entry_sheet(
    ctx: &egui::Context,
    model: &mut CustomDictionaryPageModel,
    strings: &StringResolver,
    store: &Arc<CustomDictionaryStore>,
) {
    let Some(editing) = model.editing.as_mut() else {
        return;
    };
    let is_new = editing.original.roman.is_empty();
    let title = strings.resolve(if is_new {
        StringKey::DictionaryAddEntry
    } else {
        StringKey::DictionaryEditEntry
    });
    let mut outcome: Option<Option<CustomDictionaryRow>> = None;
    egui::Window::new(title)
        .id(egui::Id::new("custom_dictionary_entry"))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .fixed_size(egui::vec2(ENTRY_SHEET_WIDTH, 0.0))
        .show(ctx, |ui| {
            // Enter submits only as the field it was pressed in gives up
            // focus for it — never a stray Enter, and never the Enter that
            // confirmed an IME composition (egui's own idiom for the
            // pattern; still a W15 dogfood item for CJK input).
            let mut submitted = false;
            egui::Grid::new("entry_fields")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    ui.label(strings.resolve(StringKey::DictionaryRomanLabel));
                    let roman = ui.add(
                        egui::TextEdit::singleline(&mut editing.roman).desired_width(f32::INFINITY),
                    );
                    ui.end_row();
                    ui.label(strings.resolve(StringKey::DictionaryHanziLabel));
                    let hanzi = ui.add(
                        egui::TextEdit::singleline(&mut editing.hanzi).desired_width(f32::INFINITY),
                    );
                    ui.end_row();
                    let enter = ui.input(|input| input.key_pressed(egui::Key::Enter));
                    submitted = enter && (roman.lost_focus() || hanzi.lost_focus());
                });
            ui.add_space(12.0);
            // A romanization is what the entry is found by; without one
            // there is nothing to store it under.
            let can_save = !editing.roman.trim().is_empty();
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let save = ui.add_enabled(
                    can_save,
                    egui::Button::new(strings.resolve(StringKey::DictionarySave)),
                );
                if save.clicked() || (submitted && can_save) {
                    let mut edited = editing.original.clone();
                    edited.roman = editing.roman.trim().to_owned();
                    edited.hanzi = editing.hanzi.trim().to_owned();
                    edited.updated_at = utc_timestamp_now();
                    outcome = Some(Some(edited));
                }
                if ui
                    .button(strings.resolve(StringKey::CommonCancel))
                    .clicked()
                    || ui.input(|input| input.key_pressed(egui::Key::Escape))
                {
                    outcome = Some(None);
                }
            });
        });
    match outcome {
        Some(Some(row)) => {
            model.editing = None;
            model.write(
                StringKey::DesktopProgressSaving,
                Arc::clone(store),
                move |store| store.upsert(&row).map_err(|error| error.to_string()),
            );
        }
        Some(None) => model.editing = None,
        None => {}
    }
}
