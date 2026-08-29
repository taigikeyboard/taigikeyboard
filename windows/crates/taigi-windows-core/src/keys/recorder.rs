//! What a key press means to a shortcut-recording field, as a pure decision
//! — the part of `ShortcutKeyRecorder.swift` (`handle(_:)`,
//! `GlobalShortcutPolicy`) that is not AppKit. The settings window feeds it
//! the press and draws the answer.

// 中文: 快捷鍵錄製欄的純決策 — 一個按鍵對錄製中的欄位代表什麼(錄下/拒絕/清空/離開)。

use super::chord::{ChordRejection, ComposingKeyChord};
use super::shortcut_actions::global_rejection;
use super::slot_key_set::CandidateSlotKeySet;
use super::snapshot::KeyModifiers;
use crate::strings::StringKey;

/// Which registry the row writes to — what it refuses on top of the shared
/// gate differs (`ShortcutSettingsView.swift:429-436`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecorderTier {
    /// A composing action: refuses the chosen slot set's keys.
    Composing,
    /// A global chord: refuses what the system or the host owns.
    Global,
}

/// One key press as the recorder sees it: the character the key types
/// with no modifier held (`charactersIgnoringModifiers`), the chording
/// modifiers, and whether it is a held-key repeat.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordedPress {
    pub key: Option<String>,
    pub modifiers: KeyModifiers,
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
/// (`ShortcutKeyRecorder.swift:403-468`).
pub fn evaluate_press(
    tier: RecorderTier,
    slot_key_set: CandidateSlotKeySet,
    press: &RecordedPress,
) -> RecorderOutcome {
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
    let chord = match ComposingKeyChord::make(press.key.as_deref(), press.modifiers) {
        Ok(chord) => chord,
        Err(reason) => return RecorderOutcome::Refused(reason),
    };
    if chord.is_candidate_slot_chord(slot_key_set) {
        return RecorderOutcome::Refused(ChordRejection::CandidateSlotChord);
    }
    // Ctrl+Alt IS AltGr on most non-US layouts, and Windows reports AltGr
    // as exactly that: a composing binding on it would take the glyph that
    // layout types with it. Refused on BOTH tiers (the global gate refuses
    // it on its own; PR7 Codex review).
    if chord.modifiers.control && chord.modifiers.alt {
        return RecorderOutcome::Refused(ChordRejection::TakenBySystem);
    }
    if tier == RecorderTier::Global {
        if let Some(reason) = global_rejection(&chord) {
            return RecorderOutcome::Refused(reason);
        }
    }
    RecorderOutcome::Recorded(chord)
}

/// The prompt a refusal replaces (`ShortcutKeyRecorder.swift:366-381`). A
/// press the global registry cannot name reads as a reserved key: to the
/// reader both mean "not this key".
pub fn rejection_message_key(rejection: ChordRejection) -> StringKey {
    match rejection {
        ChordRejection::TypesRomanization => StringKey::DesktopShortcutRejectedTypingKey,
        ChordRejection::ReservedKey | ChordRejection::NotAGlobalKey => {
            StringKey::DesktopShortcutRejectedReservedKey
        }
        ChordRejection::NoKey => StringKey::DesktopShortcutRejectedNoKey,
        ChordRejection::CandidateSlotChord => StringKey::DesktopShortcutRejectedSlotChord,
        ChordRejection::TakenBySystem => StringKey::DesktopShortcutRejectedSystemShortcut,
        ChordRejection::BelongsToHost => StringKey::DesktopShortcutRejectedHostShortcut,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(key: &str, modifiers: KeyModifiers) -> RecordedPress {
        RecordedPress {
            key: Some(key.to_owned()),
            modifiers,
            is_repeat: false,
        }
    }

    #[test]
    fn a_free_bare_letter_records_on_both_tiers_and_a_syllable_letter_is_refused() {
        // trace: `z` is not in SYLLABLE_LETTERS → make Ok; no slot; global
        // gate: nameable, no modifiers → None.
        for tier in [RecorderTier::Composing, RecorderTier::Global] {
            let outcome = evaluate_press(
                tier,
                CandidateSlotKeySet::Shift,
                &press("z", KeyModifiers::NONE),
            );
            assert!(matches!(outcome, RecorderOutcome::Recorded(chord) if chord.key == "z"));
            assert_eq!(
                evaluate_press(
                    tier,
                    CandidateSlotKeySet::Shift,
                    &press("a", KeyModifiers::NONE)
                ),
                RecorderOutcome::Refused(ChordRejection::TypesRomanization)
            );
        }
    }

    #[test]
    fn the_slot_set_and_the_global_gate_refuse_on_top_of_the_shared_gate() {
        // trace: bare `q` is slot 0 of the BareKeys set → CandidateSlotChord;
        // under the Shift set `q` is free. Ctrl+S alone belongs to the host on
        // the global tier only.
        assert_eq!(
            evaluate_press(
                RecorderTier::Composing,
                CandidateSlotKeySet::BareKeys,
                &press("q", KeyModifiers::NONE)
            ),
            RecorderOutcome::Refused(ChordRejection::CandidateSlotChord)
        );
        assert!(matches!(
            evaluate_press(
                RecorderTier::Composing,
                CandidateSlotKeySet::Shift,
                &press("q", KeyModifiers::NONE)
            ),
            RecorderOutcome::Recorded(_)
        ));
        assert!(matches!(
            evaluate_press(
                RecorderTier::Composing,
                CandidateSlotKeySet::BareKeys,
                &press("s", KeyModifiers::CONTROL)
            ),
            RecorderOutcome::Recorded(_)
        ));
        assert_eq!(
            evaluate_press(
                RecorderTier::Global,
                CandidateSlotKeySet::BareKeys,
                &press("s", KeyModifiers::CONTROL)
            ),
            RecorderOutcome::Refused(ChordRejection::BelongsToHost)
        );
    }

    #[test]
    fn escape_tab_delete_and_repeats_end_or_swallow_without_recording() {
        let set = CandidateSlotKeySet::BareKeys;
        assert_eq!(
            evaluate_press(
                RecorderTier::Composing,
                set,
                &press("\u{1B}", KeyModifiers::NONE)
            ),
            RecorderOutcome::Blurred
        );
        assert_eq!(
            evaluate_press(
                RecorderTier::Composing,
                set,
                &press("\t", KeyModifiers::NONE)
            ),
            RecorderOutcome::PassThrough
        );
        assert_eq!(
            evaluate_press(
                RecorderTier::Composing,
                set,
                &press("\u{8}", KeyModifiers::NONE)
            ),
            RecorderOutcome::Ignored
        );
        // Shift+Tab is a chord, recordable (the previous-candidate default).
        assert!(matches!(
            evaluate_press(
                RecorderTier::Composing,
                set,
                &press("\t", KeyModifiers::SHIFT)
            ),
            RecorderOutcome::Recorded(_)
        ));
        let repeat = RecordedPress {
            is_repeat: true,
            ..press("z", KeyModifiers::NONE)
        };
        assert_eq!(
            evaluate_press(RecorderTier::Composing, set, &repeat),
            RecorderOutcome::Ignored
        );
        let none = RecordedPress {
            key: None,
            modifiers: KeyModifiers::CONTROL,
            is_repeat: false,
        };
        assert_eq!(
            evaluate_press(RecorderTier::Composing, set, &none),
            RecorderOutcome::Refused(ChordRejection::NoKey)
        );
    }

    #[test]
    fn every_rejection_has_a_prompt_and_the_unnameable_key_reads_as_reserved() {
        assert_eq!(
            rejection_message_key(ChordRejection::NotAGlobalKey),
            StringKey::DesktopShortcutRejectedReservedKey
        );
        assert_eq!(
            rejection_message_key(ChordRejection::ReservedKey),
            StringKey::DesktopShortcutRejectedReservedKey
        );
        assert_eq!(
            rejection_message_key(ChordRejection::BelongsToHost),
            StringKey::DesktopShortcutRejectedHostShortcut
        );
    }

    #[test]
    fn an_alt_gr_shaped_chord_is_refused_on_the_composing_tier_too() {
        // trace: Ctrl+Alt+Q → make Ok (host chord) → not a slot → AltGr gate.
        let alt_gr = KeyModifiers::CONTROL.with(KeyModifiers::ALT);
        assert_eq!(
            evaluate_press(
                RecorderTier::Composing,
                CandidateSlotKeySet::BareKeys,
                &press("q", alt_gr)
            ),
            RecorderOutcome::Refused(ChordRejection::TakenBySystem)
        );
    }
}
