//! The lookup table as the shell draws it: the candidate list, the symbol
//! picker or the Telex guide, with labels, highlight and orientation.

use super::{Emit, EngineState, LookupTableContent, SymbolPicker, PAGE_SIZE};
use crate::composing::CandidateCellContent;
use crate::keys::{telex_guide_rows, CandidateSlotKeySet, ComposingKeyBindings};
use crate::runtime::DesktopRuntime;
use crate::settings::{keys, CandidateLayout, SettingsDocument};
use crate::strings::StringKey;

/// The lookup table as things now stand — the symbol picker while it is
/// up, else the candidate list, else the Telex guide if it is wanted: shown,
/// updated in place, or taken down. The guide gives way to the list because
/// the framework has one table to draw (#684). Appended after the composing
/// signals so the preedit the list describes is already on screen.
pub(crate) fn present_table(
    runtime: &DesktopRuntime,
    state: &mut EngineState,
    settings: &SettingsDocument,
    bindings: &ComposingKeyBindings,
    emits: &mut Vec<Emit>,
) {
    if let Some(picker) = &state.symbol_picker {
        emits.push(Emit::LookupTable(symbol_picker_table(
            picker, settings, bindings,
        )));
        state.is_table_shown = true;
        return;
    }
    let slot_key_set = bindings.slot_key_set();
    if state.candidates.is_empty() {
        if settings.is_telex_guide_wanted() {
            emits.push(Emit::LookupTable(telex_guide_table(runtime, settings)));
            state.is_table_shown = true;
        } else if state.is_table_shown {
            emits.push(Emit::HideLookupTable);
            state.is_table_shown = false;
        }
        return;
    }
    emits.push(Emit::LookupTable(table_content(
        state,
        settings,
        slot_key_set,
    )));
    state.is_table_shown = true;
}

/// The cells as the panel draws them; labels = the slot keys, one per position of the
/// CURRENT page. The §34 literal takes no key, as on macOS and Windows: on
/// the page it begins, its label is blank and the keys start at the next
/// cell. The panel can show that because both pages its table through the
/// core, and the table — labels included — is sent again with every reply.
fn table_content(
    state: &EngineState,
    settings: &SettingsDocument,
    slot_key_set: CandidateSlotKeySet,
) -> LookupTableContent {
    let cells = state.candidates.cells();
    let shift = state.selection.key_slot_shift();
    let labels = (0..PAGE_SIZE)
        .map(|position| match position.checked_sub(shift) {
            Some(slot) => slot_key_set.label_for_slot(slot),
            None => String::new(),
        })
        .collect();
    LookupTableContent {
        cells,
        labels,
        cursor: state.selection.selected_index().unwrap_or(0) as u32,
        cursor_visible: true,
        page_size: PAGE_SIZE as u32,
        vertical: is_vertical_layout(settings),
    }
}

/// The picker as the panel draws it: the symbols with the slot-key labels,
/// the highlight, the layout's orientation.
fn symbol_picker_table(
    picker: &SymbolPicker,
    settings: &SettingsDocument,
    bindings: &ComposingKeyBindings,
) -> LookupTableContent {
    let slot_key_set = bindings.slot_key_set();
    LookupTableContent {
        cells: plain_cells(picker.symbols.iter().cloned()),
        labels: (0..PAGE_SIZE)
            .map(|slot| slot_key_set.label_for_slot(slot))
            .collect(),
        cursor: picker.selection.selected_index().unwrap_or(0) as u32,
        cursor_visible: true,
        page_size: PAGE_SIZE as u32,
        vertical: is_vertical_layout(settings),
    }
}

/// The Telex key table as a lookup table: the title row, then `key  meaning`
/// per row, no labels, no highlight, one vertical page (Linux roadmap L4 —
/// platform-adapted presentation of `keys::telex_guide_rows`).
fn telex_guide_table(runtime: &DesktopRuntime, settings: &SettingsDocument) -> LookupTableContent {
    let strings = runtime.strings();
    let mut rows = vec![strings
        .resolve(StringKey::SettingsToneSchemeTelex)
        .to_owned()];
    rows.extend(
        telex_guide_rows(settings.choice(&keys::INPUT_MODE), &strings)
            .into_iter()
            .map(|row| format!("{}  {}", row.key, row.meaning)),
    );
    let page_size = rows.len() as u32;
    LookupTableContent {
        cells: plain_cells(rows),
        labels: Vec::new(),
        cursor: 0,
        cursor_visible: false,
        page_size,
        vertical: true,
    }
}

/// Rows with one script each: the picker's symbols, the Telex guide.
fn plain_cells(rows: impl IntoIterator<Item = String>) -> Vec<CandidateCellContent> {
    rows.into_iter()
        .map(|row| CandidateCellContent::new(row, None))
        .collect()
}

/// Whether the panel draws the list as a column — what it is told to draw
/// and how the arrows read, from the one setting.
pub(crate) fn is_vertical_layout(settings: &SettingsDocument) -> bool {
    settings.choice(&keys::CANDIDATE_LAYOUT) != CandidateLayout::Horizontal
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::ComposingKeyBindings;
    use crate::platform::test_support::TEST_PLATFORM;
    use crate::session::test_support::runtime;
    use crate::session::LookupSelection;

    fn present_under(state: &mut EngineState, settings: SettingsDocument) -> Vec<Emit> {
        let bindings = ComposingKeyBindings::from_document(&settings, TEST_PLATFORM);
        let runtime = runtime(settings.clone());
        let mut emits = Vec::new();
        present_table(&runtime, state, &settings, &bindings, &mut emits);
        emits
    }

    fn present(state: &mut EngineState) -> Vec<Emit> {
        present_under(state, SettingsDocument::default())
    }

    fn guide_wanted() -> SettingsDocument {
        let mut settings = SettingsDocument::default();
        settings.set_bool(&keys::TELEX_GUIDE_SHOWN, true);
        settings
    }

    #[test]
    fn a_wanted_guide_fills_an_empty_table_and_the_picker_covers_it() {
        // trace: no picker, no candidates, telexGuideShown → the guide (no
        // labels, no highlight); a picker up → the picker instead.
        let mut state = EngineState::default();
        let emits = present_under(&mut state, guide_wanted());
        let [Emit::LookupTable(guide)] = emits.as_slice() else {
            panic!("the guide, got {emits:?}");
        };
        assert!(!guide.cursor_visible && guide.labels.is_empty());
        assert!(state.is_table_shown);

        state.symbol_picker = Some(SymbolPicker {
            symbols: vec!["，".to_owned()],
            selection: LookupSelection::new(1, PAGE_SIZE),
        });
        let emits = present_under(&mut state, guide_wanted());
        assert!(matches!(emits.as_slice(), [Emit::LookupTable(table)] if table.cursor_visible));
    }

    #[test]
    fn an_empty_list_is_taken_down_once() {
        let mut state = EngineState {
            is_table_shown: true,
            ..EngineState::default()
        };
        assert_eq!(present(&mut state), vec![Emit::HideLookupTable]);
        assert!(!state.is_table_shown);
        assert!(present(&mut state).is_empty());
    }

    #[test]
    fn the_picker_wins_over_the_list_and_labels_every_slot() {
        let mut state = EngineState {
            symbol_picker: Some(SymbolPicker {
                symbols: vec!["，".to_owned(), "。".to_owned()],
                selection: LookupSelection::new(2, PAGE_SIZE),
            }),
            ..EngineState::default()
        };
        let emits = present(&mut state);
        let [Emit::LookupTable(table)] = emits.as_slice() else {
            panic!("one lookup table, got {emits:?}");
        };
        assert_eq!(table.joined_candidates(), ["，", "。"]);
        assert_eq!(table.labels.len(), PAGE_SIZE);
        assert!(table.labels.iter().all(|label| !label.is_empty()));
        assert!(state.is_table_shown);
    }
}
