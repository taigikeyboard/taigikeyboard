//! The key contract: what one key event means to a composition, and the part
//! of that contract the user chooses. Pure classification — no Win32.
//!
//! Port of `macos/Sources/TaigiInputMethodCore/Controller/{ComposingKeyIntent,
//! ComposingAction, ComposingKeyChord, ComposingKeyBindings}.swift`. The TSF
//! shell builds a [`KeyEventSnapshot`] from `OnKeyDown` and asks
//! [`ComposingKeyIntent::intent`]; nothing in here reads the keyboard.

// 中文: 鍵盤契約 — 每個按鍵對組字的意義,以及使用者可自訂的那部分。純分類,無 Win32。

mod action;
mod bindings;
mod chord;
mod intent;
mod slot_key_set;
mod snapshot;

pub use action::ComposingAction;
pub use bindings::ComposingKeyBindings;
pub use chord::{ChordRejection, ComposingKeyChord};
pub use intent::{CandidateNavigation, ComposingKeyIntent};
pub use slot_key_set::CandidateSlotKeySet;
pub use snapshot::{KeyEventSnapshot, KeyModifiers, NavigationKey};
