//! The Manage Dictionaries search, the bottom bar of that pane's toolbar
//! view (#792): a search field, and above it the first five hits, each with
//! its source badges and buttons to look the reading up in the MOE
//! dictionary or ChhoeTaigi — mobile's `DictionarySearchPanel`, in
//! libadwaita widgets. Port of the Windows `dictionary_search.rs`. The lookup
//! (`taigi_desktop_core::engine::dictionary_search`) — the engine's
//! dictionaries and the custom-dictionary query, plus the dictionaries'
//! first load — runs off the UI thread; the newest query wins by
//! generation.

use super::PageContext;
use crate::jobs;
use crate::window::Shell;
use adw::prelude::*;
use gtk::glib;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::OnceLock;
use taigi_desktop_core::dictionary_artifacts::{dictionary_version, DictionaryArtifacts};
use taigi_desktop_core::engine::dictionary_search::{search, DictionarySearchResult};
use taigi_desktop_core::engine::lexicon_install;
use taigi_desktop_core::settings::dictionary_search::{
    DEBOUNCE, RESULTS_MAX_HEIGHT, VISIBLE_RESULT_LIMIT,
};
use taigi_desktop_core::settings::presentation::PageMessage;
use taigi_desktop_core::settings::SettingsDocument;
use taigi_desktop_core::strings::{StringKey, StringResolver};

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

/// Around the bar's content, the toolbar's own spacing.
const BAR_INSET: i32 = 6;

pub struct DictionarySearchBar {
    shell: Shell,
    strings: StringResolver,
    state: RefCell<State>,
    results: gtk::ListBox,
    /// The hits' scroller, hidden while the query is empty so the bar is
    /// the field alone.
    scroller: gtk::ScrolledWindow,
    empty: gtk::Label,
}

/// Adds the search as `view`'s bottom bar — none in a window without user
/// data, which has no custom dictionary to search.
pub fn build<'a>(mut context: PageContext<'a>, view: &adw::ToolbarView) -> PageContext<'a> {
    if !context.has_user_data {
        return context;
    }
    let query = gtk::SearchEntry::new();
    query.set_placeholder_text(Some(
        context
            .strings
            .resolve(StringKey::DictionarySearchPlaceholder),
    ));
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
    // Results ABOVE the field, as mobile pops them up over its bar.
    let scroller = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .propagate_natural_height(true)
        .max_content_height(i32::from(RESULTS_MAX_HEIGHT))
        .child(&results)
        .visible(false)
        .build();
    let panel = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(BAR_INSET)
        .margin_top(BAR_INSET)
        .margin_bottom(BAR_INSET)
        .margin_start(BAR_INSET)
        .margin_end(BAR_INSET)
        .build();
    panel.append(&scroller);
    panel.append(&query);
    // The same width as the page's groups above it.
    view.add_bottom_bar(&adw::Clamp::builder().child(&panel).build());
    let this = Rc::new(DictionarySearchBar {
        shell: context.shell.clone(),
        strings: *context.strings,
        state: RefCell::new(State {
            document: context.document.clone(),
            ..State::default()
        }),
        results,
        scroller,
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

impl DictionarySearchBar {
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
            (state.query.trim().to_owned(), state.document.clone())
        };
        let weak = Rc::downgrade(self);
        jobs::spawn(
            move || {
                let is_loaded = LEXICON_LOADED.get().is_some()
                    || (load_lexicon() && LEXICON_LOADED.set(()).is_ok());
                is_loaded.then(|| search(&query, &settings))
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
        // The label is the list's placeholder, inside the scroller: hidden
        // with it while the query is empty.
        self.scroller.set_visible(!state.query.trim().is_empty());
        self.empty
            .set_visible(state.results.is_empty() && !state.is_searching);
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
        let title = match &result.hanji {
            Some(hanji) => format!("{}  {hanji}", result.roman),
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
