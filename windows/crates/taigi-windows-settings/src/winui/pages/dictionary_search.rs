//! 辭典搜尋: a search field, the first five hits, each with its source
//! badges and a menu to look the reading up in 教典 or ChhoeTaigi. Port of
//! `DictionarySearchPage.swift`. Built but UNLISTED, as on macOS (not
//! released yet, USER 2026-08-21): reached only by `--pane dictionarySearch`.
//! The lookup — the engine's FFI and the custom-dictionary query, plus the
//! dictionaries' first load — runs off the UI thread; the newest query
//! wins by generation.

use crate::presentation::PageMessage;
use crate::search::{search, DictionarySearchResult};
use crate::winui::cards;
use crate::winui::window::{Message as WindowMessage, SettingsWindow};
use std::sync::Arc;
use std::time::Duration;
use taigi_windows_core::dictionary_artifacts::{dictionary_version, DictionaryArtifacts};
use taigi_windows_core::engine::lexicon_install;
use taigi_windows_core::settings::SettingsDocument;
use taigi_windows_core::strings::{StringKey, StringResolver};
use taigi_windows_storage::UserDataStores;
use windows_reactor::*;

/// `DictionarySearchModel.visibleResultLimit`.
const VISIBLE_RESULT_LIMIT: usize = 5;
/// `DictionarySearchModel.debounce`.
const DEBOUNCE: Duration = Duration::from_millis(300);
const ROW_GAP: f64 = 8.0;
const BADGE_RADIUS: f64 = 4.0;
const LOOKUP_GLYPH: &str = "\u{E8A7}";
const SECONDARY_OPACITY: f64 = 0.65;
const BADGE_GAP: f64 = 4.0;
const BADGE_PADDING_X: f64 = 6.0;
const BADGE_PADDING_Y: f64 = 2.0;
const BADGE_FONT_SIZE: f64 = 12.0;

/// The lookup a result's menu offers.
const MOE_ITEM: &str = "moe";
const CHHOE_ITEM: &str = "chhoe";

#[derive(Clone)]
pub enum Message {
    QueryChanged(String),
    /// The box stopped changing: run the query.
    QuerySettled(u64),
    Finished(u64, Box<SearchOutcome>),
}

#[derive(Clone)]
pub struct SearchOutcome {
    results: Vec<DictionarySearchResult>,
    /// Whether the dictionaries are loaded in this process after the job.
    is_lexicon_loaded: bool,
    /// Set when the dictionaries could not be loaded at all — which is not
    /// the same as a query that matched nothing.
    failure: Option<String>,
}

#[derive(Default)]
pub struct DictionarySearchModel {
    query: String,
    results: Vec<DictionarySearchResult>,
    /// True from the keystroke until the newest job answers.
    is_searching: bool,
    generation: u64,
    /// Whether the dictionaries are in this process yet; only the first
    /// query pays for loading them.
    is_lexicon_loaded: bool,
}

pub struct PageEnvironment<'a> {
    pub stores: &'a UserDataStores,
    pub document: &'a SettingsDocument,
    pub message: &'a mut Option<PageMessage>,
}

pub fn update(
    model: &mut DictionarySearchModel,
    message: Message,
    environment: PageEnvironment<'_>,
    context: &ComponentContext<SettingsWindow>,
) {
    let PageEnvironment {
        stores,
        document,
        message: alert,
    } = environment;
    match message {
        Message::QueryChanged(query) => {
            if query == model.query {
                return;
            }
            model.query = query;
            model.generation = model.generation.wrapping_add(1);
            let generation = model.generation;
            if model.query.trim().is_empty() {
                // An emptied box answers itself: no job, no results, and
                // the one in flight is already this generation's junior.
                model.results.clear();
                model.is_searching = false;
                return;
            }
            model.is_searching = true;
            // A wait the runtime will not start runs the query AT ONCE
            // instead: a settle that never arrives would leave the page
            // searching for ever.
            _ = context.spawn_background_with_rejection(
                move |_| {
                    std::thread::sleep(DEBOUNCE);
                    WindowMessage::DictionarySearch(Message::QuerySettled(generation))
                },
                WindowMessage::DictionarySearch(Message::QuerySettled(generation)),
            );
        }
        Message::QuerySettled(generation) => {
            if generation != model.generation {
                return;
            }
            start(model, document, stores, context);
        }
        Message::Finished(generation, outcome) => {
            let SearchOutcome {
                results,
                is_lexicon_loaded,
                failure,
            } = *outcome;
            // The load is this process's, not this query's: remember it
            // even when a newer query has taken over — and only ever
            // upwards, so an older job that failed cannot un-load what a
            // newer one loaded.
            model.is_lexicon_loaded |= is_lexicon_loaded;
            if generation != model.generation {
                return;
            }
            model.is_searching = false;
            match failure {
                // Dictionaries that cannot be read are worth saying so —
                // "no results" would be a lie.
                Some(error) => {
                    *alert = Some(PageMessage::failure(
                        StringKey::DesktopCustomDictReadFailed,
                        error,
                    ))
                }
                None => model.results = results,
            }
        }
    }
}

fn start(
    model: &mut DictionarySearchModel,
    document: &SettingsDocument,
    stores: &UserDataStores,
    context: &ComponentContext<SettingsWindow>,
) {
    let generation = model.generation;
    let query = model.query.trim().to_owned();
    let settings = Arc::new(document.clone());
    let store = Arc::clone(&stores.custom_dictionary);
    let is_lexicon_loaded = model.is_lexicon_loaded;
    let rejection = WindowMessage::DictionarySearch(Message::Finished(
        generation,
        Box::new(SearchOutcome {
            results: Vec::new(),
            is_lexicon_loaded,
            failure: Some("the search could not be started".to_owned()),
        }),
    ));
    _ = context.spawn_background_with_rejection(
        move |_| {
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let is_loaded = is_lexicon_loaded || load_lexicon();
                let results = if is_loaded {
                    search(&query, &settings, &store)
                } else {
                    Vec::new()
                };
                SearchOutcome {
                    results,
                    is_lexicon_loaded: is_loaded,
                    failure: (!is_loaded)
                        .then(|| "the dictionaries could not be loaded".to_owned()),
                }
            }))
            .unwrap_or_else(|_| SearchOutcome {
                results: Vec::new(),
                is_lexicon_loaded,
                failure: Some("the search did not finish".to_owned()),
            });
            WindowMessage::DictionarySearch(Message::Finished(generation, Box::new(outcome)))
        },
        rejection,
    );
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

pub fn view(
    window: &SettingsWindow,
    strings: &StringResolver,
    context: &mut ViewContext<SettingsWindow>,
) -> View {
    let model = window.dictionary_search();
    if window.is_read_only() {
        return TextBlock::new()
            .text(strings.resolve(StringKey::DictionaryNoResults))
            .opacity(SECONDARY_OPACITY)
            .into();
    }
    let has_query = !model.query.trim().is_empty();
    View::fragment((
        TextBox::new()
            .text(model.query.clone())
            .placeholder_text(strings.resolve(StringKey::DictionarySearchPlaceholder))
            .on_text_changed(
                context
                    .callback(|text| WindowMessage::DictionarySearch(Message::QueryChanged(text))),
            ),
        if has_query && model.results.is_empty() && !model.is_searching {
            TextBlock::new()
                .text(strings.resolve(StringKey::DictionaryNoResults))
                .opacity(SECONDARY_OPACITY)
                .margin(Thickness::new(0.0, ROW_GAP, 0.0, 0.0))
                .into()
        } else {
            View::empty()
        },
        View::keyed_fragment(
            model
                .results
                .iter()
                .take(VISIBLE_RESULT_LIMIT)
                .map(|result| (result.key(), result_row(result, strings, context)))
                .collect::<Vec<_>>(),
        ),
    ))
}

fn result_row(
    result: &DictionarySearchResult,
    strings: &StringResolver,
    context: &mut ViewContext<SettingsWindow>,
) -> View {
    // One badge per distinct word: the three supplements share one.
    let mut seen = Vec::new();
    for source in &result.sources {
        let key = source.badge_key();
        if !seen.contains(&key) {
            seen.push(key);
        }
    }
    let badges = View::keyed_fragment(
        seen.iter()
            .map(|key| (format!("{key:?}"), badge(strings.resolve(*key))))
            .collect::<Vec<_>>(),
    );
    // The URLs travel WITH the callback, not an index into a list a
    // background completion may have replaced by the time the menu is
    // used.
    let moe = result.moe_url();
    let chhoe = result.chhoe_url();
    let mut items = Vec::new();
    if moe.is_some() {
        items.push(MenuItem::item(
            MOE_ITEM,
            strings.resolve(StringKey::DictionaryLookupMoe),
        ));
    }
    if chhoe.is_some() {
        items.push(MenuItem::item(
            CHHOE_ITEM,
            strings.resolve(StringKey::DictionaryLookupChhoe),
        ));
    }
    let lookup = if items.is_empty() {
        View::empty()
    } else {
        Button::new()
            .content(FontIcon::new().glyph(LOOKUP_GLYPH))
            .menu(Menu::new(
                items,
                context.callback(move |item: String| {
                    let url = if item == MOE_ITEM { &moe } else { &chhoe };
                    // A menu item that silently does nothing reads as a
                    // broken one; an empty URL is one no item offered.
                    WindowMessage::OpenUrl(url.clone().unwrap_or_default())
                }),
            ))
    };
    cards::frame(
        Grid::new()
            .columns([GridLength::Auto, GridLength::STAR, GridLength::Auto])
            .column_spacing(ROW_GAP)
            .children((
                StackPanel::new()
                    .orientation(Orientation::Horizontal)
                    .spacing(ROW_GAP)
                    .vertical_alignment(VerticalAlignment::Center)
                    .grid_column(0)
                    .children((
                        TextBlock::new()
                            .text(result.roman.clone())
                            .opacity(SECONDARY_OPACITY),
                        TextBlock::new().text_optional(result.hanzi.clone()),
                    )),
                StackPanel::new()
                    .orientation(Orientation::Horizontal)
                    .spacing(BADGE_GAP)
                    .vertical_alignment(VerticalAlignment::Center)
                    .grid_column(1)
                    .children([badges]),
                Border::new()
                    .grid_column(2)
                    .vertical_alignment(VerticalAlignment::Center)
                    .content(lookup),
            )),
    )
}

fn badge(text: &str) -> View {
    Border::new()
        .background(ThemeBrush::CardBackground)
        .border_brush(ThemeBrush::CardStroke)
        .border_thickness(Thickness::uniform(1.0))
        .corner_radius(CornerRadius::uniform(BADGE_RADIUS))
        .padding(Thickness::xy(BADGE_PADDING_X, BADGE_PADDING_Y))
        .content(
            TextBlock::new()
                .text(text)
                .font_size(BADGE_FONT_SIZE)
                .opacity(SECONDARY_OPACITY),
        )
}
