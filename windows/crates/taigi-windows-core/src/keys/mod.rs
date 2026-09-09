//! The key contract: what one key event means to a composition, and the part
//! of that contract the user chooses. Pure classification — no Win32.
//!
//! Port of `macos/Sources/TaigiInputMethodCore/Controller/{ComposingKeyIntent,
//! ComposingAction, ComposingKeyChord, ComposingKeyBindings,
//! ToneInputScheme}.swift`. The TSF
//! shell builds a [`KeyEventSnapshot`] from `OnKeyDown` and asks
//! [`ComposingKeyIntent::intent`]; nothing in here reads the keyboard.

mod action;
mod bindings;
mod chord;
mod intent;
mod language_mode;
mod recorder;
mod shift_tap;
mod shortcut_actions;
mod slot_key_set;
mod snapshot;
mod symbol_picker;
mod telex_guide_rows;
mod tone_input_scheme;

pub use action::ComposingAction;
pub use bindings::ComposingKeyBindings;
pub use chord::{ChordRejection, ComposingKeyChord};
pub use intent::{CandidateNavigation, ComposingKeyIntent};
pub use language_mode::LanguageMode;
pub use recorder::{
    evaluate_press, rejection_message_key, RecordedPress, RecorderOutcome, RecorderTier,
};
pub use shift_tap::{
    ShiftTapTracker, LEFT_SHIFT_SCAN_CODE, RIGHT_SHIFT_SCAN_CODE, SHIFT_TAP_MAX_MILLISECONDS,
    VK_SHIFT_CODE,
};
pub use shortcut_actions::{global_rejection, ShortcutAction, ShortcutConflicts};
pub use slot_key_set::CandidateSlotKeySet;
pub use snapshot::{KeyEventSnapshot, KeyModifiers, NavigationKey};
pub use symbol_picker::SymbolPickerIntent;
pub use telex_guide_rows::{telex_guide_rows, TelexGuideRow};
pub use tone_input_scheme::ToneInputScheme;
