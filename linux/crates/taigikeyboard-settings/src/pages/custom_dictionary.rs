//! Custom Dictionary: the words the user added themselves. Port of
//! `CustomDictionaryPage.swift` / the Windows `custom_dictionary.rs`: rows
//! fetched one PAGE at a time (10), a filter that reloads once it settles,
//! a list whose selection drives the add / edit / delete trio, the pager
//! under it, CSV import / export, and delete all.
//!
//! The listing rules and every job body are
//! `taigi_desktop_core::settings::custom_dictionary`'s, shared with the
//! Windows pane; this file draws them and runs them. Every engine request —
//! the loads included — runs off the UI thread (`jobs::spawn`): the newest
//! load wins by generation, and the writes share the window's one work
//! slot, `JobSlot` (refused, not queued — a queued delete would name a row
//! the list may no longer show).
//!
//! The table is the Mac's: two columns, romanization then Hanji, under a header,
//! a double-click (or Enter) on a row edits it, the ✎ button over the
//! selection is the keyboard-reachable way (the Windows shape). The empty
//! list says so in words.

use super::{
    append_pager, busy_indicator, confirm, destructive_row, icon_button, remove_rows, table_line,
    PageContext,
};
use crate::jobs;
use crate::window::{JobSlot, Shell};
use adw::prelude::*;
use gtk::{gio, glib};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use taigi_desktop_core::engine::user_data::CustomDictionaryEntry;
use taigi_desktop_core::settings::custom_dictionary::{
    delete_all_job, delete_entry_job, export_file_name, export_job, fetch, import_job,
    save_entry_job, Listing, DELETE_ALL,
};
use taigi_desktop_core::settings::keys;
use taigi_desktop_core::settings::listing::{
    JobOutcome, JobState, LoadLanded, FILTER_SETTLE, LOAD_DID_NOT_FINISH, OVERLAY_DELAY,
};
use taigi_desktop_core::settings::presentation::PageMessage;
use taigi_desktop_core::strings::{StringKey, StringResolver};

#[derive(Default)]
struct State {
    listing: Listing,
    /// The job this page started and still waits on (the slot itself is
    /// the window's, `JobSlot`).
    job: JobState,
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
    if !context.has_user_data {
        return context;
    }
    let dictionary = CustomDictionaryPage::new(&context, page);
    dictionary.load();
    context.retain(dictionary);
    context
}

impl CustomDictionaryPage {
    fn new(context: &PageContext<'_>, page: &adw::PreferencesPage) -> Rc<Self> {
        let strings = *context.strings;
        // The entries: the filter, the list, the verbs and the pager.
        let entries = adw::PreferencesGroup::builder()
            .title(strings.resolve(StringKey::DesktopEntriesSection))
            .build();
        let (busy, busy_box) = busy_indicator(&strings);
        entries.set_header_suffix(Some(&busy_box));
        let filter = gtk::SearchEntry::new();
        filter.set_placeholder_text(Some(
            strings.resolve(StringKey::DictionarySearchPlaceholder),
        ));
        entries.add(&filter);
        // The column names over the list, in the rows' own two-column grid.
        entries.add(&table_line(
            [
                strings.resolve(StringKey::DictionaryRomanLabel),
                strings.resolve(StringKey::DictionaryHanziLabel),
            ],
            true,
        ));
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
        let (previous, page_label, next) = append_pager(&verbs, &strings);
        entries.add(&verbs);
        page.add(&entries);

        // Import / export, then Delete All in its own group (as on the
        // other desktops).
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

        let controls: Vec<gtk::Widget> = vec![
            filter.clone().upcast(),
            add.clone().upcast(),
            import.clone().upcast(),
            export.clone().upcast(),
            delete_all.clone().upcast(),
        ];
        let this = Rc::new(Self {
            shell: context.shell.clone(),
            strings,
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
        this.connect(&add, &import, &export, &delete_all);
        this.render();
        this
    }

    fn connect(
        self: &Rc<Self>,
        add: &gtk::Button,
        import: &adw::ActionRow,
        export: &adw::ActionRow,
        delete_all: &gtk::Button,
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
                .listing
                .rows
                .get(row.index() as usize)
                .map(|row| row.id.clone());
            if id.is_some() {
                page.state.borrow_mut().listing.selected_id = id;
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
            let target = page
                .state
                .borrow()
                .listing
                .rows
                .get(row.index() as usize)
                .cloned();
            if let Some(target) = target {
                page.entry_dialog(target);
            }
        });
        let weak = Rc::downgrade(self);
        add.connect_clicked(move |_| {
            if let Some(page) = weak.upgrade() {
                page.entry_dialog(CustomDictionaryEntry::default());
            }
        });
        let weak = Rc::downgrade(self);
        self.widgets.edit.connect_clicked(move |_| {
            let Some(page) = weak.upgrade() else { return };
            let row = page.state.borrow().listing.selected_row().cloned();
            if let Some(row) = row {
                page.entry_dialog(row);
            }
        });
        let weak = Rc::downgrade(self);
        self.widgets.delete.connect_clicked(move |_| {
            let Some(page) = weak.upgrade() else { return };
            let Some(id) = page.state.borrow().listing.selected_id.clone() else {
                return;
            };
            page.begin_job(move || delete_entry_job(&id));
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
                page.ask_delete_all();
            }
        });
    }

    /// The reload waits for the box to settle, so a word typed letter by
    /// letter is one query, not six.
    fn filter_changed(self: &Rc<Self>, filter: String) {
        let Some(generation) = self.state.borrow_mut().listing.set_filter(filter) else {
            return;
        };
        let weak = Rc::downgrade(self);
        glib::timeout_add_local_once(FILTER_SETTLE, move || {
            let Some(page) = weak.upgrade() else { return };
            let is_settled = page.state.borrow_mut().listing.settle(generation);
            if is_settled {
                page.load();
            }
        });
    }

    /// How many rows the list shows (for the tests).
    pub fn shown_row_count(&self) -> usize {
        self.state.borrow().listing.rows.len()
    }

    /// Reloads the page on screen (for the tests; the page reloads itself
    /// after every write).
    pub fn reload(self: &Rc<Self>) {
        self.load();
    }

    fn step_page(self: &Rc<Self>, delta: isize) {
        let has_stepped = self.state.borrow_mut().listing.step_page(delta);
        if has_stepped {
            self.load();
        }
    }

    /// Starts a load of the page on screen. A load has no overlay: the rows
    /// already on screen stay put while it runs.
    fn load(self: &Rc<Self>) {
        let request = self.state.borrow_mut().listing.begin_load();
        let generation = request.generation;
        let weak = Rc::downgrade(self);
        jobs::spawn(
            move || fetch(&request),
            move |outcome| {
                let Some(page) = weak.upgrade() else { return };
                let outcome = outcome.unwrap_or_else(|| Err(LOAD_DID_NOT_FINISH.to_owned()));
                // Landed and released before the notice or the render
                // touches a widget.
                let landed = page.state.borrow_mut().listing.land(generation, outcome);
                match landed {
                    LoadLanded::Stale => return,
                    LoadLanded::Adopted => {}
                    LoadLanded::Failed(notice) => page.report(notice),
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
        self.state.borrow_mut().job.start(generation);
        let weak = Rc::downgrade(self);
        let slot = self.job_slot.clone();
        let shell = self.shell.clone();
        let strings = self.strings;
        jobs::spawn(job, move |outcome| {
            slot.release(generation);
            let outcome =
                outcome.unwrap_or_else(JobOutcome::did_not_finish::<CustomDictionaryEntry>);
            if let Some(message) = outcome.message {
                shell.report(&message, &strings);
            }
            let Some(page) = weak.upgrade() else { return };
            let is_this_pages = page.state.borrow_mut().job.finish(generation);
            if !is_this_pages {
                return;
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
            let is_busy = page.state.borrow_mut().job.show_busy(generation);
            if is_busy {
                page.render();
            }
        });
    }

    /// Add or edit one entry (`CustomDictionaryEntrySheet`): two entry rows
    /// in an alert dialog; Save only with a romanization, which is what the
    /// entry is found by.
    fn entry_dialog(self: &Rc<Self>, original: CustomDictionaryEntry) {
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
        let hanji = adw::EntryRow::builder()
            .title(strings.resolve(StringKey::DictionaryHanziLabel))
            .text(&original.hanji)
            .use_markup(false)
            .build();
        fields.append(&roman);
        fields.append(&hanji);
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
        let (roman_field, hanji_field) = (roman.clone(), hanji.clone());
        dialog.connect_response(None, move |_, response| {
            if response != "save" {
                return;
            }
            let Some(page) = weak.upgrade() else { return };
            let roman = roman_field.text().to_string();
            if roman.trim().is_empty() {
                return;
            }
            let id = original.id.clone();
            let hanji = hanji_field.text().to_string();
            page.begin_job(move || save_entry_job(&id, &roman, &hanji));
        });
        dialog.present(self.shell.window().as_ref());
    }

    /// Delete All asks first (`Confirmation` says why).
    fn ask_delete_all(self: &Rc<Self>) {
        if self.job_slot.is_taken() {
            return;
        }
        let weak = Rc::downgrade(self);
        confirm(&self.shell, self.strings, DELETE_ALL, move || {
            if let Some(page) = weak.upgrade() {
                page.begin_job(delete_all_job);
            }
        });
    }

    fn export(self: &Rc<Self>) {
        if self.job_slot.is_taken() {
            return;
        }
        let dialog = gtk::FileDialog::new();
        dialog.set_initial_name(Some(&export_file_name(&local_date())));
        let weak = Rc::downgrade(self);
        dialog.save(
            self.shell.window().as_ref(),
            gio::Cancellable::NONE,
            move |result| {
                let Some(page) = weak.upgrade() else { return };
                let Some(path) = page.chosen_path(result, StringKey::CommonExportFailed) else {
                    return;
                };
                page.begin_job(move || export_job(&path, write_atomically));
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
                page.begin_job(move || import_job(&path));
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
            let listing = &state.listing;
            Drawn {
                rows: listing
                    .rows
                    .iter()
                    .map(|row| (row.roman.clone(), row.hanji.clone()))
                    .collect(),
                selected_index: listing.selected_index(),
                count: listing.count_label(),
                empty: listing.empty_state_key(),
                page_label: format!("{} / {}", listing.page + 1, listing.page_count()),
                has_previous: listing.page > 0,
                has_next: listing.page + 1 < listing.page_count(),
                is_busy: state.job.is_busy_shown(),
                has_selection: listing.selected_row().is_some(),
            }
        };
        let widgets = &self.widgets;
        widgets.entries.set_description(Some(&drawn.count));
        remove_rows(&widgets.list);
        for (roman, hanji) in &drawn.rows {
            // Two columns, romanization then Hanji (the Mac's table).
            let cells = table_line([roman.as_str(), hanji.as_str()], false);
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
            (
                state.job.is_busy_shown(),
                state.listing.selected_row().is_some(),
            )
        };
        self.widgets.edit.set_sensitive(!is_busy && has_selection);
        self.widgets.delete.set_sensitive(!is_busy && has_selection);
    }
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
fn write_atomically(path: &std::path::Path, contents: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let directory = path.parent().ok_or_else(|| "no directory".to_owned())?;
    let mut temporary = tempfile::Builder::new()
        .prefix(".taigi-export-")
        .suffix(".tmp")
        .tempfile_in(directory)
        .map_err(|error| error.to_string())?;
    temporary
        .write_all(contents)
        .map_err(|error| error.to_string())?;
    temporary
        .persist(path)
        .map(|_| ())
        .map_err(|error| error.error.to_string())
}
