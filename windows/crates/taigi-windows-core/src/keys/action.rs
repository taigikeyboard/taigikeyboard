//! The composing actions a user can put on a key of their own choosing.
//! Port of `ComposingAction.swift`.

// 使用者可自訂按鍵的組字動作;預設鍵跟隨系統注音輸入法。

use super::chord::ComposingKeyChord;
use super::intent::{CandidateNavigation, ComposingKeyIntent};
use super::snapshot::KeyModifiers;
use crate::strings::StringKey;

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
    /// Commits the highlighted candidate in the OTHER script — the 漢羅 key.
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

    /// Actions that must always be reachable: between them the only two ways
    /// to end a composition into the document.
    pub const ALWAYS_BOUND: [ComposingAction; 2] = [Self::ConfirmHighlighted, Self::CommitLiteral];

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

    /// The chord a fresh install has on this action, built through the same
    /// gate a recorded one goes through. A default the gate refuses is a
    /// mistake in this file, so it traps rather than shipping "unbound".
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
        ComposingKeyChord::make(Some(key), modifiers).unwrap_or_else(|rejection| {
            panic!("default chord for {self:?} is not bindable: {rejection:?}")
        })
    }

    /// Whether this action needs candidates on screen to mean anything. A
    /// chord whose action does not apply is not consumed: `]` still types a
    /// bracket when there is no page to turn.
    pub fn requires_candidates(self) -> bool {
        !matches!(self, Self::CommitLiteral)
    }

    /// What this action does, once its chord has matched and its state checked.
    pub fn intent(self) -> ComposingKeyIntent {
        match self {
            Self::NextCandidate => ComposingKeyIntent::Navigate(CandidateNavigation::NextCandidate),
            Self::PreviousCandidate => {
                ComposingKeyIntent::Navigate(CandidateNavigation::PreviousCandidate)
            }
            Self::PageForward => ComposingKeyIntent::Navigate(CandidateNavigation::PageDown),
            Self::PageBackward => ComposingKeyIntent::Navigate(CandidateNavigation::PageUp),
            Self::ConfirmHighlighted => ComposingKeyIntent::CommitHighlightedCandidate,
            Self::CommitLiteral => ComposingKeyIntent::Commit,
            Self::CommitAlternateScript => ComposingKeyIntent::CommitAlternateScript,
        }
    }

    /// The settings key this action's chord is stored under. An absent key
    /// means "never touched"; a stored empty string means "cleared".
    pub fn settings_key_name(self) -> String {
        Self::settings_key_name_for_raw(self.raw())
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
            Self::PageForward => StringKey::DesktopActionPageForward,
            Self::PageBackward => StringKey::DesktopActionPageBackward,
            Self::ConfirmHighlighted => StringKey::DesktopActionConfirmHighlighted,
            Self::CommitLiteral => StringKey::DesktopActionCommitLiteral,
            Self::CommitAlternateScript => StringKey::DesktopActionCommitAlternateScript,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            .map(|a| a.settings_key_name())
            .collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), ComposingAction::ALL.len());
        for action in ComposingAction::ALL {
            assert_eq!(ComposingAction::from_raw(action.raw()), Some(action));
            assert_eq!(
                action.settings_key_name(),
                format!("composingShortcut.{}", action.raw())
            );
        }
    }

    #[test]
    fn defaults_follow_the_system_zhuyin_keyboard() {
        // trace: ComposingKeyBindingsTests.swift:196-215.
        let chord = |key: &str, m| ComposingKeyChord::make(Some(key), m).unwrap();
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
}
