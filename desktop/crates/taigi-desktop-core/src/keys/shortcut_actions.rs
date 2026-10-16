//! The user-configurable GLOBAL shortcuts: which actions exist, their
//! defaults, the extra refusals a global row has, and how the two
//! registries — global chords and composing chords — are kept from
//! colliding — for Windows and Linux. The Mac's global tier stays in Swift
//! over `KeyboardShortcuts`, a twin on the same rules: `ShortcutActions.swift`
//! (`ShortcutAction`, `ShortcutConflicts`) and `GlobalShortcutPolicy`
//! (`ShortcutKeyRecorder.swift`).
//!
//! On the Mac the global tier stores Carbon key codes and needs a bridge to
//! compare with the composing tier's characters; on Windows both tiers store
//! the same [`ComposingKeyChord`] (the TSF shell maps a chord's character to
//! a virtual key when it registers the preserved key), so the bridge is the
//! identity and every conflict rule is a pure function over one settings
//! document.

use super::action::{ChordField, ComposingAction};
use super::bindings::ComposingKeyBindings;
use super::chord::{ChordRejection, ComposingKeyChord};
use super::snapshot::{KeyEventSnapshot, KeyModifiers};
use crate::platform::DesktopPlatform;
use crate::settings::{keys, InputMode, SettingsDocument};
use crate::strings::StringKey;

/// One user-assignable global action. The list is the single source for the
/// recorder rows, the preserved-key registration and the menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ShortcutAction {
    ToggleRomanization,
    /// TPS ↔ the romanization last used (`InputModeRequest::ToggleTps`).
    /// Its own row, not a third stop on the romanization switch: TPS is
    /// not a romanization (desktop TPS roadmap U2, D5).
    ToggleTps,
    ToggleTranslateSwapped,
    /// Steps Candidate Display through its picker order (`CandidateDisplayMode::next`).
    CycleCandidateDisplayMode,
    /// Opens the symbol picker over the caret.
    ShowSymbolPicker,
    /// Toggles the floating Telex key table (`ui/telex_guide.rs`): explains
    /// the keyboard rather than changing what it types, but is read while
    /// typing.
    ShowTelexGuide,
    /// Shows / hides the on-screen TPS key panel (desktop TPS roadmap D6):
    /// under TPS only, where the keys type the glyphs it draws.
    ShowTpsKeyboard,
    /// Opens the settings window on whichever pane the user left it on. Last
    /// on the pane (USER 2026-09-21): the one row that leaves the typing
    /// session. The Mac's Swift twin keeps the same order
    /// (`ShortcutActions.swift` `ShortcutAction`).
    OpenLastSettingsPane,
}

impl ShortcutAction {
    pub const ALL: [ShortcutAction; 8] = [
        Self::ToggleRomanization,
        Self::ToggleTps,
        Self::ToggleTranslateSwapped,
        Self::CycleCandidateDisplayMode,
        Self::ShowSymbolPicker,
        Self::ShowTelexGuide,
        Self::ShowTpsKeyboard,
        Self::OpenLastSettingsPane,
    ];

    /// The roster split into the blocks the settings pane draws under the
    /// composing rows (USER 2026-10-16): Romanization, the switches about a
    /// romanization's candidates; Phonetic Symbols, the TPS switch and its
    /// key panel; Other, the symbol picker, the Telex guide and the settings
    /// window.
    /// Each block keeps `ALL`'s order. Written out so adding a case has to
    /// say where it goes. The Mac's Swift twin: `ShortcutAction.groups`.
    pub const GROUPS: [&'static [ShortcutAction]; 3] = [
        &[
            Self::ToggleRomanization,
            Self::ToggleTranslateSwapped,
            Self::CycleCandidateDisplayMode,
        ],
        &[Self::ToggleTps, Self::ShowTpsKeyboard],
        &[
            Self::ShowSymbolPicker,
            Self::ShowTelexGuide,
            Self::OpenLastSettingsPane,
        ],
    ];

    /// The pane's title of each block in `GROUPS`.
    pub const GROUP_TITLES: [StringKey; 3] = [
        StringKey::SettingsOutputScriptRoman,
        StringKey::SettingsTpsMode,
        StringKey::CommonOther,
    ];

    /// Whether the action does nothing under `mode`, writing no setting:
    /// under TPS a cell is the Hanji alone, so the Hanji / romanization swap
    /// and the display modes have nothing to switch, and there are no Telex
    /// keys for the guide to show (roadmap § D3). The TPS key panel is the
    /// mirror case: nothing to show outside TPS, whose glyphs it draws (D6).
    pub fn is_inert_under(self, mode: InputMode) -> bool {
        match self {
            Self::ToggleTranslateSwapped
            | Self::CycleCandidateDisplayMode
            | Self::ShowTelexGuide => mode == InputMode::Tps,
            Self::ShowTpsKeyboard => mode != InputMode::Tps,
            Self::ToggleRomanization
            | Self::ToggleTps
            | Self::ShowSymbolPicker
            | Self::OpenLastSettingsPane => false,
        }
    }

    /// The global action `snapshot` is, if its recorded chord matches — read
    /// before the composing classifier, whether or not a composition runs.
    pub fn matching(
        snapshot: &KeyEventSnapshot,
        settings: &SettingsDocument,
        platform: DesktopPlatform,
    ) -> Option<Self> {
        Self::ALL.into_iter().find(|action| {
            action
                .chord_in(settings, platform)
                .is_some_and(|chord| chord.matches(snapshot, platform))
        })
    }

    pub fn raw(self) -> &'static str {
        match self {
            Self::OpenLastSettingsPane => "openLastSettingsPane",
            Self::ToggleRomanization => "toggleRomanization",
            Self::ToggleTps => "toggleTps",
            Self::ToggleTranslateSwapped => "toggleTranslateSwapped",
            Self::CycleCandidateDisplayMode => "cycleCandidateDisplayMode",
            Self::ShowSymbolPicker => "showSymbolPicker",
            Self::ShowTelexGuide => "showTelexGuide",
            Self::ShowTpsKeyboard => "showTpsKeyboard",
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

    /// Whether a held chord must fire ONCE: the guide, the picker and the TPS
    /// key panel toggle,
    /// so an auto-repeat would flip them back. The switches read as one
    /// press already (a switch repeated is a switch back — left as is).
    pub fn fires_once_per_press(self) -> bool {
        matches!(
            self,
            Self::ShowTelexGuide | Self::ShowSymbolPicker | Self::ShowTpsKeyboard
        )
    }

    /// The chord a fresh install has on this action. The bare backtick is the
    /// Mac's own default and is consumed in the key sink only while the TIP is
    /// active, never registered as a preserved key.
    ///
    /// Ctrl+Alt is the Mac's ⌃⌘ under this platform's modifier mapping
    /// (`windows-guidelines.md`: ⌘→Ctrl, ⌃→Alt), so the two desktops keep one
    /// roster: ⌃⌘S → Ctrl+Alt+S for Settings, ⌃⌘C → Ctrl+Alt+C for the
    /// romanization switch, ⌃⌘H → Ctrl+Alt+H for the display-mode cycle. The
    /// letters are the Mac's reasons, unchanged — S for Settings / siat-tīng /
    /// settei, C for the bottom row a key pressed all day should sit on, and H
    /// for Hàn-Lô (Hanji/romanization), the thing the cycle switches. USER
    /// 2026-08-31: the chord logic has to match macOS's, whose Swift twin
    /// spells out the same reasons (`ShortcutActions.swift`
    /// `openLastSettingsPane`, `toggleRomanization`).
    ///
    /// Ctrl+Shift is NOT that family and its S and C are both taken —
    /// Ctrl+Shift+S is Save As in Word / Excel / LibreOffice / GIMP / Inkscape
    /// and Save All in Visual Studio; Ctrl+Shift+C opens the DevTools element
    /// picker in Chrome / Edge. Nor is any other pair of modifiers free with
    /// these letters: Alt+Shift is the input-language-switch chord and Word's
    /// Alt+Shift+<letter> family, Win+Shift+S is Snipping Tool and Win+Ctrl+S is
    /// speech recognition (and Win chords are refused outright below).
    /// Ctrl+Alt+S is JetBrains' own Settings chord, i.e. an existing Windows
    /// convention for exactly this command. What it does cost, and what a
    /// user rebinds away from if it bites: Word's Split, Visual Studio's
    /// Server Explorer (S) and Call Stack (C), and Teams' see-all-chats (C).
    ///
    /// `/` for the Telex guide is the Mac's ⌃⌘/ carried over the same way:
    /// `/` is the key help lives on (`?` is Shift+/, and every app that
    /// answers "which keys do what" answers it there). Not Ctrl+Alt+T, the
    /// mnemonic first reached for — JetBrains binds it to Surround With, and
    /// a user in an IDE would lose one or the other (the Swift twin,
    /// `ShortcutActions.swift` `showTelexGuide`, says the same). On a layout where `/`
    /// itself needs Shift, the preserved key registers with that Shift
    /// OR-ed in (`preserved_key`) and still fires; only the key sink's
    /// fallback — for hosts that bypass preserved keys — cannot match
    /// such a press, because the stored chord names `/` unshifted.
    ///
    /// `P` for the TPS switch — Phonetic Symbols — is the Mac's ⌃⌘P
    /// (desktop TPS roadmap D5). Not Ctrl+Alt+T, GNOME's terminal chord.
    ///
    /// `J` for the TPS key panel is the Mac's ⌃⌘J, the home-row key beside
    /// the TPS switch's hand. Not ⌃⌘K, the roadmap's first pick: Apple Notes
    /// binds it (maintainer 2026-10-04).
    ///
    /// `,` for the symbol picker is the Mac's ⌃⌘, carried over the same way:
    /// the picker is a punctuation menu and the comma is the punctuation key.
    /// Not the bare backtick 新注音 (New Phonetic) / McBopomofo / vChewing open their symbol
    /// menus on: that key is Hanji/Romanization Swap here, and stays (USER 2026-09-09:
    /// "do not change the ` shortcut; it is the consensus among Taigi input methods").
    ///
    /// Built as the value the gate would produce rather than through it, since
    /// the gate is per desktop; `defaults_pass_the_gate_on_every_desktop`
    /// (`keys/action.rs`) pins that it lets each one through unchanged. These are the Windows /
    /// Linux global tier's own defaults — the Mac's global rows stay in Swift.
    ///
    /// Changing a default here moves every install that never recorded the row:
    /// nothing writes a default into `settings.json`, so an absent key IS the
    /// default (`chord_in`). No migration flag, unlike the Mac's.
    pub fn default_chord(self) -> ComposingKeyChord {
        let (key, modifiers) = match self {
            Self::OpenLastSettingsPane => ("s", KeyModifiers::CONTROL.with(KeyModifiers::ALT)),
            Self::ToggleRomanization => ("c", KeyModifiers::CONTROL.with(KeyModifiers::ALT)),
            Self::ToggleTps => ("p", KeyModifiers::CONTROL.with(KeyModifiers::ALT)),
            Self::ToggleTranslateSwapped => ("`", KeyModifiers::NONE),
            Self::CycleCandidateDisplayMode => ("h", KeyModifiers::CONTROL.with(KeyModifiers::ALT)),
            Self::ShowSymbolPicker => (",", KeyModifiers::CONTROL.with(KeyModifiers::ALT)),
            Self::ShowTelexGuide => ("/", KeyModifiers::CONTROL.with(KeyModifiers::ALT)),
            Self::ShowTpsKeyboard => ("j", KeyModifiers::CONTROL.with(KeyModifiers::ALT)),
        };
        ComposingKeyChord {
            key: key.to_owned(),
            modifiers,
        }
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
            Self::ToggleTps => StringKey::DesktopShortcutToggleTps,
            Self::ToggleTranslateSwapped => StringKey::CommonToggleTranslateSwapped,
            Self::CycleCandidateDisplayMode => StringKey::DesktopShortcutCycleCandidateDisplayMode,
            Self::ShowSymbolPicker => StringKey::DesktopShortcutShowSymbolPicker,
            Self::ShowTelexGuide => StringKey::DesktopShortcutShowTelexGuide,
            Self::ShowTpsKeyboard => StringKey::DesktopShortcutShowTpsKeyboard,
        }
    }

    /// The chord the document holds for this action, or `None` for a cleared
    /// or unparsable row.
    pub fn chord_in(
        self,
        document: &SettingsDocument,
        platform: DesktopPlatform,
    ) -> Option<ComposingKeyChord> {
        self.translation_in(document, platform).and_then(Result::ok)
    }

    /// The bridge with its refusal kept: `None` for a cleared, absent-default
    /// or unreadable row; `Some(Err)` for a stored value the gate refuses.
    /// The launch pass reads WHY a row fails to translate, because a row on a
    /// typing key is one the recorder would refuse today and the preserved
    /// key would still be dispatched first. macOS keeps a Swift twin:
    /// `ShortcutActions.swift` `translation(of:)`.
    fn translation_in(
        self,
        document: &SettingsDocument,
        platform: DesktopPlatform,
    ) -> Option<Result<ComposingKeyChord, ChordRejection>> {
        match document.raw_string(&self.settings_key_name()) {
            None => Some(Ok(self.default_chord())),
            Some(raw) => ComposingKeyChord::translate_raw(raw, platform),
        }
    }

    /// Records `chord` on this action, or clears the row.
    pub fn store_in(
        self,
        document: &mut SettingsDocument,
        chord: Option<&ComposingKeyChord>,
        platform: DesktopPlatform,
    ) {
        let value = chord.map_or_else(
            || keys::CLEARED_COMPOSING_CHORD.to_owned(),
            |chord| chord.raw_value(platform),
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
    // Ctrl+Alt is allowed on both tiers (`evaluate_press`). It is AltGr on
    // layouts that have one, and a binding would take the glyph such a
    // layout types with it — but a global chord answers only while this Taiwanese TIP is the
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
        platform: DesktopPlatform,
    ) -> Vec<ShortcutAction> {
        let Some(recorded) = changed.chord_in(document, platform) else {
            return Vec::new();
        };
        ShortcutAction::ALL
            .into_iter()
            .filter(|other| {
                *other != changed && other.chord_in(document, platform).as_ref() == Some(&recorded)
            })
            .collect()
    }

    /// Which global actions hold a chord only because it is their default,
    /// while another global action holds the same chord because the user
    /// recorded it there (the upgrade case).
    pub fn defaults_shadowed_by_recordings(
        document: &SettingsDocument,
        platform: DesktopPlatform,
    ) -> Vec<ShortcutAction> {
        let held: Vec<(ShortcutAction, Option<ComposingKeyChord>)> = ShortcutAction::ALL
            .into_iter()
            .map(|action| (action, action.chord_in(document, platform)))
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
        platform: DesktopPlatform,
    ) {
        for loser in Self::conflicting_global_actions(document, changed, platform) {
            loser.store_in(document, None, platform);
        }
        let Some(chord) = changed.chord_in(document, platform) else {
            return;
        };
        let bindings = ComposingKeyBindings::from_document(document, platform);
        for (action, field) in
            Self::composing_fields_holding(document, &bindings, &chord, None, platform)
        {
            document.set_composing_chord(action, field, None, platform);
        }
    }

    /// A composing recording just landed on `changed` with `chord`: take the
    /// chord off every other composing field and every global row that held it.
    pub fn resolve_after_composing_recording(
        document: &mut SettingsDocument,
        changed: (ComposingAction, ChordField),
        chord: &ComposingKeyChord,
        platform: DesktopPlatform,
    ) {
        let bindings = ComposingKeyBindings::from_document(document, platform);
        for (action, field) in
            Self::composing_fields_holding(document, &bindings, chord, Some(changed), platform)
        {
            document.set_composing_chord(action, field, None, platform);
        }
        for action in ShortcutAction::ALL {
            if action.chord_in(document, platform).as_ref() == Some(chord) {
                action.store_in(document, None, platform);
            }
        }
    }

    /// Reconciles the two registries at launch, where no recorder ran.
    /// Recording-beats-default; when BOTH sides are recordings the GLOBAL
    /// tier wins (it is the tier that fires first — a preserved key is
    /// dispatched before the classifier ever runs). A global row left on a
    /// typing key is cleared first. Idempotent.
    pub fn resolve_across_registries(document: &mut SettingsDocument, platform: DesktopPlatform) {
        for loser in Self::defaults_shadowed_by_recordings(document, platform) {
            loser.store_in(document, None, platform);
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
        // macOS keeps a Swift twin: `ShortcutActions.swift`
        // `resolveAcrossRegistries`.
        for action in ShortcutAction::ALL {
            if action.translation_in(document, platform)
                == Some(Err(ChordRejection::TypesRomanization))
            {
                action.store_in(document, None, platform);
            }
        }
        // Every holder is read off the document as it stands before any
        // clearing, so one row's clearing cannot change who else collides.
        let bindings = ComposingKeyBindings::from_document(document, platform);
        let held: Vec<_> = ShortcutAction::ALL
            .into_iter()
            .filter_map(|action| {
                let chord = action.chord_in(document, platform)?;
                let holders =
                    Self::composing_fields_holding(document, &bindings, &chord, None, platform);
                Some((action, chord, holders))
            })
            .collect();
        for (action, chord, holders) in held {
            let global_is_default = chord == action.default_chord();
            for (holder, field) in holders {
                let composing_recording_outranks =
                    global_is_default && Some(&chord) != holder.shipped_chord(field).as_ref();
                if composing_recording_outranks {
                    action.store_in(document, None, platform);
                } else {
                    document.set_composing_chord(holder, field, None, platform);
                }
            }
        }
    }

    /// Every composing field holding `chord`, other than `excluding`: the
    /// holders the resolver shows, and any recording it hid behind them — a
    /// chord stored on two fields by hand, or by an older build that never
    /// read the second field. Clearing only the shown holder would let the
    /// hidden one resurface on the next read.
    pub fn composing_fields_holding(
        document: &SettingsDocument,
        bindings: &ComposingKeyBindings,
        chord: &ComposingKeyChord,
        excluding: Option<(ComposingAction, ChordField)>,
        platform: DesktopPlatform,
    ) -> Vec<(ComposingAction, ChordField)> {
        ComposingAction::all_fields()
            .filter(|key| Some(*key) != excluding)
            .filter(|&(action, field)| {
                let stored = document.stored_composing_chord(action, field, platform);
                bindings.chord(action, field) == Some(chord)
                    || stored.flatten().as_ref() == Some(chord)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::TEST_PLATFORM as PLATFORM;

    fn chord(key: &str, modifiers: KeyModifiers) -> ComposingKeyChord {
        ComposingKeyChord::make(Some(key), modifiers, PLATFORM).unwrap()
    }

    #[test]
    fn roster_defaults_and_keys() {
        // trace: `default_chord` — the Mac's ⌃⌘ roster as Ctrl+Alt (S, C, H, `,`,
        // `/`, P, J), the bare backtick unchanged; eight names, eight distinct defaults,
        // each recordable and clear of the composing defaults.
        let mut names: Vec<_> = ShortcutAction::ALL
            .iter()
            .map(|a| a.settings_key_name())
            .collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), 8);
        assert_eq!(
            ShortcutAction::OpenLastSettingsPane.default_chord(),
            chord("s", KeyModifiers::CONTROL.with(KeyModifiers::ALT))
        );
        assert_eq!(
            ShortcutAction::ToggleRomanization.default_chord(),
            chord("c", KeyModifiers::CONTROL.with(KeyModifiers::ALT))
        );
        assert_eq!(
            ShortcutAction::ToggleTps.default_chord(),
            chord("p", KeyModifiers::CONTROL.with(KeyModifiers::ALT))
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
        assert_eq!(
            ShortcutAction::ShowTpsKeyboard.default_chord(),
            chord("j", KeyModifiers::CONTROL.with(KeyModifiers::ALT))
        );
        let mut defaults: Vec<_> = ShortcutAction::ALL
            .iter()
            .map(|a| a.default_chord())
            .collect();
        defaults.sort();
        defaults.dedup();
        assert_eq!(defaults.len(), 8, "the defaults are all different");
        for action in ShortcutAction::ALL {
            assert_eq!(
                global_rejection(&action.default_chord()),
                None,
                "{action:?}'s default must be recordable"
            );
        }
        let composing_defaults: Vec<_> = ComposingAction::all_fields()
            .filter_map(|(action, field)| action.shipped_chord(field))
            .collect();
        for action in ShortcutAction::ALL {
            assert!(
                !composing_defaults.contains(&action.default_chord()),
                "{action:?} collides with a composing default"
            );
        }
    }

    #[test]
    fn groups_hold_every_action_exactly_once_in_roster_order() {
        let grouped: Vec<_> = ShortcutAction::GROUPS
            .iter()
            .flat_map(|group| group.iter().copied())
            .collect();
        let mut unique = grouped.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), ShortcutAction::ALL.len());
        assert_eq!(grouped.len(), ShortcutAction::ALL.len());
        for group in ShortcutAction::GROUPS {
            let in_roster: Vec<_> = ShortcutAction::ALL
                .into_iter()
                .filter(|action| group.contains(action))
                .collect();
            assert_eq!(in_roster, group);
        }
    }

    #[test]
    fn roster_order_is_the_pane_order() {
        // `ALL` is the global recorder rows' order inside each block of
        // `GROUPS` (`pages/shortcuts.rs`, under the composing rows) and the
        // Mac's `allCases`: the switches in the General pane's order of what they
        // switch — the input script twice, romanization then TPS — then the
        // windows used while typing, the settings window last (USER
        // 2026-09-21). The Mac's roster gains Switch TPS in desktop TPS P4
        // and the TPS key panel in P5.
        assert_eq!(
            ShortcutAction::ALL,
            [
                ShortcutAction::ToggleRomanization,
                ShortcutAction::ToggleTps,
                ShortcutAction::ToggleTranslateSwapped,
                ShortcutAction::CycleCandidateDisplayMode,
                ShortcutAction::ShowSymbolPicker,
                ShortcutAction::ShowTelexGuide,
                ShortcutAction::ShowTpsKeyboard,
                ShortcutAction::OpenLastSettingsPane,
            ]
        );
        assert_eq!(ShortcutAction::ShowSymbolPicker.raw(), "showSymbolPicker");
        assert_eq!(
            ShortcutAction::ShowSymbolPicker.label_key(),
            StringKey::DesktopShortcutShowSymbolPicker
        );
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
                ShortcutAction::ShowTelexGuide,
                ShortcutAction::ShowTpsKeyboard,
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
        assert_eq!(ShortcutAction::ShowTelexGuide.raw(), "showTelexGuide");
        assert_eq!(
            ShortcutAction::ShowTelexGuide.label_key(),
            StringKey::DesktopShortcutShowTelexGuide
        );
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
            ComposingKeyChord::make(Some("["), modifiers, PLATFORM)
                .map(|chord| global_rejection(&chord))
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
            ComposingKeyChord::make(Some("s"), KeyModifiers::NONE, PLATFORM).map(|_| ()),
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
        // trace: `conflicting_global_actions` reports the other holder;
        // `resolve_after_global_recording` clears it (last writer wins).
        let mut doc = SettingsDocument::default();
        let shared = chord("k", KeyModifiers::CONTROL.with(KeyModifiers::SHIFT));
        ShortcutAction::OpenLastSettingsPane.store_in(&mut doc, Some(&shared), PLATFORM);
        ShortcutAction::ToggleRomanization.store_in(&mut doc, Some(&shared), PLATFORM);
        assert_eq!(
            ShortcutConflicts::conflicting_global_actions(
                &doc,
                ShortcutAction::ToggleRomanization,
                PLATFORM
            ),
            vec![ShortcutAction::OpenLastSettingsPane]
        );
        ShortcutConflicts::resolve_after_global_recording(
            &mut doc,
            ShortcutAction::ToggleRomanization,
            PLATFORM,
        );
        assert_eq!(
            ShortcutAction::OpenLastSettingsPane.chord_in(&doc, PLATFORM),
            None
        );
        assert_eq!(
            ShortcutAction::ToggleRomanization.chord_in(&doc, PLATFORM),
            Some(shared)
        );
        ShortcutAction::ToggleRomanization.store_in(&mut doc, None, PLATFORM);
        assert_eq!(
            ShortcutConflicts::conflicting_global_actions(
                &doc,
                ShortcutAction::ToggleRomanization,
                PLATFORM
            ),
            vec![]
        );
    }

    #[test]
    fn a_default_gives_way_to_the_same_chord_recorded_elsewhere() {
        let mut doc = SettingsDocument::default();
        let settings_default = ShortcutAction::OpenLastSettingsPane.default_chord();
        ShortcutAction::ToggleRomanization.store_in(&mut doc, Some(&settings_default), PLATFORM);
        assert_eq!(
            ShortcutConflicts::defaults_shadowed_by_recordings(&doc, PLATFORM),
            vec![ShortcutAction::OpenLastSettingsPane]
        );
        ShortcutConflicts::resolve_across_registries(&mut doc, PLATFORM);
        assert_eq!(
            ShortcutAction::OpenLastSettingsPane.chord_in(&doc, PLATFORM),
            None
        );
        assert_eq!(
            ShortcutAction::ToggleRomanization.chord_in(&doc, PLATFORM),
            Some(settings_default)
        );
    }

    #[test]
    fn launch_pass_reconciles_the_two_registries() {
        // trace: `resolve_across_registries` — recording beats default; recording
        // vs recording, the global tier wins; typing-key global rows cleared.
        // A global default shadowed by a composing recording: the recording wins.
        let mut doc = SettingsDocument::default();
        let default = ShortcutAction::ToggleRomanization.default_chord();
        doc.set_composing_chord(
            ComposingAction::PageForward,
            ChordField::First,
            Some(&default),
            PLATFORM,
        );
        ShortcutConflicts::resolve_across_registries(&mut doc, PLATFORM);
        assert_eq!(
            ShortcutAction::ToggleRomanization.chord_in(&doc, PLATFORM),
            None,
            "the default gives way"
        );
        assert_eq!(
            ComposingKeyBindings::from_document(&doc, PLATFORM)
                .chord(ComposingAction::PageForward, ChordField::First),
            Some(&default)
        );

        // The mirror image: a global recording on a composing default.
        let mut doc = SettingsDocument::default();
        let shift_tab = chord("\t", KeyModifiers::SHIFT);
        ShortcutAction::ToggleRomanization.store_in(&mut doc, Some(&shift_tab), PLATFORM);
        ShortcutConflicts::resolve_across_registries(&mut doc, PLATFORM);
        assert_eq!(
            ComposingKeyBindings::from_document(&doc, PLATFORM)
                .chord(ComposingAction::PreviousCandidate, ChordField::First),
            None
        );
        assert_eq!(
            ShortcutAction::ToggleRomanization.chord_in(&doc, PLATFORM),
            Some(shift_tab.clone())
        );

        // Recording vs recording: the global tier wins.
        let mut doc = SettingsDocument::default();
        let recorded = chord("f", KeyModifiers::CONTROL.with(KeyModifiers::ALT));
        doc.set_composing_chord(
            ComposingAction::PageForward,
            ChordField::First,
            Some(&recorded),
            PLATFORM,
        );
        ShortcutAction::ToggleRomanization.store_in(&mut doc, Some(&recorded), PLATFORM);
        ShortcutConflicts::resolve_across_registries(&mut doc, PLATFORM);
        assert_eq!(
            ShortcutAction::ToggleRomanization.chord_in(&doc, PLATFORM),
            Some(recorded)
        );
        assert_eq!(
            ComposingKeyBindings::from_document(&doc, PLATFORM)
                .chord(ComposingAction::PageForward, ChordField::First),
            None
        );
        // Idempotent.
        let before = doc.clone();
        ShortcutConflicts::resolve_across_registries(&mut doc, PLATFORM);
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
        ShortcutAction::ToggleRomanization.store_in(
            &mut doc,
            Some(&chord("z", KeyModifiers::CONTROL)),
            PLATFORM,
        );
        ShortcutAction::CycleCandidateDisplayMode.store_in(
            &mut doc,
            Some(&chord("3", KeyModifiers::CONTROL)),
            PLATFORM,
        );
        ShortcutConflicts::resolve_across_registries(&mut doc, PLATFORM);
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
            ShortcutAction::ToggleRomanization.chord_in(&doc, PLATFORM),
            Some(chord("z", KeyModifiers::CONTROL))
        );
        assert_eq!(
            ShortcutAction::CycleCandidateDisplayMode.chord_in(&doc, PLATFORM),
            Some(chord("3", KeyModifiers::CONTROL))
        );
        let before = doc.clone();
        ShortcutConflicts::resolve_across_registries(&mut doc, PLATFORM);
        assert_eq!(doc.to_json(), before.to_json(), "idempotent");

        // An uncolliding setup is left alone.
        let mut doc = SettingsDocument::default();
        ShortcutConflicts::resolve_across_registries(&mut doc, PLATFORM);
        assert_eq!(doc.revision, 0, "nothing was written");

        // Two collisions at once, and a cleared commit row is refilled.
        let mut doc = SettingsDocument::default();
        ShortcutAction::ToggleRomanization.store_in(&mut doc, Some(&shift_tab), PLATFORM);
        ShortcutAction::OpenLastSettingsPane.store_in(
            &mut doc,
            Some(&chord("]", KeyModifiers::NONE)),
            PLATFORM,
        );
        ShortcutConflicts::resolve_across_registries(&mut doc, PLATFORM);
        let bindings = ComposingKeyBindings::from_document(&doc, PLATFORM);
        assert_eq!(
            bindings.chord(ComposingAction::PreviousCandidate, ChordField::First),
            None
        );
        assert_eq!(
            bindings.chord(ComposingAction::PageForward, ChordField::First),
            None
        );
        let mut doc = SettingsDocument::default();
        let ctrl_r = chord("r", KeyModifiers::CONTROL.with(KeyModifiers::SHIFT));
        doc.set_composing_chord(
            ComposingAction::CommitLiteral,
            ChordField::First,
            Some(&ctrl_r),
            PLATFORM,
        );
        ShortcutAction::OpenLastSettingsPane.store_in(&mut doc, Some(&ctrl_r), PLATFORM);
        ShortcutConflicts::resolve_across_registries(&mut doc, PLATFORM);
        let restored = ComposingKeyBindings::from_document(&doc, PLATFORM)
            .chord(ComposingAction::CommitLiteral, ChordField::First)
            .cloned();
        assert!(
            restored.is_some(),
            "a commit row is refilled when its default is free"
        );
        assert_ne!(restored.as_ref(), Some(&ctrl_r));
    }

    #[test]
    fn launch_reconcile_treats_a_second_field_as_a_recording() {
        // trace: a second field ships empty, so any chord on it is a
        // recording. Against a global DEFAULT it outranks (the global row
        // empties); against a global RECORDING the global tier wins.
        let confirm = ComposingAction::ConfirmHighlighted;
        let global_default = ShortcutAction::ToggleRomanization.default_chord();
        let mut doc = SettingsDocument::default();
        doc.set_composing_chord(confirm, ChordField::Second, Some(&global_default), PLATFORM);
        ShortcutConflicts::resolve_across_registries(&mut doc, PLATFORM);
        assert_eq!(
            ShortcutAction::ToggleRomanization.chord_in(&doc, PLATFORM),
            None
        );
        assert_eq!(
            ComposingKeyBindings::from_document(&doc, PLATFORM).chord(confirm, ChordField::Second),
            Some(&global_default)
        );

        let recorded = chord("k", KeyModifiers::CONTROL.with(KeyModifiers::ALT));
        let mut doc = SettingsDocument::default();
        doc.set_composing_chord(confirm, ChordField::Second, Some(&recorded), PLATFORM);
        ShortcutAction::ToggleRomanization.store_in(&mut doc, Some(&recorded), PLATFORM);
        ShortcutConflicts::resolve_across_registries(&mut doc, PLATFORM);
        assert_eq!(
            ShortcutAction::ToggleRomanization.chord_in(&doc, PLATFORM),
            Some(recorded.clone())
        );
        assert_eq!(
            ComposingKeyBindings::from_document(&doc, PLATFORM).chord(confirm, ChordField::Second),
            None
        );
        let before = doc.clone();
        ShortcutConflicts::resolve_across_registries(&mut doc, PLATFORM);
        assert_eq!(doc.to_json(), before.to_json(), "idempotent");
    }

    /// A deliberate change for first-field-only data too: a chord stored on
    /// two first fields (a hand edit) shows on the later one only, and a
    /// global recording of it now clears BOTH. Before the second field, only
    /// the shown one was cleared and the hidden one came back once the global
    /// row let the chord go.
    #[test]
    fn a_global_recording_clears_a_first_field_copy_the_resolver_hid() {
        let control_k = chord("k", KeyModifiers::CONTROL.with(KeyModifiers::ALT));
        let mut doc = SettingsDocument::default();
        for action in [ComposingAction::NextCandidate, ComposingAction::PageForward] {
            doc.set_composing_chord(action, ChordField::First, Some(&control_k), PLATFORM);
        }
        ShortcutAction::ToggleRomanization.store_in(&mut doc, Some(&control_k), PLATFORM);
        ShortcutConflicts::resolve_after_global_recording(
            &mut doc,
            ShortcutAction::ToggleRomanization,
            PLATFORM,
        );
        ShortcutAction::ToggleRomanization.store_in(&mut doc, None, PLATFORM);
        let bindings = ComposingKeyBindings::from_document(&doc, PLATFORM);
        for action in [ComposingAction::NextCandidate, ComposingAction::PageForward] {
            assert_eq!(
                bindings.chord(action, ChordField::First),
                None,
                "{action:?}"
            );
        }
    }

    #[test]
    fn composing_recording_clears_the_global_row_that_held_the_chord() {
        let mut doc = SettingsDocument::default();
        let shared = chord("]", KeyModifiers::CONTROL.with(KeyModifiers::SHIFT));
        ShortcutAction::ToggleRomanization.store_in(&mut doc, Some(&shared), PLATFORM);
        doc.set_composing_chord(
            ComposingAction::PageForward,
            ChordField::First,
            Some(&shared),
            PLATFORM,
        );
        ShortcutConflicts::resolve_after_composing_recording(
            &mut doc,
            (ComposingAction::PageForward, ChordField::First),
            &shared,
            PLATFORM,
        );
        assert_eq!(
            ShortcutAction::ToggleRomanization.chord_in(&doc, PLATFORM),
            None
        );
        assert_eq!(
            ComposingKeyBindings::from_document(&doc, PLATFORM)
                .chord(ComposingAction::PageForward, ChordField::First),
            Some(&shared)
        );
    }

    #[test]
    fn the_inert_actions_under_each_mode() {
        // Under TPS the display switches and the Telex guide (no romanization
        // to switch, no Telex keys); under a romanization the TPS key panel.
        // The Swift twin asserts the same lists
        // (`testTheInertActions_underEachMode_matchTheCore`).
        let inert = |mode| -> Vec<ShortcutAction> {
            ShortcutAction::ALL
                .into_iter()
                .filter(|action| action.is_inert_under(mode))
                .collect()
        };
        assert_eq!(
            inert(InputMode::Tps),
            [
                ShortcutAction::ToggleTranslateSwapped,
                ShortcutAction::CycleCandidateDisplayMode,
                ShortcutAction::ShowTelexGuide,
            ]
        );
        for mode in [InputMode::Tl, InputMode::Poj] {
            assert_eq!(inert(mode), [ShortcutAction::ShowTpsKeyboard], "{mode:?}");
        }
    }
}
