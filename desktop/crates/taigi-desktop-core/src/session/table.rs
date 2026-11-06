//! The lookup table as the shell draws it: the candidate list or the symbol
//! picker, with labels, highlight and orientation.

use super::{Emit, EngineState, LookupTableContent, SymbolPicker, PAGE_SIZE};
use crate::keys::{CandidateSlotKeySet, ComposingKeyBindings};
use crate::settings::{keys, CandidateLayout, SettingsDocument};

/// The lookup table as things now stand — the symbol picker while it is
/// up, else the candidate list: shown, updated in place, or taken down.
/// Appended after the composing signals so the preedit the list describes
/// is already on screen. The Telex guide is emitted by its toggle alone.
pub fn present_table(
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
        if state.is_table_shown {
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

/// The cells as the panel draws them: the leading script, the other script
/// (Combined) after a space; labels = the slot keys, one per position of the
/// CURRENT page. The §34 literal takes no key, as on macOS and Windows: on
/// the page it begins, its label is blank and the keys start at the next
/// cell. The panel can show that because both pages its table through the
/// core, and the table — labels included — is sent again with every reply.
fn table_content(
    state: &EngineState,
    settings: &SettingsDocument,
    slot_key_set: CandidateSlotKeySet,
) -> LookupTableContent {
    let candidates = state
        .candidates
        .cells()
        .into_iter()
        .map(|cell| match cell.annotation {
            Some(annotation) => format!("{} {annotation}", cell.text),
            None => cell.text,
        })
        .collect();
    let shift = state.selection.key_slot_shift();
    let labels = (0..PAGE_SIZE)
        .map(|position| match position.checked_sub(shift) {
            Some(slot) => slot_key_set.label_for_slot(slot),
            None => String::new(),
        })
        .collect();
    LookupTableContent {
        candidates,
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
        candidates: picker.symbols.clone(),
        labels: (0..PAGE_SIZE)
            .map(|slot| slot_key_set.label_for_slot(slot))
            .collect(),
        cursor: picker.selection.selected_index().unwrap_or(0) as u32,
        cursor_visible: true,
        page_size: PAGE_SIZE as u32,
        vertical: is_vertical_layout(settings),
    }
}

/// Whether the panel draws the list as a column — what it is told to draw
/// and how the arrows read, from the one setting.
pub fn is_vertical_layout(settings: &SettingsDocument) -> bool {
    settings.choice(&keys::CANDIDATE_LAYOUT) != CandidateLayout::Horizontal
}
