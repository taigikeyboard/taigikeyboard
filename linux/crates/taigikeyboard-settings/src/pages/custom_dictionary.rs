//! Custom Dictionary: the words the user added themselves. Port of
//! `CustomDictionaryPage.swift` / the Windows `custom_dictionary.rs`: rows
//! fetched one PAGE at a time (10), a filter that reloads once it settles,
//! a list whose selection drives the add / edit / delete trio, the pager
//! under it, CSV import / export, delete all, and the one destructive verb
//! for the learning records.
//!
//! Every store call — the loads included — runs off the UI thread
//! (`jobs::spawn`): the newest load wins by generation, and the writes
//! share the page's one work slot (refused, not queued — a queued delete
//! would name a row the list may no longer show).
//!
//! The table is the Mac's: two columns, romanization then Hanji, under a header,
//! a double-click (or Enter) on a row edits it, the ✎ button over the
//! selection is the keyboard-reachable way (the Windows shape). The empty
//! list says so in words.

use super::{icon_button, remove_rows, PageContext};
use crate::jobs;
use crate::presentation::PageMessage;
use crate::window::{JobSlot, Shell};
use adw::prelude::*;
use gtk::{gio, glib};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;
use taigi_desktop_core::settings::keys;
use taigi_desktop_core::strings::{StringKey, StringResolver};
use taigi_desktop_storage::{
    utc_timestamp_now, CustomDictionaryCSV, CustomDictionaryCSVError, CustomDictionaryRow,
    CustomDictionaryStore, LearnedPhraseStore, UserAssociationStore, UserDataStores,
    UserFrequencyStore,
};

/// `CustomDictionaryPageModel.pageSize`.
const PAGE_SIZE: usize = 10;
/// `reloadWhenFilterSettles`.
const FILTER_SETTLE: Duration = Duration::from_millis(200);
/// How long a job may run before the page says so (`overlayDelay`): a
/// millisecond-long write must not flash a spinner.
const OVERLAY_DELAY: Duration = Duration::from_millis(400);

/// A command that empties a store, waiting on its confirmation — there is
/// no undo, and these two empty a table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Confirm {
    DeleteAll,
    ClearLearningRecords,
}

impl Confirm {
    fn title_key(self) -> StringKey {
        match self {
            Self::DeleteAll => StringKey::DictionaryDeleteAll,
            Self::ClearLearningRecords => StringKey::DesktopClearLearningRecords,
        }
    }

    /// The question under the title. `ClearLearningRecords` has none
    /// authored, and its title already asks it.
    fn message_key(self) -> Option<StringKey> {
        match self {
            Self::DeleteAll => Some(StringKey::DictionaryDeleteAllMessage),
            Self::ClearLearningRecords => None,
        }
    }
}

/// What a load hands back: the page it fetched, or why it could not.
struct Loaded {
    page: usize,
    rows: Vec<CustomDictionaryRow>,
    match_count: usize,
    total_count: usize,
}

/// What a write hands back.
struct JobOutcome {
    message: Option<PageMessage>,
    /// Whether the list changed and must be reloaded.
    is_reload_wanted: bool,
}

#[derive(Default)]
struct State {
    rows: Vec<CustomDictionaryRow>,
    /// Every entry, for the section header — what the dictionary HOLDS.
    total_count: usize,
    /// How many entries the current filter matches; what the pager divides.
    match_count: usize,
    /// Which page is on screen, zero-based.
    page: usize,
    filter: String,
    /// Which load the rows on screen came from: the newest wins by number.
    load_generation: u64,
    /// The row the list has selected, by ID — never by index, which moves
    /// under a reload.
    selected_id: Option<String>,
    /// The job this page started and still waits on (the slot itself is
    /// the window's, `JobSlot`).
    job_generation: Option<u64>,
    is_busy_shown: bool,
}

/// What `render` draws, taken from the state and released before any
/// widget is touched: a widget's own signal (a programmatic `select_row`)
/// must never find the state still borrowed.
struct Drawn {
    rows: Vec<(String, String)>,
    selected_index: Option<usize>,
    count: String,
    empty: Option<StringKey>,
    page_label: String,
    has_previous: bool,
    has_next: bool,
    is_busy: bool,
    has_selection: bool,
}

impl State {
    fn page_count_for(match_count: usize) -> usize {
        match_count.div_ceil(PAGE_SIZE).max(1)
    }

    fn page_count(&self) -> usize {
        Self::page_count_for(self.match_count)
    }

    fn selected_row(&self) -> Option<&CustomDictionaryRow> {
        let id = self.selected_id.as_deref()?;
        self.rows.iter().find(|row| row.id == id)
    }

    /// Against the counts, not against the filter box
    /// (`CustomDictionaryPage.swift:countLabel`).
    fn count_label(&self) -> String {
        if self.match_count < self.total_count {
            format!("{} / {}", self.match_count, self.total_count)
        } else {
            self.total_count.to_string()
        }
    }

    /// Which sentence the empty list shows, or `None` while there are rows:
    /// an empty dictionary is a STATE the + button answers, a filter
    /// matching nothing is a RESULT of what was typed.
    fn empty_state_key(&self) -> Option<StringKey> {
        if !self.rows.is_empty() {
            return None;
        }
        Some(if self.filter.is_empty() {
            StringKey::DictionaryCustomDictEmpty
        } else {
            StringKey::DictionaryNoResults
        })
    }
}

/// The page's widgets the state is drawn into.
struct Widgets {
    entries: adw::PreferencesGroup,
    filter: gtk::SearchEntry,
    list: gtk::ListBox,
    empty: gtk::Label,
    edit: gtk::Button,
    delete: gtk::Button,
    previous: gtk::Button,
    next: gtk::Button,
    page_label: gtk::Label,
    busy: gtk::Spinner,
    busy_box: gtk::Box,
    /// Everything a running job turns off.
    controls: Vec<gtk::Widget>,
}

pub struct CustomDictionaryPage {
    shell: Shell,
    strings: StringResolver,
    store: Arc<CustomDictionaryStore>,
    frequency: Arc<UserFrequencyStore>,
    association: Arc<UserAssociationStore>,
    learned_phrases: Arc<LearnedPhraseStore>,
    job_slot: JobSlot,
    state: RefCell<State>,
    widgets: Widgets,
    /// Set while `render` selects a row, so the list's own handler does not
    /// read it back as the user's choice.
    is_rendering: Cell<bool>,
}

pub fn build<'a>(mut context: PageContext<'a>, page: &adw::PreferencesPage) -> PageContext<'a> {
    let switches = adw::PreferencesGroup::new();
    context.switch_row(
        &switches,
        StringKey::DictionaryCustomDictEnabled,
        keys::IS_CUSTOM_DICT_ENABLED,
    );
    page.add(&switches);
    // No user-data directory: the stores never opened, and the banner at
    // the top of the window says so — nothing to list, nothing to write.
    let Some(stores) = context.stores else {
        return context;
    };
    let dictionary = CustomDictionaryPage::new(&context, stores, page);
    dictionary.load();
    context.retain(dictionary);
    context
}

impl CustomDictionaryPage {
    fn new(
        context: &PageContext<'_>,
        stores: &UserDataStores,
        page: &adw::PreferencesPage,
    ) -> Rc<Self> {
        let strings = *context.strings;
        // The entries: the filter, the list, the verbs and the pager.
        let entries = adw::PreferencesGroup::builder()
            .title(strings.resolve(StringKey::DesktopEntriesSection))
            .build();
        // The job's name beside a spinner, once it has run long enough to
        // say so (`busy_overlay` on Windows, the overlay card on the Mac).
        let busy = gtk::Spinner::new();
        let busy_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        busy_box.append(&busy);
        busy_box.append(&gtk::Label::new(Some(
            strings.resolve(StringKey::DesktopProgressWorking),
        )));
        busy_box.set_visible(false);
        entries.set_header_suffix(Some(&busy_box));
        let filter = gtk::SearchEntry::new();
        filter.set_placeholder_text(Some(
            strings.resolve(StringKey::DictionarySearchPlaceholder),
        ));
        entries.add(&filter);
        // The column names over the list (`DictionaryRomanLabel`,
        // `DictionaryHanziLabel`), in the rows' own two-column grid.
        let header = two_columns(
            &gtk::Label::builder()
                .label(strings.resolve(StringKey::DictionaryRomanLabel))
                .xalign(0.0)
                .css_classes(["heading"])
                .build(),
            &gtk::Label::builder()
                .label(strings.resolve(StringKey::DictionaryHanziLabel))
                .xalign(0.0)
                .css_classes(["heading"])
                .build(),
        );
        header.set_margin_start(TABLE_INSET);
        header.set_margin_end(TABLE_INSET);
        header.set_margin_bottom(6);
        entries.add(&header);
        let list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::Single)
            .css_classes(["boxed-list"])
            .build();
        let empty = gtk::Label::builder()
            .wrap(true)
            .css_classes(["dim-label"])
            .margin_top(12)
            .margin_bottom(12)
            .build();
        list.set_placeholder(Some(&empty));
        entries.add(&list);
        let verbs = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(6)
            .margin_top(6)
            .build();
        let add = icon_button(
            "list-add-symbolic",
            strings.resolve(StringKey::DictionaryAddEntry),
        );
        let edit = icon_button(
            "document-edit-symbolic",
            strings.resolve(StringKey::DictionaryEditEntry),
        );
        let delete = icon_button(
            "list-remove-symbolic",
            strings.resolve(StringKey::CommonDelete),
        );
        verbs.append(&add);
        verbs.append(&edit);
        verbs.append(&delete);
        let spacer = gtk::Box::builder().hexpand(true).build();
        verbs.append(&spacer);
        let previous = icon_button(
            "go-previous-symbolic",
            strings.resolve(StringKey::CommonPagePrevious),
        );
        let page_label = gtk::Label::new(None);
        let next = icon_button(
            "go-next-symbolic",
            strings.resolve(StringKey::CommonPageNext),
        );
        verbs.append(&previous);
        verbs.append(&page_label);
        verbs.append(&next);
        entries.add(&verbs);
        page.add(&entries);

        // Import / export, then the two destructive verbs, each its own
        // group (as on the other desktops).
        let csv = adw::PreferencesGroup::new();
        let import = action_row(
            &csv,
            strings.resolve(StringKey::DictionaryImportCSV),
            "document-open-symbolic",
        );
        let export = action_row(
            &csv,
            strings.resolve(StringKey::DictionaryExportCSV),
            "document-save-symbolic",
        );
        page.add(&csv);
        let delete_all_group = adw::PreferencesGroup::new();
        let delete_all = destructive_row(
            &delete_all_group,
            strings.resolve(StringKey::DictionaryDeleteAll),
            strings.resolve(StringKey::CommonDelete),
        );
        page.add(&delete_all_group);
        let clear_group = adw::PreferencesGroup::new();
        let clear = destructive_row(
            &clear_group,
            strings.resolve(StringKey::DesktopClearLearningRecords),
            strings.resolve(StringKey::CommonDelete),
        );
        page.add(&clear_group);

        let controls: Vec<gtk::Widget> = vec![
            filter.clone().upcast(),
            add.clone().upcast(),
            import.clone().upcast(),
            export.clone().upcast(),
            delete_all.clone().upcast(),
            clear.clone().upcast(),
        ];
        let this = Rc::new(Self {
            shell: context.shell.clone(),
            strings,
            store: Arc::clone(&stores.custom_dictionary),
            frequency: Arc::clone(&stores.frequency),
            association: Arc::clone(&stores.association),
            learned_phrases: Arc::clone(&stores.learned_phrases),
            job_slot: context.job_slot.clone(),
            state: RefCell::new(State::default()),
            is_rendering: Cell::new(false),
            widgets: Widgets {
                entries,
                filter,
                list,
                empty,
                edit,
                delete,
                previous,
                next,
                page_label,
                busy,
                busy_box,
                controls,
            },
        });
        this.connect(&add, &import, &export, &delete_all, &clear);
        this.render();
        this
    }

    fn connect(
        self: &Rc<Self>,
        add: &gtk::Button,
        import: &adw::ActionRow,
        export: &adw::ActionRow,
        delete_all: &gtk::Button,
        clear: &gtk::Button,
    ) {
        let weak = Rc::downgrade(self);
        // `changed`, not `search-changed`: the latter is GTK's own 150 ms
        // delay, and a load answered inside it would land under text the
        // box no longer shows. The generation moves at the keystroke; the
        // one settle is ours.
        self.widgets.filter.connect_changed(move |entry| {
            if let Some(page) = weak.upgrade() {
                page.filter_changed(entry.text().to_string());
            }
        });
        let weak = Rc::downgrade(self);
        self.widgets.list.connect_row_selected(move |_, row| {
            let Some(page) = weak.upgrade() else { return };
            if page.is_rendering.get() {
                return;
            }
            // A cleared selection is not acted on: single-select gives the
            // user no way to clear one, so the row the user chose stays.
            let Some(row) = row else { return };
            let id = page
                .state
                .borrow()
                .rows
                .get(row.index() as usize)
                .map(|row| row.id.clone());
            if id.is_some() {
                page.state.borrow_mut().selected_id = id;
                page.render_verbs();
            }
        });
        // A double-click (or Enter) on a row edits it, as on the Mac.
        let weak = Rc::downgrade(self);
        self.widgets.list.connect_row_activated(move |_, row| {
            let Some(page) = weak.upgrade() else { return };
            if page.job_slot.is_taken() {
                return;
            }
            let target = page.state.borrow().rows.get(row.index() as usize).cloned();
            if let Some(target) = target {
                page.entry_dialog(target);
            }
        });
        let weak = Rc::downgrade(self);
        add.connect_clicked(move |_| {
            if let Some(page) = weak.upgrade() {
                page.entry_dialog(CustomDictionaryRow::new("", ""));
            }
        });
        let weak = Rc::downgrade(self);
        self.widgets.edit.connect_clicked(move |_| {
            let Some(page) = weak.upgrade() else { return };
            let row = page.state.borrow().selected_row().cloned();
            if let Some(row) = row {
                page.entry_dialog(row);
            }
        });
        let weak = Rc::downgrade(self);
        self.widgets.delete.connect_clicked(move |_| {
            let Some(page) = weak.upgrade() else { return };
            let Some(id) = page.state.borrow().selected_id.clone() else {
                return;
            };
            let store = Arc::clone(&page.store);
            page.write(move || {
                store
                    .delete(&id)
                    .map(|_| ())
                    .map_err(|error| error.to_string())
            });
        });
        let weak = Rc::downgrade(self);
        self.widgets.previous.connect_clicked(move |_| {
            if let Some(page) = weak.upgrade() {
                page.step_page(-1);
            }
        });
        let weak = Rc::downgrade(self);
        self.widgets.next.connect_clicked(move |_| {
            if let Some(page) = weak.upgrade() {
                page.step_page(1);
            }
        });
        let weak = Rc::downgrade(self);
        import.connect_activated(move |_| {
            if let Some(page) = weak.upgrade() {
                page.import();
            }
        });
        let weak = Rc::downgrade(self);
        export.connect_activated(move |_| {
            if let Some(page) = weak.upgrade() {
                page.export();
            }
        });
        let weak = Rc::downgrade(self);
        delete_all.connect_clicked(move |_| {
            if let Some(page) = weak.upgrade() {
                page.ask(Confirm::DeleteAll);
            }
        });
        let weak = Rc::downgrade(self);
        clear.connect_clicked(move |_| {
            if let Some(page) = weak.upgrade() {
                page.ask(Confirm::ClearLearningRecords);
            }
        });
    }

    /// The reload waits for the box to settle, so a word typed letter by
    /// letter is one query, not six.
    fn filter_changed(self: &Rc<Self>, filter: String) {
        let generation = {
            let mut state = self.state.borrow_mut();
            if filter == state.filter {
                return;
            }
            state.filter = filter;
            state.load_generation = state.load_generation.wrapping_add(1);
            state.load_generation
        };
        let weak = Rc::downgrade(self);
        glib::timeout_add_local_once(FILTER_SETTLE, move || {
            let Some(page) = weak.upgrade() else { return };
            if page.state.borrow().load_generation != generation {
                return;
            }
            page.state.borrow_mut().page = 0;
            page.load();
        });
    }

    /// How many rows the list shows (for the tests).
    pub fn shown_row_count(&self) -> usize {
        self.state.borrow().rows.len()
    }

    /// Reloads the page on screen (for the tests; the page reloads itself
    /// after every write).
    pub fn reload(self: &Rc<Self>) {
        self.load();
    }

    fn step_page(self: &Rc<Self>, delta: isize) {
        {
            let mut state = self.state.borrow_mut();
            let count = state.page_count();
            let target = state.page as isize + delta;
            if target < 0 || target as usize >= count {
                return;
            }
            state.page = target as usize;
        }
        self.load();
    }

    /// Starts a load of the page on screen, pulling it back inside the
    /// list if the list shrank under it. A load has no overlay: the rows
    /// already on screen stay put while it runs.
    fn load(self: &Rc<Self>) {
        let (generation, filter, wanted_page) = {
            let mut state = self.state.borrow_mut();
            state.load_generation = state.load_generation.wrapping_add(1);
            (
                state.load_generation,
                state.filter.trim().to_owned(),
                state.page,
            )
        };
        let store = Arc::clone(&self.store);
        let weak = Rc::downgrade(self);
        jobs::spawn(
            move || {
                let match_count = store.count_matching(&filter)?;
                let page = wanted_page.min(State::page_count_for(match_count) - 1);
                let rows = store.rows(&filter, PAGE_SIZE, page * PAGE_SIZE)?;
                let total_count = if filter.is_empty() {
                    match_count
                } else {
                    store.count()?
                };
                Ok::<_, taigi_desktop_storage::CustomDictionaryError>(Loaded {
                    page,
                    rows,
                    match_count,
                    total_count,
                })
            },
            move |outcome| {
                let Some(page) = weak.upgrade() else { return };
                if page.state.borrow().load_generation != generation {
                    return;
                }
                match outcome {
                    Some(Ok(loaded)) => {
                        let mut state = page.state.borrow_mut();
                        state.page = loaded.page;
                        state.rows = loaded.rows;
                        state.match_count = loaded.match_count;
                        state.total_count = loaded.total_count;
                        // A selection the new page does not hold is no
                        // selection.
                        if state.selected_row().is_none() {
                            state.selected_id = None;
                        }
                    }
                    // Not an empty list: "empty" and "could not be read"
                    // look the same on screen, and only one is worth doing
                    // something about. The rows already shown stay.
                    Some(Err(error)) => page.report(PageMessage::failure(
                        StringKey::DesktopCustomDictReadFailed,
                        error,
                    )),
                    None => page.report(PageMessage::failure(
                        StringKey::DesktopCustomDictReadFailed,
                        "the load did not finish",
                    )),
                }
                page.render();
            },
        );
    }

    /// Takes the window's one work slot for `job`, or does nothing because
    /// something else holds it (the macOS model: refused, not queued). The
    /// slot and the notice outlive this page: a page rebuilt under a
    /// running job (a display language change) finds the slot taken, and
    /// the outcome still reaches the user as a toast.
    fn begin_job(self: &Rc<Self>, job: impl FnOnce() -> JobOutcome + Send + 'static) {
        let Some(generation) = self.job_slot.take() else {
            return;
        };
        {
            let mut state = self.state.borrow_mut();
            state.job_generation = Some(generation);
            state.is_busy_shown = false;
        }
        let weak = Rc::downgrade(self);
        let slot = self.job_slot.clone();
        let shell = self.shell.clone();
        let strings = self.strings;
        jobs::spawn(job, move |outcome| {
            slot.release(generation);
            let outcome = outcome.unwrap_or_else(|| JobOutcome {
                message: Some(PageMessage::failure(
                    StringKey::DesktopCustomDictWriteFailed,
                    "the operation did not finish",
                )),
                is_reload_wanted: true,
            });
            if let Some(message) = outcome.message {
                shell.report(&message, &strings);
            }
            let Some(page) = weak.upgrade() else { return };
            if page.state.borrow().job_generation != Some(generation) {
                return;
            }
            {
                let mut state = page.state.borrow_mut();
                state.job_generation = None;
                state.is_busy_shown = false;
            }
            if outcome.is_reload_wanted {
                page.load();
            } else {
                page.render();
            }
        });
        // The overlay waits, so a millisecond-long write does not flash it.
        let weak = Rc::downgrade(self);
        glib::timeout_add_local_once(OVERLAY_DELAY, move || {
            let Some(page) = weak.upgrade() else { return };
            if page.state.borrow().job_generation == Some(generation) {
                page.state.borrow_mut().is_busy_shown = true;
                page.render();
            }
        });
    }

    /// A write that answers with nothing but its failure, and asks for a
    /// reload either way.
    fn write(self: &Rc<Self>, body: impl FnOnce() -> Result<(), String> + Send + 'static) {
        self.begin_job(move || JobOutcome {
            message: body()
                .err()
                .map(|error| PageMessage::failure(StringKey::DesktopCustomDictWriteFailed, error)),
            is_reload_wanted: true,
        });
    }

    /// Add or edit one entry (`CustomDictionaryEntrySheet`): two entry rows
    /// in an alert dialog; Save only with a romanization, which is what the
    /// entry is found by.
    fn entry_dialog(self: &Rc<Self>, original: CustomDictionaryRow) {
        let is_new = original.roman.is_empty();
        let strings = self.strings;
        let dialog = adw::AlertDialog::new(
            Some(strings.resolve(if is_new {
                StringKey::DictionaryAddEntry
            } else {
                StringKey::DictionaryEditEntry
            })),
            None,
        );
        let fields = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(["boxed-list"])
            .build();
        let roman = adw::EntryRow::builder()
            .title(strings.resolve(StringKey::DictionaryRomanLabel))
            .text(&original.roman)
            .use_markup(false)
            .build();
        let hanzi = adw::EntryRow::builder()
            .title(strings.resolve(StringKey::DictionaryHanziLabel))
            .text(&original.hanzi)
            .use_markup(false)
            .build();
        fields.append(&roman);
        fields.append(&hanzi);
        dialog.set_extra_child(Some(&fields));
        dialog.add_response("cancel", strings.resolve(StringKey::CommonCancel));
        dialog.add_response("save", strings.resolve(StringKey::CommonSave));
        dialog.set_response_appearance("save", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("save"));
        dialog.set_close_response("cancel");
        dialog.set_response_enabled("save", !original.roman.trim().is_empty());
        // Weak: the dialog holds the row, and a row holding the dialog back
        // would keep both alive after the dialog closed.
        let dialog_weak = dialog.downgrade();
        roman.connect_changed(move |entry| {
            if let Some(dialog) = dialog_weak.upgrade() {
                dialog.set_response_enabled("save", !entry.text().trim().is_empty());
            }
        });
        let weak = Rc::downgrade(self);
        let (roman_field, hanzi_field) = (roman.clone(), hanzi.clone());
        dialog.connect_response(None, move |_, response| {
            if response != "save" {
                return;
            }
            let Some(page) = weak.upgrade() else { return };
            let roman = roman_field.text().trim().to_owned();
            if roman.is_empty() {
                return;
            }
            // The ID is the original's: an edit is an edit, not a new
            // entry that happens to replace one.
            let mut row = original.clone();
            row.roman = roman;
            row.hanzi = hanzi_field.text().trim().to_owned();
            row.updated_at = utc_timestamp_now();
            let store = Arc::clone(&page.store);
            page.write(move || store.upsert(&row).map_err(|error| error.to_string()));
        });
        dialog.present(self.shell.window().as_ref());
    }

    /// A destructive command asks first: the primary button is the
    /// destructive one; Escape, the close button and a dismissal all leave
    /// the store alone.
    fn ask(self: &Rc<Self>, confirm: Confirm) {
        if self.job_slot.is_taken() {
            return;
        }
        let strings = self.strings;
        let dialog = adw::AlertDialog::new(
            Some(strings.resolve(confirm.title_key())),
            confirm.message_key().map(|key| strings.resolve(key)),
        );
        dialog.add_response("cancel", strings.resolve(StringKey::CommonCancel));
        dialog.add_response("delete", strings.resolve(StringKey::CommonDelete));
        dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
        dialog.set_close_response("cancel");
        let weak = Rc::downgrade(self);
        dialog.connect_response(None, move |_, response| {
            if response != "delete" {
                return;
            }
            let Some(page) = weak.upgrade() else { return };
            match confirm {
                Confirm::DeleteAll => {
                    let store = Arc::clone(&page.store);
                    page.write(move || {
                        store
                            .delete_all()
                            .map(|_| ())
                            .map_err(|error| error.to_string())
                    });
                }
                Confirm::ClearLearningRecords => page.clear_learning_records(),
            }
        });
        dialog.present(self.shell.window().as_ref());
    }

    /// Deletes the three learning tables — three calls, three files, no
    /// transaction that could span them; each is attempted even when an
    /// earlier one fails, and the notice reports rather than claims.
    fn clear_learning_records(self: &Rc<Self>) {
        let frequency = Arc::clone(&self.frequency);
        let association = Arc::clone(&self.association);
        let learned_phrases = Arc::clone(&self.learned_phrases);
        self.begin_job(move || {
            let mut failures = Vec::new();
            if let Err(error) = frequency.delete_all() {
                failures.push(format!("user_frequency: {error}"));
            }
            if let Err(error) = association.delete_all() {
                failures.push(format!("user_association: {error}"));
            }
            if let Err(error) = learned_phrases.delete_all() {
                failures.push(format!("learned_phrases: {error}"));
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
        });
    }

    fn export(self: &Rc<Self>) {
        if self.job_slot.is_taken() {
            return;
        }
        let dialog = gtk::FileDialog::new();
        dialog.set_initial_name(Some(&format!(
            "taigi_custom_dictionary_{}.csv",
            local_date()
        )));
        let weak = Rc::downgrade(self);
        dialog.save(
            self.shell.window().as_ref(),
            gio::Cancellable::NONE,
            move |result| {
                let Some(page) = weak.upgrade() else { return };
                let Some(path) = page.chosen_path(result, StringKey::CommonExportFailed) else {
                    return;
                };
                // The WHOLE dictionary, not the page or the filter's matches.
                let store = Arc::clone(&page.store);
                page.begin_job(move || {
                    let outcome = store
                        .all_rows()
                        .map_err(|error| error.to_string())
                        .and_then(|rows| {
                            write_atomically(&path, CustomDictionaryCSV::encode(&rows))
                        });
                    JobOutcome {
                        message: outcome.err().map(|error| {
                            PageMessage::failure(StringKey::CommonExportFailed, error)
                        }),
                        is_reload_wanted: false,
                    }
                });
            },
        );
    }

    fn import(self: &Rc<Self>) {
        if self.job_slot.is_taken() {
            return;
        }
        let dialog = gtk::FileDialog::new();
        let weak = Rc::downgrade(self);
        dialog.open(
            self.shell.window().as_ref(),
            gio::Cancellable::NONE,
            move |result| {
                let Some(page) = weak.upgrade() else { return };
                let Some(path) = page.chosen_path(result, StringKey::CommonImportFailed) else {
                    return;
                };
                let store = Arc::clone(&page.store);
                page.begin_job(move || {
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
                            Err(error) => {
                                PageMessage::failure(StringKey::CommonImportFailed, error)
                            }
                        },
                    };
                    JobOutcome {
                        message: Some(message),
                        is_reload_wanted: true,
                    }
                });
            },
        );
    }

    /// The file the chooser answered: a dismissal is nothing, any other
    /// refusal — and a file with no local path — is said as `failure`.
    fn chosen_path(
        &self,
        result: Result<gio::File, glib::Error>,
        failure: StringKey,
    ) -> Option<std::path::PathBuf> {
        super::chosen_path(result).unwrap_or_else(|error| {
            self.report(PageMessage::failure(failure, error));
            None
        })
    }

    fn report(&self, message: PageMessage) {
        self.shell.report(&message, &self.strings);
    }

    /// The state, drawn: the rows, the count, the empty sentence, the
    /// verbs, the pager, the busy spinner. The state is read into `Drawn`
    /// and released first: `select_row` fires the list's handler
    /// synchronously.
    fn render(&self) {
        let drawn = {
            let state = self.state.borrow();
            Drawn {
                rows: state
                    .rows
                    .iter()
                    .map(|row| (row.roman.clone(), row.hanzi.clone()))
                    .collect(),
                selected_index: state
                    .selected_id
                    .as_deref()
                    .and_then(|id| state.rows.iter().position(|row| row.id == id)),
                count: state.count_label(),
                empty: state.empty_state_key(),
                page_label: format!("{} / {}", state.page + 1, state.page_count()),
                has_previous: state.page > 0,
                has_next: state.page + 1 < state.page_count(),
                is_busy: state.is_busy_shown,
                has_selection: state.selected_row().is_some(),
            }
        };
        let widgets = &self.widgets;
        widgets.entries.set_description(Some(&drawn.count));
        remove_rows(&widgets.list);
        for (roman, hanzi) in &drawn.rows {
            // Two columns, romanization then Hanji (the Mac's table). Plain labels:
            // user text, never markup.
            let cells = two_columns(
                &gtk::Label::builder()
                    .label(roman)
                    .xalign(0.0)
                    .ellipsize(gtk::pango::EllipsizeMode::End)
                    .build(),
                &gtk::Label::builder()
                    .label(hanzi)
                    .xalign(0.0)
                    .ellipsize(gtk::pango::EllipsizeMode::End)
                    .build(),
            );
            cells.set_margin_start(TABLE_INSET);
            cells.set_margin_end(TABLE_INSET);
            cells.set_margin_top(ROW_INSET);
            cells.set_margin_bottom(ROW_INSET);
            widgets
                .list
                .append(&gtk::ListBoxRow::builder().child(&cells).build());
        }
        self.is_rendering.set(true);
        match drawn.selected_index {
            Some(index) => widgets
                .list
                .select_row(widgets.list.row_at_index(index as i32).as_ref()),
            None => widgets.list.unselect_all(),
        }
        self.is_rendering.set(false);
        match drawn.empty {
            Some(key) => {
                widgets.empty.set_label(self.strings.resolve(key));
                widgets.empty.set_visible(true);
            }
            None => widgets.empty.set_visible(false),
        }
        widgets.page_label.set_label(&drawn.page_label);
        let is_enabled = !drawn.is_busy;
        widgets
            .previous
            .set_sensitive(is_enabled && drawn.has_previous);
        widgets.next.set_sensitive(is_enabled && drawn.has_next);
        widgets.busy_box.set_visible(drawn.is_busy);
        widgets.busy.set_spinning(drawn.is_busy);
        for control in &widgets.controls {
            control.set_sensitive(is_enabled);
        }
        widgets
            .edit
            .set_sensitive(is_enabled && drawn.has_selection);
        widgets
            .delete
            .set_sensitive(is_enabled && drawn.has_selection);
    }

    fn render_verbs(&self) {
        let (is_busy, has_selection) = {
            let state = self.state.borrow();
            (state.is_busy_shown, state.selected_row().is_some())
        };
        self.widgets.edit.set_sensitive(!is_busy && has_selection);
        self.widgets.delete.set_sensitive(!is_busy && has_selection);
    }
}

/// The table's horizontal inset and a row's vertical one, the list's own
/// row metrics (`adw::ActionRow`).
const TABLE_INSET: i32 = 12;
const ROW_INSET: i32 = 8;

/// Two equal columns side by side (`Metrics.tableColumns` on the Mac).
fn two_columns(left: &impl IsA<gtk::Widget>, right: &impl IsA<gtk::Widget>) -> gtk::Box {
    let columns = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .homogeneous(true)
        .spacing(12)
        .build();
    columns.append(left);
    columns.append(right);
    columns
}

/// An activatable row that runs a command (import, export).
fn action_row(group: &adw::PreferencesGroup, title: &str, icon: &str) -> adw::ActionRow {
    let row = adw::ActionRow::builder()
        .title(title)
        .activatable(true)
        .build();
    row.add_suffix(&gtk::Image::from_icon_name(icon));
    group.add(&row);
    row
}

/// A row whose button empties a store; the button is the destructive one.
fn destructive_row(group: &adw::PreferencesGroup, title: &str, verb: &str) -> gtk::Button {
    let button = gtk::Button::builder()
        .label(verb)
        .valign(gtk::Align::Center)
        .css_classes(["destructive-action"])
        .build();
    let row = adw::ActionRow::builder().title(title).build();
    row.add_suffix(&button);
    row.set_activatable_widget(Some(&button));
    group.add(&row);
    button
}

/// `YYYY-MM-DD` in local time for the export's suggested name.
fn local_date() -> String {
    glib::DateTime::now_local()
        .and_then(|now| now.format("%Y-%m-%d"))
        .map(|date| date.to_string())
        .unwrap_or_else(|_| "export".to_owned())
}

/// `Data.write(to:options:.atomic)`: the file appears whole or not at all,
/// and a failed export never damages the one it would have replaced. The
/// temporary file is created exclusively in the target's directory (no
/// name to collide with, no symlink to follow) and removed with its handle
/// if the write fails.
fn write_atomically(path: &std::path::Path, contents: String) -> Result<(), String> {
    use std::io::Write;
    let directory = path.parent().ok_or_else(|| "no directory".to_owned())?;
    let mut temporary = tempfile::Builder::new()
        .prefix(".taigi-export-")
        .suffix(".tmp")
        .tempfile_in(directory)
        .map_err(|error| error.to_string())?;
    temporary
        .write_all(contents.as_bytes())
        .map_err(|error| error.to_string())?;
    temporary
        .persist(path)
        .map(|_| ())
        .map_err(|error| error.error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(match_count: usize, total_count: usize, filter: &str) -> State {
        State {
            match_count,
            total_count,
            filter: filter.to_owned(),
            ..State::default()
        }
    }

    #[test]
    fn the_count_reads_as_one_number_until_a_filter_actually_narrows_it() {
        // trace: `CustomDictionaryPage.swift:countLabel`.
        assert_eq!(state(2, 2, "").count_label(), "2");
        assert_eq!(state(2, 2, "tsia").count_label(), "2");
        assert_eq!(state(1, 2, "tsia").count_label(), "1 / 2");
    }

    #[test]
    fn an_empty_list_says_which_kind_of_empty_it_is() {
        assert_eq!(
            state(0, 0, "").empty_state_key(),
            Some(StringKey::DictionaryCustomDictEmpty)
        );
        assert_eq!(
            state(0, 2, "zzz").empty_state_key(),
            Some(StringKey::DictionaryNoResults)
        );
        let mut listed = state(1, 1, "");
        listed.rows.push(CustomDictionaryRow::new("tsia̍h", "食"));
        assert_eq!(listed.empty_state_key(), None);
    }

    #[test]
    fn pages_are_ten_rows_and_never_fewer_than_one() {
        // trace: 0 → 1 page, 10 → 1, 11 → 2, 25 → 3.
        assert_eq!(State::page_count_for(0), 1);
        assert_eq!(State::page_count_for(10), 1);
        assert_eq!(State::page_count_for(11), 2);
        assert_eq!(State::page_count_for(25), 3);
    }
}
