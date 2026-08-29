//! The user-configurable GLOBAL shortcuts: which actions exist, their
//! defaults, the extra refusals a global row has, and how the two
//! registries — global chords and composing chords — are kept from
//! colliding. Port of `ShortcutActions.swift` (`ShortcutAction`,
//! `ShortcutConflicts`) + `GlobalShortcutPolicy` (`ShortcutKeyRecorder.swift:45-92`).
//!
//! On the Mac the global tier stores Carbon key codes and needs a bridge to
//! compare with the composing tier's characters; on Windows both tiers store
//! the same [`ComposingKeyChord`] (the TSF shell maps a chord's character to
//! a virtual key when it registers the preserved key), so the bridge is the
//! identity and every conflict rule is a pure function over one settings
//! document.

// 中文: 全域快捷鍵動作 — 名冊、預設、全域列額外拒絕規則、兩個登錄簿的衝突解析(純函式)。

use super::action::ComposingAction;
use super::bindings::ComposingKeyBindings;
use super::chord::{ChordRejection, ComposingKeyChord};
use super::slot_key_set::CandidateSlotKeySet;
use super::snapshot::KeyModifiers;
use crate::settings::{keys, SettingsDocument};
use crate::strings::StringKey;

/// One user-assignable global action. The list is the single source for the
/// recorder rows, the preserved-key registration and the menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ShortcutAction {
    /// Opens the settings window on whichever pane the user left it on.
    OpenLastSettingsPane,
    ToggleRomanization,
    ToggleTranslateSwapped,
}

impl ShortcutAction {
    pub const ALL: [ShortcutAction; 3] = [
        Self::OpenLastSettingsPane,
        Self::ToggleRomanization,
        Self::ToggleTranslateSwapped,
    ];

    pub fn raw(self) -> &'static str {
        match self {
            Self::OpenLastSettingsPane => "openLastSettingsPane",
            Self::ToggleRomanization => "toggleRomanization",
            Self::ToggleTranslateSwapped => "toggleTranslateSwapped",
        }
    }

    /// Whether this action opens the settings window rather than doing
    /// something to the composition in flight (different dispatch).
    pub fn opens_settings(self) -> bool {
        self == Self::OpenLastSettingsPane
    }

    /// The chord a fresh install has on this action. Ctrl+Shift rather than
    /// the Mac's ⌃⌘ (Codex F7: Ctrl+Alt is AltGr on many layouts); the bare
    /// backtick is the Mac's own default and is consumed in the key sink only
    /// while the TIP is active, never registered as a preserved key.
    pub fn default_chord(self) -> ComposingKeyChord {
        let (key, modifiers) = match self {
            Self::OpenLastSettingsPane => ("s", KeyModifiers::CONTROL.with(KeyModifiers::SHIFT)),
            Self::ToggleRomanization => ("c", KeyModifiers::CONTROL.with(KeyModifiers::SHIFT)),
            Self::ToggleTranslateSwapped => ("`", KeyModifiers::NONE),
        };
        ComposingKeyChord::make(Some(key), modifiers).unwrap_or_else(|rejection| {
            panic!("default chord for {self:?} is not bindable: {rejection:?}")
        })
    }

    /// The settings key this action's chord is stored under. Absent = the
    /// default; `""` = the user cleared the row.
    pub fn settings_key_name(self) -> String {
        format!("globalShortcut.{}", self.raw())
    }

    pub fn label_key(self) -> StringKey {
        match self {
            Self::OpenLastSettingsPane => StringKey::DesktopShortcutOpenSettings,
            Self::ToggleRomanization => StringKey::DesktopShortcutToggleRomanization,
            Self::ToggleTranslateSwapped => StringKey::DesktopShortcutToggleTranslateSwapped,
        }
    }

    /// The chord the document holds for this action, or `None` for a cleared
    /// or unparsable row.
    pub fn chord_in(self, document: &SettingsDocument) -> Option<ComposingKeyChord> {
        match document.raw_string(&self.settings_key_name()) {
            None => Some(self.default_chord()),
            Some(raw) => ComposingKeyChord::from_raw(raw),
        }
    }

    /// Records `chord` on this action, or clears the row.
    pub fn store_in(self, document: &mut SettingsDocument, chord: Option<&ComposingKeyChord>) {
        let value = chord.map_or_else(
            || keys::CLEARED_COMPOSING_CHORD.to_owned(),
            |chord| chord.raw_value(),
        );
        document.set_raw_string(&self.settings_key_name(), &value);
    }
}

/// What a GLOBAL row refuses on top of the shared gate (`ComposingKeyChord::make`).
/// Windows-adapted from `GlobalShortcutPolicy`:
///
/// - `BelongsToHost`: Ctrl alone, or Alt alone, with a key — where a Windows
///   application puts its own commands (Ctrl+S) and its menu mnemonics
///   (Alt+F). A preserved key on one would take it from whatever the user is
///   typing into for as long as this input method is selected. Ctrl+Shift
///   and Alt+Shift are ours to offer, like the Mac's ⌃⌘.
/// - `TakenBySystem`: a Win-key chord — the OS answers those first — or a
///   Ctrl+Alt chord, which is AltGr on most non-US layouts. Windows has no
///   `CopySymbolicHotKeys`; this is a conservative static policy, not a
///   complete collision check (a dogfood item per layout).
/// - `NotAGlobalKey`: a key the preserved-key registration cannot name (not
///   a single printable ASCII character).
pub fn global_rejection(chord: &ComposingKeyChord) -> Option<ChordRejection> {
    let key_is_nameable = {
        let mut chars = chord.key.chars();
        matches!((chars.next(), chars.next()), (Some(c), None) if c.is_ascii_graphic() || c == ' ')
    };
    if !key_is_nameable {
        return Some(ChordRejection::NotAGlobalKey);
    }
    let modifiers = chord.modifiers;
    if modifiers.win {
        return Some(ChordRejection::TakenBySystem);
    }
    // Ctrl+Alt IS AltGr on most non-US layouts: a preserved key on one would
    // take the glyph that layout types with it (Codex F7 / PR4 review). No
    // layout-aware check is worth the false confidence — refused outright,
    // whatever else is held.
    if modifiers.control && modifiers.alt {
        return Some(ChordRejection::TakenBySystem);
    }
    let host_only_control = modifiers.control && !modifiers.shift;
    let host_only_alt = modifiers.alt && !modifiers.shift;
    if host_only_control || host_only_alt {
        return Some(ChordRejection::BelongsToHost);
    }
    None
}

/// Keeps one chord meaning one thing across both registries — last writer
/// wins, and the loser's row visibly empties (the System Settings keyboard
/// pane's behaviour). Every function here is pure over the document.
pub struct ShortcutConflicts;

impl ShortcutConflicts {
    /// Which other global actions currently hold the same chord as `changed`.
    pub fn conflicting_global_actions(
        document: &SettingsDocument,
        changed: ShortcutAction,
    ) -> Vec<ShortcutAction> {
        let Some(recorded) = changed.chord_in(document) else {
            return Vec::new();
        };
        ShortcutAction::ALL
            .into_iter()
            .filter(|other| {
                *other != changed && other.chord_in(document).as_ref() == Some(&recorded)
            })
            .collect()
    }

    /// Which global actions hold a chord only because it is their default,
    /// while another global action holds the same chord because the user
    /// recorded it there (the upgrade case).
    pub fn defaults_shadowed_by_recordings(document: &SettingsDocument) -> Vec<ShortcutAction> {
        let held: Vec<(ShortcutAction, Option<ComposingKeyChord>)> = ShortcutAction::ALL
            .into_iter()
            .map(|action| (action, action.chord_in(document)))
            .collect();
        let recorded: Vec<&ComposingKeyChord> = held
            .iter()
            .filter_map(|(action, chord)| {
                chord
                    .as_ref()
                    .filter(|chord| **chord != action.default_chord())
            })
            .collect();
        held.iter()
            .filter(|(action, chord)| chord.as_ref() == Some(&action.default_chord()))
            .filter(|(_, chord)| recorded.iter().any(|r| Some(*r) == chord.as_ref()))
            .map(|(action, _)| *action)
            .collect()
    }

    /// A global recording just landed on `changed`: take the chord off every
    /// other global row and every composing row that held it.
    pub fn resolve_after_global_recording(
        document: &mut SettingsDocument,
        changed: ShortcutAction,
    ) {
        for loser in Self::conflicting_global_actions(document, changed) {
            loser.store_in(document, None);
        }
        let Some(chord) = changed.chord_in(document) else {
            return;
        };
        let bindings = ComposingKeyBindings::from_document(document);
        for loser in bindings.actions_holding(&chord, None) {
            document.set_composing_chord(loser, None);
        }
    }

    /// A composing recording just landed on `changed` with `chord`: take the
    /// chord off every other composing row and every global row that held it.
    pub fn resolve_after_composing_recording(
        document: &mut SettingsDocument,
        changed: ComposingAction,
        chord: &ComposingKeyChord,
    ) {
        let bindings = ComposingKeyBindings::from_document(document);
        for loser in bindings.actions_holding(chord, Some(changed)) {
            document.set_composing_chord(loser, None);
        }
        for loser in Self::global_actions_holding(document, |held| held == chord) {
            loser.store_in(document, None);
        }
    }

    /// The slot key set just changed: take its keys off any global row that
    /// held one (the picker is the last writer; the recorder refuses the
    /// other order).
    pub fn resolve_after_slot_key_set_change(
        document: &mut SettingsDocument,
        slot_key_set: CandidateSlotKeySet,
    ) {
        for loser in Self::global_actions_holding(document, |held| {
            held.is_candidate_slot_chord(slot_key_set)
        }) {
            loser.store_in(document, None);
        }
    }

    /// Which global actions hold a chord answering `predicate`.
    pub fn global_actions_holding(
        document: &SettingsDocument,
        predicate: impl Fn(&ComposingKeyChord) -> bool,
    ) -> Vec<ShortcutAction> {
        ShortcutAction::ALL
            .into_iter()
            .filter(|action| {
                action
                    .chord_in(document)
                    .is_some_and(|chord| predicate(&chord))
            })
            .collect()
    }

    /// Reconciles the two registries at launch, where no recorder ran.
    /// Recording-beats-default; when BOTH sides are recordings the GLOBAL
    /// tier wins (it is the tier that fires first — a preserved key is
    /// dispatched before the classifier ever runs). Then a global row sitting
    /// on a live slot chord, or on Shift+1…9, is cleared. Idempotent.
    pub fn resolve_across_registries(document: &mut SettingsDocument) {
        for loser in Self::defaults_shadowed_by_recordings(document) {
            loser.store_in(document, None);
        }
        let bindings = ComposingKeyBindings::from_document(document);
        let held: Vec<(ShortcutAction, ComposingKeyChord)> = ShortcutAction::ALL
            .into_iter()
            .filter_map(|action| action.chord_in(document).map(|chord| (action, chord)))
            .collect();
        for (action, chord) in &held {
            let holders = bindings.actions_holding(chord, None);
            if holders.is_empty() {
                continue;
            }
            let global_is_default = *chord == action.default_chord();
            for holder in holders {
                let composing_recording_outranks =
                    global_is_default && *chord != holder.default_chord();
                if composing_recording_outranks {
                    action.store_in(document, None);
                } else {
                    document.set_composing_chord(holder, None);
                }
            }
        }
        for (action, chord) in &held {
            if chord.is_candidate_slot_chord(bindings.slot_key_set) {
                action.store_in(document, None);
            }
        }
        // Shift+1…9 cannot be a chord at all (the gate refuses them), so a
        // raw value carrying one fails to parse and reads as cleared already —
        // `chord_in` returns None and nothing is left to clear.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chord(key: &str, modifiers: KeyModifiers) -> ComposingKeyChord {
        ComposingKeyChord::make(Some(key), modifiers).unwrap()
    }

    #[test]
    fn roster_defaults_and_keys() {
        // trace: ShortcutActionsTests.swift:31-160 (Windows chords).
        let mut names: Vec<_> = ShortcutAction::ALL
            .iter()
            .map(|a| a.settings_key_name())
            .collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), 3);
        assert_eq!(
            ShortcutAction::OpenLastSettingsPane.default_chord(),
            chord("s", KeyModifiers::CONTROL.with(KeyModifiers::SHIFT))
        );
        assert_eq!(
            ShortcutAction::ToggleRomanization.default_chord(),
            chord("c", KeyModifiers::CONTROL.with(KeyModifiers::SHIFT))
        );
        assert_eq!(
            ShortcutAction::ToggleTranslateSwapped.default_chord(),
            chord("`", KeyModifiers::NONE)
        );
        let mut defaults: Vec<_> = ShortcutAction::ALL
            .iter()
            .map(|a| a.default_chord())
            .collect();
        defaults.sort();
        defaults.dedup();
        assert_eq!(defaults.len(), 3, "the defaults are all different");
        assert_eq!(
            ShortcutAction::ALL
                .iter()
                .filter(|a| a.opens_settings())
                .count(),
            1
        );
        for action in ShortcutAction::ALL {
            assert_eq!(
                global_rejection(&action.default_chord()),
                None,
                "{action:?}'s default must be recordable"
            );
            for set in [
                CandidateSlotKeySet::BareKeys,
                CandidateSlotKeySet::Shift,
                CandidateSlotKeySet::Control,
                CandidateSlotKeySet::Option,
            ] {
                assert!(
                    !action.default_chord().is_candidate_slot_chord(set),
                    "{action:?} under {set:?}"
                );
            }
        }
        let composing_defaults: Vec<_> = ComposingAction::ALL
            .iter()
            .map(|a| a.default_chord())
            .collect();
        for action in ShortcutAction::ALL {
            assert!(
                !composing_defaults.contains(&action.default_chord()),
                "{action:?} collides with a composing default"
            );
        }
    }

    #[test]
    fn global_policy_refuses_host_and_system_chords() {
        assert_eq!(
            global_rejection(&chord("s", KeyModifiers::CONTROL)),
            Some(ChordRejection::BelongsToHost)
        );
        assert_eq!(
            global_rejection(&chord("f", KeyModifiers::ALT)),
            Some(ChordRejection::BelongsToHost)
        );
        assert_eq!(
            global_rejection(&chord("s", KeyModifiers::WIN.with(KeyModifiers::SHIFT))),
            Some(ChordRejection::TakenBySystem)
        );
        assert_eq!(
            global_rejection(&chord("s", KeyModifiers::CONTROL.with(KeyModifiers::ALT))),
            Some(ChordRejection::TakenBySystem),
            "Ctrl+Alt is AltGr"
        );
        assert_eq!(
            global_rejection(&chord("s", KeyModifiers::ALT.with(KeyModifiers::SHIFT))),
            None
        );
        assert_eq!(
            global_rejection(&chord("q", KeyModifiers::NONE)),
            None,
            "a bare free letter is recordable"
        );
        assert_eq!(
            global_rejection(&chord("±", KeyModifiers::CONTROL.with(KeyModifiers::SHIFT))),
            Some(ChordRejection::NotAGlobalKey),
            "a non-ASCII key cannot name a preserved key"
        );
    }

    #[test]
    fn recording_a_chord_another_global_action_holds_reports_and_clears_it() {
        // trace: ShortcutActionsTests.swift:182-236.
        let mut doc = SettingsDocument::default();
        let shared = chord("k", KeyModifiers::CONTROL.with(KeyModifiers::SHIFT));
        ShortcutAction::OpenLastSettingsPane.store_in(&mut doc, Some(&shared));
        ShortcutAction::ToggleRomanization.store_in(&mut doc, Some(&shared));
        assert_eq!(
            ShortcutConflicts::conflicting_global_actions(&doc, ShortcutAction::ToggleRomanization),
            vec![ShortcutAction::OpenLastSettingsPane]
        );
        ShortcutConflicts::resolve_after_global_recording(
            &mut doc,
            ShortcutAction::ToggleRomanization,
        );
        assert_eq!(ShortcutAction::OpenLastSettingsPane.chord_in(&doc), None);
        assert_eq!(
            ShortcutAction::ToggleRomanization.chord_in(&doc),
            Some(shared)
        );
        ShortcutAction::ToggleRomanization.store_in(&mut doc, None);
        assert_eq!(
            ShortcutConflicts::conflicting_global_actions(&doc, ShortcutAction::ToggleRomanization),
            vec![]
        );
    }

    #[test]
    fn a_default_gives_way_to_the_same_chord_recorded_elsewhere() {
        let mut doc = SettingsDocument::default();
        let settings_default = ShortcutAction::OpenLastSettingsPane.default_chord();
        ShortcutAction::ToggleRomanization.store_in(&mut doc, Some(&settings_default));
        assert_eq!(
            ShortcutConflicts::defaults_shadowed_by_recordings(&doc),
            vec![ShortcutAction::OpenLastSettingsPane]
        );
        ShortcutConflicts::resolve_across_registries(&mut doc);
        assert_eq!(ShortcutAction::OpenLastSettingsPane.chord_in(&doc), None);
        assert_eq!(
            ShortcutAction::ToggleRomanization.chord_in(&doc),
            Some(settings_default)
        );
    }

    #[test]
    fn launch_pass_reconciles_the_two_registries() {
        // trace: CrossTierShortcutConflictTests.swift:264-400.
        // A global default shadowed by a composing recording: the recording wins.
        let mut doc = SettingsDocument::default();
        let default = ShortcutAction::ToggleRomanization.default_chord();
        doc.set_composing_chord(ComposingAction::PageForward, Some(&default));
        ShortcutConflicts::resolve_across_registries(&mut doc);
        assert_eq!(
            ShortcutAction::ToggleRomanization.chord_in(&doc),
            None,
            "the default gives way"
        );
        assert_eq!(
            ComposingKeyBindings::from_document(&doc).chord(ComposingAction::PageForward),
            Some(&default)
        );

        // The mirror image: a global recording on a composing default.
        let mut doc = SettingsDocument::default();
        let shift_tab = chord("\t", KeyModifiers::SHIFT);
        ShortcutAction::ToggleRomanization.store_in(&mut doc, Some(&shift_tab));
        ShortcutConflicts::resolve_across_registries(&mut doc);
        assert_eq!(
            ComposingKeyBindings::from_document(&doc).chord(ComposingAction::PreviousCandidate),
            None
        );
        assert_eq!(
            ShortcutAction::ToggleRomanization.chord_in(&doc),
            Some(shift_tab.clone())
        );

        // Recording vs recording: the global tier wins.
        let mut doc = SettingsDocument::default();
        let recorded = chord("f", KeyModifiers::CONTROL.with(KeyModifiers::ALT));
        doc.set_composing_chord(ComposingAction::PageForward, Some(&recorded));
        ShortcutAction::ToggleRomanization.store_in(&mut doc, Some(&recorded));
        ShortcutConflicts::resolve_across_registries(&mut doc);
        assert_eq!(
            ShortcutAction::ToggleRomanization.chord_in(&doc),
            Some(recorded)
        );
        assert_eq!(
            ComposingKeyBindings::from_document(&doc).chord(ComposingAction::PageForward),
            None
        );
        // Idempotent.
        let before = doc.clone();
        ShortcutConflicts::resolve_across_registries(&mut doc);
        assert_eq!(doc.to_json(), before.to_json());

        // A global row on a live slot chord is cleared; one another set claims stays.
        let mut doc = SettingsDocument::default();
        ShortcutAction::OpenLastSettingsPane
            .store_in(&mut doc, Some(&chord("q", KeyModifiers::NONE)));
        ShortcutAction::ToggleRomanization
            .store_in(&mut doc, Some(&chord("3", KeyModifiers::CONTROL)));
        ShortcutConflicts::resolve_across_registries(&mut doc);
        assert_eq!(ShortcutAction::OpenLastSettingsPane.chord_in(&doc), None);
        assert_eq!(
            ShortcutAction::ToggleRomanization.chord_in(&doc),
            Some(chord("3", KeyModifiers::CONTROL))
        );

        // An uncolliding setup is left alone.
        let mut doc = SettingsDocument::default();
        ShortcutConflicts::resolve_across_registries(&mut doc);
        assert_eq!(doc.revision, 0, "nothing was written");

        // Two collisions at once, and an always-bound row survives being cleared.
        let mut doc = SettingsDocument::default();
        ShortcutAction::ToggleRomanization.store_in(&mut doc, Some(&shift_tab));
        ShortcutAction::OpenLastSettingsPane
            .store_in(&mut doc, Some(&chord("]", KeyModifiers::NONE)));
        ShortcutConflicts::resolve_across_registries(&mut doc);
        let bindings = ComposingKeyBindings::from_document(&doc);
        assert_eq!(bindings.chord(ComposingAction::PreviousCandidate), None);
        assert_eq!(bindings.chord(ComposingAction::PageForward), None);
        let mut doc = SettingsDocument::default();
        let ctrl_r = chord("r", KeyModifiers::CONTROL.with(KeyModifiers::SHIFT));
        doc.set_composing_chord(ComposingAction::CommitLiteral, Some(&ctrl_r));
        ShortcutAction::OpenLastSettingsPane.store_in(&mut doc, Some(&ctrl_r));
        ShortcutConflicts::resolve_across_registries(&mut doc);
        let restored = ComposingKeyBindings::from_document(&doc)
            .chord(ComposingAction::CommitLiteral)
            .cloned();
        assert!(
            restored.is_some(),
            "an always-bound row must not be left empty"
        );
        assert_ne!(restored.as_ref(), Some(&ctrl_r));
    }

    #[test]
    fn composing_recording_and_slot_set_change_clear_global_rows() {
        let mut doc = SettingsDocument::default();
        let shared = chord("]", KeyModifiers::CONTROL.with(KeyModifiers::SHIFT));
        ShortcutAction::ToggleRomanization.store_in(&mut doc, Some(&shared));
        doc.set_composing_chord(ComposingAction::PageForward, Some(&shared));
        ShortcutConflicts::resolve_after_composing_recording(
            &mut doc,
            ComposingAction::PageForward,
            &shared,
        );
        assert_eq!(ShortcutAction::ToggleRomanization.chord_in(&doc), None);
        assert_eq!(
            ComposingKeyBindings::from_document(&doc).chord(ComposingAction::PageForward),
            Some(&shared)
        );

        let mut doc = SettingsDocument::default();
        ShortcutAction::ToggleRomanization
            .store_in(&mut doc, Some(&chord("3", KeyModifiers::CONTROL)));
        ShortcutConflicts::resolve_after_slot_key_set_change(&mut doc, CandidateSlotKeySet::Option);
        assert!(
            ShortcutAction::ToggleRomanization.chord_in(&doc).is_some(),
            "Alt digits do not claim Ctrl+3"
        );
        ShortcutConflicts::resolve_after_slot_key_set_change(
            &mut doc,
            CandidateSlotKeySet::Control,
        );
        assert_eq!(ShortcutAction::ToggleRomanization.chord_in(&doc), None);
    }
}
