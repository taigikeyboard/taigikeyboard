//! The key contract: what one key event means to a composition, and the part
//! of that contract the user chooses. Pure classification — no Win32.
//!
//! The rules live here once for the three desktops (macOS reaches them
//! through `taigi-macos-ffi`; the Swift side keeps the values it is
//! handed — `ComposingKeyChord`, `SymbolPickerIntent`). Each shell builds a
//! [`KeyEventSnapshot`] from its key event (TSF `OnKeyDown`, IBus / Fcitx5,
//! `taigi-macos-ffi` `key_translation.rs`) and asks
//! [`ComposingKeyIntent::intent`]; nothing in here reads the keyboard.

mod action;
mod bindings;
mod chord;
mod input_method_menu;
mod intent;
mod language_mode;
mod recorder;
mod shift_tap;
mod shortcut_actions;
pub mod shortcut_labels;
mod slot_key_set;
mod snapshot;
mod symbol_picker;
mod telex_guide_rows;
mod tone_input_scheme;
mod tps_keyboard_rows;
mod tps_layout;

pub use action::{ChordField, ComposingAction};
pub use bindings::ComposingKeyBindings;
pub use chord::{ChordRejection, ComposingKeyChord, SEMICOLON_KEY_CODE};
pub use input_method_menu::{menu_rows, MenuCommand, MenuRow, MENU};
pub use intent::{caret_chord_modifiers, CandidateNavigation, CaretDirection, ComposingKeyIntent};
pub use language_mode::LanguageMode;
pub use recorder::{
    evaluate_press, rejection_message_key, RecordedPress, RecorderOutcome, RecorderTarget,
    RecorderTier,
};
pub use shift_tap::{
    ShiftTapTracker, LEFT_SHIFT_SCAN_CODE, RIGHT_SHIFT_SCAN_CODE, SHIFT_TAP_MAX_MILLISECONDS,
    VK_SHIFT_CODE,
};
pub use shortcut_actions::{global_rejection, ShortcutAction, ShortcutConflicts};
pub use slot_key_set::CandidateSlotKeySet;
pub use snapshot::{KeyEventSnapshot, KeyModifiers, LineEdgeKey, NavigationKey};
pub use symbol_picker::SymbolPickerIntent;
pub use telex_guide_rows::{telex_guide_rows, TelexGuideRow};
pub use tone_input_scheme::ToneInputScheme;
pub use tps_keyboard_rows::{
    tps_keyboard_cap_of, tps_keyboard_rows, TpsKeyCap, TpsKeyCapIndex, TpsKeyboardRow,
};
pub use tps_layout::tps_glyph_for_event;
