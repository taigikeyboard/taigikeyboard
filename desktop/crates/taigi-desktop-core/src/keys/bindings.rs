//! The user's composing key contract, resolved and ready to classify against.

use std::collections::BTreeMap;

use super::action::{ChordField, ComposingAction};
use super::chord::ComposingKeyChord;
use super::slot_key_set::CandidateSlotKeySet;
use super::snapshot::KeyEventSnapshot;
use super::tone_input_scheme::ToneInputScheme;
use crate::platform::DesktopPlatform;
use crate::settings::{keys, InputMode, SettingChoice, SettingsDocument};

/// "Resolved" means three things have already happened, so the classifier
/// can trust the value: every chord came through [`ComposingKeyChord::make`];
/// no chord is on two fields (a later recording takes it from the earlier
/// one); an empty [`ComposingAction::REFILLED_FROM_DEFAULT`] row takes back
/// whichever of the pair's defaults no other row holds — never a key off
/// another row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComposingKeyBindings {
    /// Up to two chords per action ([`ChordField`]); both fire it alike.
    chords: BTreeMap<(ComposingAction, ChordField), ComposingKeyChord>,
    /// Which keys type a tone. Carried here because it is the other half of
    /// the same contract the chords are: the classifier reads both off one
    /// value, and the slot keys follow from it.
    pub tone_scheme: ToneInputScheme,
    /// What the keys type. Carried for the same reason: under TPS the letter,
    /// digit and punctuation keys type glyphs and the keypad picks, and the
    /// classifier decides that off this one value.
    pub input_mode: InputMode,
    /// Whether a candidate window exists to act on. Carried here for the
    /// same reason as `tone_scheme`: with the window off, the keys that
    /// would confirm or page a candidate end the composition as typed
    /// instead, and the classifier decides that off one value.
    pub is_candidate_window_enabled: bool,
}

impl Default for ComposingKeyBindings {
    /// What a fresh install types with.
    fn default() -> Self {
        Self::resolve(&BTreeMap::new(), ToneInputScheme::Standard)
    }
}

impl ComposingKeyBindings {
    /// `stored` maps a field to `Some(chord)` (recorded) or `None` (the
    /// user cleared it); a field absent from the map was never touched and
    /// reads as what it ships with (`ComposingAction::shipped_chord`). The
    /// double optional is what tells the two apart.
    pub fn resolve(
        stored: &BTreeMap<(ComposingAction, ChordField), Option<ComposingKeyChord>>,
        tone_scheme: ToneInputScheme,
    ) -> Self {
        let mut resolved = BTreeMap::new();
        for (action, field) in ComposingAction::all_fields() {
            let chord = stored
                .get(&(action, field))
                .cloned()
                .unwrap_or_else(|| action.shipped_chord(field));
            if let Some(chord) = chord {
                resolved.insert((action, field), chord);
            }
        }
        // No pass against the slot tier: every chord came through
        // `ComposingKeyChord::make`, which refuses every bare letter, digit
        // and `;` — the keys either scheme's slots use — so no chord can be
        // a slot key under any scheme.
        Self::remove_duplicates(&mut resolved);
        Self::restore_unbound(&mut resolved);
        Self {
            chords: resolved,
            tone_scheme,
            // Resolving is about the chords and the scheme; the input mode
            // and the window switch are orthogonal, so they start as shipped
            // and a caller with an opinion sets them (`from_document`).
            input_mode: InputMode::DEFAULT,
            is_candidate_window_enabled: true,
        }
    }

    /// The bindings a settings document describes: one stored string per
    /// field (`ComposingAction::settings_key_name`; absent = shipped, `""`
    /// = cleared, unparsable = cleared — silently restoring the default
    /// would undo a deliberate clearing) plus the tone scheme. The
    /// stored values are read in `platform`'s chord grammar.
    pub fn from_document(document: &SettingsDocument, platform: DesktopPlatform) -> Self {
        let stored = ComposingAction::all_fields()
            .filter_map(|(action, field)| {
                let chord = document.stored_composing_chord(action, field, platform)?;
                Some(((action, field), chord))
            })
            .collect();
        Self {
            input_mode: document.choice(&keys::INPUT_MODE),
            is_candidate_window_enabled: document.bool(&keys::IS_CANDIDATE_WINDOW_ENABLED),
            ..Self::resolve(&stored, document.choice(&keys::TONE_INPUT_SCHEME))
        }
    }

    /// The keys that pick a candidate — derived, never stored: the keypad
    /// under TPS, whose main block types glyphs; otherwise the tone scheme's
    /// (`ToneInputScheme::slot_key_set`).
    pub fn slot_key_set(&self) -> CandidateSlotKeySet {
        match self.input_mode {
            InputMode::Tps => CandidateSlotKeySet::TpsDigits,
            InputMode::Tl | InputMode::Poj => self.tone_scheme.slot_key_set(),
        }
    }

    /// The chord on `action`'s `field`, or `None` when that field is empty.
    pub fn chord(&self, action: ComposingAction, field: ChordField) -> Option<&ComposingKeyChord> {
        self.chords.get(&(action, field))
    }

    /// The action `event` is bound to, if any, on either field. Linear over
    /// the roster: fourteen fields, and a chord-keyed map would need the
    /// duplicate handling twice.
    pub fn action_for(
        &self,
        event: &KeyEventSnapshot,
        platform: DesktopPlatform,
    ) -> Option<ComposingAction> {
        ComposingAction::all_fields()
            .find(|key| {
                self.chords
                    .get(key)
                    .is_some_and(|chord| chord.matches(event, platform))
            })
            .map(|(action, _)| action)
    }

    /// Drops a chord from every field but the last one holding it, with the
    /// fields still on what they ship with going first — so a recorded chord
    /// outranks a default that arrives on top of it in an upgrade. Within
    /// each half, roster order then field order is the tiebreak — so the
    /// same chord stored on both fields of one action stays on the second.
    ///
    /// "On its default" is judged by VALUE: a field the user explicitly
    /// recorded back onto its own shipped chord reads as untouched here. The store keeps the
    /// distinction (absent vs stored), the resolver deliberately does not —
    /// same observable behaviour on all three desktops.
    fn remove_duplicates(
        resolved: &mut BTreeMap<(ComposingAction, ChordField), ComposingKeyChord>,
    ) {
        let on_its_default = |&(action, field): &(ComposingAction, ChordField)| {
            resolved.get(&(action, field)) == action.shipped_chord(field).as_ref()
        };
        let order: Vec<(ComposingAction, ChordField)> = ComposingAction::all_fields()
            .filter(on_its_default)
            .chain(ComposingAction::all_fields().filter(|key| !on_its_default(key)))
            .collect();
        let mut seen: BTreeMap<ComposingKeyChord, (ComposingAction, ChordField)> = BTreeMap::new();
        for key in order {
            let Some(chord) = resolved.get(&key).cloned() else {
                continue;
            };
            if let Some(earlier) = seen.insert(chord, key) {
                resolved.remove(&earlier);
            }
        }
    }

    /// Refills an empty commit row — both fields empty — from the pair's
    /// shipped defaults, onto its first field: whichever pool chord no
    /// composing field holds, its own first. A cleared row gets its key back;
    /// a row whose second field still holds a chord is not empty and is left
    /// as the user set it; a row holding a pool chord the user recorded there
    /// keeps it, and the emptied commit
    /// row stays empty — the last recording wins here as everywhere else on
    /// the pane (USER 2026-09-19: Enter recorded on Output the Other Script used to be handed
    /// straight back to Confirm Key). An unbound Return still ends the composition:
    /// it commits before passing through to the host
    /// (`ComposingKeyIntent::host_key`).
    fn restore_unbound(resolved: &mut BTreeMap<(ComposingAction, ChordField), ComposingKeyChord>) {
        let first_default = |action: &ComposingAction| action.shipped_chord(ChordField::First);
        let pool: Vec<ComposingKeyChord> = ComposingAction::REFILLED_FROM_DEFAULT
            .iter()
            .filter_map(first_default)
            .collect();
        for action in ComposingAction::REFILLED_FROM_DEFAULT {
            if ChordField::ALL
                .iter()
                .any(|field| resolved.contains_key(&(action, *field)))
            {
                continue;
            }
            let own = first_default(&action);
            if let Some(free) = own
                .iter()
                .chain(pool.iter())
                .find(|chord| !resolved.values().any(|held| held == *chord))
                .cloned()
            {
                resolved.insert((action, ChordField::First), free);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::KeyModifiers;
    use crate::platform::test_support::TEST_PLATFORM as PLATFORM;
    use crate::settings::SettingChoice;

    fn chord(key: &str, modifiers: KeyModifiers) -> ComposingKeyChord {
        ComposingKeyChord::make(Some(key), modifiers, PLATFORM).unwrap()
    }

    /// First fields only — what every row stored before the second field.
    fn stored(
        pairs: &[(ComposingAction, Option<ComposingKeyChord>)],
    ) -> BTreeMap<(ComposingAction, ChordField), Option<ComposingKeyChord>> {
        pairs
            .iter()
            .map(|(action, chord)| ((*action, ChordField::First), chord.clone()))
            .collect()
    }

    fn stored_fields(
        entries: &[(ComposingAction, ChordField, Option<ComposingKeyChord>)],
    ) -> BTreeMap<(ComposingAction, ChordField), Option<ComposingKeyChord>> {
        entries
            .iter()
            .map(|(action, field, chord)| ((*action, *field), chord.clone()))
            .collect()
    }

    #[test]
    fn defaults_resolve_to_exactly_what_ships() {
        // trace: no stored row, no duplicate among the shipped chords → every
        // field resolves to `shipped_chord`; Output the Other Script starts
        // blank (USER 2026-10-15).
        let bindings = ComposingKeyBindings::default();
        for (action, field) in ComposingAction::all_fields() {
            assert_eq!(
                bindings.chord(action, field),
                action.shipped_chord(field).as_ref(),
                "{action:?} {field:?}"
            );
        }
        assert!(ChordField::ALL.iter().all(|field| bindings
            .chord(ComposingAction::CommitAlternateScript, *field)
            .is_none()));
        assert_eq!(bindings.tone_scheme, ToneInputScheme::Standard);
        assert_eq!(bindings.slot_key_set(), CandidateSlotKeySet::BareKeys);
    }

    #[test]
    fn a_stored_chord_outranks_a_default_that_arrives_on_top_of_it() {
        // trace: `remove_duplicates` drops the row on its default first — both roster orders.
        let bracket = chord("]", KeyModifiers::NONE);
        let bindings = ComposingKeyBindings::resolve(
            &stored(&[(ComposingAction::NextCandidate, Some(bracket.clone()))]),
            ToneInputScheme::Standard,
        );
        assert_eq!(
            bindings.chord(ComposingAction::NextCandidate, ChordField::First),
            Some(&bracket)
        );
        assert_eq!(
            bindings.chord(ComposingAction::PageForward, ChordField::First),
            None
        );

        let bindings = ComposingKeyBindings::resolve(
            &stored(&[(ComposingAction::PageBackward, Some(bracket.clone()))]),
            ToneInputScheme::Standard,
        );
        assert_eq!(
            bindings.chord(ComposingAction::PageBackward, ChordField::First),
            Some(&bracket)
        );
        assert_eq!(
            bindings.chord(ComposingAction::PageForward, ChordField::First),
            None
        );
    }

    #[test]
    fn a_user_who_recorded_space_on_walking_keeps_it() {
        // trace: next.first=Space (recorded) > confirm.second=Space (shipped)
        // → confirm.second dropped; confirm keeps Return on its first field.
        let space = chord(" ", KeyModifiers::NONE);
        let bindings = ComposingKeyBindings::resolve(
            &stored(&[(ComposingAction::NextCandidate, Some(space.clone()))]),
            ToneInputScheme::Standard,
        );
        assert_eq!(
            bindings.chord(ComposingAction::NextCandidate, ChordField::First),
            Some(&space)
        );
        assert_eq!(
            bindings.chord(ComposingAction::ConfirmHighlighted, ChordField::Second),
            None
        );
    }

    #[test]
    fn a_chord_recorded_twice_stays_on_the_last_action_only() {
        let recorded = chord("]", KeyModifiers::CONTROL);
        let bindings = ComposingKeyBindings::resolve(
            &stored(&[
                (ComposingAction::PageForward, Some(recorded.clone())),
                (ComposingAction::PageBackward, Some(recorded.clone())),
            ]),
            ToneInputScheme::Standard,
        );
        assert_eq!(
            bindings.chord(ComposingAction::PageBackward, ChordField::First),
            Some(&recorded)
        );
        assert_eq!(
            bindings.chord(ComposingAction::PageForward, ChordField::First),
            None
        );
    }

    #[test]
    fn commit_rows_get_their_default_back_when_cleared() {
        // trace: both fields cleared (Confirm ships Space on its second) →
        // the row is empty → its own first default is free → refilled.
        for action in ComposingAction::REFILLED_FROM_DEFAULT {
            let bindings = ComposingKeyBindings::resolve(
                &stored_fields(&[
                    (action, ChordField::First, None),
                    (action, ChordField::Second, None),
                ]),
                ToneInputScheme::Standard,
            );
            assert_eq!(
                bindings.chord(action, ChordField::First),
                action.shipped_chord(ChordField::First).as_ref()
            );
        }
    }

    /// The reported failure (USER 2026-09-19): Enter recorded on Output the Other Script
    /// was handed straight back to Confirm Key. The emptied commit row takes the
    /// pair's OTHER default when it is free and stays empty when it is not.
    #[test]
    fn a_row_recorded_onto_a_commit_default_keeps_it() {
        // trace: alternate=Return (recorded) > confirm=Return (default) →
        // confirm emptied; pool {Return, ⇧Return}: Return held by alternate,
        // ⇧Return by literal → confirm stays empty.
        let bindings = ComposingKeyBindings::resolve(
            &stored(&[
                (ComposingAction::ConfirmHighlighted, None),
                (
                    ComposingAction::CommitAlternateScript,
                    Some(chord("\r", KeyModifiers::NONE)),
                ),
            ]),
            ToneInputScheme::Standard,
        );
        assert_eq!(
            bindings.chord(ComposingAction::CommitAlternateScript, ChordField::First),
            Some(&chord("\r", KeyModifiers::NONE))
        );
        assert_eq!(
            bindings.chord(ComposingAction::ConfirmHighlighted, ChordField::First),
            None
        );
        assert_eq!(
            bindings.chord(ComposingAction::CommitLiteral, ChordField::First),
            Some(&chord("\r", KeyModifiers::SHIFT))
        );
        // trace: literal cleared, nextCandidate=⇧Return (recorded),
        // confirm=`]`; Return is free → literal takes Return.
        let refilled = ComposingKeyBindings::resolve(
            &stored(&[
                (ComposingAction::CommitLiteral, None),
                (
                    ComposingAction::NextCandidate,
                    Some(chord("\r", KeyModifiers::SHIFT)),
                ),
                (
                    ComposingAction::ConfirmHighlighted,
                    Some(chord("]", KeyModifiers::NONE)),
                ),
            ]),
            ToneInputScheme::Standard,
        );
        assert_eq!(
            refilled.chord(ComposingAction::NextCandidate, ChordField::First),
            Some(&chord("\r", KeyModifiers::SHIFT))
        );
        assert_eq!(
            refilled.chord(ComposingAction::CommitLiteral, ChordField::First),
            Some(&chord("\r", KeyModifiers::NONE))
        );
        // trace: both defaults recorded on ordinary rows → both commit rows
        // cleared by the pane, nothing free → both stay empty.
        let both_taken = ComposingKeyBindings::resolve(
            &stored(&[
                (ComposingAction::ConfirmHighlighted, None),
                (ComposingAction::CommitLiteral, None),
                (
                    ComposingAction::PageForward,
                    Some(chord("\r", KeyModifiers::NONE)),
                ),
                (
                    ComposingAction::PageBackward,
                    Some(chord("\r", KeyModifiers::SHIFT)),
                ),
            ]),
            ToneInputScheme::Standard,
        );
        assert_eq!(
            both_taken.chord(ComposingAction::ConfirmHighlighted, ChordField::First),
            None
        );
        assert_eq!(
            both_taken.chord(ComposingAction::CommitLiteral, ChordField::First),
            None
        );
        assert_eq!(
            both_taken.chord(ComposingAction::PageForward, ChordField::First),
            Some(&chord("\r", KeyModifiers::NONE))
        );
    }

    #[test]
    fn commit_rows_are_refilled_only_from_free_defaults_and_may_swap() {
        // Every arrangement of the
        // two commit rows over their two chords plus a third row holding one:
        // no chord on two rows, a recorded chord stays on its row, and a
        // commit row is empty only when both defaults are held elsewhere.
        let candidates = [
            None,
            Some(chord("\r", KeyModifiers::NONE)),
            Some(chord("\r", KeyModifiers::SHIFT)),
            Some(chord("]", KeyModifiers::NONE)),
        ];
        let pool: Vec<ComposingKeyChord> = ComposingAction::REFILLED_FROM_DEFAULT
            .iter()
            .filter_map(|action| action.shipped_chord(ChordField::First))
            .collect();
        let others: Vec<_> = ComposingAction::ALL
            .into_iter()
            .filter(|a| !ComposingAction::REFILLED_FROM_DEFAULT.contains(a))
            .collect();
        for confirm in &candidates {
            for literal in &candidates {
                for other in &others {
                    for other_chord in &candidates {
                        let bindings = ComposingKeyBindings::resolve(
                            &stored(&[
                                (ComposingAction::ConfirmHighlighted, confirm.clone()),
                                (ComposingAction::CommitLiteral, literal.clone()),
                                (*other, other_chord.clone()),
                            ]),
                            ToneInputScheme::Standard,
                        );
                        let held: Vec<&ComposingKeyChord> = bindings.chords.values().collect();
                        let distinct: std::collections::BTreeSet<_> = held.iter().collect();
                        assert_eq!(held.len(), distinct.len(), "one chord on two rows: {confirm:?} {literal:?} {other:?}={other_chord:?}");
                        // Two rows stored with one chord is a settings file
                        // edited behind the pane's back; that tiebreak is
                        // `remove_duplicates`', not this pass's.
                        if other_chord.is_some() && other_chord != confirm && other_chord != literal
                        {
                            assert_eq!(bindings.chord(*other, ChordField::First), other_chord.as_ref(), "recorded chord lost: {confirm:?} {literal:?} {other:?}={other_chord:?}");
                        }
                        // A row is empty only with both fields empty (Confirm
                        // keeps its shipped Space on the second).
                        if ComposingAction::REFILLED_FROM_DEFAULT.iter().any(|action| {
                            ChordField::ALL
                                .iter()
                                .all(|field| bindings.chord(*action, *field).is_none())
                        }) {
                            assert!(pool.iter().all(|c| held.contains(&c)), "a commit row left empty with a default free: {confirm:?} {literal:?} {other:?}={other_chord:?}");
                        }
                    }
                }
            }
        }
        let swapped = ComposingKeyBindings::resolve(
            &stored(&[
                (
                    ComposingAction::CommitLiteral,
                    Some(chord("\r", KeyModifiers::NONE)),
                ),
                (
                    ComposingAction::ConfirmHighlighted,
                    Some(chord("\r", KeyModifiers::SHIFT)),
                ),
            ]),
            ToneInputScheme::Standard,
        );
        assert_eq!(
            swapped.chord(ComposingAction::CommitLiteral, ChordField::First),
            Some(&chord("\r", KeyModifiers::NONE))
        );
        assert_eq!(
            swapped.chord(ComposingAction::ConfirmHighlighted, ChordField::First),
            Some(&chord("\r", KeyModifiers::SHIFT))
        );
    }

    #[test]
    fn a_recorded_chord_is_kept_under_either_scheme() {
        // trace: no slot-tier pass any more — the gate refuses every key a
        // slot could take, so Ctrl+3 is an ordinary chord under both schemes.
        let control_three = chord("3", KeyModifiers::CONTROL);
        for scheme in ToneInputScheme::ALL {
            let bindings = ComposingKeyBindings::resolve(
                &stored(&[(ComposingAction::PageForward, Some(control_three.clone()))]),
                *scheme,
            );
            assert_eq!(
                bindings.chord(ComposingAction::PageForward, ChordField::First),
                Some(&control_three),
                "{scheme:?}"
            );
            assert_eq!(bindings.slot_key_set(), scheme.slot_key_set());
        }
    }

    #[test]
    fn candidate_window_ships_on_and_reads_what_the_general_pane_writes() {
        // trace: S33 — `resolve` ships the window on; `from_document` reads
        // `IS_CANDIDATE_WINDOW_ENABLED`, the key the General pane writes.
        assert!(ComposingKeyBindings::default().is_candidate_window_enabled);
        let mut document = SettingsDocument::default();
        assert!(
            ComposingKeyBindings::from_document(&document, PLATFORM).is_candidate_window_enabled
        );
        document.set_bool(&keys::IS_CANDIDATE_WINDOW_ENABLED, false);
        assert!(
            !ComposingKeyBindings::from_document(&document, PLATFORM).is_candidate_window_enabled
        );
    }

    #[test]
    fn document_round_trip_reads_absent_cleared_and_recorded_rows() {
        let mut document = SettingsDocument::default();
        document.set_raw_string(
            &ComposingAction::PageForward.settings_key_name(ChordField::First),
            "",
        );
        document.set_raw_string(
            &ComposingAction::NextCandidate.settings_key_name(ChordField::First),
            &chord("]", KeyModifiers::CONTROL).raw_value(PLATFORM),
        );
        document.set_raw_string(
            &ComposingAction::PageBackward.settings_key_name(ChordField::First),
            "garbage",
        );
        document.set_choice(&keys::TONE_INPUT_SCHEME, ToneInputScheme::Telex);
        let bindings = ComposingKeyBindings::from_document(&document, PLATFORM);
        assert_eq!(
            bindings.chord(ComposingAction::PageForward, ChordField::First),
            None,
            "cleared"
        );
        assert_eq!(
            bindings.chord(ComposingAction::NextCandidate, ChordField::First),
            Some(&chord("]", KeyModifiers::CONTROL))
        );
        assert_eq!(
            bindings.chord(ComposingAction::PageBackward, ChordField::First),
            None,
            "unparsable reads as cleared"
        );
        assert_eq!(
            bindings.chord(ComposingAction::ConfirmHighlighted, ChordField::First),
            Some(&chord("\r", KeyModifiers::NONE)),
            "untouched"
        );
        assert_eq!(bindings.tone_scheme, ToneInputScheme::Telex);
        assert_eq!(bindings.slot_key_set(), CandidateSlotKeySet::Digits);
        let event = KeyEventSnapshot::chord(Some("\u{1D}"), "]", KeyModifiers::CONTROL);
        assert_eq!(
            bindings.action_for(&event, PLATFORM),
            Some(ComposingAction::NextCandidate)
        );
    }

    #[test]
    fn the_mac_reads_its_own_stored_letters_and_its_keypad_and_back_tab() {
        // trace: the Mac's grammar spells ⌥ `o` (`chord.rs` `modifier_letters`);
        // an `a` is not a Mac letter, so that row reads as cleared (inventory K2).
        let mac = DesktopPlatform::MacOS;
        let mut document = SettingsDocument::default();
        document.set_raw_string(
            &ComposingAction::PageForward.settings_key_name(ChordField::First),
            "o|000D",
        );
        document.set_raw_string(
            &ComposingAction::NextCandidate.settings_key_name(ChordField::First),
            "a|005D",
        );
        let bindings = ComposingKeyBindings::from_document(&document, mac);
        let option_return = ComposingKeyChord::make(Some("\r"), KeyModifiers::ALT, mac).unwrap();
        assert_eq!(
            bindings.chord(ComposingAction::PageForward, ChordField::First),
            Some(&option_return)
        );
        assert_eq!(
            bindings.chord(ComposingAction::NextCandidate, ChordField::First),
            None
        );
        assert_eq!(
            bindings.action_for(&KeyEventSnapshot::text("\r", KeyModifiers::ALT), mac),
            Some(ComposingAction::PageForward)
        );
        // trace: `normalized` folds `\u{3}` → `\r` and `\u{19}` → `\t` — out of the box the
        // keypad Enter confirms and AppKit's back tab walks back.
        let defaults = ComposingKeyBindings::default();
        assert_eq!(
            defaults.action_for(&KeyEventSnapshot::text("\u{3}", KeyModifiers::NONE), mac),
            Some(ComposingAction::ConfirmHighlighted)
        );
        assert_eq!(
            defaults.action_for(&KeyEventSnapshot::text("\u{19}", KeyModifiers::SHIFT), mac),
            Some(ComposingAction::PreviousCandidate)
        );
    }

    /// The reported setup (Discord #issues): Space and Enter both confirm in
    /// the current script, Shift+Enter commits the other one.
    #[test]
    fn a_second_field_fires_its_action_alongside_the_first() {
        // trace: confirm.second=Space (its shipped chord, stored by value);
        // alternate.first=⇧Return (recorded) > literal.first=⇧Return
        // (default) → literal.first dropped; literal is a commit row with no
        // chord → pool {Return, ⇧Return} both held → literal stays empty.
        // A `.second` stored on any other action is no field and is not read.
        let space = chord(" ", KeyModifiers::NONE);
        let enter = chord("\r", KeyModifiers::NONE);
        let shift_enter = chord("\r", KeyModifiers::SHIFT);
        let bindings = ComposingKeyBindings::resolve(
            &stored_fields(&[
                (
                    ComposingAction::ConfirmHighlighted,
                    ChordField::Second,
                    Some(space.clone()),
                ),
                (
                    ComposingAction::CommitAlternateScript,
                    ChordField::First,
                    Some(shift_enter.clone()),
                ),
                (
                    ComposingAction::PageForward,
                    ChordField::Second,
                    Some(chord("k", KeyModifiers::CONTROL)),
                ),
            ]),
            ToneInputScheme::Standard,
        );
        let confirm = ComposingAction::ConfirmHighlighted;
        assert_eq!(bindings.chord(confirm, ChordField::First), Some(&enter));
        assert_eq!(bindings.chord(confirm, ChordField::Second), Some(&space));
        let alternate = ComposingAction::CommitAlternateScript;
        assert_eq!(
            bindings.chord(alternate, ChordField::First),
            Some(&shift_enter)
        );
        assert_eq!(
            bindings.chord(ComposingAction::PageForward, ChordField::Second),
            None
        );
        for field in ChordField::ALL {
            assert_eq!(bindings.chord(ComposingAction::CommitLiteral, field), None);
        }
        let press = |key: &str, modifiers| {
            bindings.action_for(&KeyEventSnapshot::text(key, modifiers), PLATFORM)
        };
        assert_eq!(press(" ", KeyModifiers::NONE), Some(confirm));
        assert_eq!(press("\r", KeyModifiers::NONE), Some(confirm));
        assert_eq!(press("\r", KeyModifiers::SHIFT), Some(alternate));
        assert_eq!(press("k", KeyModifiers::CONTROL), None);
    }

    #[test]
    fn a_chord_on_both_fields_of_one_action_stays_on_the_second() {
        let recorded = chord("]", KeyModifiers::CONTROL);
        let bindings = ComposingKeyBindings::resolve(
            &stored_fields(&[
                (
                    ComposingAction::ConfirmHighlighted,
                    ChordField::First,
                    Some(recorded.clone()),
                ),
                (
                    ComposingAction::ConfirmHighlighted,
                    ChordField::Second,
                    Some(recorded.clone()),
                ),
            ]),
            ToneInputScheme::Standard,
        );
        assert_eq!(
            bindings.chord(ComposingAction::ConfirmHighlighted, ChordField::First),
            None
        );
        assert_eq!(
            bindings.chord(ComposingAction::ConfirmHighlighted, ChordField::Second),
            Some(&recorded)
        );
    }

    #[test]
    fn a_commit_row_holding_a_second_chord_is_not_refilled() {
        // trace: confirm.first cleared, confirm.second=Space → the row is not
        // empty, so `restore_unbound` skips it and Return stays unbound.
        let space = chord(" ", KeyModifiers::NONE);
        let bindings = ComposingKeyBindings::resolve(
            &stored_fields(&[
                (ComposingAction::ConfirmHighlighted, ChordField::First, None),
                (
                    ComposingAction::ConfirmHighlighted,
                    ChordField::Second,
                    Some(space.clone()),
                ),
            ]),
            ToneInputScheme::Standard,
        );
        let confirm = ComposingAction::ConfirmHighlighted;
        assert_eq!(bindings.chord(confirm, ChordField::First), None);
        assert_eq!(bindings.chord(confirm, ChordField::Second), Some(&space));
        assert_eq!(
            bindings.action_for(&KeyEventSnapshot::text("\r", KeyModifiers::NONE), PLATFORM),
            None
        );
    }

    #[test]
    fn an_emptied_commit_row_is_refilled_onto_its_first_field() {
        // trace: confirm.first cleared, confirm.second stored "" (cleared) →
        // both fields empty → Return is free → confirm.first=Return.
        let bindings = ComposingKeyBindings::resolve(
            &stored_fields(&[
                (ComposingAction::ConfirmHighlighted, ChordField::First, None),
                (
                    ComposingAction::ConfirmHighlighted,
                    ChordField::Second,
                    None,
                ),
            ]),
            ToneInputScheme::Standard,
        );
        assert_eq!(
            bindings.chord(ComposingAction::ConfirmHighlighted, ChordField::First),
            Some(&chord("\r", KeyModifiers::NONE))
        );
        assert_eq!(
            bindings.chord(ComposingAction::ConfirmHighlighted, ChordField::Second),
            None
        );
    }

    #[test]
    fn document_reads_the_second_field_and_an_empty_one_as_unset() {
        let mut document = SettingsDocument::default();
        document.set_raw_string(
            &ComposingAction::ConfirmHighlighted.settings_key_name(ChordField::Second),
            &chord(" ", KeyModifiers::NONE).raw_value(PLATFORM),
        );
        document.set_raw_string(
            &ComposingAction::PageForward.settings_key_name(ChordField::Second),
            "",
        );
        let bindings = ComposingKeyBindings::from_document(&document, PLATFORM);
        assert_eq!(
            bindings.chord(ComposingAction::ConfirmHighlighted, ChordField::Second),
            Some(&chord(" ", KeyModifiers::NONE))
        );
        assert_eq!(
            bindings.chord(ComposingAction::PageForward, ChordField::Second),
            None
        );
        assert_eq!(
            bindings.chord(ComposingAction::PageForward, ChordField::First),
            Some(&chord("]", KeyModifiers::NONE)),
            "an empty second field leaves the first on its default"
        );
    }

    #[test]
    fn under_tps_the_keypad_picks_whatever_the_tone_scheme() {
        let mut document = SettingsDocument::default();
        document.set_choice(&keys::INPUT_MODE, InputMode::Tps);
        for scheme in ToneInputScheme::ALL {
            document.set_choice(&keys::TONE_INPUT_SCHEME, *scheme);
            let bindings = ComposingKeyBindings::from_document(&document, DesktopPlatform::Windows);
            assert_eq!(bindings.input_mode, InputMode::Tps);
            assert_eq!(bindings.slot_key_set(), CandidateSlotKeySet::TpsDigits);
        }
    }
}
