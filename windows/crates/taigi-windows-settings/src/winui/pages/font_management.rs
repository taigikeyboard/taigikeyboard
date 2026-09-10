//! The 字型管理 pane: every typeface the candidate window can be set in — the
//! bundled roster and the ones the user added — as one list whose SELECTION is
//! the typeface in use. Port of `FontManagementPage.swift`.
//!
//! One list, one selection, one meaning. A pop-up of bundled faces beside a
//! list of the user's would have carried two selections that look alike and
//! mean different things (which face draws, and which row `−` acts on); the
//! Mac's pane says the same thing at more length.
//!
//! `+` takes a font file into `%APPDATA%\TaigiKeyboard\Fonts` and selects it;
//! `−` is disabled on the bundled rows — only a typeface the user added can
//! leave the list.

use super::super::cards;
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

#[derive(Default)]
pub struct FontManagementModel {
    /// The user's own typefaces, as the last read of the folder found them.
    custom_fonts: Vec<CustomFontRow>,
    /// Which rows the list on screen holds, so the selection index reaches
    /// XAML a render after the row it names does (`list_selection`).
    settled: SettledRows,
}

impl FontManagementModel {
    /// The rows the list shows: the bundled roster in the order the four
    /// platforms share, then what the user added.
    fn rows(&self, strings: &StringResolver) -> Vec<Row> {
        CandidateFontChoice::ALL
            .iter()
            .map(|choice| Row {
                title: strings.resolve(choice.label_key()).to_owned(),
                selection: StoredFontSelection::BuiltIn(*choice),
                custom_file_name: None,
            })
            .chain(self.custom_fonts.iter().map(|font| Row {
                title: displayed_name(&font.file_name).to_owned(),
                selection: StoredFontSelection::Custom(font.file_name.clone()),
                custom_file_name: Some(font.file_name.clone()),
            }))
            .collect()
    }
}

/// One row of the list.
struct Row {
    title: String,
    selection: StoredFontSelection,
    /// `Some` for a typeface the user added — the only kind `−` can remove.
    custom_file_name: Option<String>,
}

impl Row {
    /// What the list keys this row by. The SELECTION, never the title: two
    /// files can declare the same family name, and a duplicate key is a
    /// reconciliation error rather than a second row.
    fn key(&self) -> String {
        match &self.selection {
            StoredFontSelection::BuiltIn(choice) => format!("builtIn.{}", choice.raw()),
            StoredFontSelection::Custom(file_name) => format!("custom.{file_name}"),
        }
    }
}

#[derive(Clone)]
pub enum Message {
    /// The list's selection moved: the row's index, or `None` when the list
    /// cleared it (which writes nothing — a typeface is always in use).
    Select(Option<usize>),
    Add,
    Remove,
    /// What the list on screen holds, as `list_selection` reports it.
    RowsApplied(Option<Vec<String>>),
}

pub struct PageEnvironment<'a> {
    pub settings: &'a mut SettingsWriter,
    pub message: &'a mut Option<PageMessage>,
}

/// Reads the library, every time the pane is entered.
///
/// Every time rather than once: the user may have been in Explorer since they
/// last looked, and the folder is theirs (`FontManagementPage.swift`'s
/// `onAppear`). Not at window launch, though — parsing font files is not free,
/// and a user who never opens this pane should never pay for it, which is the
/// same reason the DLL loads only the SELECTED typeface.
pub fn on_enter(model: &mut FontManagementModel) {
    reload(model);
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
            let rows = model.rows(&environment.settings.strings());
            if let Some(row) = rows.iter().find(|row| row.key() == key) {
                let selection = row.selection.clone();
                environment
                    .settings
                    .update(|document| set_stored_font_selection(document, &selection));
            }
        }
        Message::Select(None) => {}
        Message::Add => add(model, environment),
        Message::Remove => remove(model, environment),
        Message::RowsApplied(rows) => model.settled.report(rows),
    }
}

/// Takes a font file into the library and selects it.
///
/// Validation runs against the COPY, not the file the user picked: the copy is
/// what will be drawn, and the original may change or go away. A file that is
/// not a typeface this Windows can read is taken back out again, so a refused
/// import leaves nothing behind (`CustomFontLibrary.addFont`).
fn add(model: &mut FontManagementModel, environment: PageEnvironment<'_>) {
    let Some(source) = file_dialog::pick_font() else {
        return;
    };
    let directory = match storage::fonts_directory() {
        Ok(directory) => directory,
        Err(error) => {
            *environment.message = Some(PageMessage::failure(StringKey::CommonImportFailed, error));
            return;
        }
    };
    let stored = match storage::copy_in(&directory, &source) {
        Ok(stored) => stored,
        Err(error) => {
            *environment.message = Some(PageMessage::failure(StringKey::CommonImportFailed, error));
            return;
        }
    };
    if let Err(error) = font_file::inspect(&directory.join(&stored)) {
        // The copy is taken back out; a copy that will not go is worth saying
        // so, because it stays in the list as a row that cannot be selected.
        *environment.message = match storage::remove_stored(&directory, &stored) {
            Ok(()) => Some(PageMessage::failure(StringKey::CommonImportFailed, error)),
            Err(removal) => Some(PageMessage::failure(
                StringKey::CommonImportFailed,
                format!("{error}; the copy could not be removed either: {removal}"),
            )),
        };
        reload(model);
        return;
    }
    let selection = StoredFontSelection::Custom(stored);
    environment
        .settings
        .update(|document| set_stored_font_selection(document, &selection));
    reload(model);
}

/// Takes the selected typeface out of the library.
///
/// The selection moves off it first, so the candidate window's next show does
/// not ask for a file that is about to go. A file another host process has
/// mapped may refuse to be deleted; that is reported and the row stays, so the
/// user can try again once those hosts have moved on.
fn remove(model: &mut FontManagementModel, environment: PageEnvironment<'_>) {
    let document = environment.settings.document();
    let StoredFontSelection::Custom(file_name) = stored_font_selection(document) else {
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
    reload(model);
}

/// Re-reads the library.
///
/// A file that no longer parses is skipped rather than failing the scan: one
/// unreadable file must not empty the list of the others.
fn reload(model: &mut FontManagementModel) {
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
    let rows = model.rows(strings);
    let selected = selected_index(window.document(), &rows);
    let is_enabled = !window.is_read_only();
    let can_remove = is_enabled
        && selected
            .and_then(|index| rows.get(index))
            .is_some_and(|row| row.custom_file_name.is_some());
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
        missing_note(window, strings),
        list,
        list_controls(strings, context, is_enabled, can_remove),
    ))),))
}

/// The selected typeface's file is gone, or stopped being one this Windows can
/// read. Said rather than silently corrected: the preference is kept, and an
/// external volume or a restore may bring the file back.
fn missing_note(window: &SettingsWindow, strings: &StringResolver) -> View {
    let is_missing = matches!(
        stored_font_selection(window.document()),
        StoredFontSelection::Custom(ref file_name)
            if !window
                .font_management()
                .custom_fonts
                .iter()
                .any(|font| &font.file_name == file_name)
    );
    if !is_missing {
        return View::empty();
    }
    TextBlock::new()
        .text(strings.resolve(StringKey::DesktopCustomFontMissing))
        .opacity(SECONDARY_OPACITY)
        .margin(Thickness::new(0.0, 0.0, 0.0, CONTROL_GAP))
        .into()
}

/// The `+` / `−` pair under the list, where Windows puts the add and remove
/// verbs for an editable list. `−` is disabled rather than hidden, so the pair
/// keeps its shape.
fn list_controls(
    strings: &StringResolver,
    context: &mut ViewContext<SettingsWindow>,
    is_enabled: bool,
    can_remove: bool,
) -> impl Into<View> {
    StackPanel::new()
        .orientation(Orientation::Horizontal)
        .spacing(CONTROL_GAP)
        .margin(Thickness::new(0.0, CONTROL_GAP, 0.0, 0.0))
        .children((
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
        ))
}

fn icon_button(
    glyph: &str,
    tooltip: &str,
    is_enabled: bool,
    on_click: Callback<()>,
) -> impl Into<View> {
    Button::new()
        .is_enabled(is_enabled)
        .on_click(on_click)
        .content(FontIcon::new().glyph(glyph))
        .tooltip(tooltip)
}

/// Which row the two stored keys name.
fn selected_index(
    document: &taigi_windows_core::settings::SettingsDocument,
    rows: &[Row],
) -> Option<usize> {
    let stored = stored_font_selection(document);
    rows.iter().position(|row| row.selection == stored)
}

/// Seven rows: the bundled five plus a couple the user added, without leaving
/// a stretch of empty rows under a fresh install's list.
const LIST_HEIGHT: f64 = 7.0 * 32.0;
const CONTROL_GAP: f64 = 8.0;
const SECONDARY_OPACITY: f64 = 0.6;
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

    fn strings() -> StringResolver {
        StringResolver::new(taigi_windows_core::strings::DisplayLanguage::English)
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

        let rows = model.rows(&strings());
        let keys: Vec<String> = rows.iter().map(Row::key).collect();
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

        let rows = model.rows(&strings);
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

        let titles: Vec<String> = model
            .rows(&strings())
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
}
