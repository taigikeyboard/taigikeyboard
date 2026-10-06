//! The pages, one module per pane, and the row shapes they share. A page
//! is an `adw::PreferencesPage` plus the closures that put the document's
//! values back into its rows (`refresh`); a row's own handler writes
//! through the window, and stays quiet while a refresh is setting it.

pub mod about;
pub mod appearance;
pub mod custom_dictionary;
pub mod dictionary_search;
pub mod dictionary_sources;
pub mod general;
pub mod learning_records;
pub mod shortcuts;

use crate::window::{JobSlot, SettingsWindow, Shell};
use adw::prelude::*;
use std::any::Any;
use std::cell::Cell;
use std::rc::Rc;
use taigi_desktop_core::settings::presentation::Confirmation;
use taigi_desktop_core::settings::{SettingChoice, SettingsDocument, SettingsKey, SettingsPane};
use taigi_desktop_core::strings::{StringKey, StringResolver};

/// The panes this crate draws, listed or not (Dictionary Search and About have no
/// sidebar row, as on the other desktops).
pub const BUILT: [SettingsPane; 8] = [
    SettingsPane::General,
    SettingsPane::Appearance,
    SettingsPane::Shortcuts,
    SettingsPane::DictionarySources,
    SettingsPane::CustomDictionary,
    SettingsPane::LearningRecords,
    SettingsPane::DictionarySearch,
    SettingsPane::About,
];

/// A refresher: one row following the document.
type Refresher = Box<dyn Fn(&SettingsDocument)>;

pub struct Page {
    pub widget: adw::PreferencesPage,
    refreshers: Vec<Refresher>,
    /// A page's own state object (Custom Dictionary, Learning Records, Dictionary Search), kept for the page's
    /// life; its widgets hold it weakly.
    retained: Vec<Rc<dyn Any>>,
    /// Set while `refresh` runs, so a row's notify handler does not write
    /// the value it was just given.
    suppress: Rc<Cell<bool>>,
}

impl Page {
    /// The page's retained state object of type `T`, if it has one (the
    /// tests drive Custom Dictionary through it).
    pub fn state<T: 'static>(&self) -> Option<Rc<T>> {
        self.retained
            .iter()
            .find_map(|state| Rc::clone(state).downcast::<T>().ok())
    }

    pub fn refresh(&self, document: &SettingsDocument) {
        self.suppress.set(true);
        for refresher in &self.refreshers {
            refresher(document);
        }
        self.suppress.set(false);
    }
}

/// What a page is built from.
pub struct PageContext<'a> {
    pub shell: Shell,
    pub strings: &'a StringResolver,
    pub document: &'a SettingsDocument,
    /// Whether this launch opened the engine's user-data stores; false in
    /// a read-only launch, where the pages over user data build only their
    /// switch (the banner says why).
    pub has_user_data: bool,
    pub job_slot: JobSlot,
    suppress: Rc<Cell<bool>>,
    refreshers: Vec<Refresher>,
    retained: Vec<Rc<dyn Any>>,
}

impl<'a> PageContext<'a> {
    fn new(
        window: &Rc<SettingsWindow>,
        strings: &'a StringResolver,
        document: &'a SettingsDocument,
        has_user_data: bool,
        job_slot: &JobSlot,
    ) -> Self {
        Self {
            shell: Shell(Rc::downgrade(window)),
            strings,
            document,
            has_user_data,
            job_slot: job_slot.clone(),
            suppress: Rc::new(Cell::new(false)),
            refreshers: Vec::new(),
            retained: Vec::new(),
        }
    }

    fn finish(self, widget: adw::PreferencesPage) -> Page {
        Page {
            widget,
            refreshers: self.refreshers,
            retained: self.retained,
            suppress: self.suppress,
        }
    }

    /// Keeps `state` alive with the page (its widgets hold it weakly).
    pub fn retain(&mut self, state: Rc<dyn Any>) {
        self.retained.push(state);
    }

    /// A switch row bound to a boolean key.
    pub fn switch_row(
        &mut self,
        group: &adw::PreferencesGroup,
        title: StringKey,
        key: SettingsKey<bool>,
    ) {
        self.switch_row_in(|row| group.add(row), title, key);
    }

    /// A switch row bound to a boolean key, placed by `add` — in a group,
    /// or inside an expander row's own list.
    pub fn switch_row_in(
        &mut self,
        add: impl FnOnce(&adw::SwitchRow),
        title: StringKey,
        key: SettingsKey<bool>,
    ) -> adw::SwitchRow {
        let row = adw::SwitchRow::builder()
            .title(self.strings.resolve(title))
            .active(self.document.bool(&key))
            .build();
        let shell = self.shell.clone();
        let suppress = Rc::clone(&self.suppress);
        row.connect_active_notify(move |row| {
            if suppress.get() {
                return;
            }
            let is_on = row.is_active();
            shell.update(|document| document.set_bool(&key, is_on));
        });
        add(&row);
        let refreshed = row.clone();
        self.refreshers.push(Box::new(move |document| {
            let value = document.bool(&key);
            if refreshed.is_active() != value {
                refreshed.set_active(value);
            }
        }));
        row
    }

    /// The flag a refresh raises while it sets the rows, for a handler
    /// that is not one of the shapes above (the MOE dictionary expander's switch).
    pub fn refreshing_flag(&self) -> Rc<Cell<bool>> {
        Rc::clone(&self.suppress)
    }

    /// A combo row over a `SettingChoice` roster, bound to its key.
    pub fn choice_row<T: SettingChoice>(
        &mut self,
        group: &adw::PreferencesGroup,
        title: StringKey,
        roster: &'static [T],
        key: SettingsKey<T>,
        label: impl Fn(T) -> StringKey,
    ) -> adw::ComboRow {
        let labels: Vec<String> = roster
            .iter()
            .map(|choice| self.strings.resolve(label(*choice)).to_owned())
            .collect();
        let current = self.document.choice(&key);
        self.picker_row(
            group,
            self.strings.resolve(title),
            labels,
            roster,
            current,
            move |choice, document| document.set_choice(&key, choice),
            move |document| document.choice(&key),
        )
    }

    /// A combo row over any roster: `write` stores a pick, `read` answers
    /// the stored value for a refresh.
    #[allow(clippy::too_many_arguments)]
    pub fn picker_row<T: Copy + PartialEq + 'static>(
        &mut self,
        group: &adw::PreferencesGroup,
        title: &str,
        labels: Vec<String>,
        roster: &'static [T],
        current: T,
        write: impl Fn(T, &mut SettingsDocument) + 'static,
        read: impl Fn(&SettingsDocument) -> T + 'static,
    ) -> adw::ComboRow {
        let label_refs: Vec<&str> = labels.iter().map(String::as_str).collect();
        let row = adw::ComboRow::builder()
            .title(title)
            .model(&gtk::StringList::new(&label_refs))
            .selected(roster.iter().position(|c| *c == current).unwrap_or(0) as u32)
            .build();
        let shell = self.shell.clone();
        let suppress = Rc::clone(&self.suppress);
        row.connect_selected_notify(move |row| {
            if suppress.get() {
                return;
            }
            let Some(choice) = roster.get(row.selected() as usize).copied() else {
                return;
            };
            shell.update(|document| write(choice, document));
        });
        group.add(&row);
        let refreshed = row.clone();
        self.refreshers.push(Box::new(move |document| {
            let value = read(document);
            if let Some(index) = roster.iter().position(|c| *c == value) {
                if refreshed.selected() != index as u32 {
                    refreshed.set_selected(index as u32);
                }
            }
        }));
        row
    }

    /// A page's Reset to Defaults row, its own group at the end, acting on every
    /// row above it. One shape for every pane that has one.
    pub fn reset_row(&mut self, page: &adw::PreferencesPage, reset: fn(&mut SettingsDocument)) {
        let group = adw::PreferencesGroup::new();
        let button = gtk::Button::builder()
            .label(self.strings.resolve(StringKey::SettingsReset))
            .valign(gtk::Align::Center)
            .build();
        let row = adw::ActionRow::builder()
            .title(self.strings.resolve(StringKey::ThemeEditorResetAll))
            .build();
        row.add_suffix(&button);
        row.set_activatable_widget(Some(&button));
        let shell = self.shell.clone();
        button.connect_clicked(move |_| shell.update(reset));
        group.add(&row);
        page.add(&group);
    }

    /// A row that opens a link, marked with the external-link arrow.
    pub fn link_row(&mut self, group: &adw::PreferencesGroup, title: &str, url: &'static str) {
        let row = adw::ActionRow::builder()
            .title(title)
            .activatable(true)
            .build();
        row.add_suffix(&gtk::Image::from_icon_name("adw-external-link-symbolic"));
        let shell = self.shell.clone();
        row.connect_activated(move |_| shell.open_url(url));
        group.add(&row);
    }

    /// A refresher for a row the shapes above do not cover.
    pub fn on_refresh(&mut self, refresher: impl Fn(&SettingsDocument) + 'static) {
        self.refreshers.push(Box::new(refresher));
    }
}

/// Builds the page for `pane` against the window.
pub fn build(
    pane: SettingsPane,
    window: &Rc<SettingsWindow>,
    strings: &StringResolver,
    document: &SettingsDocument,
    has_user_data: bool,
    job_slot: &JobSlot,
) -> Page {
    let context = PageContext::new(window, strings, document, has_user_data, job_slot);
    let widget = adw::PreferencesPage::new();
    // Explicit per pane: a pane added to `BUILT` without a page is a
    // mistake to hear about, not a General page under the wrong title.
    let context = match pane {
        SettingsPane::General => general::build(context, &widget),
        SettingsPane::Appearance => appearance::build(context, &widget),
        SettingsPane::Shortcuts => shortcuts::build(context, &widget),
        SettingsPane::DictionarySources => dictionary_sources::build(context, &widget),
        SettingsPane::CustomDictionary => custom_dictionary::build(context, &widget),
        SettingsPane::LearningRecords => learning_records::build(context, &widget),
        SettingsPane::DictionarySearch => dictionary_search::build(context, &widget),
        SettingsPane::About => about::build(context, &widget),
        other => unreachable!("{other:?} is not in pages::BUILT"),
    };
    context.finish(widget)
}

/// Removes the rows and nothing else: `remove_all` would take the
/// placeholder with them.
pub(crate) fn remove_rows(list: &gtk::ListBox) {
    while let Some(row) = list.row_at_index(0) {
        list.remove(&row);
    }
}

/// A list table's horizontal inset and a row's vertical one, the list's
/// own row metrics (`adw::ActionRow`).
const TABLE_INSET: i32 = 12;
const ROW_INSET: i32 = 8;

/// One line of a user-data table (Custom Dictionary, Learning Records):
/// equal columns side by side (`Metrics.tableColumns` on the Mac), inset as
/// the list's rows are. A heading line is the column names over the list;
/// a row's cells are plain labels — user text, never markup.
pub(crate) fn table_line<const N: usize>(texts: [&str; N], is_heading: bool) -> gtk::Box {
    let line = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .homogeneous(true)
        .spacing(12)
        .margin_start(TABLE_INSET)
        .margin_end(TABLE_INSET)
        .build();
    for text in texts {
        let cell = gtk::Label::builder().label(text).xalign(0.0);
        line.append(&if is_heading {
            cell.css_classes(["heading"]).build()
        } else {
            cell.ellipsize(gtk::pango::EllipsizeMode::End).build()
        });
    }
    if is_heading {
        line.set_margin_bottom(6);
    } else {
        line.set_margin_top(ROW_INSET);
        line.set_margin_bottom(ROW_INSET);
    }
    line
}

/// The job's name beside a spinner, hidden until a job has run long
/// enough to say so (`busy_overlay` on Windows, the overlay card on the
/// Mac); the box goes in a group's header suffix.
pub(crate) fn busy_indicator(strings: &StringResolver) -> (gtk::Spinner, gtk::Box) {
    let spinner = gtk::Spinner::new();
    let indicator = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    indicator.append(&spinner);
    indicator.append(&gtk::Label::new(Some(
        strings.resolve(StringKey::DesktopProgressWorking),
    )));
    indicator.set_visible(false);
    (spinner, indicator)
}

/// The pager at the end of a verb row: a spacer, then ‹ page › .
pub(crate) fn append_pager(
    verbs: &gtk::Box,
    strings: &StringResolver,
) -> (gtk::Button, gtk::Label, gtk::Button) {
    verbs.append(&gtk::Box::builder().hexpand(true).build());
    let previous = icon_button(
        "go-previous-symbolic",
        strings.resolve(StringKey::CommonPagePrevious),
    );
    let label = gtk::Label::new(None);
    let next = icon_button(
        "go-next-symbolic",
        strings.resolve(StringKey::CommonPageNext),
    );
    verbs.append(&previous);
    verbs.append(&label);
    verbs.append(&next);
    (previous, label, next)
}

pub(crate) fn icon_button(icon: &str, tooltip: &str) -> gtk::Button {
    let button = gtk::Button::from_icon_name(icon);
    if !tooltip.is_empty() {
        button.set_tooltip_text(Some(tooltip));
    }
    button
}

/// A row whose button empties a store; the button is the destructive one.
pub(crate) fn destructive_row(
    group: &adw::PreferencesGroup,
    title: &str,
    verb: &str,
) -> gtk::Button {
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

/// Asks `confirmation` over `shell`'s window and runs `on_confirmed` on
/// Delete: Delete is the destructive response; Escape, Cancel and a
/// dismissal all leave the store alone.
pub(crate) fn confirm(
    shell: &Shell,
    strings: StringResolver,
    confirmation: Confirmation,
    on_confirmed: impl Fn() + 'static,
) {
    let dialog = adw::AlertDialog::new(
        Some(strings.resolve(confirmation.title)),
        Some(strings.resolve(confirmation.message)),
    );
    dialog.add_response("cancel", strings.resolve(StringKey::CommonCancel));
    dialog.add_response("delete", strings.resolve(StringKey::CommonDelete));
    dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
    dialog.set_close_response("cancel");
    dialog.connect_response(None, move |_, response| {
        if response == "delete" {
            on_confirmed();
        }
    });
    dialog.present(shell.window().as_ref());
}

/// The file a chooser answered: `Ok(None)` for a dismissal, `Err` for any
/// other refusal and for a file with no local path.
pub(crate) fn chosen_path(
    result: Result<gtk::gio::File, gtk::glib::Error>,
) -> Result<Option<std::path::PathBuf>, String> {
    match result {
        Ok(file) => file
            .path()
            .map(Some)
            .ok_or_else(|| "not a local file".to_owned()),
        Err(error) if error.matches(gtk::DialogError::Dismissed) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}
