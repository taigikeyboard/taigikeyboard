//! The composing actions a user can put on a key of their own choosing; default keys follow the
//! system Zhuyin IME.

use super::chord::ComposingKeyChord;
use super::intent::{CandidateNavigation, ComposingKeyIntent};
use super::snapshot::KeyModifiers;
use crate::strings::StringKey;

/// One of the two chords a composing action can hold — the settings row's two
/// recorder fields. Both fire the action alike; only `First` has a shipped
/// default (Discord #issues: Space and Enter both confirming).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ChordField {
    First,
    Second,
}

impl ChordField {
    pub const ALL: [ChordField; 2] = [Self::First, Self::Second];
}

/// One thing the input method does while a composition is running, as the
/// settings window names it. Only the actions whose key is the user's to
/// choose are here; the fixed navigation tier and the slot keys are not.
/// No action ships unbound (USER 2026-08-21).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ComposingAction {
    /// Moves the highlight one candidate along. Tab.
    NextCandidate,
    /// Moves the highlight one candidate back. Shift+Tab (McBopomofo).
    PreviousCandidate,
    /// Shows the next page. `]`, as in the system candidate window.
    PageForward,
    /// Shows the previous page. `[`.
    PageBackward,
    /// Commits the highlighted candidate as the settings write it. Enter.
    ConfirmHighlighted,
    /// Commits the romanization exactly as typed. Shift+Enter — the escape
    /// hatch, one of the two actions that may never be left unbound.
    CommitLiteral,
    /// Commits the highlighted candidate in the OTHER script — the Hanji/romanization key.
    /// Space (rime-phah-taibun binds the same gesture on `\`).
    CommitAlternateScript,
}

impl ComposingAction {
    /// Roster order — the order `allCases` has on macOS, which is also the
    /// tiebreak order the bindings resolver reads.
    pub const ALL: [ComposingAction; 7] = [
        Self::NextCandidate,
        Self::PreviousCandidate,
        Self::PageForward,
        Self::PageBackward,
        Self::ConfirmHighlighted,
        Self::CommitLiteral,
        Self::CommitAlternateScript,
    ];

    /// The roster split into the groups the settings pane draws: the keys
    /// that move through the candidates, and the keys that end the
    /// composition. Written out so adding a case has to say where it goes.
    pub const GROUPS: [&'static [ComposingAction]; 2] = [
        &[
            Self::NextCandidate,
            Self::PreviousCandidate,
            Self::PageForward,
            Self::PageBackward,
        ],
        &[
            Self::ConfirmHighlighted,
            Self::CommitLiteral,
            Self::CommitAlternateScript,
        ],
    ];

    /// Actions whose row, when empty, is refilled from the pair's shipped
    /// defaults: the two ways to end a composition into the document.
    /// Refilled only with a default no other composing row holds
    /// (`ComposingKeyBindings::restore_unbound`).
    pub const REFILLED_FROM_DEFAULT: [ComposingAction; 2] =
        [Self::ConfirmHighlighted, Self::CommitLiteral];

    /// The persisted raw name (`ComposingAction.rawValue`).
    pub fn raw(self) -> &'static str {
        match self {
            Self::NextCandidate => "nextCandidate",
            Self::PreviousCandidate => "previousCandidate",
            Self::PageForward => "pageForward",
            Self::PageBackward => "pageBackward",
            Self::ConfirmHighlighted => "confirmHighlighted",
            Self::CommitLiteral => "commitLiteral",
            Self::CommitAlternateScript => "commitAlternateScript",
        }
    }

    pub fn from_raw(raw: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|action| action.raw() == raw)
    }

    /// The chord a fresh install has on this action — the same on every
    /// desktop. Built as the value the gate
    /// would produce rather than through it, since the gate is per desktop;
    /// `defaults_pass_the_gate_on_every_desktop` pins that it lets each one
    /// through unchanged.
    pub fn default_chord(self) -> ComposingKeyChord {
        let (key, modifiers) = match self {
            Self::NextCandidate => ("\t", KeyModifiers::NONE),
            Self::PreviousCandidate => ("\t", KeyModifiers::SHIFT),
            Self::PageForward => ("]", KeyModifiers::NONE),
            Self::PageBackward => ("[", KeyModifiers::NONE),
            Self::ConfirmHighlighted => ("\r", KeyModifiers::NONE),
            Self::CommitLiteral => ("\r", KeyModifiers::SHIFT),
            Self::CommitAlternateScript => (" ", KeyModifiers::NONE),
        };
        ComposingKeyChord {
            key: key.to_owned(),
            modifiers,
        }
    }

    /// What `field` holds on a fresh install: the default on the first field,
    /// nothing on the second.
    pub fn shipped_chord(self, field: ChordField) -> Option<ComposingKeyChord> {
        match field {
            ChordField::First => Some(self.default_chord()),
            ChordField::Second => None,
        }
    }

    /// Every field of every action, roster order then field order — the
    /// tiebreak order the bindings resolver reads.
    pub fn all_fields() -> impl Iterator<Item = (ComposingAction, ChordField)> {
        Self::ALL
            .into_iter()
            .flat_map(|action| ChordField::ALL.map(move |field| (action, field)))
    }

    /// Whether this action needs candidates on screen to mean anything. A
    /// chord whose action does not apply is not consumed: `]` still types a
    /// bracket when there is no page to turn.
    pub fn requires_candidates(self) -> bool {
        !matches!(self, Self::CommitLiteral)
    }

    /// The move through the list this action is, or `None` for the three
    /// that commit. One table for the candidate list and the symbol picker
    /// (`SymbolPickerIntent`), which walk and page with the same rows.
    pub fn navigation(self) -> Option<CandidateNavigation> {
        match self {
            Self::NextCandidate => Some(CandidateNavigation::NextCandidate),
            Self::PreviousCandidate => Some(CandidateNavigation::PreviousCandidate),
            Self::PageForward => Some(CandidateNavigation::PageDown),
            Self::PageBackward => Some(CandidateNavigation::PageUp),
            Self::ConfirmHighlighted | Self::CommitLiteral | Self::CommitAlternateScript => None,
        }
    }

    /// What this action does, once its chord has matched and its state checked.
    pub fn intent(self) -> ComposingKeyIntent {
        if let Some(navigation) = self.navigation() {
            return ComposingKeyIntent::Navigate(navigation);
        }
        match self {
            Self::ConfirmHighlighted => ComposingKeyIntent::CommitHighlightedCandidate,
            Self::CommitLiteral => ComposingKeyIntent::Commit,
            Self::CommitAlternateScript => ComposingKeyIntent::CommitAlternateScript,
            Self::NextCandidate
            | Self::PreviousCandidate
            | Self::PageForward
            | Self::PageBackward => unreachable!("navigation actions answered above"),
        }
    }

    /// The settings key `field`'s chord is stored under. The first field's
    /// key predates the second and keeps its name: absent means "never
    /// touched" (the default), a stored empty string means "cleared". The
    /// second field ships empty, so absent and empty read the same there.
    pub fn settings_key_name(self, field: ChordField) -> String {
        let first = Self::settings_key_name_for_raw(self.raw());
        match field {
            ChordField::First => first,
            ChordField::Second => format!("{first}.second"),
        }
    }

    /// The same key for a raw value the roster may no longer have — what a
    /// retired-settings sweep would remove.
    pub fn settings_key_name_for_raw(raw: &str) -> String {
        format!("composingShortcut.{raw}")
    }

    /// The recorder row's label key.
    pub fn label_key(self) -> StringKey {
        match self {
            Self::NextCandidate => StringKey::DesktopActionNextCandidate,
            Self::PreviousCandidate => StringKey::DesktopActionPreviousCandidate,
            Self::PageForward => StringKey::CommonPageNext,
            Self::PageBackward => StringKey::CommonPagePrevious,
            Self::ConfirmHighlighted => StringKey::DesktopActionConfirmHighlighted,
            Self::CommitLiteral => StringKey::DesktopActionCommitLiteral,
            Self::CommitAlternateScript => StringKey::DesktopActionCommitAlternateScript,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::{ALL_PLATFORMS, TEST_PLATFORM as PLATFORM};

    #[test]
    fn groups_hold_every_action_exactly_once() {
        let mut grouped: Vec<_> = ComposingAction::GROUPS
            .iter()
            .flat_map(|group| group.iter().copied())
            .collect();
        assert_eq!(grouped.len(), ComposingAction::ALL.len());
        grouped.sort();
        grouped.dedup();
        assert_eq!(grouped.len(), ComposingAction::ALL.len());
    }

    #[test]
    fn every_action_has_its_own_settings_key_and_raw_round_trips() {
        let mut names: Vec<_> = ComposingAction::ALL
            .iter()
            .flat_map(|a| ChordField::ALL.map(|field| a.settings_key_name(field)))
            .collect();
        names.sort();
        names.dedup();
        assert_eq!(
            names.len(),
            ComposingAction::ALL.len() * ChordField::ALL.len()
        );
        // trace: `settings_key_name` = `composingShortcut.<raw>` — the raw values ARE the
        // stored keys; renaming one drops every chord recorded under it.
        assert_eq!(
            ComposingAction::ALL.map(ComposingAction::raw),
            [
                "nextCandidate",
                "previousCandidate",
                "pageForward",
                "pageBackward",
                "confirmHighlighted",
                "commitLiteral",
                "commitAlternateScript",
            ]
        );
        assert_eq!(
            ComposingAction::NextCandidate.settings_key_name(ChordField::First),
            "composingShortcut.nextCandidate"
        );
        assert_eq!(
            ComposingAction::NextCandidate.settings_key_name(ChordField::Second),
            "composingShortcut.nextCandidate.second"
        );
        for action in ComposingAction::ALL {
            assert_eq!(ComposingAction::from_raw(action.raw()), Some(action));
            assert_eq!(
                action.settings_key_name(ChordField::First),
                format!("composingShortcut.{}", action.raw())
            );
        }
    }

    #[test]
    fn defaults_follow_the_system_zhuyin_keyboard() {
        // trace: `default_chord` — ⇥ / ⇧⇥ step, Space commits the other script,
        // ↩ / ⇧↩ commit, `[` / `]` page.
        let chord = |key: &str, m| ComposingKeyChord::make(Some(key), m, PLATFORM).unwrap();
        assert_eq!(
            ComposingAction::NextCandidate.default_chord(),
            chord("\t", KeyModifiers::NONE)
        );
        assert_eq!(
            ComposingAction::PreviousCandidate.default_chord(),
            chord("\t", KeyModifiers::SHIFT)
        );
        assert_eq!(
            ComposingAction::CommitAlternateScript.default_chord(),
            chord(" ", KeyModifiers::NONE)
        );
        assert_eq!(
            ComposingAction::ConfirmHighlighted.default_chord(),
            chord("\r", KeyModifiers::NONE)
        );
        assert_eq!(
            ComposingAction::CommitLiteral.default_chord(),
            chord("\r", KeyModifiers::SHIFT)
        );
        assert_eq!(
            ComposingAction::PageBackward.default_chord(),
            chord("[", KeyModifiers::NONE)
        );
        assert_eq!(
            ComposingAction::PageForward.default_chord(),
            chord("]", KeyModifiers::NONE)
        );
        assert!(!ComposingAction::CommitLiteral.requires_candidates());
        assert!(ComposingAction::ConfirmHighlighted.requires_candidates());
    }

    #[test]
    fn defaults_pass_the_gate_on_every_desktop() {
        // Both rosters' `default_chord` build the value without the gate;
        // the gate must hand each one back unchanged on all three desktops.
        let defaults = ComposingAction::ALL
            .map(|action| (format!("{action:?}"), action.default_chord()))
            .into_iter()
            .chain(
                crate::keys::ShortcutAction::ALL
                    .map(|action| (format!("{action:?}"), action.default_chord())),
            );
        for (action, chord) in defaults {
            for platform in ALL_PLATFORMS {
                assert_eq!(
                    ComposingKeyChord::make(Some(&chord.key), chord.modifiers, platform),
                    Ok(chord.clone()),
                    "{action} on {platform:?}"
                );
            }
        }
    }
}
