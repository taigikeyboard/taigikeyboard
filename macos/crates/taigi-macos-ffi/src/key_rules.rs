//! The key rules the Swift Shortcuts pane and symbol picker ask
//! (docs/architecture/macos-desktop-core-roadmap.md P14): desktop-core's
//! chord gate, recorder decision, resolved bindings, fixed-row labels and
//! picker reading, under the Mac's grammar — and the input-mode writer the
//! Swift settings ask (desktop TPS roadmap D5), and the TPS key panel's rows
//! (D6). Pure — no token, no session, no
//! runtime: a settings snapshot one of them carries is read for its one
//! answer and never installed, so the key path keeps what the last session
//! request carried, a panic here included.

use taigi_desktop_core::keys::{
    evaluate_press, shortcut_labels, tps_keyboard_rows, ChordField, ChordRejection,
    ComposingAction, ComposingKeyBindings, ComposingKeyChord, RecorderOutcome, RecorderTier,
    SymbolPickerIntent,
};
use taigi_desktop_core::settings::{
    keys, InputMode, InputModeRequest, SettingChoice, SettingsDocument,
};

use crate::key_translation::{self, ESCAPE_KEY_CODE};
use crate::proto::{
    chord_reply, desktop_request, desktop_response, press_reply, symbol_picker_reply, Chord,
    ChordReply, ChordRequest, ComposingShortcut, ComposingShortcutField, ComposingShortcutsReply,
    PressReply, PressRequest, RecorderAction, SettingsSnapshot, SwitchInputModeReply,
    SwitchInputModeRequest, SymbolPickerAction, SymbolPickerKeyRequest, SymbolPickerReply,
    TpsKeyCap, TpsKeyboardRow, TpsKeyboardRowsReply,
};
use crate::runtime::{Refusal, DESKTOP_PLATFORM};
use crate::session::navigation;
use crate::settings;

/// The answer to `request` if it is a key rule, under the snapshot it
/// carries; `None` for a session request, which `Shell::serve` runs under
/// the snapshot it puts in force. The one list of the key rules.
pub(crate) fn answer(
    request: &desktop_request::Request,
    snapshot: Option<&SettingsSnapshot>,
) -> Option<Result<desktop_response::Reply, Refusal>> {
    use desktop_request::Request;
    use desktop_response::Reply;
    Some(match request {
        Request::Press(press) => self::press(press).map(Reply::Press),
        Request::Chord(chord) => Ok(Reply::Chord(self::chord(chord))),
        Request::ComposingShortcuts(_) => {
            composing_shortcuts(snapshot).map(Reply::ComposingShortcuts)
        }
        Request::SymbolPickerKey(key) => symbol_picker_key(key, snapshot).map(Reply::SymbolPicker),
        Request::SwitchInputMode(switch) => switch_input_mode(switch).map(Reply::SwitchInputMode),
        Request::TpsKeyboardRows(_) => Ok(Reply::TpsKeyboardRows(tps_keyboard_rows_reply())),
        _ => return None,
    })
}

/// One press in a recording field: `evaluate_press` on the composing tier,
/// the Mac's press translation in front of it (`recorded_press`).
///
/// The Mac recorder leaves on the Escape KEY (`kVK_Escape`), not on the ESC
/// character: a bare Escape blurs whatever the layout types for it, and a
/// bare ESC another key types is refused as the reserved key it is, as the
/// gate refuses it — never the way out. The core's blanking keys come first:
/// an Escape key a layout reports as Backspace or Delete blanks the field.
fn press(request: &PressRequest) -> Result<PressReply, Refusal> {
    let event = request.event.as_ref().ok_or(Refusal::Missing("event"))?;
    let press = key_translation::recorded_press(event);
    let is_bare_escape_key = press.modifiers.is_empty() && event.key_code == Some(ESCAPE_KEY_CODE);
    let outcome = match evaluate_press(RecorderTier::Composing, &press, DESKTOP_PLATFORM) {
        RecorderOutcome::Ignored => RecorderOutcome::Ignored,
        _ if is_bare_escape_key => RecorderOutcome::Blurred,
        // The core blurs on the ESC character; typed by another key, it is
        // the reserved key the gate refuses.
        RecorderOutcome::Blurred => RecorderOutcome::Refused(ChordRejection::ReservedKey),
        other => other,
    };
    use press_reply::Outcome;
    let outcome = match outcome {
        RecorderOutcome::Recorded(chord) => Outcome::Recorded(chord_message(&chord)),
        RecorderOutcome::Refused(reason) => Outcome::Refused(rejection(reason) as i32),
        RecorderOutcome::Blurred => Outcome::Action(RecorderAction::Blur as i32),
        // A repeat is Swift's to drop, so `Ignored` is only ever a bare
        // Backspace / Delete / forward Delete, which blanks the field.
        RecorderOutcome::Ignored => Outcome::Action(RecorderAction::Blank as i32),
    };
    Ok(PressReply {
        outcome: Some(outcome),
    })
}

/// The gate a key and modifiers go through (`ComposingKeyChord::make`).
fn chord(request: &ChordRequest) -> ChordReply {
    let modifiers = key_translation::modifiers(request.modifier_flags);
    let answer = match ComposingKeyChord::make(request.key.as_deref(), modifiers, DESKTOP_PLATFORM)
    {
        Ok(chord) => chord_reply::Answer::Chord(chord_message(&chord)),
        Err(reason) => chord_reply::Answer::Rejection(rejection(reason) as i32),
    };
    ChordReply {
        answer: Some(answer),
    }
}

/// The pane's composing rows, resolved from `snapshot`, and its fixed rows.
fn composing_shortcuts(
    snapshot: Option<&SettingsSnapshot>,
) -> Result<ComposingShortcutsReply, Refusal> {
    let document = document(snapshot)?;
    let bindings = ComposingKeyBindings::from_document(&document, DESKTOP_PLATFORM);
    let slot_keys = bindings.slot_key_set();
    let actions = ComposingAction::ALL
        .into_iter()
        .map(|action| ComposingShortcut {
            name: action.raw().to_owned(),
            label_key: action.label_key().as_str().to_owned(),
            group: group(action),
            fixed_keys: shortcut_labels::fixed_keys_label(action, slot_keys, DESKTOP_PLATFORM)
                .unwrap_or_default(),
            fields: ChordField::ALL
                .into_iter()
                .map(|field| {
                    let stored = document
                        .stored_composing_chord(action, field, DESKTOP_PLATFORM)
                        .flatten();
                    ComposingShortcutField {
                        chord: bindings.chord(action, field).map(chord_message),
                        shipped_chord: action.shipped_chord(field).as_ref().map(chord_message),
                        stored_chord: stored.as_ref().map(chord_message),
                        settings_key: action.settings_key_name(field),
                    }
                })
                .collect(),
        })
        .collect();
    Ok(ComposingShortcutsReply {
        actions,
        slot_keys: shortcut_labels::slot_keys_label(slot_keys, DESKTOP_PLATFORM),
        navigation_keys: shortcut_labels::navigation_keys_label(DESKTOP_PLATFORM).to_owned(),
        caret_chords: shortcut_labels::caret_chords_label(DESKTOP_PLATFORM),
    })
}

/// One key while the picker is up, read under `snapshot`'s bindings.
fn symbol_picker_key(
    request: &SymbolPickerKeyRequest,
    snapshot: Option<&SettingsSnapshot>,
) -> Result<SymbolPickerReply, Refusal> {
    let event = request.event.as_ref().ok_or(Refusal::Missing("event"))?;
    let bindings = bindings(snapshot)?;
    let key = key_translation::snapshot(event);
    use symbol_picker_reply::Intent;
    let intent = match SymbolPickerIntent::intent(&key, &bindings, DESKTOP_PLATFORM) {
        SymbolPickerIntent::Close => Intent::Action(SymbolPickerAction::Close as i32),
        SymbolPickerIntent::Navigate(direction) => Intent::Navigate(navigation(direction) as i32),
        // A slot is one of the nine a page holds.
        SymbolPickerIntent::PickSlot(slot) => Intent::PickSlot(slot as u32),
        SymbolPickerIntent::Confirm => Intent::Action(SymbolPickerAction::Confirm as i32),
        SymbolPickerIntent::CloseAndPassThrough => {
            Intent::Action(SymbolPickerAction::CloseAndPassThrough as i32)
        }
    };
    Ok(SymbolPickerReply {
        intent: Some(intent),
    })
}

/// Where `request` moves the stored mode, through the core's one writer
/// (`SettingsDocument::switch_input_mode`), on a document holding only the
/// two stored values the request carries.
fn switch_input_mode(request: &SwitchInputModeRequest) -> Result<SwitchInputModeReply, Refusal> {
    use crate::proto::switch_input_mode_request::Request as Switch;
    let switch = match request.request.as_ref() {
        Some(Switch::Pick(raw)) => InputModeRequest::Pick(
            InputMode::from_raw(raw).ok_or(Refusal::Missing("switch_input_mode.pick"))?,
        ),
        Some(Switch::ToggleRomanization(_)) => InputModeRequest::ToggleRomanization,
        Some(Switch::ToggleTps(_)) => InputModeRequest::ToggleTps,
        None => return Err(Refusal::Missing("switch_input_mode.request")),
    };
    let last_romanization = keys::LAST_ROMANIZATION_MODE.name;
    let mut document = SettingsDocument::default();
    document.set_raw_string(keys::INPUT_MODE.name, &request.input_mode);
    document.set_raw_string(last_romanization, &request.last_romanization_mode);
    let before = document.raw_string(last_romanization).map(str::to_owned);
    let next = document.switch_input_mode(switch);
    let after = document.raw_string(last_romanization);
    Ok(SwitchInputModeReply {
        input_mode: next.raw().to_owned(),
        last_romanization_mode: (after != before.as_deref())
            .then(|| after.unwrap_or_default().to_owned()),
    })
}

/// The core's TPS key panel rows as the proto carries them.
fn tps_keyboard_rows_reply() -> TpsKeyboardRowsReply {
    TpsKeyboardRowsReply {
        rows: tps_keyboard_rows()
            .into_iter()
            .map(|row| TpsKeyboardRow {
                indent: row.indent,
                caps: row
                    .caps
                    .into_iter()
                    .map(|cap| TpsKeyCap {
                        label: cap.label.to_string(),
                        glyph: cap.glyph.to_owned(),
                        shift_glyph: cap.shift_glyph.map(str::to_owned),
                    })
                    .collect(),
            })
            .collect(),
    }
}

/// The bindings the snapshot describes. A snapshot is required: the last
/// one the key path installed may be a different store's.
fn bindings(snapshot: Option<&SettingsSnapshot>) -> Result<ComposingKeyBindings, Refusal> {
    Ok(ComposingKeyBindings::from_document(
        &document(snapshot)?,
        DESKTOP_PLATFORM,
    ))
}

/// The settings the snapshot carries, for the same reason it is required.
fn document(snapshot: Option<&SettingsSnapshot>) -> Result<SettingsDocument, Refusal> {
    let snapshot = snapshot.ok_or(Refusal::Missing("settings"))?;
    settings::document_from(&snapshot.entries).map_err(Refusal::Settings)
}

fn chord_message(chord: &ComposingKeyChord) -> Chord {
    Chord {
        raw: chord.raw_value(DESKTOP_PLATFORM),
        display: chord.display(DESKTOP_PLATFORM),
    }
}

/// The pane block `action` is drawn in.
fn group(action: ComposingAction) -> u32 {
    ComposingAction::GROUPS
        .iter()
        .position(|group| group.contains(&action))
        .map_or(0, |index| index as u32)
}

fn rejection(reason: ChordRejection) -> crate::proto::ChordRejection {
    use crate::proto::ChordRejection as Wire;
    match reason {
        ChordRejection::TypesRomanization => Wire::TypesRomanization,
        ChordRejection::ReservedKey => Wire::ReservedKey,
        ChordRejection::NoKey => Wire::NoKey,
        ChordRejection::TakenBySystem => Wire::TakenBySystem,
        ChordRejection::NotAGlobalKey => Wire::NotAGlobalKey,
        ChordRejection::BelongsToHost => Wire::BelongsToHost,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key_translation::{COMMAND, CONTROL, OPTION, SHIFT};
    use crate::proto::{CandidateNavigation as WireNavigation, ChordRejection as Wire, KeyEvent};
    use crate::test_support::{key_event, text};

    use crate::test_support::{CAPS_LOCK, FUNCTION};

    fn press_of(event: KeyEvent) -> press_reply::Outcome {
        press(&PressRequest { event: Some(event) })
            .expect("answered")
            .outcome
            .expect("an outcome")
    }

    fn recorded(raw: &str, display: &str) -> press_reply::Outcome {
        press_reply::Outcome::Recorded(Chord {
            raw: raw.to_owned(),
            display: display.to_owned(),
        })
    }

    fn refused(reason: Wire) -> press_reply::Outcome {
        press_reply::Outcome::Refused(reason as i32)
    }

    fn action(action: RecorderAction) -> press_reply::Outcome {
        press_reply::Outcome::Action(action as i32)
    }

    fn with_key_code(event: KeyEvent, key_code: u32) -> KeyEvent {
        KeyEvent {
            key_code: Some(key_code),
            ..event
        }
    }

    /// P8 → P14: a bare ⌦ blanks the field as `.deleteForward` did in
    /// Swift — Caps Lock and Fn (a laptop's ⌦ is Fn+⌫) do not make it a
    /// chord — and with a chording modifier it records as F728.
    #[test]
    fn a_bare_forward_delete_blanks_and_a_chorded_one_records() {
        for flags in [0, CAPS_LOCK | FUNCTION] {
            assert_eq!(
                press_of(key_event("\u{F728}", flags, Some(0xF728))),
                action(RecorderAction::Blank),
                "{flags:#x}"
            );
        }
        assert_eq!(
            press_of(key_event("\u{F728}", CONTROL | FUNCTION, Some(0xF728))),
            recorded("c|F728", "⌃\u{F728}")
        );
        assert_eq!(
            press_of(key_event("\u{F728}", SHIFT, Some(0xF728))),
            recorded("s|F728", "⇧\u{F728}")
        );
        for (characters, special_key) in [("\u{7F}", 0x7F), ("\u{8}", 0x08)] {
            assert_eq!(
                press_of(key_event(characters, 0, Some(special_key))),
                action(RecorderAction::Blank),
                "{characters:?}"
            );
        }
        assert_eq!(
            press_of(key_event("\u{7F}", SHIFT, Some(0x7F))),
            refused(Wire::ReservedKey),
            "⇧⌫ is the reserved key, not the way to blank"
        );
    }

    /// The Escape KEY blurs whatever the layout types for it; an ESC typed
    /// by another key is refused like the reserved key it is
    /// (`ESCAPE_KEY_CODE`, `kVK_Escape`); ⌃[ (ESC under ⌃) is
    /// a chord on the reserved key.
    #[test]
    fn the_escape_key_blurs_and_an_escape_character_is_refused() {
        assert_eq!(
            press_of(with_key_code(key_event("\u{1B}", 0, None), ESCAPE_KEY_CODE)),
            action(RecorderAction::Blur)
        );
        assert_eq!(
            press_of(with_key_code(key_event("[", 0, None), ESCAPE_KEY_CODE)),
            action(RecorderAction::Blur),
            "a layout that types `[` on Escape still leaves"
        );
        assert_eq!(
            press_of(with_key_code(key_event("\u{1B}", 0, None), 0x00)),
            refused(Wire::ReservedKey)
        );
        // The deletes are read first: an Escape key a layout reports as
        // Delete or forward Delete blanks rather than blurs.
        for (characters, special_key) in [("\u{7F}", 0x7F), ("\u{F728}", 0xF728)] {
            assert_eq!(
                press_of(with_key_code(
                    key_event(characters, 0, Some(special_key)),
                    ESCAPE_KEY_CODE
                )),
                action(RecorderAction::Blank),
                "{characters:?}"
            );
        }
        assert_eq!(
            press_of(with_key_code(key_event("\u{1B}", CONTROL, None), 0x21)),
            refused(Wire::ReservedKey)
        );
    }

    /// The gate through the press: Tab and ⇧Tab (AppKit's back tab) record,
    /// a syllable letter is refused bare, Shift on the `3` and `;` keys is
    /// the digit / slot key whatever the layout types (Carbon codes 0x14 /
    /// 0x29), a `#` reached elsewhere records, no characters is no key.
    #[test]
    fn a_press_goes_through_the_gate() {
        assert_eq!(press_of(key_event("\t", 0, None)), recorded("|0009", "⇥"));
        assert_eq!(
            press_of(key_event("\u{19}", SHIFT, None)),
            recorded("s|0009", "⇧⇥")
        );
        assert_eq!(
            press_of(key_event("a", 0, None)),
            refused(Wire::TypesRomanization)
        );
        for (characters, carbon) in [("#", 0x14), (":", 0x29)] {
            assert_eq!(
                press_of(with_key_code(key_event(characters, SHIFT, None), carbon)),
                refused(Wire::TypesRomanization),
                "{characters}"
            );
        }
        assert_eq!(
            press_of(key_event("#", SHIFT, None)),
            recorded("s|0023", "⇧#")
        );
        assert_eq!(
            press_of(key_event("r", COMMAND | CONTROL | CAPS_LOCK, None)),
            recorded("dc|0072", "⌃⌘R")
        );
        let no_characters = KeyEvent {
            characters: None,
            characters_ignoring_modifiers: None,
            ..key_event("", CONTROL, None)
        };
        assert_eq!(press_of(no_characters), refused(Wire::NoKey));
        assert_eq!(
            press(&PressRequest { event: None }),
            Err(Refusal::Missing("event"))
        );
    }

    #[test]
    fn a_chord_is_made_through_the_gate() {
        let chord_of = |key: Option<&str>, modifier_flags: u64| {
            chord(&ChordRequest {
                key: key.map(str::to_owned),
                modifier_flags,
            })
            .answer
            .expect("an answer")
        };
        assert_eq!(
            chord_of(Some("\r"), SHIFT),
            chord_reply::Answer::Chord(Chord {
                raw: "s|000D".to_owned(),
                display: "⇧↩".to_owned(),
            })
        );
        assert_eq!(
            chord_of(Some("r"), COMMAND | CONTROL | CAPS_LOCK),
            chord_reply::Answer::Chord(Chord {
                raw: "dc|0072".to_owned(),
                display: "⌃⌘R".to_owned(),
            })
        );
        for (key, flags, reason) in [
            (Some("3"), SHIFT, Wire::TypesRomanization),
            (Some("\u{7F}"), 0, Wire::ReservedKey),
            (Some("\u{F702}"), OPTION, Wire::ReservedKey),
            (None, 0, Wire::NoKey),
            (Some(""), CONTROL, Wire::NoKey),
        ] {
            assert_eq!(
                chord_of(key, flags),
                chord_reply::Answer::Rejection(reason as i32),
                "{key:?}"
            );
        }
    }

    fn shortcuts(entries: Vec<crate::proto::SettingEntry>) -> ComposingShortcutsReply {
        composing_shortcuts(Some(&SettingsSnapshot { entries })).expect("answered")
    }

    /// trace: `ComposingAction::ALL` order and `GROUPS`; shipped chords
    /// ⇥ ⇧⇥ ] [ ↩+Space ⇧↩, the other script none (`action.rs`
    /// `shipped_chord`, `display(MacOS)`);
    /// the fixed rows are `shortcut_labels`' Mac arms.
    #[test]
    fn the_pane_reads_every_row_in_roster_order() {
        let reply = shortcuts(vec![]);
        // Per field: what it shows, what it ships with.
        type Field<'a> = (Option<&'a str>, Option<&'a str>);
        let rows: Vec<(&str, u32, Vec<Field>)> = reply
            .actions
            .iter()
            .map(|row| {
                let fields = row
                    .fields
                    .iter()
                    .map(|field| {
                        (
                            field.chord.as_ref().map(|chord| chord.display.as_str()),
                            field.shipped_chord.as_ref().map(|chord| chord.raw.as_str()),
                        )
                    })
                    .collect();
                (row.name.as_str(), row.group, fields)
            })
            .collect();
        let none = (None, None);
        assert_eq!(
            rows,
            [
                ("nextCandidate", 0, vec![(Some("⇥"), Some("|0009")), none]),
                (
                    "previousCandidate",
                    0,
                    vec![(Some("⇧⇥"), Some("s|0009")), none]
                ),
                ("pageForward", 0, vec![(Some("]"), Some("|005D")), none]),
                ("pageBackward", 0, vec![(Some("["), Some("|005B")), none]),
                (
                    "confirmHighlighted",
                    1,
                    vec![(Some("↩"), Some("|000D")), (Some("Space"), Some("|0020"))]
                ),
                ("commitLiteral", 1, vec![(Some("⇧↩"), Some("s|000D")), none]),
                ("commitAlternateScript", 1, vec![none, none]),
            ]
        );
        let first = &reply.actions[0];
        let keys: Vec<&str> = first
            .fields
            .iter()
            .map(|f| f.settings_key.as_str())
            .collect();
        assert_eq!(
            keys,
            [
                "composingShortcut.nextCandidate",
                "composingShortcut.nextCandidate.second"
            ]
        );
        assert_eq!(first.label_key, "i18n_desktop_actionNextCandidate");
        assert_eq!(reply.slot_keys, "qwdfzxvy;");
        assert_eq!(reply.actions[6].fixed_keys, "⇧QWDFZXVY;");
        assert_eq!(reply.actions[5].fixed_keys, "");
        assert_eq!(reply.navigation_keys, "←  →  ↑  ↓  ⇞  ⇟");
        assert_eq!(reply.caret_chords, "⌥←  ⌥→  ⌥↑  ⌥↓  ↖  ↘");
    }

    /// A cleared row reads empty, a recorded one as the snapshot holds it,
    /// and the slot rows follow the tone scheme.
    #[test]
    fn the_pane_reads_the_snapshot_it_carries() {
        let reply = shortcuts(vec![
            text("composingShortcut.nextCandidate", ""),
            text("composingShortcut.pageForward", "c|005D"),
            text("toneInputScheme", "telex"),
        ]);
        assert_eq!(reply.actions[0].fields[0].chord, None);
        let control_bracket = Some(Chord {
            raw: "c|005D".to_owned(),
            display: "⌃]".to_owned(),
        });
        assert_eq!(reply.actions[2].fields[0].chord, control_bracket);
        assert_eq!(reply.actions[2].fields[0].stored_chord, control_bracket);
        assert_eq!(reply.actions[0].fields[0].stored_chord, None, "cleared");
        assert_eq!(reply.slot_keys, "123456789");
        assert_eq!(reply.actions[6].fixed_keys, "⇧123456789");
        assert_eq!(composing_shortcuts(None), Err(Refusal::Missing("settings")));
        assert!(matches!(
            composing_shortcuts(Some(&SettingsSnapshot {
                entries: vec![text("notASetting", "")],
            })),
            Err(Refusal::Settings(_))
        ));
    }

    fn picker(
        event: KeyEvent,
        entries: Vec<crate::proto::SettingEntry>,
    ) -> symbol_picker_reply::Intent {
        symbol_picker_key(
            &SymbolPickerKeyRequest { event: Some(event) },
            Some(&SettingsSnapshot { entries }),
        )
        .expect("answered")
        .intent
        .expect("an intent")
    }

    fn picker_action(action: SymbolPickerAction) -> symbol_picker_reply::Intent {
        symbol_picker_reply::Intent::Action(action as i32)
    }

    /// The picker's keys through the Mac's translation (`snapshot`): the
    /// bare Escape, an arrow by its `specialKey`, the slot keys of the
    /// snapshot's scheme, the bound rows, and a row recorded with ⌥.
    #[test]
    fn the_picker_reads_a_key_under_the_snapshot_it_carries() {
        use symbol_picker_reply::Intent;
        assert_eq!(
            picker(key_event("\u{1B}", 0, None), vec![]),
            picker_action(SymbolPickerAction::Close)
        );
        assert_eq!(
            picker(key_event("\u{F701}", 0, Some(0xF701)), vec![]),
            Intent::Navigate(WireNavigation::Down as i32)
        );
        assert_eq!(picker(key_event(";", 0, None), vec![]), Intent::PickSlot(8));
        assert_eq!(
            picker(
                key_event("9", 0, None),
                vec![text("toneInputScheme", "telex")]
            ),
            Intent::PickSlot(8)
        );
        assert_eq!(
            picker(key_event("\r", 0, Some(0x0D)), vec![]),
            picker_action(SymbolPickerAction::Confirm)
        );
        assert_eq!(
            picker(key_event("\r", SHIFT, Some(0x0D)), vec![]),
            picker_action(SymbolPickerAction::CloseAndPassThrough)
        );
        let option_paging = vec![text("composingShortcut.pageForward", "o|000D")];
        assert_eq!(
            picker(key_event("\r", OPTION, Some(0x0D)), option_paging.clone()),
            Intent::Navigate(WireNavigation::PageDown as i32)
        );
        assert_eq!(
            picker(key_event("]", 0, None), option_paging),
            picker_action(SymbolPickerAction::CloseAndPassThrough)
        );
        assert_eq!(
            symbol_picker_key(
                &SymbolPickerKeyRequest {
                    event: Some(key_event("q", 0, None)),
                },
                None
            ),
            Err(Refusal::Missing("settings"))
        );
        assert_eq!(
            symbol_picker_key(
                &SymbolPickerKeyRequest { event: None },
                Some(&SettingsSnapshot::default())
            ),
            Err(Refusal::Missing("event"))
        );
    }

    fn switched(
        input_mode: &str,
        last_romanization_mode: &str,
        request: crate::proto::switch_input_mode_request::Request,
    ) -> Result<(String, Option<String>), Refusal> {
        switch_input_mode(&SwitchInputModeRequest {
            input_mode: input_mode.to_owned(),
            last_romanization_mode: last_romanization_mode.to_owned(),
            request: Some(request),
        })
        .map(|reply| (reply.input_mode, reply.last_romanization_mode))
    }

    /// The core's writer, answered for the Swift store: a romanization left
    /// for TPS is handed back to store; every other switch stores only the
    /// mode. trace: `next_input_mode` (engine_settings.rs) — ToggleTps from a
    /// romanization → tps; from tps → last; ToggleRomanization from tps →
    /// last's other; an unknown stored mode reads as tl.
    #[test]
    fn a_switch_answers_what_the_core_writer_stores() {
        use crate::proto::switch_input_mode_request::Request as Switch;
        let pair = |mode: &str, last: Option<&str>| Ok((mode.to_owned(), last.map(str::to_owned)));
        assert_eq!(
            switched("poj", "", Switch::ToggleTps(true)),
            pair("tps", Some("poj"))
        );
        assert_eq!(
            switched("tps", "poj", Switch::ToggleTps(true)),
            pair("poj", None)
        );
        assert_eq!(
            switched("tps", "poj", Switch::ToggleRomanization(true)),
            pair("tl", None)
        );
        assert_eq!(
            switched("tl", "", Switch::ToggleRomanization(true)),
            pair("poj", None)
        );
        assert_eq!(
            switched("", "", Switch::Pick("tps".to_owned())),
            pair("tps", Some("tl"))
        );
        assert_eq!(
            switched("tps", "", Switch::ToggleTps(true)),
            pair("tl", None)
        );
        assert_eq!(
            switched("bopomofo", "", Switch::ToggleTps(true)),
            pair("tps", Some("tl"))
        );
        assert_eq!(
            switched("tl", "", Switch::Pick("bopomofo".to_owned())),
            Err(Refusal::Missing("switch_input_mode.pick"))
        );
        assert_eq!(
            switch_input_mode(&SwitchInputModeRequest::default()),
            Err(Refusal::Missing("switch_input_mode.request"))
        );
    }

    /// The seam hands Swift the core's rows as they are — trace:
    /// `tps_keyboard_rows` — 12 caps at indent 0 leading with `1` ㄅ / ㆠ;
    /// `Q` ㄆ with no Shift glyph; 42 caps in all.
    #[test]
    fn the_tps_key_panel_rows_are_the_cores() {
        use crate::proto::{desktop_request, desktop_response};
        let Some(Ok(desktop_response::Reply::TpsKeyboardRows(reply))) = answer(
            &desktop_request::Request::TpsKeyboardRows(Default::default()),
            None,
        ) else {
            panic!("the rows are answered with no snapshot");
        };
        assert_eq!(reply.rows.len(), 4);
        assert_eq!(reply.rows[0].indent, 0.0);
        assert_eq!(
            reply.rows[0].caps[0],
            TpsKeyCap {
                label: "1".to_owned(),
                glyph: "ㄅ".to_owned(),
                shift_glyph: Some("ㆠ".to_owned()),
            }
        );
        assert_eq!(reply.rows[1].caps[0].shift_glyph, None);
        let count: usize = reply.rows.iter().map(|row| row.caps.len()).sum();
        assert_eq!(count, 42);
    }
}
