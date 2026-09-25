//! Dictionary Search: a search field, the first five hits, each with its source
//! badges and buttons to look the reading up in the MOE dictionary or ChhoeTaigi. Port of
//! `DictionarySearchPage.swift` / the Windows `dictionary_search.rs`. Built
//! but UNLISTED, as on the other desktops: reached only by
//! `--pane dictionarySearch`. The lookup — the engine's dictionaries and
//! the custom-dictionary query, plus the dictionaries' first load — runs
//! off the UI thread; the newest query wins by generation.

use super::PageContext;
use crate::jobs;
use crate::presentation::PageMessage;
use crate::search::{search, DictionarySearchResult};
use crate::window::Shell;
use adw::prelude::*;
use gtk::glib;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use taigi_desktop_core::dictionary_artifacts::{dictionary_version, DictionaryArtifacts};
use taigi_desktop_core::engine::lexicon_install;
use taigi_desktop_core::settings::SettingsDocument;
use taigi_desktop_core::strings::{StringKey, StringResolver};
use taigi_desktop_storage::CustomDictionaryStore;

/// `DictionarySearchModel.visibleResultLimit`.
const VISIBLE_RESULT_LIMIT: usize = 5;
/// `DictionarySearchModel.debounce`.
const DEBOUNCE: Duration = Duration::from_millis(300);

/// Set once the dictionaries are in this process: only the first query
/// pays for loading them, the load is the process's, not a page's — and
/// only a SUCCESS is remembered, so a dictionary file fixed after a failed
/// load is read on the next query.
static LEXICON_LOADED: OnceLock<()> = OnceLock::new();

#[derive(Default)]
struct State {
    query: String,
    results: Vec<DictionarySearchResult>,
    /// True from the keystroke until the newest job answers.
    is_searching: bool,
    generation: u64,
    document: SettingsDocument,
}

pub struct DictionarySearchPage {
    shell: Shell,
    strings: StringResolver,
    store: Arc<CustomDictionaryStore>,
    state: RefCell<State>,
    results: gtk::ListBox,
    empty: gtk::Label,
}

pub fn build<'a>(mut context: PageContext<'a>, page: &adw::PreferencesPage) -> PageContext<'a> {
    let group = adw::PreferencesGroup::new();
    let Some(stores) = context.stores else {
        group.add(
            &gtk::Label::builder()
                .label(context.strings.resolve(StringKey::DictionaryNoResults))
                .css_classes(["dim-label"])
                .build(),
        );
        page.add(&group);
        return context;
    };
    let query = gtk::SearchEntry::new();
    query.set_placeholder_text(Some(
        context
            .strings
            .resolve(StringKey::DictionarySearchPlaceholder),
    ));
    group.add(&query);
    let results = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    let empty = gtk::Label::builder()
        .label(context.strings.resolve(StringKey::DictionaryNoResults))
        .css_classes(["dim-label"])
        .margin_top(12)
        .margin_bottom(12)
        .visible(false)
        .build();
    results.set_placeholder(Some(&empty));
    group.add(&results);
    page.add(&group);
    let this = Rc::new(DictionarySearchPage {
        shell: context.shell.clone(),
        strings: *context.strings,
        store: Arc::clone(&stores.custom_dictionary),
        state: RefCell::new(State {
            document: context.document.clone(),
            ..State::default()
        }),
        results,
        empty,
    });
    // `changed`, not `search-changed` (GTK's own 150 ms delay): the
    // generation moves at the keystroke, the one debounce is ours.
    let weak = Rc::downgrade(&this);
    query.connect_changed(move |entry| {
        if let Some(page) = weak.upgrade() {
            page.query_changed(entry.text().to_string());
        }
    });
    // The search reads the source toggles and the romanization of the
    // moment.
    let weak = Rc::downgrade(&this);
    context.on_refresh(move |document| {
        if let Some(page) = weak.upgrade() {
            page.state.borrow_mut().document = document.clone();
        }
    });
    context.retain(this);
    context
}

impl DictionarySearchPage {
    fn query_changed(self: &Rc<Self>, query: String) {
        let generation = {
            let mut state = self.state.borrow_mut();
            if query == state.query {
                return;
            }
            state.query = query;
            state.generation = state.generation.wrapping_add(1);
            if state.query.trim().is_empty() {
                // An emptied box answers itself: no job, no results, and
                // the one in flight is already this generation's junior.
                state.results.clear();
                state.is_searching = false;
                drop(state);
                self.render();
                return;
            }
            state.is_searching = true;
            state.generation
        };
        self.render();
        let weak = Rc::downgrade(self);
        glib::timeout_add_local_once(DEBOUNCE, move || {
            let Some(page) = weak.upgrade() else { return };
            if page.state.borrow().generation != generation {
                return;
            }
            page.start(generation);
        });
    }

    fn start(self: &Rc<Self>, generation: u64) {
        let (query, settings) = {
            let state = self.state.borrow();
            (
                state.query.trim().to_owned(),
                Arc::new(state.document.clone()),
            )
        };
        let store = Arc::clone(&self.store);
        let weak = Rc::downgrade(self);
        jobs::spawn(
            move || {
                let is_loaded = LEXICON_LOADED.get().is_some()
                    || (load_lexicon() && LEXICON_LOADED.set(()).is_ok());
                is_loaded.then(|| search(&query, &settings, &store))
            },
            move |outcome| {
                let Some(page) = weak.upgrade() else { return };
                if page.state.borrow().generation != generation {
                    return;
                }
                page.state.borrow_mut().is_searching = false;
                match outcome {
                    Some(Some(results)) => page.state.borrow_mut().results = results,
                    // Dictionaries that cannot be read are worth saying so
                    // — "no results" would be a lie.
                    Some(None) => page.report("the dictionaries could not be loaded"),
                    None => page.report("the search did not finish"),
                }
                page.render();
            },
        );
    }

    fn report(&self, detail: &str) {
        let message = PageMessage::failure(StringKey::DesktopCustomDictReadFailed, detail);
        self.shell.report(&message, &self.strings);
    }

    fn render(&self) {
        let state = self.state.borrow();
        super::remove_rows(&self.results);
        for result in state.results.iter().take(VISIBLE_RESULT_LIMIT) {
            self.results.append(&self.result_row(result));
        }
        let has_query = !state.query.trim().is_empty();
        self.empty
            .set_visible(has_query && state.results.is_empty() && !state.is_searching);
    }

    /// One hit: the romanization and the hanji, the source badges as the
    /// subtitle (one per distinct word — the three supplements share one),
    /// and the two lookups as buttons.
    fn result_row(&self, result: &DictionarySearchResult) -> adw::ActionRow {
        // One badge per distinct word (by key: the three supplements share
        // one).
        let mut keys: Vec<StringKey> = Vec::new();
        for source in &result.sources {
            let key = source.badge_key();
            if !keys.contains(&key) {
                keys.push(key);
            }
        }
        let badges: Vec<&str> = keys.iter().map(|key| self.strings.resolve(*key)).collect();
        let title = match &result.hanzi {
            Some(hanzi) => format!("{}  {hanzi}", result.roman),
            None => result.roman.clone(),
        };
        // Dictionary and user text, never markup.
        let row = adw::ActionRow::builder()
            .title(title)
            .subtitle(badges.join(" · "))
            .use_markup(false)
            .build();
        for (url, label) in [
            (result.moe_url(), StringKey::DictionaryLookupMoe),
            (result.chhoe_url(), StringKey::DictionaryLookupChhoe),
        ] {
            let Some(url) = url else { continue };
            let button = gtk::Button::builder()
                .label(self.strings.resolve(label))
                .valign(gtk::Align::Center)
                .css_classes(["flat"])
                .build();
            let shell = self.shell.clone();
            button.connect_clicked(move |_| shell.open_url(&url));
            row.add_suffix(&button);
        }
        row
    }
}

/// Loads the installed dictionaries into this process's engine; answers
/// whether a search can run.
fn load_lexicon() -> bool {
    let directory = taigi_linux_platform::dictionaries_directory();
    match DictionaryArtifacts::locate(&directory) {
        Ok(artifacts) => {
            lexicon_install(&artifacts, dictionary_version(env!("CARGO_PKG_VERSION"))).is_some()
        }
        Err(error) => {
            log::error!(
                "lexicon.not_installed directory={} error={error}",
                directory.display()
            );
            false
        }
    }
}
