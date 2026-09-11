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

use super::action::ComposingAction;
use super::bindings::ComposingKeyBindings;
use super::chord::{ChordRejection, ComposingKeyChord};
use super::snapshot::KeyModifiers;
use crate::settings::{keys, SettingsDocument};
use crate::strings::StringKey;

/// One user-assignable global action. The list is the single source for the
/// recorder rows, the preserved-key registration and the menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ShortcutAction {
    ToggleRomanization,
    /// Steps 候選詞顯示 through its picker order (`CandidateDisplayMode::next`).
    CycleCandidateDisplayMode,
    ToggleTranslateSwapped,
    /// Opens the symbol picker over the caret. Beside the settings window
    /// since 2026-09-10 (USER): both rows read 拍開X
    /// (`ShortcutActions.swift` `ShortcutAction`).
    ShowSymbolPicker,
    /// Opens the settings window on whichever pane the user left it on.
    OpenLastSettingsPane,
    /// Toggles the floating Telex key table (`ui/telex_guide.rs`). Last on the
    /// pane (USER 2026-09-10): the row that explains the keyboard rather than
    /// doing anything to what is being typed.
    ShowTelexGuide,
}

impl ShortcutAction {
    pub const ALL: [ShortcutAction; 6] = [
        Self::ToggleRomanization,
        Self::CycleCandidateDisplayMode,
        Self::ToggleTranslateSwapped,
        Self::ShowSymbolPicker,
        Self::OpenLastSettingsPane,
        Self::ShowTelexGuide,
    ];

    pub fn raw(self) -> &'static str {
        match self {
            Self::OpenLastSettingsPane => "openLastSettingsPane",
            Self::ToggleRomanization => "toggleRomanization",
            Self::ToggleTranslateSwapped => "toggleTranslateSwapped",
            Self::CycleCandidateDisplayMode => "cycleCandidateDisplayMode",
            Self::ShowSymbolPicker => "showSymbolPicker",
            Self::ShowTelexGuide => "showTelexGuide",
        }
    }

    /// Whether this action runs against the context the chord was pressed
    /// in — the picker anchors its window to that context's caret and writes
    /// the pick into its document (`session.rs` `toggle_symbol_picker`) —
    /// rather than through `perform_global`, which flips a setting or raises
    /// a card and needs no context. Both doorways route it there: the key
    /// sink's match, and `OnPreservedKey` with the context TSF hands it.
    ///
    /// It IS a preserved key, like every other Ctrl+Alt chord here. The key
    /// sink alone cannot carry it: a key pressed with Alt held never reached
    /// `OnKeyDown` in Notepad or Chrome (dev box, 2026-09-11 — the sink saw
    /// the Ctrl press and the Alt release, never the comma), while the
    /// preserved keys fired. Differs from macOS `firesFromTheKeyPath`, which
    /// skips Carbon registration because the key path there sees the chord.
    pub fn needs_key_context(self) -> bool {
        self == Self::ShowSymbolPicker
    }

    /// Whether a held chord must fire ONCE: the guide and the picker toggle,
    /// so an auto-repeat would flip them back. The switches read as one
    /// press already (a switch repeated is a switch back — left as is).
    pub fn fires_once_per_press(self) -> bool {
        matches!(self, Self::ShowTelexGuide | Self::ShowSymbolPicker)
    }

    /// Whether this action opens the settings window rather than doing
    /// something to the composition in flight (different dispatch).
    pub fn opens_settings(self) -> bool {
        self == Self::OpenLastSettingsPane
    }

    /// The chord a fresh install has on this action. The bare backtick is the
    /// Mac's own default and is consumed in the key sink only while the TIP is
    /// active, never registered as a preserved key.
    ///
    /// Ctrl+Alt is the Mac's ⌃⌘ under this platform's modifier mapping
    /// (`windows-guidelines.md`: ⌘→Ctrl, ⌃→Alt), so the two desktops keep one
    /// roster: ⌃⌘S → Ctrl+Alt+S for 設定, ⌃⌘C → Ctrl+Alt+C for the
    /// romanization switch, ⌃⌘H → Ctrl+Alt+H for the display-mode cycle. The
    /// letters are the Mac's reasons, unchanged — S for Settings / siat-tīng /
    /// settei, C for the bottom row a key pressed all day should sit on
    /// (`ShortcutActions.swift:31-52`), and H for Hàn-Lô / 漢羅, the thing the
    /// cycle switches. USER 2026-08-31: the chord logic has to match macOS's.
    ///
    /// Ctrl+Shift is NOT that family and its S and C are both taken —
    /// Ctrl+Shift+S is 另存新檔 in Word / Excel / LibreOffice / GIMP / Inkscape
    /// and 全部儲存 in Visual Studio; Ctrl+Shift+C opens the DevTools element
    /// picker in Chrome / Edge. Nor is any other pair of modifiers free with
    /// these letters: Alt+Shift is the input-language-switch chord and Word's
    /// Alt+Shift+<letter> family, Win+Shift+S is 剪取工具 and Win+Ctrl+S is
    /// speech recognition (and Win chords are refused outright below).
    /// Ctrl+Alt+S is JetBrains' own Settings chord, i.e. an existing Windows
    /// convention for exactly this command. What it does cost, and what a
    /// user rebinds away from if it bites: Word's 分割視窗, Visual Studio's
    /// Server Explorer (S) and Call Stack (C), and Teams' see-all-chats (C).
    ///
    /// `/` for the Telex guide is the Mac's ⌃⌘/ carried over the same way:
    /// `/` is the key help lives on (`?` is Shift+/, and every app that
    /// answers "which keys do what" answers it there). Not Ctrl+Alt+T, the
    /// mnemonic first reached for — JetBrains binds it to Surround With, and
    /// a user in an IDE would lose one or the other
    /// (`ShortcutActions.swift` `showTelexGuide`). On a layout where `/`
    /// itself needs Shift, the preserved key registers with that Shift
    /// OR-ed in (`preserved_key`) and still fires; only the key sink's
    /// fallback — for hosts that bypass preserved keys — cannot match
    /// such a press, because the stored chord names `/` unshifted.
    ///
    /// `,` for the symbol picker is the Mac's ⌃⌘, carried over the same way:
    /// the picker is a punctuation menu and the comma is the punctuation key.
    /// Not the bare backtick 新注音 / McBopomofo / vChewing open their symbol
    /// menus on: that key is 漢羅對調 here, and stays (USER 2026-09-09:
    /// 「不要更改 ` 快捷鍵,這是台語輸入法的共識」).
    ///
    /// Changing a default here moves every install that never recorded the row:
    /// nothing writes a default into `settings.json`, so an absent key IS the
    /// default (`chord_in`). No migration flag, unlike the Mac's.
    pub fn default_chord(self) -> ComposingKeyChord {
        let (key, modifiers) = match self {
            Self::OpenLastSettingsPane => ("s", KeyModifiers::CONTROL.with(KeyModifiers::ALT)),
            Self::ToggleRomanization => ("c", KeyModifiers::CONTROL.with(KeyModifiers::ALT)),
            Self::ToggleTranslateSwapped => ("`", KeyModifiers::NONE),
            Self::CycleCandidateDisplayMode => ("h", KeyModifiers::CONTROL.with(KeyModifiers::ALT)),
            Self::ShowSymbolPicker => (",", KeyModifiers::CONTROL.with(KeyModifiers::ALT)),
            Self::ShowTelexGuide => ("/", KeyModifiers::CONTROL.with(KeyModifiers::ALT)),
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
            Self::CycleCandidateDisplayMode => StringKey::DesktopShortcutCycleCandidateDisplayMode,
            Self::ShowSymbolPicker => StringKey::DesktopShortcutShowSymbolPicker,
            Self::ShowTelexGuide => StringKey::DesktopShortcutShowTelexGuide,
        }
    }

    /// The chord the document holds for this action, or `None` for a cleared
    /// or unparsable row.
    pub fn chord_in(self, document: &SettingsDocument) -> Option<ComposingKeyChord> {
        self.translation_in(document).and_then(Result::ok)
    }

    /// The bridge with its refusal kept: `None` for a cleared, absent-default
    /// or unreadable row; `Some(Err)` for a stored value the gate refuses.
    /// The launch pass reads WHY a row fails to translate, because a row on a
    /// typing key is one the recorder would refuse today and the preserved
    /// key would still be dispatched first (`ShortcutActions.swift`
    /// `translation(of:)`).
    fn translation_in(
        self,
        document: &SettingsDocument,
    ) -> Option<Result<ComposingKeyChord, ChordRejection>> {
        match document.raw_string(&self.settings_key_name()) {
            None => Some(Ok(self.default_chord())),
            Some(raw) => ComposingKeyChord::translate_raw(raw),
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
///   typing into for as long as this input method is selected. Ctrl+Shift,
///   Alt+Shift and Ctrl+Alt are ours to offer, like the Mac's ⌃⌘: a SECOND
///   modifier is what lifts a chord out of the host's own space.
/// - `TakenBySystem`: a Win-key chord — the OS answers those first. Windows
///   has no `CopySymbolicHotKeys`; this is a conservative static policy, not
///   a complete collision check (a dogfood item per layout).
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
    // Ctrl+Alt is allowed HERE and refused on the composing tier
    // (`evaluate_press`). It is AltGr on layouts that have one, and a
    // composing binding would take the glyph such a layout types with it —
    // but a global chord answers only while this Taiwanese TIP is the
    // selected one, both ways it can be reached: as a preserved key
    // registered at activation and unregistered at Deactivate
    // (`preserved_keys.rs`), and as the key sink's fallback for hosts that
    // bypass preserved keys (`session.rs`). Reaching a layout where
    // AltGr+<letter> types a glyph means having selected a different
    // language profile, which is not this TIP. That leaves Ctrl+Alt as the
    // one two-modifier family that carries the Mac's ⌃⌘ roster over intact
    // (USER 2026-08-31; Codex confirmed the lifetime on both paths). NOT
    // proven by an invariant — a layout substitution under this TIP is a
    // dogfood item, as is a layout on which `VkKeyScanExW` reports the
    // letter itself as needing AltGr (`preserved_key` ORs the two).
    // "Only" as in nothing else held: Ctrl+Alt is neither the host's Ctrl+S
    // nor its Alt+F, so each test excludes the other modifier as well as
    // Shift.
    let host_only_control = modifiers.control && !modifiers.shift && !modifiers.alt;
    let host_only_alt = modifiers.alt && !modifiers.shift && !modifiers.control;
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
        for action in ShortcutAction::ALL {
            if action.chord_in(document).as_ref() == Some(chord) {
                action.store_in(document, None);
            }
        }
    }

    /// Reconciles the two registries at launch, where no recorder ran.
    /// Recording-beats-default; when BOTH sides are recordings the GLOBAL
    /// tier wins (it is the tier that fires first — a preserved key is
    /// dispatched before the classifier ever runs). A global row left on a
    /// typing key is cleared first. Idempotent.
    pub fn resolve_across_registries(document: &mut SettingsDocument) {
        for loser in Self::defaults_shadowed_by_recordings(document) {
            loser.store_in(document, None);
        }
        // A global row on a key the gate refuses as a typing key — a bare
        // `z` or `q` recorded while the eight non-syllable letters were
        // bindable (before 2026-09-08), or a Shift+3 — is not a collision to
        // compare: it is a row that predates the refusal, and the preserved
        // key would be dispatched before the classifier ever saw the Telex
        // key or the slot key it now types. Cleared, the way the recorder
        // would have refused it; a refusal is not "no conflict". `ReservedKey`
        // rows cannot exist (the arrows and the deletes were never
        // recordable); only the typing-key refusal names an upgrade path.
        // Mirrors `ShortcutActions.swift` `resolveAcrossRegistries`.
        for action in ShortcutAction::ALL {
            if action.translation_in(document) == Some(Err(ChordRejection::TypesRomanization)) {
                action.store_in(document, None);
            }
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
        assert_eq!(names.len(), 6);
        assert_eq!(
            ShortcutAction::OpenLastSettingsPane.default_chord(),
            chord("s", KeyModifiers::CONTROL.with(KeyModifiers::ALT))
        );
        assert_eq!(
            ShortcutAction::ToggleRomanization.default_chord(),
            chord("c", KeyModifiers::CONTROL.with(KeyModifiers::ALT))
        );
        assert_eq!(
            ShortcutAction::ToggleTranslateSwapped.default_chord(),
            chord("`", KeyModifiers::NONE)
        );
        assert_eq!(
            ShortcutAction::CycleCandidateDisplayMode.default_chord(),
            chord("h", KeyModifiers::CONTROL.with(KeyModifiers::ALT))
        );
        assert_eq!(
            ShortcutAction::ShowSymbolPicker.default_chord(),
            chord(",", KeyModifiers::CONTROL.with(KeyModifiers::ALT))
        );
        assert_eq!(
            ShortcutAction::ShowTelexGuide.default_chord(),
            chord("/", KeyModifiers::CONTROL.with(KeyModifiers::ALT))
        );
        let mut defaults: Vec<_> = ShortcutAction::ALL
            .iter()
            .map(|a| a.default_chord())
            .collect();
        defaults.sort();
        defaults.dedup();
        assert_eq!(defaults.len(), 6, "the defaults are all different");
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
    fn roster_order_is_the_pane_order() {
        // `ALL` is the global recorder rows top to bottom
        // (`pages/shortcuts.rs`, under the composing rows) and the Mac's
        // `allCases`: the three switches, then the two 拍開X doorways
        // together, then the Telex card last (USER 2026-09-10).
        assert_eq!(
            ShortcutAction::ALL,
            [
                ShortcutAction::ToggleRomanization,
                ShortcutAction::CycleCandidateDisplayMode,
                ShortcutAction::ToggleTranslateSwapped,
                ShortcutAction::ShowSymbolPicker,
                ShortcutAction::OpenLastSettingsPane,
                ShortcutAction::ShowTelexGuide,
            ]
        );
        assert_eq!(ShortcutAction::ShowSymbolPicker.raw(), "showSymbolPicker");
        assert_eq!(
            ShortcutAction::ShowSymbolPicker.label_key(),
            StringKey::DesktopShortcutShowSymbolPicker
        );
        assert!(!ShortcutAction::ShowSymbolPicker.opens_settings());
        // The picker is the one row that runs against the pressed context:
        // a pick writes into its document.
        let context_actions: Vec<_> = ShortcutAction::ALL
            .into_iter()
            .filter(|action| action.needs_key_context())
            .collect();
        assert_eq!(context_actions, [ShortcutAction::ShowSymbolPicker]);
        let once: Vec<_> = ShortcutAction::ALL
            .into_iter()
            .filter(|action| action.fires_once_per_press())
            .collect();
        assert_eq!(
            once,
            [
                ShortcutAction::ShowSymbolPicker,
                ShortcutAction::ShowTelexGuide
            ]
        );
        assert_eq!(
            ShortcutAction::CycleCandidateDisplayMode.raw(),
            "cycleCandidateDisplayMode"
        );
        assert_eq!(
            ShortcutAction::CycleCandidateDisplayMode.label_key(),
            StringKey::DesktopShortcutCycleCandidateDisplayMode
        );
        assert!(!ShortcutAction::CycleCandidateDisplayMode.opens_settings());
        assert_eq!(ShortcutAction::ShowTelexGuide.raw(), "showTelexGuide");
        assert_eq!(
            ShortcutAction::ShowTelexGuide.label_key(),
            StringKey::DesktopShortcutShowTelexGuide
        );
        assert!(!ShortcutAction::ShowTelexGuide.opens_settings());
    }

    #[test]
    fn the_global_policy_answers_every_modifier_combination() {
        // The whole 16-cell table, so widening one rule cannot quietly open
        // another cell. A SECOND modifier is what lifts a chord out of the
        // host's space; the Win key is the OS's whatever else is held.
        use ChordRejection::{BelongsToHost, TakenBySystem, TypesRomanization};
        let cell = |shift, control, alt, win| {
            let modifiers = KeyModifiers {
                shift,
                control,
                alt,
                win,
            };
            // `[` is no typing key (every letter is one, under either tone
            // scheme), so a bare chord on it is makeable and the answer is
            // the POLICY's, not the constructor's.
            ComposingKeyChord::make(Some("["), modifiers).map(|chord| global_rejection(&chord))
        };
        // shift, control, alt, win → what the policy says
        let expected = [
            ((false, false, false, false), Ok(None)),
            ((true, false, false, false), Ok(None)),
            ((false, true, false, false), Ok(Some(BelongsToHost))),
            ((false, false, true, false), Ok(Some(BelongsToHost))),
            ((true, true, false, false), Ok(None)),
            ((true, false, true, false), Ok(None)),
            ((false, true, true, false), Ok(None)),
            ((true, true, true, false), Ok(None)),
            ((false, false, false, true), Ok(Some(TakenBySystem))),
            ((true, false, false, true), Ok(Some(TakenBySystem))),
            ((false, true, false, true), Ok(Some(TakenBySystem))),
            ((false, false, true, true), Ok(Some(TakenBySystem))),
            ((true, true, false, true), Ok(Some(TakenBySystem))),
            ((true, false, true, true), Ok(Some(TakenBySystem))),
            ((false, true, true, true), Ok(Some(TakenBySystem))),
            ((true, true, true, true), Ok(Some(TakenBySystem))),
        ];
        for ((shift, control, alt, win), want) in expected {
            assert_eq!(
                cell(shift, control, alt, win),
                want,
                "shift={shift} control={control} alt={alt} win={win}"
            );
        }
        // A syllable letter is the constructor's refusal, before the policy,
        // and only while no host chord is held.
        assert_eq!(
            ComposingKeyChord::make(Some("s"), KeyModifiers::NONE).map(|_| ()),
            Err(TypesRomanization)
        );
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
            None,
            "Ctrl+Alt carries the Mac's ⌃⌘ roster: a preserved key lives only \
             while this TIP does, so an AltGr layout is never the one in force"
        );
        assert_eq!(
            global_rejection(&chord("s", KeyModifiers::ALT.with(KeyModifiers::SHIFT))),
            None
        );
        assert_eq!(
            global_rejection(&chord("[", KeyModifiers::NONE)),
            None,
            "a bare punctuation key is recordable"
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

        // A global row left on a typing key — a bare `z` recorded before the
        // eight non-syllable letters were refused, or a Shift+3 — is cleared
        // (written as cleared, not merely read as unparsable); Ctrl+z and
        // Ctrl+3 are ordinary chords and stay.
        let mut doc = SettingsDocument::default();
        let settings_row = ShortcutAction::OpenLastSettingsPane.settings_key_name();
        doc.set_raw_string(&settings_row, "|007A");
        let translate_row = ShortcutAction::ToggleTranslateSwapped.settings_key_name();
        doc.set_raw_string(&translate_row, "s|0033");
        ShortcutAction::ToggleRomanization
            .store_in(&mut doc, Some(&chord("z", KeyModifiers::CONTROL)));
        ShortcutAction::CycleCandidateDisplayMode
            .store_in(&mut doc, Some(&chord("3", KeyModifiers::CONTROL)));
        ShortcutConflicts::resolve_across_registries(&mut doc);
        assert_eq!(
            doc.raw_string(&settings_row),
            Some(keys::CLEARED_COMPOSING_CHORD),
            "bare z cleared"
        );
        assert_eq!(
            doc.raw_string(&translate_row),
            Some(keys::CLEARED_COMPOSING_CHORD),
            "Shift+3 cleared"
        );
        assert_eq!(
            ShortcutAction::ToggleRomanization.chord_in(&doc),
            Some(chord("z", KeyModifiers::CONTROL))
        );
        assert_eq!(
            ShortcutAction::CycleCandidateDisplayMode.chord_in(&doc),
            Some(chord("3", KeyModifiers::CONTROL))
        );
        let before = doc.clone();
        ShortcutConflicts::resolve_across_registries(&mut doc);
        assert_eq!(doc.to_json(), before.to_json(), "idempotent");

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
    fn composing_recording_clears_the_global_row_that_held_the_chord() {
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
    }
}
