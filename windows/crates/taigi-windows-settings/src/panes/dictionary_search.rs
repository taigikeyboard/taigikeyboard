//! 辭典搜尋: a search field, the first five hits, each with its source
//! badges and a menu to look the reading up in 教典 or ChhoeTaigi. Port of
//! `DictionarySearchPage.swift`. Built but UNLISTED, as on macOS (not
//! released yet, USER 2026-08-21): reached only by `--pane dictionarySearch`.
//! The lookup — the engine's FFI and the custom-dictionary query, plus the
//! dictionaries' first load — runs off the UI thread (`Task.detached` on
//! the Mac); the newest query wins by generation.

// 中文: 辭典搜尋頁(未上架,只能用 `--pane dictionarySearch` 開)— 查詢在背景執行緒,最新一次才算數。

use crate::app::SettingsApp;
use crate::search::{search, DictionarySearchResult};
use crate::widgets::alert::PageMessage;
use crate::work::PendingWork;
use std::sync::Arc;
use std::time::{Duration, Instant};
use taigi_windows_core::dictionary_artifacts::{dictionary_version, DictionaryArtifacts};
use taigi_windows_core::engine::lexicon_install;
use taigi_windows_core::strings::{StringKey, StringResolver};

/// `DictionarySearchModel.visibleResultLimit`.
const VISIBLE_RESULT_LIMIT: usize = 5;
/// `DictionarySearchModel.debounce`.
const DEBOUNCE: Duration = Duration::from_millis(300);

/// What a search job hands back.
struct SearchResult {
    generation: u64,
    results: Vec<DictionarySearchResult>,
    /// Whether the dictionaries are loaded in this process after the job.
    is_lexicon_loaded: bool,
}

#[derive(Default)]
pub struct DictionarySearchModel {
    query: String,
    query_changed_at: Option<Instant>,
    results: Vec<DictionarySearchResult>,
    /// True from the keystroke until the newest job answers.
    is_searching: bool,
    generation: u64,
    job: Option<PendingWork<SearchResult>>,
    pub message: Option<PageMessage>,
}

impl DictionarySearchModel {
    fn start_search(&mut self, app: &SettingsApp) {
        self.generation += 1;
        let generation = self.generation;
        let query = self.query.trim().to_owned();
        if query.is_empty() {
            self.results.clear();
            self.is_searching = false;
            return;
        }
        let settings = Arc::new(app.document().clone());
        let store = Arc::clone(&app.stores().custom_dictionary);
        let needs_lexicon = !app.is_lexicon_loaded();
        self.is_searching = true;
        self.job = Some(PendingWork::spawn_quiet(move || {
            let is_lexicon_loaded = !needs_lexicon || load_lexicon();
            let results = if is_lexicon_loaded {
                search(&query, &settings, &store)
            } else {
                Vec::new()
            };
            SearchResult {
                generation,
                results,
                is_lexicon_loaded,
            }
        }));
    }

    fn poll(&mut self, app: &mut SettingsApp) {
        let Some(finished) = self.job.as_ref().and_then(PendingWork::poll) else {
            return;
        };
        self.job = None;
        let Some(result) = finished else {
            self.is_searching = false;
            self.message = Some(PageMessage::failure(
                StringKey::DesktopCustomDictReadFailed,
                "the search did not finish",
            ));
            return;
        };
        app.note_lexicon_loaded(result.is_lexicon_loaded);
        if result.generation != self.generation {
            return;
        }
        self.results = result.results;
        self.is_searching = false;
    }
}

/// Loads the dictionaries beside the executable into this process's
/// engine; answers whether a search can run.
fn load_lexicon() -> bool {
    let Some(directory) = taigi_windows_platform::executable_directory() else {
        return false;
    };
    match DictionaryArtifacts::locate(&directory.join(DictionaryArtifacts::DIRECTORY_NAME)) {
        Ok(artifacts) => {
            lexicon_install(&artifacts, dictionary_version(env!("CARGO_PKG_VERSION"))).is_some()
        }
        Err(error) => {
            log::error!("lexicon.not_installed error={error}");
            false
        }
    }
}

pub fn show(ui: &mut egui::Ui, app: &mut SettingsApp) {
    let strings = app.strings();
    let mut model = std::mem::take(&mut app.search);
    if app.is_read_only() {
        ui.weak(strings.resolve(StringKey::DictionaryNoResults));
        app.search = model;
        return;
    }
    model.poll(app);
    let field = ui.add(
        egui::TextEdit::singleline(&mut model.query)
            .hint_text(strings.resolve(StringKey::DictionarySearchPlaceholder))
            .desired_width(f32::INFINITY),
    );
    if field.changed() {
        model.query_changed_at = Some(Instant::now());
        model.is_searching = !model.query.trim().is_empty();
        ui.ctx().request_repaint_after(DEBOUNCE);
    }
    if model
        .query_changed_at
        .is_some_and(|changed| changed.elapsed() >= DEBOUNCE)
    {
        model.query_changed_at = None;
        model.start_search(app);
    }
    if model.job.is_some() {
        ui.ctx().request_repaint_after(Duration::from_millis(50));
    }
    ui.add_space(8.0);
    if !model.query.trim().is_empty() {
        if model.results.is_empty() {
            if !model.is_searching {
                ui.weak(strings.resolve(StringKey::DictionaryNoResults));
            }
        } else {
            for result in model.results.iter().take(VISIBLE_RESULT_LIMIT) {
                result_row(ui, result, &strings, &mut model.message);
                ui.add_space(4.0);
            }
        }
    }
    app.search = model;
}

fn result_row(
    ui: &mut egui::Ui,
    result: &DictionarySearchResult,
    strings: &StringResolver,
    message: &mut Option<PageMessage>,
) {
    ui.horizontal(|ui| {
        ui.weak(&result.roman);
        if let Some(hanzi) = &result.hanzi {
            ui.label(hanzi);
        }
        // One badge per distinct word: the three supplements share one.
        let mut seen = Vec::new();
        for source in &result.sources {
            let key = source.badge_key();
            if seen.contains(&key) {
                continue;
            }
            seen.push(key);
            egui::Frame::NONE
                .fill(ui.visuals().faint_bg_color)
                .inner_margin(egui::Margin::symmetric(6, 2))
                .corner_radius(4.0)
                .show(ui, |ui| {
                    ui.small(strings.resolve(key));
                });
        }
        let moe = result.moe_url();
        let chhoe = result.chhoe_url();
        if moe.is_some() || chhoe.is_some() {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.menu_button("↗", |ui| {
                    if let Some(url) = &moe {
                        if ui
                            .button(strings.resolve(StringKey::DictionaryLookupMoe))
                            .clicked()
                        {
                            open(url, message);
                            ui.close_menu();
                        }
                    }
                    if let Some(url) = &chhoe {
                        if ui
                            .button(strings.resolve(StringKey::DictionaryLookupChhoe))
                            .clicked()
                        {
                            open(url, message);
                            ui.close_menu();
                        }
                    }
                });
            });
        }
    });
}

fn open(url: &str, message: &mut Option<PageMessage>) {
    // A menu item that silently does nothing reads as a broken one.
    if !taigi_windows_platform::open_url(url) {
        *message = Some(PageMessage::UrlFailed(url.to_owned()));
    }
}
