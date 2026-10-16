//! The fixed rows of the Shortcuts pane: keys the user cannot rebind, named
//! in the recorder rows' own spelling so the pane reads as one list. Each
//! desktop draws them in its own spelling ([`ComposingKeyChord::display`]);
//! the macOS pane draws these strings in `ShortcutSettingsView.swift`.

use super::chord::ComposingKeyChord;
use super::intent::caret_chord_modifiers;
use super::slot_key_set::CandidateSlotKeySet;
use super::snapshot::KeyModifiers;
use crate::candidates::HorizontalPageLayout;
use crate::platform::DesktopPlatform;

/// The six keys the fixed navigation tier reads, in the keycap legends:
/// `←  →  ↑  ↓  PgUp  PgDn`, the Mac's
/// `←  →  ↑  ↓  ⇞  ⇟`.
pub fn navigation_keys_label(platform: DesktopPlatform) -> &'static str {
    match platform {
        DesktopPlatform::Windows | DesktopPlatform::Linux => "←  →  ↑  ↓  PgUp  PgDn",
        DesktopPlatform::MacOS => "←  →  ↑  ↓  ⇞  ⇟",
    }
}

/// `Ctrl+←  Ctrl+→  Ctrl+↑  Ctrl+↓  Home  End` (the Mac's
/// `⌥←  ⌥→  ⌥↑  ⌥↓  ↖  ↘`): the chords named by the same modifier labels the
/// recorder rows use, from the modifier the classifier reads, then the bare
/// Home / End keycap legends.
pub fn caret_chords_label(platform: DesktopPlatform) -> String {
    let chords = ["←", "→", "↑", "↓"].map(|arrow| {
        ComposingKeyChord {
            key: arrow.to_owned(),
            modifiers: caret_chord_modifiers(platform),
        }
        .display(platform)
    });
    let edges = match platform {
        DesktopPlatform::Windows | DesktopPlatform::Linux => ["Home", "End"],
        DesktopPlatform::MacOS => ["↖", "↘"],
    };
    chords
        .iter()
        .map(String::as_str)
        .chain(edges)
        .collect::<Vec<_>>()
        .join("  ")
}

/// The slot keys under TPS, named rather than drawn as `123456789`: the
/// number row picks only once the window is up (desktop TPS roadmap D7), and
/// the keypad picks too. A key name like `Ctrl`, so not translated.
pub const TPS_SLOT_KEYS_LABEL: &str = "1–9, Num 1–9";

/// `qwdfzxvy;` under Standard, `123456789` under Telex: every key of the
/// live slot set, bare — lowercase because a bare key shows the character
/// it types. Under TPS, [`TPS_SLOT_KEYS_LABEL`].
pub fn slot_keys_label(slot_keys: CandidateSlotKeySet, platform: DesktopPlatform) -> String {
    if slot_keys == CandidateSlotKeySet::TpsDigits {
        return TPS_SLOT_KEYS_LABEL.to_owned();
    }
    ComposingKeyChord {
        key: slot_keys_run(slot_keys),
        modifiers: KeyModifiers::NONE,
    }
    .display(platform)
}

/// `Shift+QWDFZXVY;` under Standard, `Shift+123456789` under Telex (the
/// Mac's `⇧QWDFZXVY;`): every key of the live slot set behind ONE Shift —
/// the Hanji / romanization commit aimed at a slot. `None` under TPS, where
/// a commit is always the Hanji and the row is not drawn.
pub fn shifted_slot_keys_label(
    slot_keys: CandidateSlotKeySet,
    platform: DesktopPlatform,
) -> Option<String> {
    if slot_keys == CandidateSlotKeySet::TpsDigits {
        return None;
    }
    Some(
        ComposingKeyChord {
            key: slot_keys_run(slot_keys),
            modifiers: KeyModifiers::SHIFT,
        }
        .display(platform),
    )
}

/// The nine slot keys of `slot_keys` as one run, in page order.
fn slot_keys_run(slot_keys: CandidateSlotKeySet) -> String {
    (0..HorizontalPageLayout::PAGE_SIZE)
        .map(|slot| slot_keys.label_for_slot(slot))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::WINDOWS_AND_LINUX;

    #[test]
    fn the_fixed_rows_read_in_the_recorder_rows_spelling() {
        // trace: caret_chord_modifiers = CONTROL on Windows and Linux, whose
        // label is "Ctrl"; `display` joins with "+" (read by running the
        // functions, 2026-09-30).
        for platform in WINDOWS_AND_LINUX {
            assert_eq!(
                caret_chords_label(platform),
                "Ctrl+←  Ctrl+→  Ctrl+↑  Ctrl+↓  Home  End"
            );
            assert_eq!(navigation_keys_label(platform), "←  →  ↑  ↓  PgUp  PgDn");
        }
    }

    #[test]
    fn the_mac_fixed_rows_read_in_glyphs() {
        // trace: caret ⌥ (`caret_chord_modifiers`), the Mac's ⇞ ⇟ legends,
        // no separator.
        let mac = DesktopPlatform::MacOS;
        assert_eq!(caret_chords_label(mac), "⌥←  ⌥→  ⌥↑  ⌥↓  ↖  ↘");
        assert_eq!(navigation_keys_label(mac), "←  →  ↑  ↓  ⇞  ⇟");
    }

    #[test]
    fn the_slot_rows_follow_the_live_slot_set() {
        // trace: BARE_KEY_ROW q w d f z x v y ; under Standard, digits under
        // Telex; Shift upper-cases the letters it is shown with.
        for platform in WINDOWS_AND_LINUX {
            assert_eq!(
                slot_keys_label(CandidateSlotKeySet::BareKeys, platform),
                "qwdfzxvy;"
            );
            assert_eq!(
                shifted_slot_keys_label(CandidateSlotKeySet::BareKeys, platform).as_deref(),
                Some("Shift+QWDFZXVY;")
            );
            assert_eq!(
                slot_keys_label(CandidateSlotKeySet::Digits, platform),
                "123456789"
            );
            assert_eq!(
                shifted_slot_keys_label(CandidateSlotKeySet::Digits, platform).as_deref(),
                Some("Shift+123456789")
            );
        }
        // trace: the Mac's `display` — one ⇧ ahead of the run, no separator.
        let mac = DesktopPlatform::MacOS;
        assert_eq!(
            slot_keys_label(CandidateSlotKeySet::BareKeys, mac),
            "qwdfzxvy;"
        );
        assert_eq!(
            shifted_slot_keys_label(CandidateSlotKeySet::BareKeys, mac).as_deref(),
            Some("⇧QWDFZXVY;")
        );
        assert_eq!(
            slot_keys_label(CandidateSlotKeySet::Digits, mac),
            "123456789"
        );
        assert_eq!(
            shifted_slot_keys_label(CandidateSlotKeySet::Digits, mac).as_deref(),
            Some("⇧123456789")
        );
    }

    #[test]
    fn under_tps_the_digits_are_named_and_the_shift_row_is_gone() {
        for platform in WINDOWS_AND_LINUX {
            assert_eq!(
                slot_keys_label(CandidateSlotKeySet::TpsDigits, platform),
                "1–9, Num 1–9"
            );
            assert_eq!(
                shifted_slot_keys_label(CandidateSlotKeySet::TpsDigits, platform),
                None
            );
        }
    }
}
