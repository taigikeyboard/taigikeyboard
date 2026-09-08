//! The global shortcuts as TSF preserved keys (`ITfKeystrokeMgr::PreserveKey`,
//! roadmap W5): registered from the stored chords at activation and
//! re-registered when the settings revision moves. A preserved key is
//! delivered through `OnPreservedKey` before the key sink sees the key, in
//! every host — the Carbon hotkey's role on the Mac.
//!
//! The bare-backtick 漢羅對調 chord is NOT a preserved key: a bare key would
//! be taken from every application for as long as this input method is
//! selected. It is matched in the key sink instead.

use crate::guids::{
    GUID_PRESERVED_KEY_CYCLE_CANDIDATE_DISPLAY_MODE, GUID_PRESERVED_KEY_ROMANIZATION,
    GUID_PRESERVED_KEY_SETTINGS, GUID_PRESERVED_KEY_TELEX_GUIDE,
};
use crate::wide::to_wide_nul;
use taigi_windows_core::keys::{ComposingKeyChord, ShortcutAction};
use taigi_windows_core::settings::SettingsDocument;
use windows::core::GUID;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyboardLayout, VkKeyScanExW, MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN,
};
use windows::Win32::UI::TextServices::{ITfKeystrokeMgr, TF_PRESERVEDKEY};

/// The actions that ARE preserved keys, with their GUIDs.
const PRESERVED: [(ShortcutAction, GUID); 4] = [
    (
        ShortcutAction::OpenLastSettingsPane,
        GUID_PRESERVED_KEY_SETTINGS,
    ),
    (
        ShortcutAction::ToggleRomanization,
        GUID_PRESERVED_KEY_ROMANIZATION,
    ),
    (
        ShortcutAction::CycleCandidateDisplayMode,
        GUID_PRESERVED_KEY_CYCLE_CANDIDATE_DISPLAY_MODE,
    ),
    (
        ShortcutAction::ShowTelexGuide,
        GUID_PRESERVED_KEY_TELEX_GUIDE,
    ),
];

/// The action a preserved-key GUID names.
pub fn action_for_guid(guid: &GUID) -> Option<ShortcutAction> {
    PRESERVED
        .iter()
        .find(|(_, known)| known == guid)
        .map(|(action, _)| *action)
}

/// A chord as TSF wants it: the virtual key the character sits on in the
/// current layout plus the modifier bits. `None` when the layout has no
/// key for the character (the recorder's `NotAGlobalKey`, hit late).
///
/// ⚠ The layout's own required modifiers are OR-ed into the chord's, and a
/// bitmask cannot say "twice": on a layout where the character itself needs
/// AltGr, `Ctrl+Alt+<char>` registers the key that layout types the
/// character WITH, not that key plus Ctrl+Alt. Neither `s` nor `c` needs a
/// modifier on the layouts this input method is typed on, so the shipped
/// defaults are unaffected — a dogfood item per layout, not a live bug.
pub fn preserved_key(chord: &ComposingKeyChord) -> Option<TF_PRESERVEDKEY> {
    let character = chord.key.encode_utf16().next()?;
    // SAFETY: plain layout queries on the calling thread.
    let scan = unsafe { VkKeyScanExW(character, GetKeyboardLayout(0)) };
    if scan == -1 {
        return None;
    }
    let virtual_key = u32::from((scan & 0xFF) as u8);
    // The high byte says which modifiers the LAYOUT needs to type this
    // character (bit 1 Shift, bit 2 Ctrl, bit 4 Alt) — a stored chord names
    // the character as typed with no modifiers, so those are added to the
    // chord's own; on a US layout they are zero for every recordable key.
    let required = ((scan >> 8) & 0xFF) as u8;
    let mut modifiers = 0;
    if required & 0x1 != 0 {
        modifiers |= MOD_SHIFT.0;
    }
    if required & 0x2 != 0 {
        modifiers |= MOD_CONTROL.0;
    }
    if required & 0x4 != 0 {
        modifiers |= MOD_ALT.0;
    }
    if chord.modifiers.shift {
        modifiers |= MOD_SHIFT.0;
    }
    if chord.modifiers.control {
        modifiers |= MOD_CONTROL.0;
    }
    if chord.modifiers.alt {
        modifiers |= MOD_ALT.0;
    }
    if chord.modifiers.win {
        modifiers |= MOD_WIN.0;
    }
    Some(TF_PRESERVEDKEY {
        uVKey: virtual_key,
        uModifiers: modifiers,
    })
}

/// What is registered right now, so it can be unregistered exactly.
#[derive(Default)]
pub struct PreservedKeys {
    registered: Vec<(GUID, TF_PRESERVEDKEY)>,
    /// The settings revision the registration was read from.
    pub revision: Option<u64>,
}

impl PreservedKeys {
    /// Re-registers from `settings` when its revision moved (or on first
    /// call). Unregisters everything first so a cleared row really frees
    /// its key.
    pub fn sync(
        &mut self,
        keystroke_mgr: &ITfKeystrokeMgr,
        client_id: u32,
        settings: &SettingsDocument,
    ) {
        if self.revision == Some(settings.revision) {
            return;
        }
        self.unregister(keystroke_mgr);
        for (action, guid) in PRESERVED {
            let Some(chord) = action.chord_in(settings) else {
                continue;
            };
            let Some(key) = preserved_key(&chord) else {
                log::warn!(
                    "preserved_key.no_virtual_key action={action:?} chord={}",
                    chord.display()
                );
                continue;
            };
            let description = to_wide_nul(action.raw());
            // SAFETY: the keystroke manager is the thread manager's; every
            // pointer is a live local.
            match unsafe { keystroke_mgr.PreserveKey(client_id, &guid, &key, &description) } {
                Ok(()) => self.registered.push((guid, key)),
                Err(error) => {
                    log::warn!("preserved_key.register_failed action={action:?} error={error}")
                }
            }
        }
        self.revision = Some(settings.revision);
    }

    /// The virtual key `action` is registered on right now, if it is.
    pub fn virtual_key_of(&self, action: ShortcutAction) -> Option<u32> {
        let guid = PRESERVED
            .iter()
            .find(|(known, _)| *known == action)
            .map(|(_, guid)| *guid)?;
        self.registered
            .iter()
            .find(|(registered, _)| *registered == guid)
            .map(|(_, key)| key.uVKey)
    }

    pub fn unregister(&mut self, keystroke_mgr: &ITfKeystrokeMgr) {
        for (guid, key) in self.registered.drain(..) {
            // SAFETY: undoing a registration this struct recorded.
            unsafe { keystroke_mgr.UnpreserveKey(&guid, &key).ok() };
        }
        self.revision = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_preserved_key_has_its_own_guid_and_names_its_action() {
        // Two entries on one GUID would make `OnPreservedKey` fire the wrong
        // action for one of them; the roster is every global action but the
        // bare backtick (key-sink only).
        for (index, (action, guid)) in PRESERVED.iter().enumerate() {
            assert_eq!(action_for_guid(guid), Some(*action));
            for (other_action, other) in &PRESERVED[index + 1..] {
                assert_ne!(guid, other, "{action:?} and {other_action:?} share a GUID");
            }
        }
        let preserved: Vec<_> = PRESERVED.iter().map(|(action, _)| *action).collect();
        let expected: Vec<_> = ShortcutAction::ALL
            .into_iter()
            .filter(|action| *action != ShortcutAction::ToggleTranslateSwapped)
            .collect();
        assert_eq!(preserved, expected);
    }
}
