//! The user's composing key contract, resolved and ready to classify against.
//! Port of `ComposingKeyBindings` (`ComposingKeyBindings.swift:109-262`).

use std::collections::BTreeMap;

use super::action::ComposingAction;
use super::chord::ComposingKeyChord;
use super::slot_key_set::CandidateSlotKeySet;
use super::snapshot::KeyEventSnapshot;
use super::tone_input_scheme::ToneInputScheme;
use crate::settings::{keys, SettingsDocument};

/// "Resolved" means three things have already happened, so the classifier
/// can trust the value: every chord came through [`ComposingKeyChord::make`];
/// no chord is on two actions (a later recording takes it from the earlier
/// one); an empty [`ComposingAction::REFILLED_FROM_DEFAULT`] row takes back
/// whichever of the pair's defaults no other row holds — never a key off
/// another row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComposingKeyBindings {
    chords: BTreeMap<ComposingAction, ComposingKeyChord>,
    /// Which keys type a tone. Carried here because it is the other half of
    /// the same contract the chords are: the classifier reads both off one
    /// value, and the slot keys follow from it.
    pub tone_scheme: ToneInputScheme,
    /// Whether a candidate window exists to act on. Carried here for the
    /// same reason as `tone_scheme`: with the window off, the keys that
    /// would confirm or page a candidate end the composition as typed
    /// instead, and the classifier decides that off one value
    /// (`ComposingKeyBindings.swift` `isCandidateWindowEnabled`).
    pub is_candidate_window_enabled: bool,
}

impl Default for ComposingKeyBindings {
    /// What a fresh install types with.
    fn default() -> Self {
        Self::resolve(&BTreeMap::new(), ToneInputScheme::Standard)
    }
}

impl ComposingKeyBindings {
    /// `stored` maps an action to `Some(chord)` (recorded) or `None` (the
    /// user cleared the row); an action absent from the map was never
    /// touched and reads as its default. The double optional is what tells
    /// the two apart.
    pub fn resolve(
        stored: &BTreeMap<ComposingAction, Option<ComposingKeyChord>>,
        tone_scheme: ToneInputScheme,
    ) -> Self {
        let mut resolved: BTreeMap<ComposingAction, ComposingKeyChord> = BTreeMap::new();
        for action in ComposingAction::ALL {
            let chord = match stored.get(&action) {
                Some(Some(chord)) => Some(chord.clone()),
                Some(None) => None,
                None => Some(action.default_chord()),
            };
            if let Some(chord) = chord {
                resolved.insert(action, chord);
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
            // Resolving is about the chords and the scheme; the window
            // switch is orthogonal, so it starts as shipped and a caller
            // with an opinion sets it (`from_document`).
            is_candidate_window_enabled: true,
        }
    }

    /// The bindings a settings document describes: `composingShortcut.<raw>`
    /// per action (absent = default, `""` = cleared, unparsable = cleared —
    /// silently restoring the default would undo a deliberate clearing) plus
    /// the tone scheme (`SettingsStore.swift` `composingKeyBindings`).
    pub fn from_document(document: &SettingsDocument) -> Self {
        let mut stored = BTreeMap::new();
        for action in ComposingAction::ALL {
            if let Some(raw) = document.raw_string(&action.settings_key_name()) {
                stored.insert(action, ComposingKeyChord::from_raw(raw));
            }
        }
        Self {
            is_candidate_window_enabled: document.bool(&keys::IS_CANDIDATE_WINDOW_ENABLED),
            ..Self::resolve(&stored, document.choice(&keys::TONE_INPUT_SCHEME))
        }
    }

    /// The keys that pick a candidate — derived, never stored
    /// (`ToneInputScheme::slot_key_set`).
    pub fn slot_key_set(&self) -> CandidateSlotKeySet {
        self.tone_scheme.slot_key_set()
    }

    /// The chord on `action`, or `None` when the row is empty.
    pub fn chord(&self, action: ComposingAction) -> Option<&ComposingKeyChord> {
        self.chords.get(&action)
    }

    /// The action `event` is bound to, if any. Linear over the roster: seven
    /// entries, and a chord-keyed map would need the duplicate handling twice.
    pub fn action_for(&self, event: &KeyEventSnapshot) -> Option<ComposingAction> {
        ComposingAction::ALL.into_iter().find(|action| {
            self.chords
                .get(action)
                .is_some_and(|chord| chord.matches(event))
        })
    }

    /// Which other actions currently hold `chord` — the pure half of
    /// recording, so the row that loses a chord empties in front of the user.
    pub fn actions_holding(
        &self,
        chord: &ComposingKeyChord,
        excluding: Option<ComposingAction>,
    ) -> Vec<ComposingAction> {
        ComposingAction::ALL
            .into_iter()
            .filter(|action| Some(*action) != excluding && self.chords.get(action) == Some(chord))
            .collect()
    }

    /// Drops a chord from every action but the last one holding it, with the
    /// rows still on their own default going first — so a recorded chord
    /// outranks a default that arrives on top of it in an upgrade. Within
    /// each half, roster order is the tiebreak.
    ///
    /// "On its default" is judged by VALUE, exactly as macOS does
    /// (`ComposingKeyBindings.swift:212`): a row the user explicitly recorded
    /// back onto its own default reads as untouched here. The store keeps the
    /// distinction (absent vs stored), the resolver deliberately does not —
    /// same observable behaviour on both desktops.
    fn remove_duplicates(resolved: &mut BTreeMap<ComposingAction, ComposingKeyChord>) {
        let on_its_default =
            |action: &ComposingAction| resolved.get(action) == Some(&action.default_chord());
        let order: Vec<ComposingAction> = ComposingAction::ALL
            .iter()
            .copied()
            .filter(on_its_default)
            .chain(
                ComposingAction::ALL
                    .iter()
                    .copied()
                    .filter(|action| !on_its_default(action)),
            )
            .collect();
        let mut seen: BTreeMap<ComposingKeyChord, ComposingAction> = BTreeMap::new();
        for action in order {
            let Some(chord) = resolved.get(&action).cloned() else {
                continue;
            };
            if let Some(earlier) = seen.insert(chord, action) {
                resolved.remove(&earlier);
            }
        }
    }

    /// Refills an empty commit row from the pair's shipped defaults: whichever
    /// pool chord no composing row holds, its own first. A cleared row gets
    /// its key back; a row holding a pool chord the user recorded there keeps
    /// it, and the emptied commit
    /// row stays empty — the last recording wins here as everywhere else on
    /// the pane (USER 2026-09-19: Enter recorded on Output the Other Script used to be handed
    /// straight back to Confirm Key). An unbound Return still ends the composition:
    /// it commits before passing through to the host
    /// (`ComposingKeyIntent::host_key`).
    fn restore_unbound(resolved: &mut BTreeMap<ComposingAction, ComposingKeyChord>) {
        let pool: Vec<ComposingKeyChord> = ComposingAction::REFILLED_FROM_DEFAULT
            .iter()
            .map(|action| action.default_chord())
            .collect();
        for action in ComposingAction::REFILLED_FROM_DEFAULT {
            if resolved.contains_key(&action) {
                continue;
            }
            let own = action.default_chord();
            if let Some(free) = std::iter::once(&own)
                .chain(pool.iter())
                .find(|chord| !resolved.values().any(|held| held == *chord))
                .cloned()
            {
                resolved.insert(action, free);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::KeyModifiers;
    use crate::settings::SettingChoice;

    fn chord(key: &str, modifiers: KeyModifiers) -> ComposingKeyChord {
        ComposingKeyChord::make(Some(key), modifiers).unwrap()
    }

    fn stored(
        pairs: &[(ComposingAction, Option<ComposingKeyChord>)],
    ) -> BTreeMap<ComposingAction, Option<ComposingKeyChord>> {
        pairs.iter().cloned().collect()
    }

    #[test]
    fn defaults_leave_no_action_unbound() {
        let bindings = ComposingKeyBindings::default();
        for action in ComposingAction::ALL {
            assert!(bindings.chord(action).is_some(), "{action:?} starts blank");
        }
        assert_eq!(bindings.tone_scheme, ToneInputScheme::Standard);
        assert_eq!(bindings.slot_key_set(), CandidateSlotKeySet::BareKeys);
    }

    #[test]
    fn a_stored_chord_outranks_a_default_that_arrives_on_top_of_it() {
        // trace: ComposingKeyBindingsTests.swift:229-249 — both roster orders.
        let bracket = ComposingAction::PageForward.default_chord();
        let bindings = ComposingKeyBindings::resolve(
            &stored(&[(ComposingAction::NextCandidate, Some(bracket.clone()))]),
            ToneInputScheme::Standard,
        );
        assert_eq!(
            bindings.chord(ComposingAction::NextCandidate),
            Some(&bracket)
        );
        assert_eq!(bindings.chord(ComposingAction::PageForward), None);

        let bindings = ComposingKeyBindings::resolve(
            &stored(&[(ComposingAction::PageBackward, Some(bracket.clone()))]),
            ToneInputScheme::Standard,
        );
        assert_eq!(
            bindings.chord(ComposingAction::PageBackward),
            Some(&bracket)
        );
        assert_eq!(bindings.chord(ComposingAction::PageForward), None);
    }

    #[test]
    fn a_user_who_recorded_space_on_walking_keeps_it() {
        let space = ComposingAction::CommitAlternateScript.default_chord();
        let bindings = ComposingKeyBindings::resolve(
            &stored(&[(ComposingAction::NextCandidate, Some(space.clone()))]),
            ToneInputScheme::Standard,
        );
        assert_eq!(bindings.chord(ComposingAction::NextCandidate), Some(&space));
        assert_eq!(bindings.chord(ComposingAction::CommitAlternateScript), None);
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
            bindings.chord(ComposingAction::PageBackward),
            Some(&recorded)
        );
        assert_eq!(bindings.chord(ComposingAction::PageForward), None);
    }

    #[test]
    fn commit_rows_get_their_default_back_when_cleared() {
        for action in ComposingAction::REFILLED_FROM_DEFAULT {
            let bindings = ComposingKeyBindings::resolve(
                &stored(&[(action, None)]),
                ToneInputScheme::Standard,
            );
            assert_eq!(bindings.chord(action), Some(&action.default_chord()));
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
            bindings.chord(ComposingAction::CommitAlternateScript),
            Some(&chord("\r", KeyModifiers::NONE))
        );
        assert_eq!(bindings.chord(ComposingAction::ConfirmHighlighted), None);
        assert_eq!(
            bindings.chord(ComposingAction::CommitLiteral),
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
            refilled.chord(ComposingAction::NextCandidate),
            Some(&chord("\r", KeyModifiers::SHIFT))
        );
        assert_eq!(
            refilled.chord(ComposingAction::CommitLiteral),
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
        assert_eq!(both_taken.chord(ComposingAction::ConfirmHighlighted), None);
        assert_eq!(both_taken.chord(ComposingAction::CommitLiteral), None);
        assert_eq!(
            both_taken.chord(ComposingAction::PageForward),
            Some(&chord("\r", KeyModifiers::NONE))
        );
    }

    #[test]
    fn commit_rows_are_refilled_only_from_free_defaults_and_may_swap() {
        // trace: ComposingKeyBindingsTests.swift — every arrangement of the
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
            .map(|action| action.default_chord())
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
                            assert_eq!(bindings.chord(*other), other_chord.as_ref(), "recorded chord lost: {confirm:?} {literal:?} {other:?}={other_chord:?}");
                        }
                        if ComposingAction::REFILLED_FROM_DEFAULT
                            .iter()
                            .any(|action| bindings.chord(*action).is_none())
                        {
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
            swapped.chord(ComposingAction::CommitLiteral),
            Some(&chord("\r", KeyModifiers::NONE))
        );
        assert_eq!(
            swapped.chord(ComposingAction::ConfirmHighlighted),
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
                bindings.chord(ComposingAction::PageForward),
                Some(&control_three),
                "{scheme:?}"
            );
            assert_eq!(bindings.slot_key_set(), scheme.slot_key_set());
        }
    }

    #[test]
    fn candidate_window_ships_on_and_reads_what_the_general_pane_writes() {
        // trace: SettingsStoreTests.swift
        // `testCandidateWindow_shipsOn_andReadsWhatTheGeneralPaneWrites`.
        assert!(ComposingKeyBindings::default().is_candidate_window_enabled);
        let mut document = SettingsDocument::default();
        assert!(ComposingKeyBindings::from_document(&document).is_candidate_window_enabled);
        document.set_bool(&keys::IS_CANDIDATE_WINDOW_ENABLED, false);
        assert!(!ComposingKeyBindings::from_document(&document).is_candidate_window_enabled);
    }

    #[test]
    fn document_round_trip_reads_absent_cleared_and_recorded_rows() {
        let mut document = SettingsDocument::default();
        document.set_raw_string(&ComposingAction::PageForward.settings_key_name(), "");
        document.set_raw_string(
            &ComposingAction::NextCandidate.settings_key_name(),
            &chord("]", KeyModifiers::CONTROL).raw_value(),
        );
        document.set_raw_string(
            &ComposingAction::PageBackward.settings_key_name(),
            "garbage",
        );
        document.set_choice(&keys::TONE_INPUT_SCHEME, ToneInputScheme::Telex);
        let bindings = ComposingKeyBindings::from_document(&document);
        assert_eq!(
            bindings.chord(ComposingAction::PageForward),
            None,
            "cleared"
        );
        assert_eq!(
            bindings.chord(ComposingAction::NextCandidate),
            Some(&chord("]", KeyModifiers::CONTROL))
        );
        assert_eq!(
            bindings.chord(ComposingAction::PageBackward),
            None,
            "unparsable reads as cleared"
        );
        assert_eq!(
            bindings.chord(ComposingAction::ConfirmHighlighted),
            Some(&chord("\r", KeyModifiers::NONE)),
            "untouched"
        );
        assert_eq!(bindings.tone_scheme, ToneInputScheme::Telex);
        assert_eq!(bindings.slot_key_set(), CandidateSlotKeySet::Digits);
        let event = KeyEventSnapshot::chord(Some("\u{1D}"), "]", KeyModifiers::CONTROL);
        assert_eq!(
            bindings.action_for(&event),
            Some(ComposingAction::NextCandidate)
        );
        assert_eq!(
            bindings.actions_holding(
                &chord("]", KeyModifiers::CONTROL),
                Some(ComposingAction::PageForward)
            ),
            vec![ComposingAction::NextCandidate]
        );
    }
}
