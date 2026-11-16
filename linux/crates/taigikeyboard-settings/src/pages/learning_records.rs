//! Learning Records: what the keyboard learned from the user's picks. The
//! Custom Dictionary page's shape (`custom_dictionary.rs`) over the engine's
//! learning stores: a kind picker (word frequency, phrases) and an order
//! picker over a filter, rows fetched one PAGE at a time (10), a list whose
//! selection drives the edit-count / add-to-custom-dictionary / delete
//! verbs (no add on word frequency: its words are dictionary words
//! already), and the pager under it — ✎ or a double-click opens the record
//! dialog: the count and, for a row the engine says can be added, an Add to
//! Custom Dictionary tab beside it — then Delete Learning Records in its own
//! group: it empties every learning store, whichever kind is on screen, on
//! the press. No add — a word the user wants is a custom word, which a
//! listed row's word can become here.
//!
//! The listing rules and every job body are
//! `taigi_desktop_core::settings::learning_records`'s, shared with the
//! Windows pane; this file draws them and runs them, off the UI thread, in
//! the window's one work slot (`JobSlot`) — as Custom Dictionary does.

use super::custom_dictionary::word_dialog;
use super::{
    append_pager, busy_indicator, destructive_row, icon_button, remove_rows, table_line,
    PageContext,
};
use crate::jobs;
use crate::window::{JobSlot, Shell};
use adw::prelude::*;
use gtk::glib;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use taigi_desktop_core::engine::user_data::{
    LearningRecord, LearningRecordKind, LearningRecordOrder,
};
use taigi_desktop_core::settings::keys;
use taigi_desktop_core::settings::learning_records::{
    add_to_custom_dictionary_job, clear_all_job, count_note, delete_job, fetch, last_used_label,
    offers_add_to_custom_dictionary, order_label, set_count_job, Listing, KINDS, MAX_COUNT, ORDERS,
};
use taigi_desktop_core::settings::listing::{
    JobOutcome, JobState, LoadLanded, FILTER_SETTLE, LOAD_DID_NOT_FINISH, OVERLAY_DELAY,
};
use taigi_desktop_core::settings::presentation::PageMessage;
use taigi_desktop_core::strings::{StringKey, StringResolver};

/// `value`'s row in a picker over `roster`; the first row for a value the
/// roster does not list.
fn roster_index<T: PartialEq, const N: usize>(roster: [T; N], value: T) -> u32 {
    roster
        .iter()
        .position(|listed| *listed == value)
        .unwrap_or(0) as u32
}

struct State {
    listing: Listing,
    kind: LearningRecordKind,
    order: LearningRecordOrder,
    /// The job this page started and still waits on (the slot itself is
    /// the window's, `JobSlot`).
    job: JobState,
}

/// What `render` draws, taken from the state and released before any
/// widget is touched: a widget's own signal (a programmatic `select_row`)
/// must never find the state still borrowed.
struct Drawn {
    rows: Vec<[String; 4]>,
    selected_index: Option<usize>,
    count: String,
    empty: Option<StringKey>,
    page_label: String,
    has_previous: bool,
    has_next: bool,
    is_busy: bool,
}

/// The page's widgets the state is drawn into.
struct Widgets {
    entries: adw::PreferencesGroup,
    list: gtk::ListBox,
    empty: gtk::Label,
    edit: gtk::Button,
    add_to_custom_dictionary: gtk::Button,
    delete: gtk::Button,
    previous: gtk::Button,
    next: gtk::Button,
    page_label: gtk::Label,
    busy: gtk::Spinner,
    busy_box: gtk::Box,
    /// Everything a running job turns off.
    controls: Vec<gtk::Widget>,
}

pub struct LearningRecordsPage {
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
    // No user-data directory: the stores never opened, and the banner at
    // the top of the window says so — nothing to list.
    if !context.has_user_data {
        return context;
    }
    let records = LearningRecordsPage::new(&context, page);
    records.load();
    context.retain(records);
    context
}

impl LearningRecordsPage {
    fn new(context: &PageContext<'_>, page: &adw::PreferencesPage) -> Rc<Self> {
        let strings = *context.strings;
        // Which rows, in what order: the last pick (#773). Selected at
        // build, before `connect`, so restoring it is no pick of its own.
        let restored_kind = context.document.choice(&keys::LEARNING_RECORDS_KIND);
        let restored_order = context.document.choice(&keys::LEARNING_RECORDS_ORDER);
        let pickers = adw::PreferencesGroup::new();
        let kind_labels: Vec<&str> = KINDS.iter().map(|(_, key)| strings.resolve(*key)).collect();
        let kind = adw::ComboRow::builder()
            .title(strings.resolve(StringKey::DictionaryLearningRecords))
            .model(&gtk::StringList::new(&kind_labels))
            .selected(roster_index(KINDS.map(|(kind, _)| kind), restored_kind))
            .build();
        let order_labels: Vec<&str> = ORDERS
            .iter()
            .map(|order| strings.resolve(order_label(*order)))
            .collect();
        let order = adw::ComboRow::builder()
            .title(strings.resolve(StringKey::DictionaryLearningRecordsOrder))
            .model(&gtk::StringList::new(&order_labels))
            .selected(roster_index(ORDERS, restored_order))
            .build();
        pickers.add(&kind);
        pickers.add(&order);
        page.add(&pickers);

        // The rows: the filter, the list, the verbs and the pager.
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
        entries.add(&table_line(
            [
                StringKey::DictionaryRomanLabel,
                StringKey::DictionaryHanziLabel,
                StringKey::DictionaryLearningRecordsCount,
                StringKey::DictionaryLearningRecordsLastUsed,
            ]
            .map(|key| strings.resolve(key)),
            true,
        ));
        // One click selects (the verbs below act on it); only a double-click
        // or Enter activates — GTK's default activates on a single click.
        let list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::Single)
            .activate_on_single_click(false)
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
        let edit = icon_button(
            "document-edit-symbolic",
            strings.resolve(StringKey::DictionaryLearningRecordsEditCount),
        );
        let add_to_custom_dictionary = icon_button(
            "list-add-symbolic",
            strings.resolve(StringKey::DictionaryLearningRecordsAddToCustomDictionary),
        );
        let delete = icon_button(
            "list-remove-symbolic",
            strings.resolve(StringKey::CommonDelete),
        );
        verbs.append(&edit);
        verbs.append(&add_to_custom_dictionary);
        verbs.append(&delete);
        let (previous, page_label, next) = append_pager(&verbs, &strings);
        entries.add(&verbs);
        page.add(&entries);

        let clear_group = adw::PreferencesGroup::new();
        let clear = destructive_row(
            &clear_group,
            strings.resolve(StringKey::DictionaryClearLearningRecords),
            strings.resolve(StringKey::CommonDelete),
        );
        page.add(&clear_group);

        let controls: Vec<gtk::Widget> = vec![
            kind.clone().upcast(),
            order.clone().upcast(),
            filter.clone().upcast(),
            clear.clone().upcast(),
        ];
        let this = Rc::new(Self {
            shell: context.shell.clone(),
            strings,
            job_slot: context.job_slot.clone(),
            state: RefCell::new(State {
                listing: Listing::default(),
                kind: restored_kind,
                order: restored_order,
                job: JobState::default(),
            }),
            is_rendering: Cell::new(false),
            widgets: Widgets {
                entries,
                list,
                empty,
                edit,
                add_to_custom_dictionary,
                delete,
                previous,
                next,
                page_label,
                busy,
                busy_box,
                controls,
            },
        });
        this.connect(&kind, &order, &filter, &clear);
        this.render();
        this
    }

    fn connect(
        self: &Rc<Self>,
        kind: &adw::ComboRow,
        order: &adw::ComboRow,
        filter: &gtk::SearchEntry,
        clear: &gtk::Button,
    ) {
        let weak = Rc::downgrade(self);
        kind.connect_selected_notify(move |row| {
            let Some(page) = weak.upgrade() else { return };
            if let Some((kind, _)) = KINDS.get(row.selected() as usize).copied() {
                page.shell
                    .update(|document| document.set_choice(&keys::LEARNING_RECORDS_KIND, kind));
                page.show_from_first_page(|state| state.kind = kind);
            }
        });
        let weak = Rc::downgrade(self);
        order.connect_selected_notify(move |row| {
            let Some(page) = weak.upgrade() else { return };
            if let Some(order) = ORDERS.get(row.selected() as usize).copied() {
                page.shell
                    .update(|document| document.set_choice(&keys::LEARNING_RECORDS_ORDER, order));
                page.show_from_first_page(|state| state.order = order);
            }
        });
        // `changed`, not `search-changed`: the generation moves at the
        // keystroke, and the one settle is ours (as on Custom Dictionary).
        let weak = Rc::downgrade(self);
        filter.connect_changed(move |entry| {
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
            let Some(row) = row else { return };
            if let Some(record) = page.row_at(row.index()) {
                page.state.borrow_mut().listing.selected_id = Some(record.id);
                page.render_verbs();
            }
        });
        // A double-click (or Enter) on a row opens its record dialog.
        let weak = Rc::downgrade(self);
        self.widgets.list.connect_row_activated(move |_, row| {
            let Some(page) = weak.upgrade() else { return };
            if page.job_slot.is_taken() {
                return;
            }
            if let Some(record) = page.row_at(row.index()) {
                page.record_dialog(record);
            }
        });
        let weak = Rc::downgrade(self);
        self.widgets.edit.connect_clicked(move |_| {
            let Some(page) = weak.upgrade() else { return };
            let row = page.state.borrow().listing.selected_row().cloned();
            if let Some(row) = row {
                page.record_dialog(row);
            }
        });
        let weak = Rc::downgrade(self);
        self.widgets.delete.connect_clicked(move |_| {
            let Some(page) = weak.upgrade() else { return };
            let Some(row) = page.state.borrow().listing.selected_row().cloned() else {
                return;
            };
            page.begin_job(move || delete_job(row));
        });
        let weak = Rc::downgrade(self);
        self.widgets
            .add_to_custom_dictionary
            .connect_clicked(move |_| {
                let Some(page) = weak.upgrade() else { return };
                let Some(row) = page.state.borrow().listing.selected_row().cloned() else {
                    return;
                };
                page.add_word_dialog(row);
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
        // No confirmation (USER 2026-10-16).
        clear.connect_clicked(move |_| {
            if let Some(page) = weak.upgrade() {
                page.begin_job(clear_all_job);
            }
        });
    }

    /// Another kind or order: the first page of it, nothing selected, the
    /// filter kept. The load it starts makes any older one stale.
    fn show_from_first_page(self: &Rc<Self>, change: impl FnOnce(&mut State)) {
        {
            let mut state = self.state.borrow_mut();
            change(&mut state);
            state.listing.rewind();
        }
        self.load();
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

    /// The row at a list index, as listed.
    fn row_at(&self, index: i32) -> Option<LearningRecord> {
        let state = self.state.borrow();
        state
            .listing
            .rows
            .get(usize::try_from(index).ok()?)
            .cloned()
    }

    /// The rows on screen (for the tests).
    pub fn shown_rows(&self) -> Vec<LearningRecord> {
        self.state.borrow().listing.rows.clone()
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

    /// Starts a load of the page on screen; the rows already on screen stay
    /// put while it runs.
    fn load(self: &Rc<Self>) {
        let (request, kind, order) = {
            let mut state = self.state.borrow_mut();
            let request = state.listing.begin_load();
            (request, state.kind, state.order)
        };
        let generation = request.generation;
        let weak = Rc::downgrade(self);
        jobs::spawn(
            move || fetch(&request, kind, order),
            move |outcome| {
                let Some(page) = weak.upgrade() else { return };
                let outcome = outcome.unwrap_or_else(|| Err(LOAD_DID_NOT_FINISH.to_owned()));
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
    /// something else holds it — Custom Dictionary's `begin_job`.
    fn begin_job(self: &Rc<Self>, job: impl FnOnce() -> JobOutcome + Send + 'static) {
        self.begin_job_then(false, job);
    }

    /// `begin_job`; `writes_custom_dictionary` also reloads Custom
    /// Dictionary's list once the job lands, so the word it filed shows
    /// there without leaving the pane.
    fn begin_job_then(
        self: &Rc<Self>,
        writes_custom_dictionary: bool,
        job: impl FnOnce() -> JobOutcome + Send + 'static,
    ) {
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
            let outcome = outcome.unwrap_or_else(JobOutcome::did_not_finish::<LearningRecord>);
            if let Some(message) = outcome.message {
                shell.report(&message, &strings);
            }
            if writes_custom_dictionary {
                shell.reload_custom_dictionary();
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
        let weak = Rc::downgrade(self);
        glib::timeout_add_local_once(OVERLAY_DELAY, move || {
            let Some(page) = weak.upgrade() else { return };
            let is_busy = page.state.borrow_mut().job.show_busy(generation);
            if is_busy {
                page.render();
            }
        });
    }

    /// One record's dialog. For a row the engine says can be added, two
    /// tabs — Edit Count and Add to Custom Dictionary — one task on screen
    /// at a time, and the dialog's one confirm button follows the tab: Save
    /// writes the count, Add files the word as edited (typed form and word,
    /// for the user to tweak hyphens and capitals; a learned phrase is then
    /// forgotten). Any other row shows the count alone. The count carries,
    /// for word frequency, the note that counts past 40 rank the same.
    fn record_dialog(self: &Rc<Self>, record: LearningRecord) {
        const COUNT_TAB: &str = "count";
        const ADD_TAB: &str = "add";
        let strings = self.strings;
        // The engine's flag alone: no Hanji (and, with the engine's
        // word-frequency rule, every frequency row) leaves the add out.
        let can_add = record.can_add_to_custom_dictionary;
        let body = format!("{}  {}", record.text, record.tl);
        let dialog = adw::AlertDialog::new(
            Some(strings.resolve(if can_add {
                StringKey::CommonEdit
            } else {
                StringKey::DictionaryLearningRecordsEditCount
            })),
            Some(body.trim()),
        );

        // The count: a spin row and, for word frequency, its note.
        let count_list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(["boxed-list"])
            .build();
        let count = adw::SpinRow::with_range(1.0, MAX_COUNT as f64, 1.0);
        count.set_title(strings.resolve(StringKey::DictionaryLearningRecordsCount));
        count.set_value(record.count.max(1) as f64);
        count_list.append(&count);
        let count_page = gtk::Box::new(gtk::Orientation::Vertical, 6);
        count_page.append(&count_list);
        if let Some(note) = record.kind.try_into().ok().and_then(count_note) {
            count_page.append(&dim_note(strings.resolve(note)));
        }

        dialog.add_response("cancel", strings.resolve(StringKey::CommonCancel));
        dialog.add_response("apply", strings.resolve(StringKey::CommonSave));
        dialog.set_response_appearance("apply", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("apply"));
        dialog.set_close_response("cancel");

        // The add tab: the Add Word fields, and the stack that switches
        // between the two. `None` on a row that cannot be added.
        let add = can_add.then(|| {
            let roman = adw::EntryRow::builder()
                .title(strings.resolve(StringKey::DictionaryRomanLabel))
                .text(record.tl.as_str())
                .use_markup(false)
                .build();
            let hanji = adw::EntryRow::builder()
                .title(strings.resolve(StringKey::DictionaryHanziLabel))
                .text(record.text.as_str())
                .use_markup(false)
                .build();
            let word_list = gtk::ListBox::builder()
                .selection_mode(gtk::SelectionMode::None)
                .css_classes(["boxed-list"])
                .build();
            word_list.append(&roman);
            word_list.append(&hanji);
            let stack = gtk::Stack::new();
            stack.add_titled(
                &count_page,
                Some(COUNT_TAB),
                strings.resolve(StringKey::DictionaryLearningRecordsEditCount),
            );
            stack.add_titled(
                &word_list,
                Some(ADD_TAB),
                strings.resolve(StringKey::DictionaryLearningRecordsAddToCustomDictionary),
            );
            (roman, hanji, stack)
        });
        match &add {
            Some((roman, _, stack)) => {
                // Text-only tabs: `adw::ViewSwitcher` always draws an icon,
                // and these tabs have none.
                let switcher = gtk::StackSwitcher::builder().stack(stack).build();
                let page = gtk::Box::new(gtk::Orientation::Vertical, 12);
                page.append(&switcher);
                page.append(stack);
                dialog.set_extra_child(Some(&page));
                // The confirm button follows the tab: its label, and — on
                // the add tab — a romanization to file the word under.
                // Weak: the dialog holds these widgets, and they holding it
                // back would keep all alive after it closed.
                let follow = {
                    let dialog = dialog.downgrade();
                    let (roman, stack) = (roman.clone(), stack.clone());
                    move || {
                        let Some(dialog) = dialog.upgrade() else {
                            return;
                        };
                        let is_add = stack.visible_child_name().as_deref() == Some(ADD_TAB);
                        dialog.set_response_label(
                            "apply",
                            strings.resolve(if is_add {
                                StringKey::DictionaryLearningRecordsAddToCustomDictionary
                            } else {
                                StringKey::CommonSave
                            }),
                        );
                        dialog.set_response_enabled(
                            "apply",
                            !is_add || !roman.text().trim().is_empty(),
                        );
                    }
                };
                let on_tab = follow.clone();
                stack.connect_visible_child_name_notify(move |_| on_tab());
                roman.connect_changed(move |_| follow());
            }
            None => dialog.set_extra_child(Some(&count_page)),
        }

        let weak = Rc::downgrade(self);
        dialog.connect_response(None, move |_, response| {
            if response != "apply" {
                return;
            }
            let Some(page) = weak.upgrade() else { return };
            let record = record.clone();
            match &add {
                Some((roman, hanji, stack))
                    if stack.visible_child_name().as_deref() == Some(ADD_TAB) =>
                {
                    let roman = roman.text().to_string();
                    if roman.trim().is_empty() {
                        return;
                    }
                    let hanji = hanji.text().to_string();
                    page.begin_job(move || add_to_custom_dictionary_job(record, &roman, &hanji));
                }
                _ => {
                    let value = count.value() as i64;
                    page.begin_job(move || set_count_job(record, value));
                }
            }
        });
        dialog.present(self.shell.window().as_ref());
    }

    /// Add to Custom Dictionary: Custom Dictionary's Add Word dialog over
    /// the row's typed form and word, for the user to tweak (hyphens,
    /// capitals) before Save files it — and forgets a learned phrase.
    fn add_word_dialog(self: &Rc<Self>, record: LearningRecord) {
        let weak = Rc::downgrade(self);
        let (roman, hanji) = (record.tl.clone(), record.text.clone());
        word_dialog(
            &self.shell,
            self.strings,
            true,
            &roman,
            &hanji,
            move |roman, hanji| {
                let Some(page) = weak.upgrade() else { return };
                let record = record.clone();
                page.begin_job_then(true, move || {
                    add_to_custom_dictionary_job(record, &roman, &hanji)
                });
            },
        );
    }

    fn report(&self, message: PageMessage) {
        self.shell.report(&message, &self.strings);
    }

    /// The state, drawn — Custom Dictionary's `render`, four columns.
    fn render(&self) {
        let drawn = {
            let state = self.state.borrow();
            let listing = &state.listing;
            Drawn {
                rows: listing.rows.iter().map(cells).collect(),
                selected_index: listing.selected_index(),
                count: listing.count_label(),
                empty: listing.empty_state_key(),
                page_label: format!("{} / {}", listing.page + 1, listing.page_count()),
                has_previous: listing.page > 0,
                has_next: listing.page + 1 < listing.page_count(),
                is_busy: state.job.is_busy_shown(),
            }
        };
        let widgets = &self.widgets;
        widgets.entries.set_description(Some(&drawn.count));
        remove_rows(&widgets.list);
        for row in &drawn.rows {
            let cells = table_line(row.each_ref().map(String::as_str), false);
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
        self.render_verbs();
    }

    /// The ✎, + and − buttons: on while a row is selected and nothing runs.
    /// The + (add to custom dictionary) is left out on word frequency, and
    /// on learned phrases also needs a row the engine says can be added: no
    /// Hanji keeps it off.
    fn render_verbs(&self) {
        let (is_busy, has_selection, can_add_selection, offers_add) = {
            let state = self.state.borrow();
            let selected_row = state.listing.selected_row();
            (
                state.job.is_busy_shown(),
                selected_row.is_some(),
                selected_row.is_some_and(|row| row.can_add_to_custom_dictionary),
                offers_add_to_custom_dictionary(state.kind),
            )
        };
        self.widgets
            .add_to_custom_dictionary
            .set_visible(offers_add);
        let is_on = !is_busy && has_selection;
        self.widgets.edit.set_sensitive(is_on);
        self.widgets
            .add_to_custom_dictionary
            .set_sensitive(is_on && can_add_selection);
        self.widgets.delete.set_sensitive(is_on);
    }
}

/// One row's four cells: reading, word, count, the day it was last used
/// in this machine's calendar.
fn cells(record: &LearningRecord) -> [String; 4] {
    let offset_seconds = (record.last_used_ms > 0)
        .then(|| glib::DateTime::from_unix_local(record.last_used_ms / 1000).ok())
        .flatten()
        .map_or(0, |time| time.utc_offset().as_seconds());
    [
        record.tl.clone(),
        record.text.clone(),
        record.count.to_string(),
        last_used_label(record.last_used_ms, offset_seconds),
    ]
}

/// A dim, wrapping note under a dialog's fields.
fn dim_note(text: &str) -> gtk::Label {
    gtk::Label::builder()
        .label(text)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["dim-label"])
        .build()
}
