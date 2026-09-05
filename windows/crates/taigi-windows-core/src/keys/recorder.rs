//! What a key press means to a shortcut-recording field, as a pure decision
//! — the part of `ShortcutKeyRecorder.swift` (`handle(_:)`,
//! `GlobalShortcutPolicy`) that is not AppKit. The settings window feeds it
//! the press and draws the answer.

// 快捷鍵錄製欄的純決策 — 一個按鍵對錄製中的欄位代表什麼(錄下/拒絕/清空/離開)。

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
/// the input method, a candidate slot, the system, or the host app — and to
/// the reader they all mean "not this key", so one message covers them.
pub fn rejection_message_key(rejection: ChordRejection) -> StringKey {
    match rejection {
        ChordRejection::NoKey => StringKey::DesktopShortcutRejectedNoKey,
        ChordRejection::TypesRomanization
        | ChordRejection::ReservedKey
        | ChordRejection::NotAGlobalKey
        | ChordRejection::CandidateSlotChord
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
    fn every_taken_rejection_shares_one_prompt_and_no_key_keeps_its_own() {
        for rejection in [
            ChordRejection::NotAGlobalKey,
            ChordRejection::ReservedKey,
            ChordRejection::BelongsToHost,
            ChordRejection::TypesRomanization,
            ChordRejection::CandidateSlotChord,
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
        // refuse it: `q` (no syllable uses it) rides along to pin that the
        // rule is about the modifiers, not about the key.
        let ctrl_alt = KeyModifiers::CONTROL.with(KeyModifiers::ALT);
        let ctrl_shift = KeyModifiers::CONTROL.with(KeyModifiers::SHIFT);
        for (key, modifiers) in [("a", ctrl_alt), ("a", ctrl_shift), ("q", ctrl_alt)] {
            for tier in [RecorderTier::Composing, RecorderTier::Global] {
                assert_eq!(
                    evaluate_press(tier, CandidateSlotKeySet::BareKeys, &press(key, modifiers)),
                    RecorderOutcome::Recorded(
                        ComposingKeyChord::make(Some(key), modifiers).expect("bindable")
                    ),
                    "{tier:?} {key} {modifiers:?}"
                );
            }
        }
    }
}
