//! The 字型管理 pane: every typeface the candidate window can be set in — the
//! bundled roster, the ones the user added, and the families this Windows has
//! installed — as one list whose SELECTION is the typeface in use. Port of
//! `FontManagementPage.swift`.
//!
//! One list, one selection, one meaning. A pop-up of bundled faces beside a
//! list of the user's would have carried two selections that look alike and
//! mean different things (which face draws, and which row `−` acts on); the
//! Mac's pane says the same thing at more length.
//!
//! A search box and a pager (`list_pager`) are what make a few hundred
//! installed families usable in one list (USER 2026-09-11) — one page of rows
//! at a time, as 自訂詞庫 does it, because a fixed-height list inside the pane
//! cannot scroll on its own.
//!
//! `+` takes a font file into `%APPDATA%\TaigiKeyboard\Fonts` and selects it —
//! unless this Windows already has the face, in which case the installed row is
//! selected instead (USER 2026-09-11 「跳出提示,並且跳轉到那個字型」); `−` is
//! disabled on the bundled and installed rows — only a typeface the user added
//! can leave the list.

use super::super::cards;
use super::super::list_pager::{self, icon_button};
use super::super::list_selection::{selectable_list, SettledRows};
use super::super::window::{Message as WindowMessage, SettingsWindow};
use crate::presentation::PageMessage;
use crate::settings_writer::SettingsWriter;
use crate::winui::file_dialog;
use taigi_windows_core::settings::{
    set_stored_font_selection, stored_font_selection, CandidateFontChoice, SettingChoice,
    StoredFontSelection,
};
use taigi_windows_core::strings::{StringKey, StringResolver};
use taigi_windows_platform::font_file;
use taigi_windows_storage as storage;
use unicode_normalization::UnicodeNormalization;
use windows_reactor::*;

/// A typeface the user added, as the pane holds it.
///
/// The library file name is the whole of it: it is the identity, it is what the
/// settings document stores, and — without its extension — it is what the list
/// shows. The family the file declares is not held here; it is DirectWrite's
/// business, read where a text format is built (`ui::render`).
#[derive(Clone, PartialEq, Eq)]
pub struct CustomFontRow {
    pub file_name: String,
}

/// A family this Windows has installed, with the key the search and the sort
/// compare it by — computed once when the list is read, not per keystroke.
#[derive(Clone, PartialEq, Eq)]
struct InstalledFamily {
    name: String,
    fold: String,
}

/// A stored file's name without its extension — what the list shows for a
/// typeface the user added.
///
/// The file's name rather than the name the font file declares (USER
/// 2026-09-10). A row is something the user has to recognise as the thing they
/// added, and what they added was a file they chose and named. Untrusted text
/// still: shown, never logged (`storage::sanitized_stem` built it).
fn displayed_name(file_name: &str) -> &str {
    file_name
        .rsplit_once('.')
        .map_or(file_name, |(stem, _)| stem)
}

/// `text` as the search compares it: lowercase, with its combining marks
/// stripped, so `tai` finds `Tâi` — the diacritic-insensitive contains the
/// Mac's `localizedStandardContains` gives the same list. One fold for the
/// sort as well, so the order the user reads is the order they search in.
fn search_key(text: &str) -> String {
    text.nfd()
        .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
        .flat_map(char::to_lowercase)
        .collect()
}

#[derive(Default)]
pub struct FontManagementModel {
    /// The user's own typefaces, as the last read of the folder found them.
    custom_fonts: Vec<CustomFontRow>,
    /// The families this Windows has installed, as the last read found them,
    /// sorted by `fold`.
    installed_families: Vec<InstalledFamily>,
    /// What the search box holds; the rows are narrowed by it.
    filter: String,
    /// Which page of the narrowed rows is on screen, zero-based. Clamped when
    /// read: the rows under it change with the filter and with what the OS
    /// has, and a page past the end shows the last one rather than nothing.
    page: usize,
    /// Which rows the list on screen holds, so the selection index reaches
    /// XAML a render after the row it names does (`list_selection`).
    settled: SettledRows,
}

impl FontManagementModel {
    /// How many rows the whole list has: the bundled roster, then what the
    /// user added, then what the OS has.
    fn row_count(&self) -> usize {
        CandidateFontChoice::ALL.len() + self.custom_fonts.len() + self.installed_families.len()
    }

    /// What selecting row `index` stores. The roster is virtual — three
    /// vectors read as one — so a row is a position, and the `Row` a page
    /// shows is built only for the rows on it.
    fn selection_at(&self, index: usize) -> Option<StoredFontSelection> {
        let bundled = CandidateFontChoice::ALL.len();
        if index < bundled {
            return Some(StoredFontSelection::BuiltIn(
                CandidateFontChoice::ALL[index],
            ));
        }
        let index = index - bundled;
        if let Some(font) = self.custom_fonts.get(index) {
            return Some(StoredFontSelection::Custom(font.file_name.clone()));
        }
        self.installed_families
            .get(index - self.custom_fonts.len())
            .map(|family| StoredFontSelection::Installed(family.name.clone()))
    }

    /// The row `index` as the list shows it.
    fn row_at(&self, index: usize, strings: &StringResolver) -> Option<Row> {
        let selection = self.selection_at(index)?;
        let title = match &selection {
            StoredFontSelection::BuiltIn(choice) => strings.resolve(choice.label_key()).to_owned(),
            // The stored file's name — text the user chose: shown, never
            // trusted.
            StoredFontSelection::Custom(file_name) => displayed_name(file_name).to_owned(),
            StoredFontSelection::Installed(family) => family.clone(),
        };
        Some(Row { title, selection })
    }

    /// Whether row `index` matches `needle` (a `search_key`). The installed
    /// families carry their key precomputed; the handful of bundled and
    /// imported rows fold on the spot.
    fn matches(&self, index: usize, needle: &str, strings: &StringResolver) -> bool {
        let bundled = CandidateFontChoice::ALL.len();
        if index < bundled {
            return search_key(strings.resolve(CandidateFontChoice::ALL[index].label_key()))
                .contains(needle);
        }
        let index = index - bundled;
        if let Some(font) = self.custom_fonts.get(index) {
            return search_key(displayed_name(&font.file_name)).contains(needle);
        }
        self.installed_families
            .get(index - self.custom_fonts.len())
            .is_some_and(|family| family.fold.contains(needle))
    }

    /// The page on screen: the rows the search box leaves, narrowed to
    /// `PAGE_SIZE`, with the clamped page it is and how many pages there are.
    fn visible_rows(&self, strings: &StringResolver) -> VisibleRows {
        let needle = search_key(&self.filter);
        let matching: Vec<usize> = (0..self.row_count())
            .filter(|&index| needle.is_empty() || self.matches(index, &needle, strings))
            .collect();
        let page_count = list_pager::page_count(matching.len(), PAGE_SIZE);
        let page = self.page.min(page_count - 1);
        let rows = matching
            .into_iter()
            .skip(page * PAGE_SIZE)
            .take(PAGE_SIZE)
            .filter_map(|index| self.row_at(index, strings))
            .collect();
        VisibleRows {
            rows,
            page,
            page_count,
        }
    }

    /// The custom typeface `−` may act on: the stored selection, while its
    /// row is on the page on screen. A custom typeface the search or the pager
    /// has hidden must not be deletable beside a list showing no selection —
    /// the one rule `view` disables the button by and `remove` refuses by.
    fn removable_custom_file(
        &self,
        stored: &StoredFontSelection,
        strings: &StringResolver,
    ) -> Option<String> {
        let StoredFontSelection::Custom(file_name) = stored else {
            return None;
        };
        self.visible_rows(strings)
            .rows
            .iter()
            .any(|row| &row.selection == stored)
            .then(|| file_name.clone())
    }

    /// Turns to the page `selection`'s row is on, with the search cleared: a
    /// row the user just added or was sent to lands after the bundled five and
    /// the other imports, which may be past the first page, and a search
    /// would hide it.
    fn show(&mut self, selection: &StoredFontSelection) {
        self.filter.clear();
        if let Some(index) = (0..self.row_count())
            .find(|&index| self.selection_at(index).as_ref() == Some(selection))
        {
            self.page = index / PAGE_SIZE;
        }
    }

    fn has_installed(&self, family: &str) -> bool {
        self.installed_families
            .iter()
            .any(|installed| installed.name == family)
    }
}

/// One row of the list, built for the page on screen.
struct Row {
    title: String,
    selection: StoredFontSelection,
}

impl Row {
    /// What the list keys this row by. The SELECTION, never the title: two
    /// files can declare the same family name, and a duplicate key is a
    /// reconciliation error rather than a second row.
    fn key(&self) -> String {
        match &self.selection {
            StoredFontSelection::BuiltIn(choice) => format!("builtIn.{}", choice.raw()),
            StoredFontSelection::Custom(file_name) => format!("custom.{file_name}"),
            StoredFontSelection::Installed(family) => format!("installed.{family}"),
        }
    }
}

/// One page of rows, as the list shows them.
struct VisibleRows {
    rows: Vec<Row>,
    page: usize,
    page_count: usize,
}

#[derive(Clone)]
pub enum Message {
    /// The list's selection moved: the row's index, or `None` when the list
    /// cleared it (which writes nothing — a typeface is always in use).
    Select(Option<usize>),
    Add,
    Remove,
    /// The search box's text moved.
    FilterChanged(String),
    /// The pager asked for this page.
    ShowPage(usize),
    /// What the list on screen holds, as `list_selection` reports it.
    RowsApplied(Option<Vec<String>>),
}

pub struct PageEnvironment<'a> {
    pub settings: &'a mut SettingsWriter,
    pub message: &'a mut Option<PageMessage>,
}

/// Reads the library and the OS's families, every time the pane is entered.
///
/// Every time rather than once: the user may have been in Explorer, or in
/// Windows' own font settings, since they last looked (`FontManagementPage
/// .swift`'s `onAppear`). Not at window launch, though — parsing font files
/// and enumerating the OS's are not free, and a user who never opens this pane
/// should never pay for them, which is the same reason the DLL loads only the
/// SELECTED typeface.
pub fn on_enter(model: &mut FontManagementModel) {
    reload_installed(model);
    reload_custom(model);
}

pub fn update(model: &mut FontManagementModel, message: Message, environment: PageEnvironment<'_>) {
    match message {
        Message::Select(Some(index)) => {
            // The index names a row of the list ON SCREEN, which is not always
            // the list this model would draw now — so it is resolved through
            // the rows XAML holds, and a row the library no longer has writes
            // nothing (`list_selection`).
            let Some(key) = model.settled.key_at(index).map(str::to_owned) else {
                return;
            };
            let strings = environment.settings.strings();
            if let Some(row) = model
                .visible_rows(&strings)
                .rows
                .into_iter()
                .find(|row| row.key() == key)
            {
                environment
                    .settings
                    .update(|document| set_stored_font_selection(document, &row.selection));
            }
        }
        Message::Select(None) => {}
        Message::Add => add(model, environment),
        Message::Remove => remove(model, environment),
        Message::FilterChanged(filter) => {
            // Unchanged text is the box echoing a value this model set (the
            // clear after an import) — not a new search, so the page it
            // turned to stays.
            if filter != model.filter {
                model.filter = filter;
                model.page = 0;
            }
        }
        Message::ShowPage(page) => model.page = page,
        Message::RowsApplied(rows) => model.settled.report(rows),
    }
}

/// Takes a font file into the library and selects it — or, when this Windows
/// already has the face, selects the installed row instead.
///
/// Validation runs against the COPY, not the file the user picked: the copy is
/// what will be drawn, and the original may change or go away. A file that is
/// not a typeface this Windows can read is taken back out again, so a refused
/// import leaves nothing behind (`CustomFontLibrary.addFont`).
///
/// A face the OS already has is not taken in twice. The user asked to type in
/// it, not to own a copy of it, so the copy goes back out and the installed
/// row is what gets selected. The file's name is irrelevant: the family is
/// known by the name inside the file, read by the same rule the installed list
/// is (`font_file::FAMILY_NAME_LOCALE`).
fn add(model: &mut FontManagementModel, environment: PageEnvironment<'_>) {
    let Some(source) = file_dialog::pick_font() else {
        return;
    };
    let selection = match take_in(model, &source) {
        Ok(selection) => selection,
        Err(error) => {
            *environment.message = Some(PageMessage::failure(StringKey::CommonImportFailed, error));
            reload_custom(model);
            return;
        }
    };
    if matches!(selection, StoredFontSelection::Installed(_)) {
        *environment.message = Some(PageMessage::Done(
            StringKey::DesktopCustomFontAlreadyInstalled,
        ));
    }
    environment
        .settings
        .update(|document| set_stored_font_selection(document, &selection));
    reload_custom(model);
    model.show(&selection);
}

/// The file operations of an import, in order: copy, validate, and — for a
/// face the OS has — take the copy back out. What the user is told is the
/// caller's; a copy that will not go is worth saying so, because it stays in
/// the list as a row that cannot be selected.
fn take_in(
    model: &FontManagementModel,
    source: &std::path::Path,
) -> Result<StoredFontSelection, String> {
    let directory = storage::fonts_directory().map_err(|error| error.to_string())?;
    let stored = storage::copy_in(&directory, source).map_err(|error| error.to_string())?;
    let discard = |error: String| match storage::remove_stored(&directory, &stored) {
        Ok(()) => error,
        Err(removal) => format!("{error}; the copy could not be removed either: {removal}"),
    };
    let info = match font_file::inspect(&directory.join(&stored)) {
        Ok(info) => info,
        Err(error) => return Err(discard(error.to_string())),
    };
    if model.has_installed(&info.family_name) {
        storage::remove_stored(&directory, &stored)
            .map_err(|removal| format!("the copy could not be removed: {removal}"))?;
        return Ok(StoredFontSelection::Installed(info.family_name));
    }
    Ok(StoredFontSelection::Custom(stored))
}

/// Takes the selected typeface out of the library.
///
/// The selection moves off it first, so the candidate window's next show does
/// not ask for a file that is about to go. A file another host process has
/// mapped may refuse to be deleted; that is reported and the row stays, so the
/// user can try again once those hosts have moved on.
fn remove(model: &mut FontManagementModel, environment: PageEnvironment<'_>) {
    let stored = stored_font_selection(environment.settings.document());
    let Some(file_name) = model.removable_custom_file(&stored, &environment.settings.strings())
    else {
        return;
    };
    let directory = match storage::fonts_directory() {
        Ok(directory) => directory,
        Err(error) => {
            *environment.message = Some(PageMessage::failure(
                StringKey::DesktopCustomFontRemoveFailed,
                error,
            ));
            return;
        }
    };
    let fallback = StoredFontSelection::BuiltIn(CandidateFontChoice::DEFAULT);
    environment
        .settings
        .update(|document| set_stored_font_selection(document, &fallback));
    if let Err(error) = storage::remove_stored(&directory, &file_name) {
        *environment.message = Some(PageMessage::failure(
            StringKey::DesktopCustomFontRemoveFailed,
            error,
        ));
    }
    reload_custom(model);
}

/// Re-reads the OS's families. A failed read logs and lists nothing — the
/// bundled and imported rows still stand, and one list not being readable
/// must not empty the other.
fn reload_installed(model: &mut FontManagementModel) {
    model.installed_families = match font_file::system_families() {
        Ok(families) => {
            let mut families: Vec<InstalledFamily> = families
                .into_iter()
                .map(|name| InstalledFamily {
                    fold: search_key(&name),
                    name,
                })
                .collect();
            families.sort_by(|a, b| a.fold.cmp(&b.fold).then_with(|| a.name.cmp(&b.name)));
            families
        }
        Err(error) => {
            log::warn!("fonts.system_families_unavailable error={error}");
            Vec::new()
        }
    };
}

/// Re-reads the library.
///
/// A file that no longer parses is skipped rather than failing the scan: one
/// unreadable file must not empty the list of the others.
fn reload_custom(model: &mut FontManagementModel) {
    let Ok(directory) = storage::fonts_directory() else {
        model.custom_fonts = Vec::new();
        return;
    };
    model.custom_fonts = storage::stored_file_names(&directory)
        .into_iter()
        .filter_map(|file_name| {
            // Inspected, and the answer thrown away: what the list needs is
            // the file name it already has, but a file DirectWrite cannot read
            // is not a row — the same skip the macOS scan makes.
            font_file::inspect(&directory.join(&file_name)).ok()?;
            Some(CustomFontRow { file_name })
        })
        .collect();
}

pub fn view(
    window: &SettingsWindow,
    strings: &StringResolver,
    context: &mut ViewContext<SettingsWindow>,
) -> View {
    let model = window.font_management();
    let stored = stored_font_selection(window.document());
    let VisibleRows {
        rows,
        page,
        page_count,
    } = model.visible_rows(strings);
    // Over the VISIBLE rows: the stored selection stays put while the search
    // or the pager hides its row, and the list then shows no selection.
    let selected = rows.iter().position(|row| row.selection == stored);
    let is_enabled = !window.is_read_only();
    let can_remove = is_enabled && model.removable_custom_file(&stored, strings).is_some();
    let items = rows
        .iter()
        .map(|row| {
            let key = row.key();
            (
                key.clone(),
                ListViewItem::new()
                    .tag(key)
                    .content(TextBlock::new().text(row.title.clone())),
            )
        })
        .collect::<Vec<_>>();
    // The index reaches XAML a render after the row it names (`list_selection`);
    // `can_remove` above stays on the STORED selection, so the `−` button does
    // not blink off for that render.
    let list = selectable_list(
        "fontManagement.rows",
        &model.settled,
        items,
        selected,
        ListView::new()
            .selection_mode(ListViewSelectionMode::Single)
            .on_selection_changed(
                context.callback(|index| WindowMessage::FontManagement(Message::Select(index))),
            )
            .height(LIST_HEIGHT),
        context,
        |rows| WindowMessage::FontManagement(Message::RowsApplied(rows)),
    );
    View::fragment((cards::frame(View::fragment((
        missing_note(model, &stored, strings),
        TextBox::new()
            .text(model.filter.clone())
            .is_enabled(is_enabled)
            .placeholder_text(strings.resolve(StringKey::DictionarySearchPlaceholder))
            .margin(Thickness::new(0.0, 0.0, 0.0, list_pager::CONTROL_GAP))
            .on_text_changed(
                context
                    .callback(|text| WindowMessage::FontManagement(Message::FilterChanged(text))),
            ),
        list,
        list_pager::bar(
            (
                icon_button(
                    ADD_GLYPH,
                    strings.resolve(StringKey::DesktopCustomFontAdd),
                    is_enabled,
                    context.callback(|()| WindowMessage::FontManagement(Message::Add)),
                ),
                icon_button(
                    REMOVE_GLYPH,
                    strings.resolve(StringKey::CommonDelete),
                    can_remove,
                    context.callback(|()| WindowMessage::FontManagement(Message::Remove)),
                ),
            ),
            page,
            page_count,
            is_enabled,
            strings,
            context,
            |page| WindowMessage::FontManagement(Message::ShowPage(page)),
        ),
    ))),))
}

/// The selected typeface's file is gone or stopped being one this Windows can
/// read, or the OS no longer has the selected family. Said rather than silently
/// corrected: the preference is kept, and an external volume, a restore or a
/// reinstall may bring it back. Against the whole roster, not the page on
/// screen: a row the search hides is not a typeface that is gone.
fn missing_note(
    model: &FontManagementModel,
    stored: &StoredFontSelection,
    strings: &StringResolver,
) -> View {
    let is_missing = match stored {
        StoredFontSelection::BuiltIn(_) => false,
        StoredFontSelection::Custom(file_name) => !model
            .custom_fonts
            .iter()
            .any(|font| &font.file_name == file_name),
        StoredFontSelection::Installed(family) => !model.has_installed(family),
    };
    if !is_missing {
        return View::empty();
    }
    TextBlock::new()
        .text(strings.resolve(StringKey::DesktopCustomFontMissing))
        .opacity(list_pager::SECONDARY_OPACITY)
        .margin(Thickness::new(0.0, 0.0, 0.0, list_pager::CONTROL_GAP))
        .into()
}

/// How many rows one page holds — the list's height, exactly, as 自訂詞庫 does
/// it: a page that fits the list never needs a scroller of its own, which a
/// list inside the pane's scroll view cannot have (`list_pager`).
const PAGE_SIZE: usize = 10;
const LIST_HEIGHT: f64 = PAGE_SIZE as f64 * 32.0;
const ADD_GLYPH: &str = "\u{E710}";
const REMOVE_GLYPH: &str = "\u{E738}";

#[cfg(test)]
mod tests {
    use super::*;

    fn custom(file_name: &str) -> CustomFontRow {
        CustomFontRow {
            file_name: file_name.to_owned(),
        }
    }

    fn installed(names: &[&str]) -> Vec<InstalledFamily> {
        names
            .iter()
            .map(|name| InstalledFamily {
                name: (*name).to_owned(),
                fold: search_key(name),
            })
            .collect()
    }

    fn strings() -> StringResolver {
        StringResolver::new(taigi_windows_core::strings::DisplayLanguage::English)
    }

    /// Every row of `model`, as the list would show them with no search.
    fn all_rows(model: &FontManagementModel) -> Vec<Row> {
        let strings = strings();
        (0..model.row_count())
            .filter_map(|index| model.row_at(index, &strings))
            .collect()
    }

    /// Two files may be stored under one stem — `mine.ttf` beside `mine.otf` —
    /// and the list keys its rows by identity for exactly that reason: a
    /// duplicate key is a reconciliation error, not a second row.
    #[test]
    fn two_typefaces_with_one_displayed_name_are_two_rows() {
        let model = FontManagementModel {
            custom_fonts: vec![custom("mine.ttf"), custom("mine-2.ttf")],
            ..FontManagementModel::default()
        };

        let keys: Vec<String> = all_rows(&model).iter().map(Row::key).collect();
        let unique: std::collections::BTreeSet<&String> = keys.iter().collect();

        assert_eq!(keys.len(), unique.len(), "duplicate list keys: {keys:?}");
        assert!(keys.contains(&"custom.mine.ttf".to_owned()));
        assert!(keys.contains(&"custom.mine-2.ttf".to_owned()));
    }

    /// A custom typeface whose file is named like a bundled row's label is
    /// still its own row.
    #[test]
    fn a_custom_typeface_named_like_a_bundled_one_is_its_own_row() {
        let strings = strings();
        let bundled_label = strings
            .resolve(CandidateFontChoice::Iansui.label_key())
            .to_owned();
        let model = FontManagementModel {
            custom_fonts: vec![custom(&format!("{bundled_label}.ttf"))],
            ..FontManagementModel::default()
        };

        let rows = all_rows(&model);
        let keys: Vec<String> = rows.iter().map(Row::key).collect();
        let unique: std::collections::BTreeSet<&String> = keys.iter().collect();

        assert_eq!(keys.len(), unique.len());
        assert_eq!(rows.len(), CandidateFontChoice::ALL.len() + 1);
    }

    /// The list shows the stored file's name without its extension — the user's
    /// own spelling, kept through the import (`storage::sanitized_stem`).
    #[test]
    fn a_custom_row_is_titled_with_its_file_name_without_the_extension() {
        let model = FontManagementModel {
            custom_fonts: vec![custom("SnailFont-Pomacea.ttf"), custom("源樣明體.otf")],
            ..FontManagementModel::default()
        };

        let titles: Vec<String> = all_rows(&model)
            .into_iter()
            .skip(CandidateFontChoice::ALL.len())
            .map(|row| row.title)
            .collect();

        assert_eq!(titles, vec!["SnailFont-Pomacea", "源樣明體"]);
    }

    /// A name carrying an interior dot keeps it — only the LAST one separates
    /// the extension.
    #[test]
    fn only_the_last_dot_separates_the_extension() {
        assert_eq!(displayed_name("jf-openhuninn-2.1.ttf"), "jf-openhuninn-2.1");
        assert_eq!(displayed_name("noextension"), "noextension");
    }

    /// The OS's families come after the bundled roster and the imports, keyed
    /// by their own kind so a family named like a file is still its own row.
    #[test]
    fn installed_families_follow_the_bundled_and_imported_rows() {
        let model = FontManagementModel {
            custom_fonts: vec![custom("Arial.ttf")],
            installed_families: installed(&["Arial", "Microsoft JhengHei"]),
            ..FontManagementModel::default()
        };

        let keys: Vec<String> = all_rows(&model).iter().map(Row::key).collect();

        assert_eq!(keys.len(), CandidateFontChoice::ALL.len() + 3);
        assert_eq!(
            &keys[CandidateFontChoice::ALL.len()..],
            &[
                "custom.Arial.ttf",
                "installed.Arial",
                "installed.Microsoft JhengHei"
            ],
        );
    }

    /// Only a typeface the user added can leave the list, and only while its
    /// row is on the page on screen.
    #[test]
    fn only_a_visible_custom_row_is_removable() {
        let model = FontManagementModel {
            custom_fonts: vec![custom("mine.ttf")],
            installed_families: installed(&["Arial"]),
            ..FontManagementModel::default()
        };
        let strings = strings();
        let mine = StoredFontSelection::Custom("mine.ttf".to_owned());

        assert_eq!(
            model.removable_custom_file(&mine, &strings),
            Some("mine.ttf".to_owned())
        );
        assert_eq!(
            model.removable_custom_file(
                &StoredFontSelection::Installed("Arial".to_owned()),
                &strings
            ),
            None,
        );
        assert_eq!(
            model.removable_custom_file(
                &StoredFontSelection::BuiltIn(CandidateFontChoice::Iansui),
                &strings,
            ),
            None,
        );

        let hidden = FontManagementModel {
            filter: "arial".to_owned(),
            ..model
        };
        assert_eq!(hidden.removable_custom_file(&mine, &strings), None);
    }

    /// Case- and diacritic-insensitive, over every kind alike.
    #[test]
    fn the_search_ignores_case_and_diacritics() {
        let model = FontManagementModel {
            filter: "tai".to_owned(),
            installed_families: installed(&["Tâi-gí Sans", "Arial", "TAIPEI Mono"]),
            ..FontManagementModel::default()
        };

        let titles: Vec<String> = model
            .visible_rows(&strings())
            .rows
            .into_iter()
            .map(|row| row.title)
            .collect();

        assert_eq!(titles, vec!["Tâi-gí Sans", "TAIPEI Mono"]);
    }

    /// A page holds `PAGE_SIZE` rows; the last one holds the rest; a page
    /// past the end is clamped rather than empty.
    #[test]
    fn pages_hold_page_size_rows_and_clamp_past_the_end() {
        let names: Vec<String> = (0..17).map(|i| format!("Family {i:02}")).collect();
        let mut model = FontManagementModel {
            installed_families: installed(&names.iter().map(String::as_str).collect::<Vec<_>>()),
            ..FontManagementModel::default()
        };
        let total = CandidateFontChoice::ALL.len() + 17; // 22

        let first = model.visible_rows(&strings());
        assert_eq!(
            (first.page, first.page_count, first.rows.len()),
            (0, 3, PAGE_SIZE)
        );

        model.page = 2;
        let last = model.visible_rows(&strings());
        assert_eq!(last.rows.len(), total - 2 * PAGE_SIZE);

        model.page = 99;
        let clamped = model.visible_rows(&strings());
        assert_eq!(clamped.page, 2);
        assert_eq!(clamped.rows.len(), last.rows.len());
    }

    /// No match is still one page, so the pager reads "1 / 1".
    #[test]
    fn a_search_with_no_match_is_one_empty_page() {
        let model = FontManagementModel {
            filter: "zzz-no-such-family".to_owned(),
            installed_families: installed(&["Arial"]),
            ..FontManagementModel::default()
        };

        let visible = model.visible_rows(&strings());

        assert_eq!((visible.page, visible.page_count), (0, 1));
        assert!(visible.rows.is_empty());
    }

    /// Sending the user to a row clears the search and turns to its page.
    #[test]
    fn show_clears_the_search_and_turns_to_the_rows_page() {
        let names: Vec<String> = (0..17).map(|i| format!("Family {i:02}")).collect();
        let mut model = FontManagementModel {
            installed_families: installed(&names.iter().map(String::as_str).collect::<Vec<_>>()),
            filter: "nothing".to_owned(),
            ..FontManagementModel::default()
        };

        model.show(&StoredFontSelection::Installed("Family 16".to_owned()));

        assert_eq!(model.filter, "");
        assert_eq!(
            model.page,
            (CandidateFontChoice::ALL.len() + 16) / PAGE_SIZE
        );
    }
}
