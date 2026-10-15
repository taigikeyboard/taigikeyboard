//! What a key press means to a shortcut-recording field, as a pure decision
//! — the part of a recording field that is not UI. The settings window feeds
//! it the press and draws the answer. The macOS field
//! (`ShortcutKeyRecorder.swift`) asks it through `KeyRules.press` for both
//! tiers, then adds the global tier's own refusals in Swift
//! (`GlobalShortcutPolicy`).

use super::action::{ChordField, ComposingAction};
use super::chord::{ChordRejection, ComposingKeyChord};
use super::shortcut_actions::{global_rejection, ShortcutAction, ShortcutConflicts};
use super::snapshot::KeyModifiers;
use crate::platform::DesktopPlatform;
use crate::settings::SettingsDocument;
use crate::strings::{StringKey, StringResolver};

/// Which registry the row writes to — what it refuses on top of the shared
/// gate differs. macOS keeps a Swift twin: `ShortcutSettingsView.swift`
/// `globalRecorderRow`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecorderTier {
    /// A composing action: only the shared gate.
    Composing,
    /// A global chord: refuses what the system or the host owns.
    Global,
}

/// Which field is recording, and so which registry it writes to. A
/// composing row has two fields ([`ChordField`]); a global row has one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecorderTarget {
    Global(ShortcutAction),
    Composing(ComposingAction, ChordField),
}

impl RecorderTarget {
    pub fn label_key(self) -> StringKey {
        match self {
            Self::Global(action) => action.label_key(),
            Self::Composing(action, _) => action.label_key(),
        }
    }

    /// What a screen reader calls this field: the row's label alone on a
    /// one-field global row, the label and the field's number on a composing
    /// row, where two fields share it (`ShortcutSettingsView.swift`
    /// `recorderRow` is the macOS twin).
    pub fn field_name(self, strings: &StringResolver) -> String {
        let label = strings.resolve(self.label_key());
        match self {
            Self::Global(_) => label.to_owned(),
            Self::Composing(_, ChordField::First) => format!("{label} 1"),
            Self::Composing(_, ChordField::Second) => format!("{label} 2"),
        }
    }

    /// The field button's accessible name: its name and what it shows
    /// (`value`), which the name would otherwise hide.
    pub fn field_accessible_name(self, strings: &StringResolver, value: &str) -> String {
        format!("{}: {value}", self.field_name(strings))
    }

    /// The accessible name of the × beside the field.
    pub fn clear_accessible_name(self, strings: &StringResolver) -> String {
        format!(
            "{} {}",
            strings.resolve(StringKey::CommonDelete),
            self.field_name(strings)
        )
    }

    pub fn tier(self) -> RecorderTier {
        match self {
            Self::Global(_) => RecorderTier::Global,
            Self::Composing(..) => RecorderTier::Composing,
        }
    }

    /// Stores `chord` on this row, emptying whatever else held it — last
    /// writer wins across BOTH registries (`ShortcutConflicts`).
    pub fn store(
        self,
        document: &mut SettingsDocument,
        chord: Option<&ComposingKeyChord>,
        platform: DesktopPlatform,
    ) {
        match self {
            Self::Global(action) => {
                action.store_in(document, chord, platform);
                ShortcutConflicts::resolve_after_global_recording(document, action, platform);
            }
            Self::Composing(action, field) => {
                if let Some(chord) = chord {
                    ShortcutConflicts::resolve_after_composing_recording(
                        document,
                        (action, field),
                        chord,
                        platform,
                    );
                }
                document.set_composing_chord(action, field, chord, platform);
            }
        }
    }
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
    /// Swallowed with no effect (a repeat, a bare Backspace / Delete).
    Ignored,
}

/// The recorder's decision for `press` on a row of `tier`. The slot keys
/// need no refusal of their own: the shared gate refuses every bare letter,
/// digit and `;` whichever tone scheme is live (`ComposingKeyChord::make`).
pub fn evaluate_press(
    tier: RecorderTier,
    press: &RecordedPress,
    platform: DesktopPlatform,
) -> RecorderOutcome {
    // Key repeat is dropped: holding a key would otherwise record it over
    // and over, each time re-running conflict resolution.
    if press.is_repeat {
        return RecorderOutcome::Ignored;
    }
    // Tab is recorded like any other key: it is the shipped key of Next Candidate
    // (`ComposingAction::NextCandidate`), and a field that let it walk the
    // form instead left Reset to Defaults — every row at once — as the only way to
    // put it back (USER 2026-09-19). Escape and a click outside remain the
    // ways to leave a field.
    if press.modifiers.is_empty() {
        match press.key.as_deref() {
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
        platform,
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
    // was a rule with no line to draw (USER 2026-09-04, real device: Open Settings
    // Menu could not take Ctrl+Alt+A).
    if tier == RecorderTier::Global {
        if let Some(reason) = global_rejection(&chord) {
            return RecorderOutcome::Refused(reason);
        }
    }
    RecorderOutcome::Recorded(chord)
}

/// The prompt a refusal replaces. Every refusal but `NoKey` means the chord
/// already belongs to something — typing, the input method, the system, or
/// the host app — and to the reader they all mean "not this key", so one
/// message covers them. macOS keeps a Swift twin: `ShortcutKeyRecorder.swift`
/// `prompt`.
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
    use crate::keys::ComposingKeyBindings;
    use crate::platform::test_support::{
        ALL_PLATFORMS, TEST_PLATFORM as PLATFORM, WINDOWS_AND_LINUX,
    };
    use crate::platform::DesktopPlatform;

    #[test]
    fn the_last_row_to_record_a_chord_is_the_one_that_keeps_it() {
        // trace: both registries go through `RecorderTarget::store`, so the
        // conflict pass cannot be forgotten on one of them. Ctrl+K on a
        // global row, then the same chord on a composing row: the global
        // row empties (`ShortcutConflicts`). Moved from the Windows window's
        // tests, where it ran only on the box.
        let chord = ComposingKeyChord::make(
            Some("k"),
            KeyModifiers {
                control: true,
                ..Default::default()
            },
            PLATFORM,
        )
        .expect("Ctrl+K is a chord");
        let mut document = SettingsDocument::default();
        let global = RecorderTarget::Global(ShortcutAction::ToggleRomanization);
        global.store(&mut document, Some(&chord), PLATFORM);
        assert_eq!(
            ShortcutAction::ToggleRomanization.chord_in(&document, PLATFORM),
            Some(chord.clone())
        );

        let composing = RecorderTarget::Composing(ComposingAction::PageForward, ChordField::First);
        composing.store(&mut document, Some(&chord), PLATFORM);
        assert_eq!(
            ComposingKeyBindings::from_document(&document, PLATFORM)
                .chord(ComposingAction::PageForward, ChordField::First),
            Some(&chord)
        );
        assert_eq!(
            ShortcutAction::ToggleRomanization.chord_in(&document, PLATFORM),
            None,
            "the row that had it first gives it up"
        );

        // Clearing empties only the row it was pressed on, on either registry.
        composing.store(&mut document, None, PLATFORM);
        assert_eq!(
            ComposingKeyBindings::from_document(&document, PLATFORM)
                .chord(ComposingAction::PageForward, ChordField::First),
            None
        );
        global.store(&mut document, Some(&chord), PLATFORM);
        global.store(&mut document, None, PLATFORM);
        assert_eq!(
            ShortcutAction::ToggleRomanization.chord_in(&document, PLATFORM),
            None
        );
    }

    #[test]
    fn a_second_field_recording_takes_the_chord_from_every_other_field() {
        // trace: Space ships on alternate.first; recorded on confirm.second it
        // empties alternate.first (`composing_fields_holding`); then recorded
        // on confirm.first it empties confirm.second — one chord, one field.
        let space = ComposingKeyChord::make(Some(" "), KeyModifiers::NONE, PLATFORM).unwrap();
        let confirm = ComposingAction::ConfirmHighlighted;
        let alternate = ComposingAction::CommitAlternateScript;
        let mut document = SettingsDocument::default();
        RecorderTarget::Composing(confirm, ChordField::Second).store(
            &mut document,
            Some(&space),
            PLATFORM,
        );
        let bindings = ComposingKeyBindings::from_document(&document, PLATFORM);
        assert_eq!(bindings.chord(confirm, ChordField::Second), Some(&space));
        assert_eq!(bindings.chord(alternate, ChordField::First), None);
        assert_eq!(
            document.raw_string(&alternate.settings_key_name(ChordField::First)),
            Some(""),
            "the loser is cleared, not left on a shadowed default"
        );

        RecorderTarget::Composing(confirm, ChordField::First).store(
            &mut document,
            Some(&space),
            PLATFORM,
        );
        let bindings = ComposingKeyBindings::from_document(&document, PLATFORM);
        assert_eq!(bindings.chord(confirm, ChordField::First), Some(&space));
        assert_eq!(bindings.chord(confirm, ChordField::Second), None);
    }

    #[test]
    fn a_recording_also_clears_a_copy_the_resolver_hid() {
        // trace: Ctrl+K stored on next.first and page_forward.second — a file
        // an older build (blind to the second field) wrote. The resolver
        // shows only page_forward.second (later in roster order). Recording
        // Ctrl+K on previous.first must clear BOTH, or next.first would come
        // back on the next read and win over the fresh recording.
        let control_k =
            ComposingKeyChord::make(Some("k"), KeyModifiers::CONTROL, PLATFORM).unwrap();
        let mut document = SettingsDocument::default();
        document.set_composing_chord(
            ComposingAction::NextCandidate,
            ChordField::First,
            Some(&control_k),
            PLATFORM,
        );
        document.set_composing_chord(
            ComposingAction::PageForward,
            ChordField::Second,
            Some(&control_k),
            PLATFORM,
        );
        let previous = ComposingAction::PreviousCandidate;
        RecorderTarget::Composing(previous, ChordField::First).store(
            &mut document,
            Some(&control_k),
            PLATFORM,
        );
        let bindings = ComposingKeyBindings::from_document(&document, PLATFORM);
        assert_eq!(
            bindings.chord(previous, ChordField::First),
            Some(&control_k)
        );
        for (action, field) in ComposingAction::all_fields() {
            if (action, field) != (previous, ChordField::First) {
                assert_ne!(
                    bindings.chord(action, field),
                    Some(&control_k),
                    "{action:?} {field:?}"
                );
            }
        }
        assert_eq!(
            document
                .raw_string(&ComposingAction::NextCandidate.settings_key_name(ChordField::First)),
            Some("")
        );
    }

    #[test]
    fn a_target_names_its_row_and_its_registry() {
        let global = RecorderTarget::Global(ShortcutAction::ToggleRomanization);
        let composing = RecorderTarget::Composing(ComposingAction::PageForward, ChordField::First);
        assert_eq!(global.tier(), RecorderTier::Global);
        assert_eq!(composing.tier(), RecorderTier::Composing);
        assert_eq!(
            global.label_key(),
            ShortcutAction::ToggleRomanization.label_key()
        );
        assert_eq!(
            composing.label_key(),
            ComposingAction::PageForward.label_key()
        );
        // trace: `field_name` — a global row has one field, a composing row
        // numbers its two; the button names add the value, the × "Delete".
        let strings = StringResolver::new(crate::strings::DisplayLanguage::English);
        let next_page = strings.resolve(ComposingAction::PageForward.label_key());
        let second = RecorderTarget::Composing(ComposingAction::PageForward, ChordField::Second);
        assert_eq!(
            global.field_name(&strings),
            strings.resolve(ShortcutAction::ToggleRomanization.label_key())
        );
        assert_eq!(composing.field_name(&strings), format!("{next_page} 1"));
        assert_eq!(second.field_name(&strings), format!("{next_page} 2"));
        assert_eq!(
            composing.field_accessible_name(&strings, "]"),
            format!("{next_page} 1: ]")
        );
        assert_eq!(
            second.clear_accessible_name(&strings),
            format!("{} {next_page} 2", strings.resolve(StringKey::CommonDelete))
        );
    }

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
            let outcome = evaluate_press(tier, &press("[", KeyModifiers::NONE), PLATFORM);
            assert!(matches!(outcome, RecorderOutcome::Recorded(chord) if chord.key == "["));
            for key in ["a", "z", "q", "3", ";"] {
                assert_eq!(
                    evaluate_press(tier, &press(key, KeyModifiers::NONE), PLATFORM),
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
            evaluate_press(
                RecorderTier::Composing,
                &press("s", KeyModifiers::CONTROL),
                PLATFORM
            ),
            RecorderOutcome::Recorded(_)
        ));
        assert_eq!(
            evaluate_press(
                RecorderTier::Global,
                &press("s", KeyModifiers::CONTROL),
                PLATFORM
            ),
            RecorderOutcome::Refused(ChordRejection::BelongsToHost)
        );
    }

    /// Every desktop, the Mac included (its recorder's press translation
    /// feeds this decision, `taigi-macos-ffi` `key_rules.rs`): Escape blurs,
    /// Backspace / Delete blank, Tab and Shift+Tab record, a repeat is
    /// swallowed, no key is refused; Shift+Backspace is the reserved key.
    #[test]
    fn escape_delete_and_repeats_end_or_swallow_without_recording() {
        for platform in ALL_PLATFORMS {
            let composing =
                |press: &RecordedPress| evaluate_press(RecorderTier::Composing, press, platform);
            assert_eq!(
                composing(&press("\u{1B}", KeyModifiers::NONE)),
                RecorderOutcome::Blurred
            );
            assert_eq!(
                composing(&press("\t", KeyModifiers::NONE)),
                RecorderOutcome::Recorded(
                    ComposingKeyChord::make(Some("\t"), KeyModifiers::NONE, platform)
                        .expect("bindable")
                )
            );
            for blank in ["\u{8}", "\u{7F}"] {
                assert_eq!(
                    composing(&press(blank, KeyModifiers::NONE)),
                    RecorderOutcome::Ignored,
                    "{blank:?}"
                );
            }
            assert_eq!(
                composing(&press("\u{7F}", KeyModifiers::SHIFT)),
                RecorderOutcome::Refused(ChordRejection::ReservedKey)
            );
            // Shift+Tab is a chord, recordable (the previous-candidate default).
            assert!(matches!(
                composing(&press("\t", KeyModifiers::SHIFT)),
                RecorderOutcome::Recorded(_)
            ));
            let repeat = RecordedPress {
                is_repeat: true,
                ..press("[", KeyModifiers::NONE)
            };
            assert_eq!(composing(&repeat), RecorderOutcome::Ignored);
            let none = RecordedPress {
                key: None,
                modifiers: KeyModifiers::CONTROL,
                key_code: None,
                is_repeat: false,
            };
            assert_eq!(
                composing(&none),
                RecorderOutcome::Refused(ChordRejection::NoKey)
            );
        }
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
                    evaluate_press(tier, &press(key, modifiers), PLATFORM),
                    RecorderOutcome::Recorded(
                        ComposingKeyChord::make(Some(key), modifiers, PLATFORM).expect("bindable")
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
                evaluate_press(tier, &shifted_three, PLATFORM),
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
            evaluate_press(RecorderTier::Composing, &bare_hash, PLATFORM),
            RecorderOutcome::Recorded(_)
        ));
    }

    /// The Windows recorder spells a key that types nothing as a private-use
    /// scalar (`key_translation::named_key_scalar`), and Windows and Linux
    /// refuse every one as reserved on both tiers (inventory K4); the Mac
    /// reserves only its nine keys, so ⌃Home records there.
    #[test]
    fn a_private_use_key_is_refused_as_reserved_on_windows_and_linux_only() {
        let control_shift = KeyModifiers::CONTROL.with(KeyModifiers::SHIFT);
        for platform in WINDOWS_AND_LINUX {
            for tier in [RecorderTier::Composing, RecorderTier::Global] {
                for key in ["\u{F700}", "\u{F704}", "\u{F729}", "\u{F72D}"] {
                    assert_eq!(
                        evaluate_press(tier, &press(key, control_shift), platform),
                        RecorderOutcome::Refused(ChordRejection::ReservedKey),
                        "{key:?} {tier:?} {platform:?}"
                    );
                }
            }
        }
        let mac = DesktopPlatform::MacOS;
        let control_home = press("\u{F729}", KeyModifiers::CONTROL);
        assert!(matches!(
            evaluate_press(RecorderTier::Composing, &control_home, mac),
            RecorderOutcome::Recorded(chord) if chord.key == "\u{F729}"
        ));
        assert_eq!(
            evaluate_press(
                RecorderTier::Composing,
                &press("\u{F702}", KeyModifiers::CONTROL),
                mac
            ),
            RecorderOutcome::Refused(ChordRejection::ReservedKey),
            "the arrows stay reserved on the Mac"
        );
    }
}
