//! The Manage Dictionaries search, pinned under that pane's form (#792; the
//! window draws the footer it sits in): a search field, and above it the
//! first five hits, each with its source badges and a menu to look the
//! reading up in the MOE dictionary or ChhoeTaigi — mobile's
//! `DictionarySearchPanel`, in WinUI controls.
//! The lookup (`taigi_desktop_core::engine::dictionary_search`) — the
//! engine's dictionaries and the custom-dictionary query, plus the
//! dictionaries' first load — runs off the UI thread; the newest query
//! wins by generation.

use crate::winui::cards;
use crate::winui::window::{Message as WindowMessage, SettingsWindow};
use taigi_desktop_core::dictionary_artifacts::{dictionary_version, DictionaryArtifacts};
use taigi_desktop_core::engine::dictionary_search::{search, DictionarySearchResult};
use taigi_desktop_core::engine::lexicon_install;
use taigi_desktop_core::settings::dictionary_search::{DEBOUNCE, VISIBLE_RESULT_LIMIT};
use taigi_desktop_core::settings::presentation::PageMessage;
use taigi_desktop_core::settings::SettingsDocument;
use taigi_desktop_core::strings::{StringKey, StringResolver};
use windows_reactor::*;

const ROW_GAP: f64 = 8.0;
const SECONDARY_OPACITY: f64 = 0.65;
const BADGE_GAP: f64 = 4.0;

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

/// `alert` is the window's one notice: what a failed search has to say.
pub fn update(
    model: &mut DictionarySearchModel,
    message: Message,
    document: &SettingsDocument,
    alert: &mut Option<PageMessage>,
    context: &ComponentContext<SettingsWindow>,
) {
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
            start(model, document, context);
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
    context: &ComponentContext<SettingsWindow>,
) {
    let generation = model.generation;
    let query = model.query.trim().to_owned();
    let settings = document.clone();
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
                    search(&query, &settings)
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
    let has_query = !model.query.trim().is_empty();
    // Results ABOVE the box, as mobile pops them up over its bar; an empty
    // query collapses the footer to the box alone. Keyed by state: the slot
    // changes type with the query, and an unkeyed type change is the
    // `InsertChild` E_BOUNDS abort `window.rs` keys its page slot against.
    let (state, results) = if !has_query {
        ("idle", View::empty())
    } else if model.results.is_empty() && !model.is_searching {
        let none = TextBlock::new()
            .text(strings.resolve(StringKey::DictionaryNoResults))
            .opacity(SECONDARY_OPACITY)
            .into();
        ("none", none)
    } else {
        // One card of rows, as mobile lists its hits; all five at full
        // height, no scroller (`VISIBLE_RESULT_LIMIT`).
        let hits = cards::link_group([View::keyed_fragment(
            model
                .results
                .iter()
                .take(VISIBLE_RESULT_LIMIT)
                .map(|result| result_row(result, strings, context))
                .collect::<Vec<_>>(),
        )]);
        ("hits", hits)
    };
    StackPanel::new().spacing(ROW_GAP).children((
        View::keyed_fragment([(state, results)]),
        TextBox::new()
            .text(model.query.clone())
            .placeholder_text(strings.resolve(StringKey::DictionarySearchPlaceholder))
            .on_text_changed(
                context
                    .callback(|text| WindowMessage::DictionarySearch(Message::QueryChanged(text))),
            ),
    ))
}

/// One hit as a whole-row button, keyed: the row and its lookup menu (as
/// mobile opens its lookups from a tap anywhere on the row). The key carries
/// whether the row has a menu, so a row that changed shape is replaced,
/// never retyped in place.
fn result_row(
    result: &DictionarySearchResult,
    strings: &StringResolver,
    context: &mut ViewContext<SettingsWindow>,
) -> (String, View) {
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
    let badge_labels: Vec<&str> = result
        .badge_keys()
        .into_iter()
        .map(|key| strings.resolve(key))
        .collect();
    let badges = View::keyed_fragment(
        badge_labels
            .iter()
            .map(|label| (*label, cards::badge(label)))
            .collect::<Vec<_>>(),
    );
    // What a screen reader announces: the word, then its sources.
    let name = [result.roman.as_str()]
        .into_iter()
        .chain(result.hanji.as_deref())
        .chain(badge_labels.iter().copied())
        .collect::<Vec<_>>()
        .join(" ");
    // The leave-the-app mark only on a row that opens something.
    let glyph = if items.is_empty() {
        View::empty()
    } else {
        cards::open_in_new_glyph().grid_column(2).into()
    };
    let content = Grid::new()
        .columns([GridLength::Auto, GridLength::STAR, GridLength::Auto])
        .column_spacing(ROW_GAP)
        .children((
            StackPanel::new()
                .orientation(Orientation::Horizontal)
                .spacing(ROW_GAP)
                .vertical_alignment(VerticalAlignment::Center)
                .grid_column(0)
                .children((
                    // Full strength, as iOS and Android draw it.
                    TextBlock::new().text(result.roman.clone()),
                    TextBlock::new().text_optional(result.hanji.clone()),
                )),
            StackPanel::new()
                .orientation(Orientation::Horizontal)
                .spacing(BADGE_GAP)
                .vertical_alignment(VerticalAlignment::Center)
                .grid_column(1)
                .children([badges]),
            glyph,
        ));
    let (shape, action) = if items.is_empty() {
        // A reading that makes no URL: the row, with nothing to open.
        ("plain", cards::RowAction::None)
    } else {
        let menu = Menu::new(
            items,
            context.callback(move |item: String| {
                let url = if item == MOE_ITEM { &moe } else { &chhoe };
                // A menu item that silently does nothing reads as a
                // broken one; an empty URL is one no item offered.
                WindowMessage::OpenUrl(url.clone().unwrap_or_default())
            }),
        );
        ("menu", cards::RowAction::Menu(menu))
    };
    (
        format!("{}:{shape}", result.key()),
        cards::row_button(content, &name, action),
    )
}
