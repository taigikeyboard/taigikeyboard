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
use taigi_desktop_core::settings::{
    keys, SettingChoice, SettingsDocument, SettingsKey, SettingsPane,
};
use taigi_desktop_core::strings::{StringKey, StringResolver};

/// The panes this crate draws, listed or not (About has no sidebar row, as on
/// the other desktops).
pub const BUILT: [SettingsPane; 7] = [
    SettingsPane::General,
    SettingsPane::Appearance,
    SettingsPane::Shortcuts,
    SettingsPane::DictionarySources,
    SettingsPane::CustomDictionary,
    SettingsPane::LearningRecords,
    SettingsPane::About,
];

/// A refresher: one row following the document.
type Refresher = Box<dyn Fn(&SettingsDocument)>;

pub struct Page {
    pub widget: gtk::Widget,
    refreshers: Vec<Refresher>,
    /// A page's own state object (Custom Dictionary, Learning Records, the Manage Dictionaries search), kept for the page's
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

    fn finish(self, widget: gtk::Widget) -> Page {
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

    /// An integer row bound to an `i64` key. A stored value outside `range`
    /// is shown at the nearer end and written back only when the user moves
    /// the row.
    pub fn spin_row(
        &mut self,
        group: &adw::PreferencesGroup,
        title: StringKey,
        key: SettingsKey<i64>,
        range: std::ops::RangeInclusive<i64>,
    ) -> adw::SpinRow {
        let row = adw::SpinRow::with_range(*range.start() as f64, *range.end() as f64, 1.0);
        row.set_title(self.strings.resolve(title));
        row.set_digits(0);
        row.set_value(self.document.i64(&key).clamp(*range.start(), *range.end()) as f64);
        let shell = self.shell.clone();
        let suppress = Rc::clone(&self.suppress);
        let write_range = range.clone();
        row.connect_value_notify(move |row| {
            if suppress.get() {
                return;
            }
            let px = (row.value().round() as i64).clamp(*write_range.start(), *write_range.end());
            shell.update(|document| document.set_i64(&key, px));
        });
        group.add(&row);
        let refreshed = row.clone();
        self.refreshers.push(Box::new(move |document| {
            let value = document.i64(&key).clamp(*range.start(), *range.end()) as f64;
            if refreshed.value() != value {
                refreshed.set_value(value);
            }
        }));
        row
    }

    /// A color button bound to a `#RRGGBB` string key.
    pub fn color_row(
        &mut self,
        group: &adw::PreferencesGroup,
        title: StringKey,
        key: SettingsKey<&'static str>,
    ) -> adw::ActionRow {
        let fallback = keys::rgb_from_stored(key.default, 0);
        let dialog = gtk::ColorDialog::new();
        dialog.set_with_alpha(false);
        let button = gtk::ColorDialogButton::new(Some(dialog));
        button.set_valign(gtk::Align::Center);
        button.set_rgba(&rgba(keys::rgb_from_stored(
            &self.document.string(&key),
            fallback,
        )));
        let row = adw::ActionRow::builder()
            .title(self.strings.resolve(title))
            .build();
        row.add_suffix(&button);
        let shell = self.shell.clone();
        let suppress = Rc::clone(&self.suppress);
        button.connect_rgba_notify(move |button| {
            if suppress.get() {
                return;
            }
            let stored = keys::rgb_to_stored(rgb_of(&button.rgba()));
            shell.update(|document| document.set_string(&key, &stored));
        });
        group.add(&row);
        let returned = row.clone();
        let refreshed = button.clone();
        self.refreshers.push(Box::new(move |document| {
            let next = rgba(keys::rgb_from_stored(&document.string(&key), fallback));
            if rgb_of(&refreshed.rgba()) != rgb_of(&next) {
                refreshed.set_rgba(&next);
            }
        }));
        returned
    }

    /// Shows `widget` while `shown` is true, and again after each refresh.
    pub fn bind_visible<W>(
        &mut self,
        widget: &W,
        shown: impl Fn(&SettingsDocument) -> bool + 'static,
    ) where
        W: IsA<gtk::Widget> + Clone + 'static,
    {
        let widget = widget.clone();
        widget.set_visible(shown(self.document));
        self.refreshers.push(Box::new(move |document| {
            let next = shown(document);
            if widget.is_visible() != next {
                widget.set_visible(next);
            }
        }));
    }

    /// Faces grouped under one expander per family. The menu has no size:
    /// the primary and secondary size rows decide that. An empty stored
    /// value is the system sans, the same default as the system face on
    /// macOS and Windows. Search filters families and faces; a regular
    /// face is stored as the family name alone. The four bundled faces
    /// lead the list, in the shared roster order.
    pub fn font_row(
        &mut self,
        group: &adw::PreferencesGroup,
        title: StringKey,
        family_key: SettingsKey<&'static str>,
    ) {
        let system = self
            .strings
            .resolve(StringKey::DesktopSystemFont)
            .to_owned();
        let families = font_families();
        let button = gtk::MenuButton::builder()
            .valign(gtk::Align::Center)
            .always_show_arrow(true)
            .label(font_button_label(
                &families,
                &self.document.string(&family_key),
                &system,
            ))
            .build();
        let search = gtk::SearchEntry::new();
        let list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .build();
        let rows = build_font_list(&list, &families, &system);
        let scroll = gtk::ScrolledWindow::builder()
            .child(&list)
            .min_content_height(240)
            .max_content_height(360)
            .propagate_natural_height(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();
        let menu = gtk::Box::new(gtk::Orientation::Vertical, 6);
        menu.set_margin_top(6);
        menu.set_margin_bottom(6);
        menu.set_margin_start(6);
        menu.set_margin_end(6);
        menu.append(&search);
        menu.append(&scroll);
        let popover = gtk::Popover::builder().child(&menu).build();
        popover.set_width_request(280);
        button.set_popover(Some(&popover));
        let row = adw::ActionRow::builder()
            .title(self.strings.resolve(title))
            .build();
        row.add_suffix(&button);
        let shell = self.shell.clone();
        let suppress = Rc::clone(&self.suppress);
        for entry in &rows {
            let stored = entry.stored.clone();
            let shell = shell.clone();
            let suppress = Rc::clone(&suppress);
            let popover = popover.clone();
            entry.row.connect_activated(move |_| {
                if suppress.get() {
                    return;
                }
                popover.popdown();
                let stored = stored.clone();
                shell.update(move |document| document.set_string(&family_key, &stored));
            });
        }
        let listed = rows.clone();
        let list_for_search = list.clone();
        // `search-changed` waits 150ms and does not always fire when the
        // entry is cleared, so the list stayed filtered after one query.
        search.connect_changed(move |entry| {
            apply_font_search(&list_for_search, &listed, &entry.text());
        });
        let listed = rows.clone();
        let list_for_close = list.clone();
        let search_for_close = search.clone();
        popover.connect_closed(move |_| {
            if !search_for_close.text().is_empty() {
                search_for_close.set_text("");
            } else {
                apply_font_search(&list_for_close, &listed, "");
            }
        });
        group.add(&row);
        let refreshed = button.clone();
        let system_label = system.clone();
        self.refreshers.push(Box::new(move |document| {
            refreshed.set_label(&font_button_label(
                &families,
                &document.string(&family_key),
                &system_label,
            ));
        }));
    }

    /// Highlight fill as five steps. The window still stores 0–100:
    /// 1 is 0, 2 is 25, 3 is 50, 4 is 75, 5 is 100.
    pub fn highlight_fill_row(&mut self, group: &adw::PreferencesGroup) {
        let row = adw::SpinRow::with_range(1.0, 5.0, 1.0);
        row.set_title(
            self.strings
                .resolve(StringKey::DesktopCandidateHighlightFill),
        );
        row.set_digits(0);
        row.set_value(fill_step(self.document.i64(&keys::CANDIDATE_HIGHLIGHT_FILL)) as f64);
        let shell = self.shell.clone();
        let suppress = Rc::clone(&self.suppress);
        row.connect_value_notify(move |row| {
            if suppress.get() {
                return;
            }
            let stored = fill_stored(row.value().round() as i64);
            shell.update(|document| document.set_i64(&keys::CANDIDATE_HIGHLIGHT_FILL, stored));
        });
        group.add(&row);
        let refreshed = row.clone();
        self.refreshers.push(Box::new(move |document| {
            let value = fill_step(document.i64(&keys::CANDIDATE_HIGHLIGHT_FILL)) as f64;
            if refreshed.value() != value {
                refreshed.set_value(value);
            }
        }));
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

fn rgba(rgb: u32) -> gtk::gdk::RGBA {
    gtk::gdk::RGBA::new(
        ((rgb >> 16) & 0xff) as f32 / 255.0,
        ((rgb >> 8) & 0xff) as f32 / 255.0,
        (rgb & 0xff) as f32 / 255.0,
        1.0,
    )
}

fn rgb_of(color: &gtk::gdk::RGBA) -> u32 {
    let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u32;
    (channel(color.red()) << 16) | (channel(color.green()) << 8) | channel(color.blue())
}

#[derive(Clone)]
struct FontFaceChoice {
    label: String,
    stored: String,
    synthesized: bool,
}

struct FontFamilyGroup {
    title: String,
    faces: Vec<FontFaceChoice>,
}

#[derive(Clone)]
struct FontMenuRow {
    row: adw::ActionRow,
    expander: Option<adw::ExpanderRow>,
    family: String,
    label: String,
    stored: String,
}

/// Families fontconfig splits by weight (`Noto Sans` and `Noto Sans Bold`)
/// share one expander. A regular face is stored as the family name, with no
/// style word and no size. A synthesized face is kept only when that weight
/// has no real file. The four bundled faces lead, in the order macOS and
/// Windows list them; every other family follows, by title.
fn font_families() -> Vec<FontFamilyGroup> {
    let context = gtk::Label::new(None).pango_context();
    let mut groups: std::collections::BTreeMap<String, FontFamilyGroup> =
        std::collections::BTreeMap::new();
    for family in context.list_families() {
        let raw_name = family.name().to_string();
        for face in family.list_faces() {
            let desc = face.describe();
            let family_name = desc
                .family()
                .map(|name| name.to_string())
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| raw_name.clone());
            let stored = stored_face(&family_name, desc.weight(), desc.style());
            let label = face_label(&family_name, &face.face_name(), desc.weight(), desc.style());
            let key = family_group(&family_name);
            let group = groups.entry(key).or_insert_with(|| FontFamilyGroup {
                title: family_stem(&family_name),
                faces: Vec::new(),
            });
            if family_stem(&family_name).chars().count() < group.title.chars().count() {
                group.title = family_stem(&family_name);
            }
            group.faces.push(FontFaceChoice {
                label,
                stored,
                synthesized: face.is_synthesized(),
            });
        }
    }
    let mut families: Vec<_> = groups.into_values().collect();
    for family in &mut families {
        let real: std::collections::BTreeSet<String> = family
            .faces
            .iter()
            .filter(|face| !face.synthesized)
            .map(|face| face.stored.clone())
            .collect();
        family
            .faces
            .retain(|face| !face.synthesized || !real.contains(&face.stored));
        family
            .faces
            .sort_by(|left, right| left.label.cmp(&right.label));
        family
            .faces
            .dedup_by(|left, right| left.stored == right.stored);
    }
    families.retain(|family| !family.faces.is_empty());
    order_font_families(&mut families);
    families
}

/// The bundled roster after the system row, in `CandidateFontChoice`'s order
/// (System is the row above this list). Names are the ones fontconfig reports
/// for `assets/fonts/font/`: the English family, the localized family, and the
/// weight suffix `R` those two files also publish as a family.
const BUNDLED_FONT_NAMES: [&[&str]; 4] = [
    &["jf-openhuninn-2.1", "jf open 粉圓 2.1"],
    &["Iansui", "芫荽"],
    &["GenYoMin2 TW", "源樣明體月"],
    &["GenYoGothic2 TW", "源樣黑體月"],
];

fn order_font_families(families: &mut [FontFamilyGroup]) {
    families.sort_by(|left, right| {
        bundled_font_rank(left)
            .cmp(&bundled_font_rank(right))
            .then_with(|| left.title.cmp(&right.title))
    });
}

/// Where a bundled family sits, or one past the roster so the rest sort by title.
fn bundled_font_rank(family: &FontFamilyGroup) -> usize {
    BUNDLED_FONT_NAMES
        .iter()
        .position(|names| family_is_bundled(family, names))
        .unwrap_or(BUNDLED_FONT_NAMES.len())
}

fn family_is_bundled(family: &FontFamilyGroup, names: &[&str]) -> bool {
    let mut folded = vec![fold_font_name(&family.title)];
    for face in &family.faces {
        folded.push(fold_font_name(&face.stored));
        folded.push(fold_font_name(&family_stem(&face.stored)));
    }
    names.iter().any(|name| {
        let alias = fold_font_name(name);
        folded
            .iter()
            .any(|candidate| font_name_matches(candidate, &alias))
    })
}

fn font_name_matches(candidate: &str, alias: &str) -> bool {
    candidate == alias
        || candidate
            .strip_suffix('r')
            .is_some_and(|stem| stem == alias)
        || candidate
            .strip_suffix("regular")
            .is_some_and(|stem| stem == alias)
}

fn fold_font_name(name: &str) -> String {
    name.chars()
        .filter(|scalar| !scalar.is_whitespace() && *scalar != '-' && *scalar != '_')
        .flat_map(|scalar| scalar.to_lowercase())
        .collect()
}

fn stored_face(family: &str, weight: gtk::pango::Weight, style: gtk::pango::Style) -> String {
    let mut words = Vec::new();
    if let Some(word) = weight_word(weight) {
        words.push(word);
    }
    if let Some(word) = style_word(style) {
        words.push(word);
    }
    if words.is_empty() {
        family.to_owned()
    } else {
        format!("{family} {}", words.join(" "))
    }
}

fn face_label(
    family: &str,
    face_name: &str,
    weight: gtk::pango::Weight,
    style: gtk::pango::Style,
) -> String {
    let named = face_name.trim();
    if !named.is_empty() && named != family {
        return named.to_owned();
    }
    match (weight_word(weight), style_word(style)) {
        (None, None) => "Regular".to_owned(),
        (Some(weight), None) => weight.to_owned(),
        (None, Some(style)) => style.to_owned(),
        (Some(weight), Some(style)) => format!("{weight} {style}"),
    }
}

fn weight_word(weight: gtk::pango::Weight) -> Option<&'static str> {
    match weight {
        gtk::pango::Weight::Thin => Some("Thin"),
        gtk::pango::Weight::Ultralight => Some("ExtraLight"),
        gtk::pango::Weight::Light | gtk::pango::Weight::Semilight => Some("Light"),
        gtk::pango::Weight::Book | gtk::pango::Weight::Normal => None,
        gtk::pango::Weight::Medium => Some("Medium"),
        gtk::pango::Weight::Semibold => Some("SemiBold"),
        gtk::pango::Weight::Bold => Some("Bold"),
        gtk::pango::Weight::Ultrabold => Some("ExtraBold"),
        gtk::pango::Weight::Heavy | gtk::pango::Weight::Ultraheavy => Some("Black"),
        _ => None,
    }
}

fn style_word(style: gtk::pango::Style) -> Option<&'static str> {
    match style {
        gtk::pango::Style::Italic => Some("Italic"),
        gtk::pango::Style::Oblique => Some("Oblique"),
        _ => None,
    }
}

fn family_stem(name: &str) -> String {
    let mut words: Vec<&str> = name.split_whitespace().collect();
    while words.len() > 1 && is_style_word(words[words.len() - 1]) {
        words.pop();
    }
    words.join(" ")
}

fn font_button_label(families: &[FontFamilyGroup], stored: &str, system: &str) -> String {
    if stored.is_empty() {
        return system.to_owned();
    }
    for family in families {
        if let Some(face) = family.faces.iter().find(|face| face.stored == stored) {
            return format!("{} — {}", family.title, face.label);
        }
    }
    stored.to_owned()
}

fn build_font_list(
    list: &gtk::ListBox,
    families: &[FontFamilyGroup],
    system: &str,
) -> Vec<FontMenuRow> {
    let mut rows = Vec::new();
    let system_row = adw::ActionRow::builder()
        .title(system)
        .activatable(true)
        .build();
    list.append(&system_row);
    rows.push(FontMenuRow {
        row: system_row,
        expander: None,
        family: String::new(),
        label: system.to_owned(),
        stored: String::new(),
    });
    for family in families {
        let expander = adw::ExpanderRow::builder().title(&family.title).build();
        for face in &family.faces {
            let face_row = adw::ActionRow::builder()
                .title(&face.label)
                .activatable(true)
                .build();
            expander.add_row(&face_row);
            rows.push(FontMenuRow {
                row: face_row,
                expander: Some(expander.clone()),
                family: family.title.clone(),
                label: face.label.clone(),
                stored: face.stored.clone(),
            });
        }
        list.append(&expander);
    }
    rows
}

fn apply_font_search(list: &gtk::ListBox, rows: &[FontMenuRow], query: &str) {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        for entry in rows {
            entry.row.set_visible(true);
            entry.row.changed();
            if let Some(expander) = &entry.expander {
                expander.set_visible(true);
                expander.set_expanded(false);
                expander.changed();
            }
        }
        list.invalidate_filter();
        return;
    }
    let mut families: Vec<(adw::ExpanderRow, bool)> = Vec::new();
    for entry in rows {
        let matches = entry.label.to_lowercase().contains(&query)
            || (!entry.family.is_empty() && entry.family.to_lowercase().contains(&query));
        entry.row.set_visible(matches);
        entry.row.changed();
        let Some(expander) = entry.expander.clone() else {
            continue;
        };
        if let Some(slot) = families.iter_mut().find(|(known, _)| known == &expander) {
            slot.1 |= matches;
        } else {
            families.push((expander, matches));
        }
    }
    for (expander, show) in families {
        expander.set_visible(show);
        if show {
            expander.set_expanded(true);
        }
        expander.changed();
    }
    list.invalidate_filter();
}

fn family_group(name: &str) -> String {
    let mut words: Vec<&str> = name.split_whitespace().collect();
    while words.len() > 1 && is_style_word(words[words.len() - 1]) {
        words.pop();
    }
    words
        .join("")
        .chars()
        .flat_map(|scalar| scalar.to_lowercase())
        .collect()
}

fn is_style_word(word: &str) -> bool {
    matches!(
        word.to_ascii_lowercase().as_str(),
        "thin"
            | "hairline"
            | "extralight"
            | "ultralight"
            | "light"
            | "medium"
            | "semibold"
            | "demibold"
            | "bold"
            | "extrabold"
            | "ultrabold"
            | "black"
            | "heavy"
            | "italic"
            | "oblique"
    )
}

/// The five highlight steps, stored as the window's 0–100 fill.
fn fill_step(stored: i64) -> i64 {
    const STEPS: [i64; 5] = [0, 25, 50, 75, 100];
    let stored = stored.clamp(0, 100);
    STEPS
        .iter()
        .enumerate()
        .min_by_key(|(_, value)| (stored - *value).abs())
        .map(|(index, _)| index as i64 + 1)
        .unwrap_or(1)
}

fn fill_stored(step: i64) -> i64 {
    const STEPS: [i64; 5] = [0, 25, 50, 75, 100];
    STEPS[(step.clamp(1, 5) - 1) as usize]
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
        SettingsPane::Appearance => {
            let (context, field) = appearance::build(context, &widget);
            let view = adw::ToolbarView::builder()
                .content(&widget)
                .top_bar_style(adw::ToolbarStyle::Flat)
                .build();
            view.add_top_bar(&field);
            return context.finish(view.upcast());
        }
        SettingsPane::Shortcuts => shortcuts::build(context, &widget),
        SettingsPane::DictionarySources => {
            let context = dictionary_sources::build(context, &widget);
            return with_search_bar(context, widget);
        }
        SettingsPane::CustomDictionary => custom_dictionary::build(context, &widget),
        SettingsPane::LearningRecords => learning_records::build(context, &widget),
        SettingsPane::About => about::build(context, &widget),
        other => unreachable!("{other:?} is not in pages::BUILT"),
    };
    context.finish(widget.upcast())
}

/// Manage Dictionaries' search, pinned under the scrolling page as on mobile
/// (#792): a raised bottom bar stays put while the toggles scroll.
fn with_search_bar(context: PageContext<'_>, page: adw::PreferencesPage) -> Page {
    let view = adw::ToolbarView::builder()
        .content(&page)
        .bottom_bar_style(adw::ToolbarStyle::Raised)
        .build();
    dictionary_search::build(context, &view).finish(view.upcast())
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

#[cfg(test)]
mod tests {
    use super::{order_font_families, FontFaceChoice, FontFamilyGroup};

    fn family(title: &str) -> FontFamilyGroup {
        FontFamilyGroup {
            title: title.to_owned(),
            faces: vec![FontFaceChoice {
                label: "Regular".to_owned(),
                stored: title.to_owned(),
                synthesized: false,
            }],
        }
    }

    #[test]
    fn bundled_faces_lead_in_the_shared_roster_order() {
        let mut families = vec![
            family("Noto Sans CJK TC"),
            family("GenYoGothic2 TW R"),
            family("DejaVu Sans"),
            family("源樣明體月"),
            family("Iansui"),
            family("jf open 粉圓 2.1"),
            family("jf-openhuninn-2.1"),
        ];
        order_font_families(&mut families);
        let titles: Vec<_> = families
            .iter()
            .map(|family| family.title.as_str())
            .collect();
        assert_eq!(
            titles,
            [
                "jf open 粉圓 2.1",
                "jf-openhuninn-2.1",
                "Iansui",
                "源樣明體月",
                "GenYoGothic2 TW R",
                "DejaVu Sans",
                "Noto Sans CJK TC",
            ]
        );
    }
}
