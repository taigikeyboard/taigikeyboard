//! What a key press means to a shortcut-recording field, as a pure decision
//! — the part of `ShortcutKeyRecorder.swift` (`handle(_:)`,
//! `GlobalShortcutPolicy`) that is not AppKit. The settings window feeds it
//! the press and draws the answer.

use super::chord::{ChordRejection, ComposingKeyChord};
use super::shortcut_actions::global_rejection;
use super::snapshot::KeyModifiers;
use crate::strings::StringKey;

/// Which registry the row writes to — what it refuses on top of the shared
/// gate differs (`ShortcutSettingsView.swift:429-436`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecorderTier {
    /// A composing action: only the shared gate.
    Composing,
    /// A global chord: refuses what the system or the host owns.
    Global,
}

/// One key press as the recorder sees it: the character the key types
/// with no modifier held (`charactersIgnoringModifiers`), the chording
/// modifiers, the virtual key (so a shifted number-row key can be refused as
/// the digit it is — `ComposingKeyChord::make_from_press`), and whether it
/// is a held-key repeat.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordedPress {
    pub key: Option<String>,
    pub modifiers: KeyModifiers,
    pub key_code: Option<u16>,
    pub is_repeat: bool,
}

/// What the field does with the press.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecorderOutcome {
    /// Bound; recording ends.
    Recorded(ComposingKeyChord),
    /// Turned down, with the reason to show in place of the prompt;
    /// recording continues.
    Refused(ChordRejection),
    /// Escape: recording ends, the row keeps what it had.
    Blurred,
    /// Tab: recording ends AND the key goes on to walk the form.
    PassThrough,
    /// Swallowed with no effect (a repeat, a bare Backspace / Delete).
    Ignored,
}

/// The recorder's decision for `press` on a row of `tier`
/// (`ShortcutKeyRecorder.swift:403-468`). The slot keys need no refusal of
/// their own: the shared gate refuses every bare letter, digit and `;`
/// whichever tone scheme is live (`ComposingKeyChord::make`).
pub fn evaluate_press(tier: RecorderTier, press: &RecordedPress) -> RecorderOutcome {
    // Key repeat is dropped: holding a key would otherwise record it over
    // and over, each time re-running conflict resolution.
    if press.is_repeat {
        return RecorderOutcome::Ignored;
    }
    if press.modifiers.is_empty() {
        match press.key.as_deref() {
            // Tab still walks the form; the cost is that Tab cannot be
            // recorded here — the trade every shortcut field makes.
            Some("\t") => return RecorderOutcome::PassThrough,
            // A blanked field, nothing recorded (the row's own × clears).
            Some("\u{8}") | Some("\u{7F}") => return RecorderOutcome::Ignored,
            // The way out.
            Some("\u{1B}") => return RecorderOutcome::Blurred,
            _ => {}
        }
    }
    let chord = match ComposingKeyChord::make_from_press(
        press.key.as_deref(),
        press.modifiers,
        press.key_code,
    ) {
        Ok(chord) => chord,
        Err(reason) => return RecorderOutcome::Refused(reason),
    };
    // Ctrl+Alt is recordable on BOTH tiers, as ⌃⌘ is on the Mac (which
    // refuses it on neither). It is what Windows reports AltGr as, but a
    // binding of ours only answers while this Taiwanese TIP is the selected
    // profile — a layout whose AltGr types a glyph is a different profile —
    // and the composing tier's bindings live exactly as long as the global
    // tier's preserved keys do. Refusing the whole family on one tier while
    // the other ships three defaults on it (`ShortcutAction::default_chord`)
    // was a rule with no line to draw (USER 2026-09-04, real device: 打開設定
    // 選單 could not take Ctrl+Alt+A).
    if tier == RecorderTier::Global {
        if let Some(reason) = global_rejection(&chord) {
            return RecorderOutcome::Refused(reason);
        }
    }
    RecorderOutcome::Recorded(chord)
}

/// The prompt a refusal replaces (`ShortcutKeyRecorder.swift:366-377`). Every
/// refusal but `NoKey` means the chord already belongs to something — typing,
/// the input method, the system, or the host app — and to the reader they
/// all mean "not this key", so one message covers them.
pub fn rejection_message_key(rejection: ChordRejection) -> StringKey {
    match rejection {
        ChordRejection::NoKey => StringKey::DesktopShortcutRejectedNoKey,
        ChordRejection::TypesRomanization
        | ChordRejection::ReservedKey
        | ChordRejection::NotAGlobalKey
        | ChordRejection::TakenBySystem
        | ChordRejection::BelongsToHost => StringKey::DesktopShortcutRejectedTaken,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(key: &str, modifiers: KeyModifiers) -> RecordedPress {
        RecordedPress {
            key: Some(key.to_owned()),
            modifiers,
            key_code: None,
            is_repeat: false,
        }
    }

    #[test]
    fn a_bare_punctuation_key_records_on_both_tiers_and_a_typing_key_is_refused() {
        // trace: `[` is not a typing key → make Ok; global gate: nameable, no
        // modifiers → None. `a` (syllable), `z` (Telex / slot) and `3`
        // (tone / slot) are refused bare under either scheme.
        for tier in [RecorderTier::Composing, RecorderTier::Global] {
            let outcome = evaluate_press(tier, &press("[", KeyModifiers::NONE));
            assert!(matches!(outcome, RecorderOutcome::Recorded(chord) if chord.key == "["));
            for key in ["a", "z", "q", "3", ";"] {
                assert_eq!(
                    evaluate_press(tier, &press(key, KeyModifiers::NONE)),
                    RecorderOutcome::Refused(ChordRejection::TypesRomanization),
                    "{tier:?} {key}"
                );
            }
        }
    }

    #[test]
    fn the_global_gate_refuses_on_top_of_the_shared_gate() {
        // trace: Ctrl+S alone belongs to the host on the global tier only.
        assert!(matches!(
            evaluate_press(RecorderTier::Composing, &press("s", KeyModifiers::CONTROL)),
            RecorderOutcome::Recorded(_)
        ));
        assert_eq!(
            evaluate_press(RecorderTier::Global, &press("s", KeyModifiers::CONTROL)),
            RecorderOutcome::Refused(ChordRejection::BelongsToHost)
        );
    }

    #[test]
    fn escape_tab_delete_and_repeats_end_or_swallow_without_recording() {
        assert_eq!(
            evaluate_press(
                RecorderTier::Composing,
                &press("\u{1B}", KeyModifiers::NONE)
            ),
            RecorderOutcome::Blurred
        );
        assert_eq!(
            evaluate_press(RecorderTier::Composing, &press("\t", KeyModifiers::NONE)),
            RecorderOutcome::PassThrough
        );
        assert_eq!(
            evaluate_press(RecorderTier::Composing, &press("\u{8}", KeyModifiers::NONE)),
            RecorderOutcome::Ignored
        );
        // Shift+Tab is a chord, recordable (the previous-candidate default).
        assert!(matches!(
            evaluate_press(RecorderTier::Composing, &press("\t", KeyModifiers::SHIFT)),
            RecorderOutcome::Recorded(_)
        ));
        let repeat = RecordedPress {
            is_repeat: true,
            ..press("[", KeyModifiers::NONE)
        };
        assert_eq!(
            evaluate_press(RecorderTier::Composing, &repeat),
            RecorderOutcome::Ignored
        );
        let none = RecordedPress {
            key: None,
            modifiers: KeyModifiers::CONTROL,
            key_code: None,
            is_repeat: false,
        };
        assert_eq!(
            evaluate_press(RecorderTier::Composing, &none),
            RecorderOutcome::Refused(ChordRejection::NoKey)
        );
    }

    #[test]
    fn every_taken_rejection_shares_one_prompt_and_no_key_keeps_its_own() {
        for rejection in [
            ChordRejection::NotAGlobalKey,
            ChordRejection::ReservedKey,
            ChordRejection::BelongsToHost,
            ChordRejection::TypesRomanization,
            ChordRejection::TakenBySystem,
        ] {
            assert_eq!(
                rejection_message_key(rejection),
                StringKey::DesktopShortcutRejectedTaken
            );
        }
        assert_eq!(
            rejection_message_key(ChordRejection::NoKey),
            StringKey::DesktopShortcutRejectedNoKey
        );
    }

    #[test]
    fn a_chord_records_on_both_tiers_once_a_host_modifier_is_held() {
        // The shape the USER hit on the real device (2026-09-04): `a` is a
        // syllable letter, so the shared gate refuses it bare — but Ctrl+Alt
        // and Ctrl+Shift make it a chord, on EITHER tier. It only ever read as
        // "this key types" because the modifiers arrived empty
        // (`os_out_buffer`). Ctrl+Alt is the family the Mac's ⌃⌘ roster maps
        // onto and the one the shipped globals are on, so neither tier may
        // refuse it: `q` (a Telex / slot key, refused bare) rides along to pin
        // that the rule is about the modifiers, not about the key.
        let ctrl_alt = KeyModifiers::CONTROL.with(KeyModifiers::ALT);
        let ctrl_shift = KeyModifiers::CONTROL.with(KeyModifiers::SHIFT);
        for (key, modifiers) in [("a", ctrl_alt), ("a", ctrl_shift), ("q", ctrl_alt)] {
            for tier in [RecorderTier::Composing, RecorderTier::Global] {
                assert_eq!(
                    evaluate_press(tier, &press(key, modifiers)),
                    RecorderOutcome::Recorded(
                        ComposingKeyChord::make(Some(key), modifiers).expect("bindable")
                    ),
                    "{tier:?} {key} {modifiers:?}"
                );
            }
        }
    }

    /// A US layout hands the recorder `#` for Shift+3; the virtual key is
    /// what still says it was the `3` key, and the recorder refuses it as
    /// the typing key it is — the same answer the settings-file path gives
    /// the unmodified `3` with Shift, so no row can hold the press on one
    /// path and lose it on the other.
    #[test]
    fn a_shifted_number_row_key_is_refused_as_the_digit_it_is() {
        let shifted_three = RecordedPress {
            key: Some("#".to_owned()),
            modifiers: KeyModifiers::SHIFT,
            key_code: Some(0x33),
            is_repeat: false,
        };
        for tier in [RecorderTier::Composing, RecorderTier::Global] {
            assert_eq!(
                evaluate_press(tier, &shifted_three),
                RecorderOutcome::Refused(ChordRejection::TypesRomanization),
                "{tier:?}"
            );
        }
        // A `#` reached without the number row is still the character it types.
        let bare_hash = RecordedPress {
            key_code: None,
            ..shifted_three
        };
        assert!(matches!(
            evaluate_press(RecorderTier::Composing, &bare_hash),
            RecorderOutcome::Recorded(_)
        ));
    }
}
