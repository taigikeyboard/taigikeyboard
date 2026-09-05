//! The user's composing key contract, resolved and ready to classify against.
//! Port of `ComposingKeyBindings` (`ComposingKeyBindings.swift:109-262`).

use std::collections::BTreeMap;

use super::action::ComposingAction;
use super::chord::ComposingKeyChord;
use super::slot_key_set::CandidateSlotKeySet;
use super::snapshot::KeyEventSnapshot;
use crate::settings::{keys, SettingsDocument};

/// "Resolved" means three things have already happened, so the classifier
/// can trust the value: every chord came through [`ComposingKeyChord::make`];
/// no chord is on two actions (a later recording takes it from the earlier
/// one); [`ComposingAction::ALWAYS_BOUND`] is honoured.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComposingKeyBindings {
    chords: BTreeMap<ComposingAction, ComposingKeyChord>,
    pub slot_key_set: CandidateSlotKeySet,
}

impl Default for ComposingKeyBindings {
    /// What a fresh install types with.
    fn default() -> Self {
        Self::resolve(&BTreeMap::new(), CandidateSlotKeySet::BareKeys)
    }
}

impl ComposingKeyBindings {
    /// `stored` maps an action to `Some(chord)` (recorded) or `None` (the
    /// user cleared the row); an action absent from the map was never
    /// touched and reads as its default. The double optional is what tells
    /// the two apart.
    pub fn resolve(
        stored: &BTreeMap<ComposingAction, Option<ComposingKeyChord>>,
        slot_key_set: CandidateSlotKeySet,
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
        Self::remove_shadowed_by_candidate_slots(&mut resolved, slot_key_set);
        Self::remove_duplicates(&mut resolved);
        Self::restore_unbound(&mut resolved);
        Self {
            chords: resolved,
            slot_key_set,
        }
    }

    /// The bindings a settings document describes: `composingShortcut.<raw>`
    /// per action (absent = default, `""` = cleared, unparsable = cleared —
    /// silently restoring the default would undo a deliberate clearing) plus
    /// the slot-key set (`SettingsStore.swift:414-434`).
    pub fn from_document(document: &SettingsDocument) -> Self {
        let mut stored = BTreeMap::new();
        for action in ComposingAction::ALL {
            if let Some(raw) = document.raw_string(&action.settings_key_name()) {
                stored.insert(action, ComposingKeyChord::from_raw(raw));
            }
        }
        Self::resolve(&stored, document.choice(&keys::CANDIDATE_SLOT_MODIFIER))
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

    /// Drops a chord the candidate-slot tier would swallow before any binding
    /// is looked at. Dropped from the resolved value, not from storage: the
    /// row comes back if the picker moves off the set that shadowed it.
    fn remove_shadowed_by_candidate_slots(
        resolved: &mut BTreeMap<ComposingAction, ComposingKeyChord>,
        slot_key_set: CandidateSlotKeySet,
    ) {
        resolved.retain(|_, chord| !chord.is_candidate_slot_chord(slot_key_set));
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

    /// Keeps every always-bound action reachable. Their default chords are a
    /// pool the always-bound actions share, and no other action may hold one;
    /// an empty always-bound row takes whichever pool chord nobody else in the
    /// pool has — which lets the two SWAP and makes this terminate.
    fn restore_unbound(resolved: &mut BTreeMap<ComposingAction, ComposingKeyChord>) {
        let pool: Vec<ComposingKeyChord> = ComposingAction::ALWAYS_BOUND
            .iter()
            .map(|action| action.default_chord())
            .collect();
        resolved.retain(|action, chord| {
            ComposingAction::ALWAYS_BOUND.contains(action) || !pool.contains(chord)
        });
        let mut taken: Vec<ComposingKeyChord> = ComposingAction::ALWAYS_BOUND
            .iter()
            .filter_map(|action| resolved.get(action).cloned())
            .collect();
        for action in ComposingAction::ALWAYS_BOUND {
            if resolved.contains_key(&action) {
                continue;
            }
            if let Some(free) = pool.iter().find(|chord| !taken.contains(chord)).cloned() {
                taken.push(free.clone());
                resolved.insert(action, free);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::KeyModifiers;

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
        assert_eq!(bindings.slot_key_set, CandidateSlotKeySet::BareKeys);
    }

    #[test]
    fn a_stored_chord_outranks_a_default_that_arrives_on_top_of_it() {
        // trace: ComposingKeyBindingsTests.swift:229-249 — both roster orders.
        let bracket = ComposingAction::PageForward.default_chord();
        let bindings = ComposingKeyBindings::resolve(
            &stored(&[(ComposingAction::NextCandidate, Some(bracket.clone()))]),
            CandidateSlotKeySet::BareKeys,
        );
        assert_eq!(
            bindings.chord(ComposingAction::NextCandidate),
            Some(&bracket)
        );
        assert_eq!(bindings.chord(ComposingAction::PageForward), None);

        let bindings = ComposingKeyBindings::resolve(
            &stored(&[(ComposingAction::PageBackward, Some(bracket.clone()))]),
            CandidateSlotKeySet::BareKeys,
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
            CandidateSlotKeySet::BareKeys,
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
            CandidateSlotKeySet::BareKeys,
        );
        assert_eq!(
            bindings.chord(ComposingAction::PageBackward),
            Some(&recorded)
        );
        assert_eq!(bindings.chord(ComposingAction::PageForward), None);
    }

    #[test]
    fn always_bound_actions_get_their_default_back_and_take_it_from_others() {
        for action in ComposingAction::ALWAYS_BOUND {
            let bindings = ComposingKeyBindings::resolve(
                &stored(&[(action, None)]),
                CandidateSlotKeySet::BareKeys,
            );
            assert_eq!(bindings.chord(action), Some(&action.default_chord()));
        }
        let bindings = ComposingKeyBindings::resolve(
            &stored(&[
                (ComposingAction::ConfirmHighlighted, None),
                (
                    ComposingAction::PageForward,
                    Some(chord("\r", KeyModifiers::NONE)),
                ),
            ]),
            CandidateSlotKeySet::BareKeys,
        );
        assert_eq!(
            bindings.chord(ComposingAction::ConfirmHighlighted),
            Some(&chord("\r", KeyModifiers::NONE))
        );
        assert_eq!(bindings.chord(ComposingAction::PageForward), None);
        let bindings = ComposingKeyBindings::resolve(
            &stored(&[(
                ComposingAction::NextCandidate,
                Some(chord("\r", KeyModifiers::SHIFT)),
            )]),
            CandidateSlotKeySet::BareKeys,
        );
        assert_eq!(bindings.chord(ComposingAction::NextCandidate), None);
        assert_eq!(
            bindings.chord(ComposingAction::CommitLiteral),
            Some(&chord("\r", KeyModifiers::SHIFT))
        );
    }

    #[test]
    fn always_bound_actions_are_never_both_left_unbound_and_may_swap() {
        // trace: ComposingKeyBindingsTests.swift:294-336 — every arrangement.
        let candidates = [
            None,
            Some(chord("\r", KeyModifiers::NONE)),
            Some(chord("\r", KeyModifiers::SHIFT)),
            Some(chord("]", KeyModifiers::NONE)),
        ];
        let others: Vec<_> = ComposingAction::ALL
            .into_iter()
            .filter(|a| !ComposingAction::ALWAYS_BOUND.contains(a))
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
                            CandidateSlotKeySet::BareKeys,
                        );
                        for action in ComposingAction::ALWAYS_BOUND {
                            assert!(bindings.chord(action).is_some(), "{action:?} unbound: {confirm:?} {literal:?} {other:?}={other_chord:?}");
                        }
                        assert_ne!(
                            bindings.chord(ComposingAction::ConfirmHighlighted),
                            bindings.chord(ComposingAction::CommitLiteral)
                        );
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
            CandidateSlotKeySet::BareKeys,
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
    fn a_chord_the_slot_tier_would_swallow_is_dropped_only_under_that_set() {
        let control_three = chord("3", KeyModifiers::CONTROL);
        let under_control = ComposingKeyBindings::resolve(
            &stored(&[(ComposingAction::PageForward, Some(control_three.clone()))]),
            CandidateSlotKeySet::Control,
        );
        assert_eq!(under_control.chord(ComposingAction::PageForward), None);
        let under_option = ComposingKeyBindings::resolve(
            &stored(&[(ComposingAction::PageForward, Some(control_three.clone()))]),
            CandidateSlotKeySet::Option,
        );
        assert_eq!(
            under_option.chord(ComposingAction::PageForward),
            Some(&control_three)
        );

        let bare_z = stored(&[(
            ComposingAction::CommitAlternateScript,
            Some(chord("z", KeyModifiers::NONE)),
        )]);
        assert_eq!(
            ComposingKeyBindings::resolve(&bare_z, CandidateSlotKeySet::BareKeys)
                .chord(ComposingAction::CommitAlternateScript),
            None
        );
        assert_eq!(
            ComposingKeyBindings::resolve(&bare_z, CandidateSlotKeySet::Control)
                .chord(ComposingAction::CommitAlternateScript),
            Some(&chord("z", KeyModifiers::NONE))
        );
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
        document.set_choice(&keys::CANDIDATE_SLOT_MODIFIER, CandidateSlotKeySet::Control);
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
        assert_eq!(bindings.slot_key_set, CandidateSlotKeySet::Control);
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
