//! What one key event means to a composition. The whole key contract of the
//! input method in one readable, testable table.

use super::action::ComposingAction;
use super::bindings::ComposingKeyBindings;
use super::snapshot::{KeyEventSnapshot, KeyModifiers, LineEdgeKey, NavigationKey};
use super::tone_input_scheme::ToneInputScheme;
use super::tps_layout::{is_layout_glyph, tps_glyph_for_event};
use crate::platform::DesktopPlatform;
use crate::settings::InputMode;

/// One move of the caret inside the composition
/// (`ComposingKeyIntent::MoveCaret`): a step, or a jump to the start / end
/// of what is still being typed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CaretDirection {
    Left,
    Right,
    Start,
    End,
}

impl From<LineEdgeKey> for CaretDirection {
    /// Home / End: the start / end of the composition.
    fn from(key: LineEdgeKey) -> Self {
        match key {
            LineEdgeKey::Home => Self::Start,
            LineEdgeKey::End => Self::End,
        }
    }
}

impl CaretDirection {
    /// The step `key` asks for, if it is one of the two horizontal arrows.
    pub fn horizontal(key: Option<NavigationKey>) -> Option<Self> {
        match key? {
            NavigationKey::LeftArrow => Some(Self::Left),
            NavigationKey::RightArrow => Some(Self::Right),
            _ => None,
        }
    }

    /// The move an arrow asks for under the caret chord: ← / → step, ↑ / ↓
    /// jump to the start / end — the host's own paragraph jump under the
    /// same modifier (⌥↑ on the Mac, Ctrl+↑ in Word).
    pub fn under_caret_chord(key: Option<NavigationKey>) -> Option<Self> {
        match key? {
            NavigationKey::UpArrow => Some(Self::Start),
            NavigationKey::DownArrow => Some(Self::End),
            other => Self::horizontal(Some(other)),
        }
    }
}

/// The modifier under which ← / → step the composing caret — the host's
/// own "jump a word" chord: Ctrl on Windows and Linux, ⌥ on the Mac
/// (⌃← is Mission Control there). The Shortcuts pane draws its read-only row from this
/// same value, so the row cannot drift from the key the classifier reads.
/// Alt+←/→ is back / forward in Explorer and the browsers and rides
/// `WM_SYSKEYDOWN`, so it is not Windows' chord.
pub fn caret_chord_modifiers(platform: DesktopPlatform) -> KeyModifiers {
    match platform {
        DesktopPlatform::Windows | DesktopPlatform::Linux => KeyModifiers::CONTROL,
        DesktopPlatform::MacOS => KeyModifiers::ALT,
    }
}

/// A move in the candidate window. The six physical keys are handed through
/// raw because what each does depends on the layout (`↓` pages a horizontal
/// window and walks a vertical list); `NextCandidate` / `PreviousCandidate`
/// name an OUTCOME — one step along the list in every layout, never a page
/// jump. The window that reads it keeps that contract; macOS keeps a Swift
/// twin of the type: `CandidatePresenter.swift` `CandidateNavigation`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CandidateNavigation {
    Left,
    Right,
    Up,
    Down,
    PageUp,
    PageDown,
    NextCandidate,
    PreviousCandidate,
}

/// The six navigation keys as directions, one-to-one on purpose: what a
/// direction DOES — walk, page, scroll, expand — belongs to the window's
/// layout, not to the classifiers that only decide the key is the window's
/// while it is up (`ComposingKeyIntent`, `SymbolPickerIntent`).
impl From<NavigationKey> for CandidateNavigation {
    fn from(key: NavigationKey) -> Self {
        match key {
            NavigationKey::LeftArrow => Self::Left,
            NavigationKey::RightArrow => Self::Right,
            NavigationKey::UpArrow => Self::Up,
            NavigationKey::DownArrow => Self::Down,
            NavigationKey::PageUp => Self::PageUp,
            NavigationKey::PageDown => Self::PageDown,
        }
    }
}

/// The composing meaning of a key event, decided before any engine call.
/// Every variant but `PassThrough` is a key the host never receives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComposingKeyIntent {
    /// A romanization character to append to the composition.
    Input(String),
    /// One of the Telex keys (`ToneInputScheme::TELEX_KEYS`), handed to the
    /// engine's `TelexKey` intent rather than appended: the engine decides
    /// which tone it writes, or which initial `z` spells in this input mode.
    TelexKey(String),
    /// One TPS key — a glyph off the layout (`tps_layout.rs`), or `" "` for
    /// Space mid-composition — handed to the engine's `TpsKey` intent, which
    /// auto-corrects at its caret and inserts, or refuses a Space on a closed
    /// syllable.
    TpsKey(String),
    DeleteBackward,
    /// Finish the composition as rendered: the literal commit under TL and
    /// POJ, the Hanji conversion as shown under TPS.
    Commit,
    /// Finish a TPS composition as typed: the glyphs of the whole
    /// composition, picks included — the Commit as Typed row under TPS
    /// (`desktop-tps-hanji-conversion-roadmap.md` B2).
    CommitAsTyped,
    /// Abandon the composition without writing to the document.
    Cancel,
    /// Finish the composition and write `text` after it, as one step.
    CommitThenInsert(String),
    /// The host must receive this event, after the composition is finished
    /// into the document first (khiin's ignored-event path).
    CommitThenPassThrough,
    /// Not ours, and nothing to finish first.
    PassThrough,
    /// Move the candidate window's selection.
    Navigate(CandidateNavigation),
    /// Fetch the candidates and put the window up on the first one — under
    /// TPS, where the window opens on demand (desktop TPS roadmap D7).
    OpenCandidates,
    /// Take the window down and keep composing — Escape over a TPS window.
    CloseCandidates,
    /// Move the caret inside the romanization being typed, so the next
    /// character lands there — `ka2`, Ctrl+← Ctrl+←, `h` → `kha2`; Ctrl+↑ /
    /// Ctrl+↓ jump to the start / end of what is still pending. The
    /// engine owns the caret (`MoveCaret`); the window, if up, is left
    /// exactly as it is under TL / POJ and taken down under TPS (D7).
    MoveCaret(CaretDirection),
    /// Commit whichever candidate the window has highlighted, as the output
    /// settings render it.
    CommitHighlightedCandidate,
    /// Commit the highlighted candidate in the script the output settings do
    /// NOT lead with — the Hanji/romanization key.
    CommitAlternateScript,
    /// Commit the candidate in this slot of the visible page, counting from
    /// zero — what the slot keys address (`CandidateSlotKeySet`: the bare
    /// letters under Standard, the bare digits under Telex). With `flip`,
    /// in the script the cell does NOT stand for — the Hanji/romanization key aimed at a
    /// slot instead of at the highlight, which is what Shift on the same
    /// key asks (USER 2026-09-10).
    SelectCandidateSlot {
        slot: usize,
        flip: bool,
    },
}

impl ComposingKeyIntent {
    /// Classifies `key` for a session whose composition is or is not active,
    /// and whose candidate window is or is not on screen.
    ///
    /// `is_composing` changes the meaning of most keys: Return, Escape, Space
    /// and the digits are the composition's while it runs and the host's the
    /// rest of the time. Under TL and POJ a bare digit never starts a
    /// composition — under Standard digits are the numeric tone markers
    /// (`tai5`), and under Telex a tone letter has nothing to mark when idle.
    /// Under TPS every layout key starts one, number-row digits included.
    ///
    /// `is_showing_candidates` is the second state a key turns on: the
    /// arrows, the paging keys, Space and the slot keys belong to the window
    /// while it is up and to the host or the document the rest of the time.
    pub fn intent(
        key: &KeyEventSnapshot,
        is_composing: bool,
        is_showing_candidates: bool,
        bindings: &ComposingKeyBindings,
        platform: DesktopPlatform,
    ) -> Self {
        let modifiers = key.modifiers;

        // Tier 0 — the caret inside the composition, on Ctrl+← / Ctrl+→
        // (⌥ on the Mac), and to its start / end on Ctrl+↑ / Ctrl+↓
        // (Discord request, maintainer 2026-10-05). Fixed, not recordable,
        // shown read-only on the Shortcuts pane (USER 2026-09-09). Exactly
        // the one modifier: Ctrl+Shift+← stays the host's selection,
        // Ctrl+Alt+← its shortcut. Idle, the chord is the host's.
        if is_composing && modifiers == caret_chord_modifiers(platform) {
            if let Some(direction) = CaretDirection::under_caret_chord(key.navigation_key) {
                return Self::MoveCaret(direction);
            }
        }
        // Bare Home / End do the same jump (maintainer 2026-10-05): the
        // convention of Microsoft IME, Kotoeri, McBopomofo and librime
        // (`references/mozc/src/data/keymap/ms-ime.tsv`, `kotoeri.tsv`,
        // `McBopomofo/Source/KeyHandler.mm` `isHome`,
        // `librime/src/rime/gear/navigator.cc`). With a modifier — Shift+Home
        // selects — and idle, they stay the host's.
        if is_composing && modifiers.is_empty() {
            if let Some(edge) = key.line_edge_key {
                return Self::MoveCaret(edge.into());
            }
        }

        // Tier 1 — fixed navigation, read before anything the user can
        // rebind so no binding can shadow it. Shift excluded: Shift+← extends
        // a selection.
        if is_showing_candidates && !modifiers.shift && !modifiers.has_host_chord() {
            if let Some(navigation) = key.navigation_key {
                return Self::navigate(navigation);
            }
        }
        // Tier 2 — Escape and Backspace (`\u{8}` from Backspace, `\u{7F}` from
        // the Mac's Delete key; a Ctrl chord carrying either is the host's,
        // caught by the host-chord check first).
        if !modifiers.has_host_chord() {
            if let Some(first) = key.characters.as_deref().and_then(|s| s.chars().next()) {
                match first {
                    // Under TPS the first Escape only closes the window the
                    // user opened (D7); the next cancels, as everywhere.
                    '\u{1B}' if is_showing_candidates && bindings.input_mode == InputMode::Tps => {
                        return Self::CloseCandidates
                    }
                    '\u{1B}' => return Self::composition_or_host(is_composing, Self::Cancel),
                    '\u{8}' | '\u{7F}' => {
                        return Self::composition_or_host(is_composing, Self::DeleteBackward)
                    }
                    _ => {}
                }
            }
        }
        // TPS — ahead of the slot keys, the bindings and the typing tier, so
        // neither a binding (Space's included) nor a slot set takes a key the
        // layout types (`desktop-tps-roadmap.md` § D3, D7).
        if bindings.input_mode == InputMode::Tps {
            let intent =
                Self::tps_window_key(key, is_composing, is_showing_candidates, bindings, platform)
                    .or_else(|| Self::tps_key(key, is_composing));
            if let Some(intent) = intent {
                return intent;
            }
        }
        // Tier 3 — the slot keys, read before the user's bindings so no
        // binding can shadow it. Which keys pick follows from the tone scheme
        // (`ToneInputScheme::slot_key_set`); both sets are bare keys, so a
        // Ctrl+3 keeps falling through to the host-chord guard below.
        // Exactly Shift aims the Hanji/romanization commit at the slot instead
        // (`CandidateSlotKeySet::shifted_slot_for_event`).
        if is_showing_candidates {
            let slot_keys = bindings.slot_key_set();
            if let Some(slot) = slot_keys.slot_for_event(key) {
                return Self::SelectCandidateSlot { slot, flip: false };
            }
            if let Some(slot) = slot_keys.shifted_slot_for_event(key) {
                return Self::SelectCandidateSlot { slot, flip: true };
            }
        }
        // Tier 4 — what the user put on this key, read before the host-chord
        // guard so a chord they deliberately recorded reaches its action.
        if is_composing {
            // Under TPS there is no other script (the engine's `Other` would
            // write raw TL), so Output the Other Script is inert wherever the
            // user put it.
            let action = bindings.action_for(key, platform).filter(|action| {
                bindings.input_mode != InputMode::Tps
                    || *action != ComposingAction::CommitAlternateScript
            });
            if let Some(action) = action {
                if is_showing_candidates || !action.requires_candidates() {
                    return action.intent();
                }
                // With the window switched off there is never a candidate
                // to confirm, swap or page to, so every key that would ends
                // the composition as typed instead — the romanization with
                // its tone marks (`tai5` → `tâi`), the same commit
                // Shift+Enter makes. Paging keys included: "any candidate
                // key commits" is one rule the user can hold. Only while the
                // window is ON does a candidate key with no window up fall
                // through to the host below.
                if !bindings.is_candidate_window_enabled {
                    return Self::Commit;
                }
            }
        }
        // Under TPS, Ctrl on a punctuation key types its full-width mark
        // (`tps_punctuation_chord`): the bare key types a glyph there. Below
        // the bindings, so a chord the user recorded on Ctrl+, still reaches
        // its action; above the host guard, because this is the one Ctrl
        // chord that is this input method's. Outside TPS it is the host's.
        if let Some(punctuation) = Self::tps_punctuation_chord(key, bindings.input_mode) {
            return Self::composition_or_host(
                is_composing,
                Self::CommitThenInsert(punctuation.to_string()),
            );
        }
        // Tier 5 — Control, Alt and Win chords are the host's shortcuts, mid-
        // composition too: swallowing Ctrl+S would cost the user their save.
        if modifiers.has_host_chord() {
            return Self::host_key(is_composing);
        }
        let Some(characters) = key.characters.as_deref().filter(|s| !s.is_empty()) else {
            return Self::host_key(is_composing);
        };
        // Tier 6 — keys the platform names that nothing above claimed. Return
        // and Tab reach here when the user moved every action off them.
        if key.is_named_special_key {
            return Self::host_key(is_composing);
        }
        if !characters.chars().all(Self::is_text_scalar) {
            return Self::host_key(is_composing);
        }
        // Tier 7 — text. Under TPS every key the layout types was taken
        // above, so the rest is document text.
        if bindings.input_mode == InputMode::Tps {
            return Self::composition_or_host(
                is_composing,
                Self::CommitThenInsert(characters.to_owned()),
            );
        }
        // Under Telex the tone letters and `f` are the
        // engine's, not the composition's text. A tone letter or `f` typed
        // outside a composition is document text (like an idle digit): there
        // is no syllable for it to mark. `z` types an initial, so it starts
        // one. One scalar only, for the same grapheme reason as the
        // romanization rule below; macOS asks the same since P11c (roadmap
        // E2).
        if bindings.tone_scheme == ToneInputScheme::Telex {
            let mut scalars = characters.chars();
            if let (Some(first), None) = (scalars.next(), scalars.next()) {
                if ToneInputScheme::is_telex_key(first) {
                    if is_composing || ToneInputScheme::starts_composition(first) {
                        return Self::TelexKey(characters.to_owned());
                    }
                    return Self::PassThrough;
                }
            }
        }
        // Under Standard a digit mid-composition is always the tone marker,
        // whatever the buffer looks like — even after `tai5` (§10.2). Under
        // Telex the digits ARE the slot keys, taken above while the window is
        // up; with no window a digit falls through to the punctuation rule
        // and commits the composition ahead of itself.
        //
        // The WHOLE string has to be romanization, not just its first scalar:
        // the first `char` of `a` + a combining mark would be the bare `a`,
        // and the engine would be handed a string it cannot parse (Codex
        // PR2b review). The same holds for `a.`, which is document text;
        // macOS asks every character too since P11c (roadmap E2).
        let digits_are_tones = is_composing && bindings.tone_scheme == ToneInputScheme::Standard;
        let is_romanization = characters.chars().all(|c| {
            Self::is_romanization_character(c) || (digits_are_tones && Self::is_tone_digit(c))
        });
        if is_romanization {
            return Self::Input(characters.to_owned());
        }
        // Everything else printable — space, punctuation, another script —
        // is document text: it ends the composition it was typed after.
        if is_composing {
            Self::CommitThenInsert(characters.to_owned())
        } else {
            Self::PassThrough
        }
    }

    /// What a key does to a TPS composition and its candidate window, which
    /// opens on demand (D7), or `None` for a key that types or falls to the
    /// shared tiers. With the window up the number row and the keypad pick
    /// and Space confirms. The Commit as Typed row writes the glyphs of the
    /// whole composition, window up or not (Hanji conversion B2). While
    /// typing, plain ← → step the caret over the converted words (H3); the
    /// other keys that would move through a window open it — ↑ ↓, the paging
    /// keys and whatever the navigation and paging rows are bound to — for
    /// the word before the caret (H4); the Confirm row commits the
    /// composition as shown. With the window switched off nothing opens, and
    /// a key that would have opened it commits and goes on to the host.
    fn tps_window_key(
        key: &KeyEventSnapshot,
        is_composing: bool,
        is_showing_candidates: bool,
        bindings: &ComposingKeyBindings,
        platform: DesktopPlatform,
    ) -> Option<Self> {
        if is_showing_candidates {
            if let Some(slot) = bindings.slot_key_set().slot_for_event(key) {
                return Some(Self::SelectCandidateSlot { slot, flip: false });
            }
            if key.modifiers.is_empty() && key.characters.as_deref() == Some(" ") {
                return Some(Self::CommitHighlightedCandidate);
            }
        }
        // A key that types a glyph types it; only the rest can move or commit.
        if !is_composing || tps_glyph_for_event(key).is_some() {
            return None;
        }
        let action = bindings.action_for(key, platform);
        if action == Some(ComposingAction::CommitLiteral) {
            return Some(Self::CommitAsTyped);
        }
        if is_showing_candidates {
            return None;
        }
        let open = if bindings.is_candidate_window_enabled {
            Self::OpenCandidates
        } else {
            Self::CommitThenPassThrough
        };
        let modifiers = key.modifiers;
        if key.navigation_key.is_some() && !modifiers.shift && !modifiers.has_host_chord() {
            let step = CaretDirection::horizontal(key.navigation_key);
            return Some(step.map_or(open, Self::MoveCaret));
        }
        match action? {
            ComposingAction::ConfirmHighlighted => Some(Self::Commit),
            action if action.navigation().is_some() => Some(open),
            _ => None,
        }
    }

    /// A layout key types its glyph, idle or composing (a tone mark may begin
    /// a composition, as on mobile); a bare Space is the separator
    /// mid-composition and the host's otherwise. `None` sends the key on to
    /// the tiers every mode shares.
    fn tps_key(key: &KeyEventSnapshot, is_composing: bool) -> Option<Self> {
        if let Some(glyph) = tps_glyph_for_event(key) {
            return Some(Self::TpsKey(glyph.to_owned()));
        }
        if key.modifiers.is_empty() && key.characters.as_deref() == Some(" ") {
            return Some(Self::composition_or_host(
                is_composing,
                Self::TpsKey(" ".to_owned()),
            ));
        }
        None
    }

    /// The commit a keyless event runs over a composition before it writes
    /// to the document itself — the symbol picker's commit-first, a host's
    /// Finalize of the candidate list: the highlighted cell while a list
    /// shows one, as Enter picks it, the composition as rendered otherwise.
    /// Under TPS always the composition, as shown: a pick there nails one
    /// word and keeps composing (Hanji conversion H4, H6), so the event
    /// would find the composition still up.
    pub fn commit_first(has_highlight: bool, input_mode: InputMode) -> Self {
        if has_highlight && input_mode != InputMode::Tps {
            Self::CommitHighlightedCandidate
        } else {
            Self::Commit
        }
    }

    /// A click on a cap of the on-screen TPS key panel (desktop TPS roadmap
    /// D6): the glyph it names, typed whatever the window or the slot keys
    /// would make of the physical key — so with the window up a click on
    /// `1` types ㄅ rather than picking (maintainer, 2026-10-04). `None`
    /// outside TPS and for anything the layout does not type, the separator
    /// included: a click never reaches the engine as a key the user did not
    /// see on a cap.
    pub fn tps_keyboard_press(glyph: &str, input_mode: InputMode) -> Option<Self> {
        (input_mode == InputMode::Tps && is_layout_glyph(glyph))
            .then(|| Self::TpsKey(glyph.to_owned()))
    }

    /// The text `key` puts into the document, or `None` when it is a key the
    /// host acts on. Asked by the pass-through path so punctuation typed
    /// outside a composition can be reported to the engine as the end of a
    /// context — while Escape, Return and the arrows are not. The TPS
    /// punctuation chord is document text too, and what it types is the key
    /// under the modifier: under Ctrl the layout types
    /// nothing for `,` (`characters` is `None`) and Escape for `[`, and the
    /// key itself is what the user asked for. macOS keeps a Swift twin:
    /// `KeyEventSnapshot.swift` `documentText(inputMode:)`.
    pub fn document_text(key: &KeyEventSnapshot, input_mode: InputMode) -> Option<String> {
        if let Some(punctuation) = Self::tps_punctuation_chord(key, input_mode) {
            return Some(punctuation.to_string());
        }
        if key.modifiers.has_host_chord() || key.is_named_special_key {
            return None;
        }
        key.characters
            .as_deref()
            .filter(|characters| {
                !characters.is_empty() && characters.chars().all(Self::is_text_scalar)
            })
            .map(str::to_owned)
    }

    /// The punctuation key under the TPS punctuation chord, or `None` when
    /// `key` is not one: TPS only (its bare `,` `.` `;` type glyphs, so Ctrl
    /// is how their marks are typed — full width, the only width TPS
    /// writes), exactly Ctrl among the chording modifiers, Shift allowed since
    /// it picks the key (Ctrl+Shift+, is Ctrl+<), and the key one the
    /// full-width policy maps (`policies::is_punctuation_chord_key`). Read off
    /// the unmodified characters because Ctrl rewrites what a key types.
    /// macOS keeps a Swift twin: `KeyEventSnapshot.swift`
    /// `tpsPunctuationChord(inputMode:)`.
    pub fn tps_punctuation_chord(key: &KeyEventSnapshot, input_mode: InputMode) -> Option<char> {
        let chording = KeyModifiers {
            shift: false,
            ..key.modifiers
        };
        if input_mode != InputMode::Tps
            || chording != KeyModifiers::CONTROL
            || key.is_named_special_key
        {
            return None;
        }
        let unmodified = key.unmodified_characters()?;
        if !crate::policies::is_punctuation_chord_key(unmodified) {
            return None;
        }
        unmodified.chars().next()
    }

    fn composition_or_host(is_composing: bool, when_composing: Self) -> Self {
        if is_composing {
            when_composing
        } else {
            Self::PassThrough
        }
    }

    /// A key the host owns. It still ends any composition first.
    fn host_key(is_composing: bool) -> Self {
        if is_composing {
            Self::CommitThenPassThrough
        } else {
            Self::PassThrough
        }
    }

    fn navigate(navigation: NavigationKey) -> Self {
        Self::Navigate(CandidateNavigation::from(navigation))
    }

    /// The slot a digit `1`…`9` names, counting from zero. `0` names none:
    /// the window holds nine candidates because nine is what the digits can
    /// name without one of them meaning "the tenth" (the `Digits` set's rule).
    pub fn direct_selection_slot(characters_ignoring_modifiers: Option<&str>) -> Option<usize> {
        let digit = characters_ignoring_modifiers?
            .chars()
            .next()?
            .to_digit(10)?;
        (1..=9).contains(&digit).then(|| digit as usize - 1)
    }

    /// The numeric tone markers of TL and POJ, which the engine reads as
    /// ASCII digits. A full-width `５` is document text, not a tone.
    ///
    /// Visible to `ComposingKeyChord`, which refuses to bind a bare digit:
    /// the digits carry tone under Standard and pick candidates under Telex,
    /// so a chord may not take one away under either.
    pub fn is_tone_digit(character: char) -> bool {
        character.is_ascii_digit()
    }

    /// The characters a TL or POJ syllable is built from. ASCII-only on
    /// purpose. Every ASCII letter, not only the eighteen a syllable is
    /// spelled with: a custom-dictionary romanization is free text, so all of
    /// them must reach the composition. Under Telex the eight the scheme
    /// claims are taken before this is asked.
    pub fn is_romanization_character(character: char) -> bool {
        character.is_ascii_alphabetic() || character == '-'
    }

    /// Not a control character (C0/C1) and not one of the Mac's private-use
    /// function-key scalars, which a settings file carried over could still
    /// name.
    fn is_text_scalar(character: char) -> bool {
        !character.is_control() && !(0xF700..=0xF8FF).contains(&(character as u32))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::ChordField;
    use crate::keys::{ComposingAction, ComposingKeyChord, KeyModifiers};
    use crate::platform::test_support::{TEST_PLATFORM as PLATFORM, WINDOWS_AND_LINUX};
    use crate::platform::DesktopPlatform;
    use crate::settings::SettingChoice;
    use std::collections::BTreeMap;

    fn classify(
        key: &KeyEventSnapshot,
        is_composing: bool,
        is_showing: bool,
    ) -> ComposingKeyIntent {
        classify_on(key, is_composing, is_showing, PLATFORM)
    }

    fn text(characters: &str) -> KeyEventSnapshot {
        KeyEventSnapshot::text(characters, KeyModifiers::NONE)
    }

    #[test]
    fn romanization_characters_are_composing_input_in_both_states() {
        for characters in ["t", "A", "-"] {
            assert_eq!(
                classify(&text(characters), false, false),
                ComposingKeyIntent::Input(characters.into())
            );
            assert_eq!(
                classify(&text(characters), true, false),
                ComposingKeyIntent::Input(characters.into())
            );
        }
    }

    #[test]
    fn digits_are_tone_markers_only_while_composing() {
        assert_eq!(
            classify(&text("5"), true, false),
            ComposingKeyIntent::Input("5".into())
        );
        assert_eq!(
            classify(&text("5"), false, false),
            ComposingKeyIntent::PassThrough
        );
        assert_eq!(
            classify(&text("\u{FF15}"), true, false),
            ComposingKeyIntent::CommitThenInsert("\u{FF15}".into()),
            "a full-width numeral is document text"
        );
    }

    #[test]
    fn composition_control_keys_belong_to_the_host_when_there_is_no_composition() {
        // trace: composing, Return commits and passes on, Escape cancels, the
        // deletes delete, space and `.` commit then insert; idle, all pass through.
        let cases = [
            ("\r", ComposingKeyIntent::CommitThenPassThrough),
            ("\u{1B}", ComposingKeyIntent::Cancel),
            ("\u{8}", ComposingKeyIntent::DeleteBackward),
            ("\u{7F}", ComposingKeyIntent::DeleteBackward),
            (" ", ComposingKeyIntent::CommitThenInsert(" ".into())),
            (".", ComposingKeyIntent::CommitThenInsert(".".into())),
        ];
        for (characters, composing) in cases {
            let mut key = text(characters);
            key.is_named_special_key = matches!(characters, "\r");
            assert_eq!(classify(&key, true, false), composing, "{characters:?}");
            assert_eq!(
                classify(&key, false, false),
                ComposingKeyIntent::PassThrough,
                "{characters:?}"
            );
        }
    }

    #[test]
    fn host_owned_events_fall_through_even_mid_composition() {
        let cases = [
            KeyEventSnapshot::chord(Some("\u{13}"), "s", KeyModifiers::CONTROL),
            KeyEventSnapshot::chord(None, "a", KeyModifiers::ALT),
            KeyEventSnapshot::chord(None, "a", KeyModifiers::WIN),
            KeyEventSnapshot::navigation(NavigationKey::LeftArrow, KeyModifiers::NONE),
            KeyEventSnapshot::named_special(KeyModifiers::NONE),
            // A line separator is a NAMED key the platform hands over, not
            // typed text — `U+2028` is Zl, not Cc, so the flag is what
            // classifies it (tier 6).
            KeyEventSnapshot {
                is_named_special_key: true,
                ..KeyEventSnapshot::text("\u{2028}", KeyModifiers::NONE)
            },
        ];
        for key in cases {
            assert_eq!(
                classify(&key, true, false),
                ComposingKeyIntent::CommitThenPassThrough,
                "{key:?}"
            );
            assert_eq!(
                classify(&key, false, false),
                ComposingKeyIntent::PassThrough,
                "{key:?}"
            );
        }
        assert_eq!(
            classify(&text("字"), true, false),
            ComposingKeyIntent::CommitThenInsert("字".into())
        );
        // A letter followed by a combining mark is one grapheme the engine
        // cannot parse — document text.
        assert_eq!(
            classify(&text("a\u{301}"), true, false),
            ComposingKeyIntent::CommitThenInsert("a\u{301}".into())
        );
        assert_eq!(
            classify(&text("a5"), false, false),
            ComposingKeyIntent::PassThrough,
            "a digit outside a composition is not romanization even beside a letter"
        );
        assert_eq!(
            classify(
                &KeyEventSnapshot::text("", KeyModifiers::NONE),
                false,
                false
            ),
            ComposingKeyIntent::PassThrough
        );
    }

    fn classify_on(
        key: &KeyEventSnapshot,
        is_composing: bool,
        is_showing: bool,
        platform: DesktopPlatform,
    ) -> ComposingKeyIntent {
        ComposingKeyIntent::intent(
            key,
            is_composing,
            is_showing,
            &ComposingKeyBindings::default(),
            platform,
        )
    }

    /// Ctrl+← / Ctrl+→ step the caret inside the composition whether or
    /// not the window is up — the bare arrows stay the window's (USER
    /// 2026-09-09).
    #[test]
    fn control_arrows_move_the_composing_caret_window_up_or_not() {
        for platform in WINDOWS_AND_LINUX {
            for is_showing_candidates in [true, false] {
                let left =
                    KeyEventSnapshot::navigation(NavigationKey::LeftArrow, KeyModifiers::CONTROL);
                let right =
                    KeyEventSnapshot::navigation(NavigationKey::RightArrow, KeyModifiers::CONTROL);
                assert_eq!(
                    classify_on(&left, true, is_showing_candidates, platform),
                    ComposingKeyIntent::MoveCaret(CaretDirection::Left)
                );
                assert_eq!(
                    classify_on(&right, true, is_showing_candidates, platform),
                    ComposingKeyIntent::MoveCaret(CaretDirection::Right)
                );
            }
        }
    }

    /// Ctrl+↑ / Ctrl+↓ (⌥ on the Mac) jump to the start / end of what is
    /// being typed — window up or not, under TL and under TPS (tier 0 is read
    /// before the TPS keys, whose plain ↑ / ↓ open the window). Idle, they
    /// are the host's paragraph jump.
    #[test]
    fn caret_chord_up_and_down_jump_to_the_start_and_end() {
        for platform in [
            DesktopPlatform::Windows,
            DesktopPlatform::Linux,
            DesktopPlatform::MacOS,
        ] {
            let chord = caret_chord_modifiers(platform);
            let up = KeyEventSnapshot::navigation(NavigationKey::UpArrow, chord);
            let down = KeyEventSnapshot::navigation(NavigationKey::DownArrow, chord);
            for is_showing_candidates in [true, false] {
                assert_eq!(
                    classify_on(&up, true, is_showing_candidates, platform),
                    ComposingKeyIntent::MoveCaret(CaretDirection::Start),
                    "{platform:?}"
                );
                assert_eq!(
                    classify_on(&down, true, is_showing_candidates, platform),
                    ComposingKeyIntent::MoveCaret(CaretDirection::End),
                    "{platform:?}"
                );
                for (key, direction) in [(&up, CaretDirection::Start), (&down, CaretDirection::End)]
                {
                    assert_eq!(
                        ComposingKeyIntent::intent(
                            key,
                            true,
                            is_showing_candidates,
                            &tps_bindings(),
                            platform
                        ),
                        ComposingKeyIntent::MoveCaret(direction),
                        "TPS on {platform:?}"
                    );
                }
            }
            for key in [&up, &down] {
                assert_eq!(
                    classify_on(key, false, false, platform),
                    ComposingKeyIntent::PassThrough,
                    "idle on {platform:?}"
                );
            }
        }
    }

    #[test]
    fn control_arrow_is_the_hosts_word_jump_when_nothing_is_composing() {
        for platform in WINDOWS_AND_LINUX {
            let left =
                KeyEventSnapshot::navigation(NavigationKey::LeftArrow, KeyModifiers::CONTROL);
            assert_eq!(
                classify_on(&left, false, false, platform),
                ComposingKeyIntent::PassThrough
            );
        }
    }

    /// Bare Home / End jump to the start / end while composing — window up
    /// or not, TL or TPS, every desktop — and never drive the window. With a
    /// modifier (Shift+Home selects) or idle they stay the host's.
    #[test]
    fn home_and_end_jump_to_the_start_and_end_while_composing() {
        for platform in [
            DesktopPlatform::Windows,
            DesktopPlatform::Linux,
            DesktopPlatform::MacOS,
        ] {
            for (edge, direction) in [
                (LineEdgeKey::Home, CaretDirection::Start),
                (LineEdgeKey::End, CaretDirection::End),
            ] {
                let key = KeyEventSnapshot::line_edge(edge, KeyModifiers::NONE);
                for is_showing_candidates in [true, false] {
                    assert_eq!(
                        classify_on(&key, true, is_showing_candidates, platform),
                        ComposingKeyIntent::MoveCaret(direction),
                        "{edge:?} on {platform:?}"
                    );
                    assert_eq!(
                        ComposingKeyIntent::intent(
                            &key,
                            true,
                            is_showing_candidates,
                            &tps_bindings(),
                            platform
                        ),
                        ComposingKeyIntent::MoveCaret(direction),
                        "TPS {edge:?} on {platform:?}"
                    );
                }
                assert_eq!(
                    classify_on(&key, false, false, platform),
                    ComposingKeyIntent::PassThrough,
                    "idle {edge:?} on {platform:?}"
                );
                let shifted = KeyEventSnapshot::line_edge(edge, KeyModifiers::SHIFT);
                assert_eq!(
                    classify_on(&shifted, true, true, platform),
                    ComposingKeyIntent::CommitThenPassThrough,
                    "Shift+{edge:?} on {platform:?}"
                );
            }
        }
    }

    /// Only exactly Ctrl: with Shift it is the host's selection, with Alt or
    /// Win its shortcut — on ↑ / ↓ as on ← / →. Paging keys are never caret
    /// keys.
    #[test]
    fn control_arrow_with_any_other_chord_or_a_paging_key_belongs_to_the_host() {
        let cases = [
            (
                NavigationKey::LeftArrow,
                KeyModifiers::CONTROL.with(KeyModifiers::SHIFT),
            ),
            (
                NavigationKey::RightArrow,
                KeyModifiers::CONTROL.with(KeyModifiers::ALT),
            ),
            (NavigationKey::LeftArrow, KeyModifiers::ALT),
            (NavigationKey::LeftArrow, KeyModifiers::WIN),
            (
                NavigationKey::UpArrow,
                KeyModifiers::CONTROL.with(KeyModifiers::SHIFT),
            ),
            (
                NavigationKey::DownArrow,
                KeyModifiers::CONTROL.with(KeyModifiers::ALT),
            ),
            (NavigationKey::PageDown, KeyModifiers::CONTROL),
        ];
        for platform in WINDOWS_AND_LINUX {
            for (key, modifiers) in cases {
                let snapshot = KeyEventSnapshot::navigation(key, modifiers);
                assert_eq!(
                    classify_on(&snapshot, true, true, platform),
                    ComposingKeyIntent::CommitThenPassThrough,
                    "{key:?} under {modifiers:?} on {platform:?}"
                );
            }
        }
    }

    /// The Mac steps the caret on ⌥← / ⌥→ (`caret_chord_modifiers`,
    /// inventory K1); ⌃← is Mission Control there, so it is a host chord
    /// that ends the composition like any other.
    #[test]
    fn the_mac_moves_the_caret_on_option_arrows() {
        let mac = DesktopPlatform::MacOS;
        for is_showing_candidates in [true, false] {
            let left = KeyEventSnapshot::navigation(NavigationKey::LeftArrow, KeyModifiers::ALT);
            let right = KeyEventSnapshot::navigation(NavigationKey::RightArrow, KeyModifiers::ALT);
            assert_eq!(
                classify_on(&left, true, is_showing_candidates, mac),
                ComposingKeyIntent::MoveCaret(CaretDirection::Left)
            );
            assert_eq!(
                classify_on(&right, true, is_showing_candidates, mac),
                ComposingKeyIntent::MoveCaret(CaretDirection::Right)
            );
        }
        let option_left = KeyEventSnapshot::navigation(NavigationKey::LeftArrow, KeyModifiers::ALT);
        assert_eq!(
            classify_on(&option_left, false, false, mac),
            ComposingKeyIntent::PassThrough,
            "idle, the chord is the host's"
        );
        for modifiers in [
            KeyModifiers::CONTROL,
            KeyModifiers::ALT.with(KeyModifiers::SHIFT),
            KeyModifiers::ALT.with(KeyModifiers::WIN),
        ] {
            // ⌃↑ is Mission Control: the host's, like ⌃←.
            for key in [NavigationKey::LeftArrow, NavigationKey::UpArrow] {
                let snapshot = KeyEventSnapshot::navigation(key, modifiers);
                assert_eq!(
                    classify_on(&snapshot, true, true, mac),
                    ComposingKeyIntent::CommitThenPassThrough,
                    "{key:?} under {modifiers:?}"
                );
            }
        }
        assert_eq!(caret_chord_modifiers(mac), KeyModifiers::ALT);
    }

    #[test]
    fn the_mac_hands_a_chorded_digit_or_slot_key_to_the_host_under_either_scheme() {
        let mac = DesktopPlatform::MacOS;
        // trace: ⌃3 arrives as Escape
        // with `3` unmodified; the fixed tier skips it under a host chord.
        let control_three = KeyEventSnapshot::chord(Some("\u{1B}"), "3", KeyModifiers::CONTROL);
        for scheme in ToneInputScheme::ALL {
            let bindings = ComposingKeyBindings::resolve(&Default::default(), *scheme);
            for is_showing_candidates in [true, false] {
                assert_eq!(
                    ComposingKeyIntent::intent(
                        &control_three,
                        true,
                        is_showing_candidates,
                        &bindings,
                        mac
                    ),
                    ComposingKeyIntent::CommitThenPassThrough,
                    "{scheme:?} window up={is_showing_candidates}"
                );
            }
        }
        // trace: `slot_for_event` misses under any host chord — ⌘ (win), ⌃, ⌥ on a
        // slot key miss the slot: `3` under Telex, `q` under Standard.
        let telex = telex_bindings();
        for modifiers in [KeyModifiers::WIN, KeyModifiers::CONTROL, KeyModifiers::ALT] {
            let three = KeyEventSnapshot::chord(Some("3"), "3", modifiers);
            assert_eq!(
                ComposingKeyIntent::intent(&three, true, true, &telex, mac),
                ComposingKeyIntent::CommitThenPassThrough,
                "{modifiers:?}3 under Telex"
            );
            let q = KeyEventSnapshot::chord(Some("q"), "q", modifiers);
            assert_eq!(
                classify_on(&q, true, true, mac),
                ComposingKeyIntent::CommitThenPassThrough,
                "{modifiers:?}Q under Standard"
            );
        }
    }

    #[test]
    fn the_mac_fires_a_bare_bound_key_only_where_its_action_applies() {
        // trace: the bindings tier needs a window (`requires_candidates`) — Page Forward on a
        // bare `'`: the bindings tier wins over document text with the window
        // up, gives the key back as text with none, and is the host's idle.
        let mac = DesktopPlatform::MacOS;
        let mut stored = BTreeMap::new();
        stored.insert(
            (ComposingAction::PageForward, ChordField::First),
            Some(ComposingKeyChord::make(Some("'"), KeyModifiers::NONE, mac).unwrap()),
        );
        let bindings = ComposingKeyBindings::resolve(&stored, ToneInputScheme::Standard);
        let apostrophe = text("'");
        assert_eq!(
            ComposingKeyIntent::intent(&apostrophe, true, true, &bindings, mac),
            ComposingKeyIntent::Navigate(CandidateNavigation::PageDown)
        );
        assert_eq!(
            ComposingKeyIntent::intent(&apostrophe, true, false, &bindings, mac),
            ComposingKeyIntent::CommitThenInsert("'".into())
        );
        assert_eq!(
            ComposingKeyIntent::intent(&apostrophe, false, false, &bindings, mac),
            ComposingKeyIntent::PassThrough
        );
    }

    /// Roadmap E4, settled P11d: a format character (Cf) is text; only a
    /// control character (Cc) is refused. The Swift key path took this rule
    /// in P11d (it refused `CharacterSet.controlCharacters` = Cc + Cf).
    #[test]
    fn e4_a_format_character_is_text() {
        let mac = DesktopPlatform::MacOS;
        // trace: Cf is not `char::is_control` and not in F700..=F8FF →
        // every scalar is text → tier 7: not romanization (not ASCII
        // letter / `-`) → CommitThenInsert while composing, PassThrough idle.
        for characters in [
            "\u{200B}",
            "\u{200C}",
            "\u{AD}",
            "\u{FEFF}",
            "\u{2066}",
            "x\u{200D}y",
            "👩\u{200D}💻",
            // An emoji tag sequence: plane-14 Cf.
            "🏴\u{E0067}\u{E0062}\u{E0073}\u{E0063}\u{E0074}\u{E007F}",
        ] {
            let key = text(characters);
            assert_eq!(
                classify_on(&key, true, false, mac),
                ComposingKeyIntent::CommitThenInsert(characters.into()),
                "{characters:?}"
            );
            assert_eq!(
                classify_on(&key, false, false, mac),
                ComposingKeyIntent::PassThrough
            );
            assert!(
                ComposingKeyIntent::document_text(&key, InputMode::Tl).is_some(),
                "{characters:?}"
            );
        }
        // Negative control: a C1 control (NEL, Cc) is the host's.
        let next_line = text("\u{85}");
        assert_eq!(
            classify_on(&next_line, true, false, mac),
            ComposingKeyIntent::CommitThenPassThrough
        );
        assert_eq!(
            ComposingKeyIntent::document_text(&next_line, InputMode::Tl),
            None
        );
    }

    /// Roadmap E2 (settled P11c): a dead key, a custom layout or an input
    /// source can hand over several characters in one event, and every one
    /// of them has to qualify. The Escape and Backspace tier reads the first
    /// character. Ported from the Swift key path's
    /// `ComposingKeyIntentTests.testAMultiCharacterEvent_isClassifiedByEveryCharacter`
    /// when it was deleted (roadmap P13).
    #[test]
    fn e2_a_multi_character_event_is_classified_by_every_character() {
        use ComposingKeyIntent::{Cancel, CommitThenInsert, DeleteBackward, Input, PassThrough};
        let mac = DesktopPlatform::MacOS;
        let standard = ComposingKeyBindings::default();
        let telex = telex_bindings();
        let cases = [
            // trace: `.` / `字` are not romanization → document text.
            ("a.", &standard, CommitThenInsert("a.".into()), PassThrough),
            (
                "a字",
                &standard,
                CommitThenInsert("a字".into()),
                PassThrough,
            ),
            // trace: a digit is a tone only mid-composition under Standard.
            ("a5", &standard, Input("a5".into()), PassThrough),
            ("a5", &telex, CommitThenInsert("a5".into()), PassThrough),
            ("5a", &standard, Input("5a".into()), PassThrough),
            // trace: a Telex key is a one-character event; `fx` / `vx` /
            // `zx` are letters, idle too.
            ("fx", &telex, Input("fx".into()), Input("fx".into())),
            ("vx", &telex, Input("vx".into()), Input("vx".into())),
            ("zx", &telex, Input("zx".into()), Input("zx".into())),
            ("f.", &telex, CommitThenInsert("f.".into()), PassThrough),
            ("z.", &telex, CommitThenInsert("z.".into()), PassThrough),
            // trace: the Escape / Backspace tier reads the first character.
            ("\u{1B}x", &standard, Cancel, PassThrough),
            ("\u{7F}x", &standard, DeleteBackward, PassThrough),
        ];
        for (characters, bindings, composing, idle) in cases {
            let key = text(characters);
            assert_eq!(
                ComposingKeyIntent::intent(&key, true, false, bindings, mac),
                composing,
                "composing {characters:?}"
            );
            assert_eq!(
                ComposingKeyIntent::intent(&key, false, false, bindings, mac),
                idle,
                "idle {characters:?}"
            );
        }
    }

    /// A digit slot reads the event's first scalar (`direct_selection_slot`):
    /// a digit with a combining scalar behind it still picks. `0`, a
    /// full-width digit and a chorded digit pick nothing. Ported from the
    /// Swift key path's `testTelex_aDigitSlot_readsTheFirstScalar` (P13).
    #[test]
    fn under_telex_a_digit_slot_reads_the_first_scalar() {
        let mac = DesktopPlatform::MacOS;
        let telex = telex_bindings();
        let slot = |slot| ComposingKeyIntent::SelectCandidateSlot { slot, flip: false };
        // trace: `1⃣` = U+0031 U+20E3 → first scalar `1` → slot 0; `3́` → 2.
        assert_eq!(
            ComposingKeyIntent::intent(&text("1\u{20E3}"), true, true, &telex, mac),
            slot(0)
        );
        assert_eq!(
            ComposingKeyIntent::intent(&text("3\u{301}"), true, true, &telex, mac),
            slot(2)
        );
        for characters in ["0", "\u{FF11}"] {
            assert_eq!(
                ComposingKeyIntent::intent(&text(characters), true, true, &telex, mac),
                ComposingKeyIntent::CommitThenInsert(characters.into()),
                "{characters:?}"
            );
        }
        let command_keycap = KeyEventSnapshot::text("1\u{20E3}", KeyModifiers::WIN);
        assert_eq!(
            ComposingKeyIntent::intent(&command_keycap, true, true, &telex, mac),
            ComposingKeyIntent::CommitThenPassThrough
        );
    }

    #[test]
    fn navigation_keys_drive_the_window_only_while_it_is_up() {
        let cases = [
            (NavigationKey::LeftArrow, CandidateNavigation::Left),
            (NavigationKey::RightArrow, CandidateNavigation::Right),
            (NavigationKey::UpArrow, CandidateNavigation::Up),
            (NavigationKey::DownArrow, CandidateNavigation::Down),
            (NavigationKey::PageUp, CandidateNavigation::PageUp),
            (NavigationKey::PageDown, CandidateNavigation::PageDown),
        ];
        for (key, expected) in cases {
            let snapshot = KeyEventSnapshot::navigation(key, KeyModifiers::NONE);
            assert_eq!(
                classify(&snapshot, true, true),
                ComposingKeyIntent::Navigate(expected)
            );
            assert_eq!(
                classify(&snapshot, true, false),
                ComposingKeyIntent::CommitThenPassThrough
            );
        }
        let shifted = KeyEventSnapshot::navigation(NavigationKey::LeftArrow, KeyModifiers::SHIFT);
        assert_eq!(
            classify(&shifted, true, true),
            ComposingKeyIntent::CommitThenPassThrough
        );
    }

    fn telex_bindings() -> ComposingKeyBindings {
        ComposingKeyBindings::resolve(&Default::default(), ToneInputScheme::Telex)
    }

    #[test]
    fn under_telex_the_tone_letters_are_the_engines_while_composing() {
        // trace: tier 7 under Telex (`is_telex_key`) — `v` composing →
        // `TelexKey`, capital too; idle `v` passes through; idle `z` starts.
        let telex = telex_bindings();
        assert_eq!(
            ComposingKeyIntent::intent(&text("v"), true, false, &telex, PLATFORM),
            ComposingKeyIntent::TelexKey("v".into())
        );
        assert_eq!(
            ComposingKeyIntent::intent(&text("V"), true, false, &telex, PLATFORM),
            ComposingKeyIntent::TelexKey("V".into())
        );
        assert_eq!(
            ComposingKeyIntent::intent(&text("f"), true, true, &telex, PLATFORM),
            ComposingKeyIntent::TelexKey("f".into())
        );
        assert_eq!(
            ComposingKeyIntent::intent(&text("v"), false, false, &telex, PLATFORM),
            ComposingKeyIntent::PassThrough,
            "a tone letter has nothing to mark when idle"
        );
        assert_eq!(
            ComposingKeyIntent::intent(&text("f"), false, false, &telex, PLATFORM),
            ComposingKeyIntent::PassThrough
        );
        assert_eq!(
            ComposingKeyIntent::intent(&text("z"), false, false, &telex, PLATFORM),
            ComposingKeyIntent::TelexKey("z".into()),
            "`z` types an initial, so it starts a composition"
        );
        // A letter neither scheme claims is still text.
        assert_eq!(
            ComposingKeyIntent::intent(&text("t"), true, false, &telex, PLATFORM),
            ComposingKeyIntent::Input("t".into())
        );
        // Under Standard the same keys are text (or slots, below).
        assert_eq!(
            classify(&text("v"), true, false),
            ComposingKeyIntent::Input("v".into())
        );
    }

    #[test]
    fn under_telex_the_digits_pick_and_the_bare_letters_do_not() {
        let telex = telex_bindings();
        assert_eq!(
            ComposingKeyIntent::intent(&text("3"), true, true, &telex, PLATFORM),
            ComposingKeyIntent::SelectCandidateSlot {
                slot: 2,
                flip: false
            }
        );
        assert_eq!(
            ComposingKeyIntent::intent(&text("3"), true, false, &telex, PLATFORM),
            ComposingKeyIntent::CommitThenInsert("3".into()),
            "with no window a digit is punctuation: commit, then insert"
        );
        assert_eq!(
            ComposingKeyIntent::intent(&text("3"), false, false, &telex, PLATFORM),
            ComposingKeyIntent::PassThrough
        );
        assert_eq!(
            ComposingKeyIntent::intent(&text("q"), true, true, &telex, PLATFORM),
            ComposingKeyIntent::TelexKey("q".into()),
            "`q` is tone 9, not slot 0"
        );
        assert_eq!(
            ComposingKeyIntent::intent(&text(";"), true, true, &telex, PLATFORM),
            ComposingKeyIntent::CommitThenInsert(";".into())
        );
        let control_three = KeyEventSnapshot::chord(Some("\u{1B}"), "3", KeyModifiers::CONTROL);
        assert_eq!(
            ComposingKeyIntent::intent(&control_three, true, true, &telex, PLATFORM),
            ComposingKeyIntent::CommitThenPassThrough,
            "a chording modifier makes the digit miss"
        );
        // Under Standard a bare digit with the window up is still the tone.
        assert_eq!(
            classify(&text("3"), true, true),
            ComposingKeyIntent::Input("3".into())
        );
    }

    #[test]
    fn shift_on_a_slot_key_flips_the_script_only_while_the_window_is_up() {
        // trace: tier 3 (`shifted_slot_for_event`) — with the window up ⇧ on a
        // slot key flips that slot; with none the shifted key is what it
        // types. A shifted digit flips under Telex and stays punctuation under
        // Standard; under Telex the shifted letter is a tone key. ⇧ beside a
        // host chord is the host's.
        let shift_w = KeyEventSnapshot::text("W", KeyModifiers::SHIFT);
        assert_eq!(
            classify(&shift_w, true, true),
            ComposingKeyIntent::SelectCandidateSlot {
                slot: 1,
                flip: true
            }
        );
        assert_eq!(
            classify(&shift_w, true, false),
            ComposingKeyIntent::Input("W".into()),
            "with no window the capital reaches the composition"
        );
        let shift_semicolon = KeyEventSnapshot::chord(Some(":"), ":", KeyModifiers::SHIFT)
            .with_key_code(crate::keys::chord::SEMICOLON_KEY_CODE);
        assert_eq!(
            classify(&shift_semicolon, true, true),
            ComposingKeyIntent::SelectCandidateSlot {
                slot: 8,
                flip: true
            }
        );
        assert_eq!(
            classify(&shift_semicolon, true, false),
            ComposingKeyIntent::CommitThenInsert(":".into())
        );
        // Under Telex the shifted digit flips; under Standard it stays the
        // punctuation it types, and the shifted letter is the tone key.
        let telex = telex_bindings();
        let shift_two =
            KeyEventSnapshot::chord(Some("@"), "@", KeyModifiers::SHIFT).with_key_code(0x32);
        assert_eq!(
            ComposingKeyIntent::intent(&shift_two, true, true, &telex, PLATFORM),
            ComposingKeyIntent::SelectCandidateSlot {
                slot: 1,
                flip: true
            }
        );
        assert_eq!(
            classify(&shift_two, true, true),
            ComposingKeyIntent::CommitThenInsert("@".into())
        );
        assert_eq!(
            ComposingKeyIntent::intent(&shift_w, true, true, &telex, PLATFORM),
            ComposingKeyIntent::TelexKey("W".into())
        );
        // Shift beside a host chord is the host's, window or not.
        let ctrl_shift_w =
            KeyEventSnapshot::text("W", KeyModifiers::CONTROL.with(KeyModifiers::SHIFT));
        assert_eq!(
            classify(&ctrl_shift_w, true, true),
            ComposingKeyIntent::CommitThenPassThrough
        );
    }

    #[test]
    fn bare_slot_keys_pick_while_the_window_is_up_and_type_otherwise() {
        assert_eq!(
            classify(&text("q"), true, true),
            ComposingKeyIntent::SelectCandidateSlot {
                slot: 0,
                flip: false
            }
        );
        assert_eq!(
            classify(&text(";"), true, true),
            ComposingKeyIntent::SelectCandidateSlot {
                slot: 8,
                flip: false
            }
        );
        assert_eq!(
            classify(&text("q"), true, false),
            ComposingKeyIntent::Input("q".into())
        );
        assert_eq!(
            classify(&text("q"), false, false),
            ComposingKeyIntent::Input("q".into())
        );
    }

    #[test]
    fn space_commits_the_other_script_only_while_the_window_is_up() {
        assert_eq!(
            classify(&text(" "), true, true),
            ComposingKeyIntent::CommitAlternateScript
        );
        assert_eq!(
            classify(&text(" "), true, false),
            ComposingKeyIntent::CommitThenInsert(" ".into())
        );
    }

    #[test]
    fn return_commits_the_candidate_and_shift_return_the_literal() {
        let plain = text("\r");
        let shifted = KeyEventSnapshot::text("\r", KeyModifiers::SHIFT);
        assert_eq!(
            classify(&plain, true, true),
            ComposingKeyIntent::CommitHighlightedCandidate
        );
        assert_eq!(classify(&shifted, true, true), ComposingKeyIntent::Commit);
        assert_eq!(
            classify(&shifted, true, false),
            ComposingKeyIntent::Commit,
            "the literal needs no window"
        );
        assert_eq!(
            classify(&plain, true, false),
            ComposingKeyIntent::CommitThenPassThrough
        );
        let tab = text("\t");
        assert_eq!(
            classify(&tab, true, true),
            ComposingKeyIntent::Navigate(CandidateNavigation::NextCandidate)
        );
        assert_eq!(
            classify(
                &KeyEventSnapshot::text("\t", KeyModifiers::SHIFT),
                true,
                true
            ),
            ComposingKeyIntent::Navigate(CandidateNavigation::PreviousCandidate)
        );
        assert_eq!(
            classify(&text("]"), true, true),
            ComposingKeyIntent::Navigate(CandidateNavigation::PageDown)
        );
        assert_eq!(
            classify(&text("]"), true, false),
            ComposingKeyIntent::CommitThenInsert("]".into())
        );
    }

    #[test]
    fn candidate_keys_commit_the_typed_text_when_the_window_is_off() {
        // trace: S33 — tier 4 with `is_candidate_window_enabled` off returns
        // `Commit` for every candidate key mid-composition; idle, the host's.
        let mut window_off = ComposingKeyBindings::default();
        window_off.is_candidate_window_enabled = false;
        for (name, characters) in [
            ("Enter", "\r"),
            ("Space", " "),
            ("Tab", "\t"),
            ("Page forward", "]"),
            ("Page backward", "["),
        ] {
            assert_eq!(
                ComposingKeyIntent::intent(&text(characters), true, false, &window_off, PLATFORM),
                ComposingKeyIntent::Commit,
                "{name} writes the romanization as typed when there is no window to act on"
            );
            assert_eq!(
                ComposingKeyIntent::intent(&text(characters), false, false, &window_off, PLATFORM),
                ComposingKeyIntent::PassThrough,
                "{name} is the host's with no composition, window or not"
            );
        }
        // Negative control: with the window ON and none up, the shipped
        // meanings stand (`return_commits_the_candidate_and_shift_return_the_literal`).
        assert_eq!(
            classify(&text("\r"), true, false),
            ComposingKeyIntent::CommitThenPassThrough
        );
        assert_eq!(
            classify(&text(" "), true, false),
            ComposingKeyIntent::CommitThenInsert(" ".into())
        );
    }

    #[test]
    fn document_text_accepts_printable_and_rejects_host_keys() {
        let is_text = |key: &KeyEventSnapshot| {
            ComposingKeyIntent::document_text(key, InputMode::Tl).is_some()
        };
        for t in ["。", "、", "!", "?", " ", "台", "x"] {
            assert!(is_text(&text(t)), "{t}");
        }
        for t in ["\u{1B}", "\r", "\u{8}", "\u{7F}", "", "\u{F702}"] {
            assert!(!is_text(&text(t)), "{t:?}");
        }
        // Outside TPS no Ctrl chord is text, a punctuation key included
        // (the TPS chord: `tps_punctuation_chord_types_the_mapped_key_in_both_states`).
        for modifiers in [KeyModifiers::CONTROL, KeyModifiers::ALT, KeyModifiers::WIN] {
            for t in ["x", "."] {
                assert!(!is_text(&KeyEventSnapshot::text(t, modifiers)), "{t}");
            }
        }
        assert!(is_text(&KeyEventSnapshot::text(".", KeyModifiers::SHIFT)));
        assert!(!is_text(&KeyEventSnapshot::named_special(
            KeyModifiers::NONE
        )));
        assert!(!is_text(&KeyEventSnapshot::default()));
    }

    #[test]
    fn tps_punctuation_chord_types_the_mapped_key_in_both_states() {
        // trace: under Ctrl the layout types nothing for `,` (`characters`
        // None, `key_translation.rs`) and Escape for `[`; the unmodified
        // translation is the key. Ctrl+Shift+, keeps Shift → `<`.
        let comma = KeyEventSnapshot::chord(None, ",", KeyModifiers::CONTROL);
        let bracket = KeyEventSnapshot::chord(Some("\u{1B}"), "[", KeyModifiers::CONTROL);
        let angle =
            KeyEventSnapshot::chord(None, "<", KeyModifiers::CONTROL.with(KeyModifiers::SHIFT));
        for (key, expected) in [(&comma, ","), (&bracket, "["), (&angle, "<")] {
            assert_eq!(
                ComposingKeyIntent::tps_punctuation_chord(key, InputMode::Tps),
                expected.chars().next()
            );
            assert_eq!(
                ComposingKeyIntent::document_text(key, InputMode::Tps).as_deref(),
                Some(expected)
            );
            assert_eq!(
                classify_tps(key, false, false),
                ComposingKeyIntent::PassThrough
            );
            assert_eq!(
                classify_tps(key, true, true),
                ComposingKeyIntent::CommitThenInsert(expected.into())
            );
        }
        // Ctrl+[ is the chord, not a cancel: the fixed tier reads Escape only
        // with no host chord held.
        assert!(!bracket.is_bare_escape());
    }

    /// The chord is TPS's alone (USER 2026-10-07): under TL and POJ, Ctrl on
    /// a punctuation key is the host's like any Ctrl chord — no text, idle
    /// passed through, a composition committed first.
    #[test]
    fn outside_tps_ctrl_on_punctuation_is_the_hosts() {
        let comma = KeyEventSnapshot::chord(None, ",", KeyModifiers::CONTROL);
        let bracket = KeyEventSnapshot::chord(Some("\u{1B}"), "[", KeyModifiers::CONTROL);
        for input_mode in [InputMode::Tl, InputMode::Poj] {
            let mut bindings = ComposingKeyBindings::default();
            bindings.input_mode = input_mode;
            for key in [&comma, &bracket] {
                assert_eq!(
                    ComposingKeyIntent::tps_punctuation_chord(key, input_mode),
                    None
                );
                assert_eq!(ComposingKeyIntent::document_text(key, input_mode), None);
                assert_eq!(
                    ComposingKeyIntent::intent(key, false, false, &bindings, PLATFORM),
                    ComposingKeyIntent::PassThrough
                );
                assert_eq!(
                    ComposingKeyIntent::intent(key, true, true, &bindings, PLATFORM),
                    ComposingKeyIntent::CommitThenPassThrough,
                    "{key:?} under {input_mode:?}"
                );
            }
        }
    }

    #[test]
    fn tps_punctuation_chord_needs_exactly_control_on_a_mapped_key() {
        // Another host chord beside Ctrl, a key the policy does not map or
        // leaves to the host (Ctrl+Shift+` is VS Code's New Terminal), a
        // named key, a bare key: all the host's or the ordinary text rule's.
        let with_alt =
            KeyEventSnapshot::chord(None, ",", KeyModifiers::CONTROL.with(KeyModifiers::ALT));
        let with_win =
            KeyEventSnapshot::chord(None, ",", KeyModifiers::CONTROL.with(KeyModifiers::WIN));
        let letter = KeyEventSnapshot::chord(Some("\u{13}"), "s", KeyModifiers::CONTROL);
        let quote = KeyEventSnapshot::chord(None, "\"", KeyModifiers::CONTROL);
        let tilde =
            KeyEventSnapshot::chord(None, "~", KeyModifiers::CONTROL.with(KeyModifiers::SHIFT));
        let arrow = KeyEventSnapshot::navigation(NavigationKey::LeftArrow, KeyModifiers::CONTROL);
        for key in [&with_alt, &with_win, &letter, &quote, &tilde, &arrow] {
            assert_eq!(
                ComposingKeyIntent::tps_punctuation_chord(key, InputMode::Tps),
                None,
                "{key:?}"
            );
            assert_eq!(
                classify_tps(key, false, false),
                ComposingKeyIntent::PassThrough
            );
        }
        for key in [&letter, &tilde] {
            assert_eq!(
                classify_tps(key, true, true),
                ComposingKeyIntent::CommitThenPassThrough,
                "{key:?}"
            );
        }
        assert_eq!(
            ComposingKeyIntent::tps_punctuation_chord(&text(","), InputMode::Tps),
            None
        );
    }

    #[test]
    fn tps_punctuation_chord_yields_to_a_recorded_binding() {
        // trace: tier 4 (bindings) is read before the TPS punctuation chord;
        // under TPS the Commit Literal action commits as typed.
        let mut stored = BTreeMap::new();
        stored.insert(
            (ComposingAction::CommitLiteral, ChordField::First),
            Some(ComposingKeyChord {
                key: ",".into(),
                modifiers: KeyModifiers::CONTROL,
            }),
        );
        let mut recorded = ComposingKeyBindings::resolve(&stored, ToneInputScheme::Standard);
        recorded.input_mode = InputMode::Tps;
        let comma = KeyEventSnapshot::chord(None, ",", KeyModifiers::CONTROL);

        assert_eq!(
            ComposingKeyIntent::intent(&comma, true, false, &recorded, PLATFORM),
            ComposingKeyIntent::CommitAsTyped
        );
        assert_eq!(
            classify_tps(&comma, true, false),
            ComposingKeyIntent::CommitThenInsert(",".into()),
            "unrecorded, the same chord is the punctuation key"
        );
    }

    fn tps_bindings() -> ComposingKeyBindings {
        let mut bindings = ComposingKeyBindings::default();
        bindings.input_mode = InputMode::Tps;
        bindings
    }

    fn classify_tps(
        key: &KeyEventSnapshot,
        is_composing: bool,
        is_showing: bool,
    ) -> ComposingKeyIntent {
        ComposingKeyIntent::intent(key, is_composing, is_showing, &tps_bindings(), PLATFORM)
    }

    fn keypad(digit: &str, code: u16) -> KeyEventSnapshot {
        coded_key(digit, code, KeyModifiers::NONE)
    }

    #[test]
    fn under_tps_a_layout_key_types_its_glyph_idle_or_composing() {
        // trace: tps_layout — `e` ㄍ, number-row `1` ㄅ, `7` tone 8 U+02D9, `,` ㄝ;
        // taken ahead of the slot tier. These events carry no key code, so
        // even a window up does not read `1` / `7` as the number row (D7,
        // `under_tps_the_window_picks_with_the_number_row_and_space_confirms`).
        let cases = [("e", "ㄍ"), ("1", "ㄅ"), ("7", "\u{02d9}"), (",", "ㄝ")];
        for (key, glyph) in cases {
            for (is_composing, is_showing) in [(false, false), (true, false), (true, true)] {
                assert_eq!(
                    classify_tps(&text(key), is_composing, is_showing),
                    ComposingKeyIntent::TpsKey(glyph.into()),
                    "{key:?}"
                );
            }
        }
    }

    #[test]
    fn under_tps_space_is_the_separator_mid_composition_and_the_hosts_idle() {
        // Space's default binding (Output the Other Script) never sees it.
        // With the window up it confirms (D7).
        assert_eq!(
            classify_tps(&text(" "), true, true),
            ComposingKeyIntent::CommitHighlightedCandidate
        );
        assert_eq!(
            classify_tps(&text(" "), true, false),
            ComposingKeyIntent::TpsKey(" ".into())
        );
        assert_eq!(
            classify_tps(&text(" "), false, false),
            ComposingKeyIntent::PassThrough
        );
    }

    #[test]
    fn under_tps_the_keypad_picks_with_a_list_and_is_text_without_one() {
        // trace: VK_NUMPAD3 (0x63) → slot 2 with a list; no list → document text.
        assert_eq!(
            classify_tps(&keypad("3", 0x63), true, true),
            ComposingKeyIntent::SelectCandidateSlot {
                slot: 2,
                flip: false
            }
        );
        assert_eq!(
            classify_tps(&keypad("3", 0x63), true, false),
            ComposingKeyIntent::CommitThenInsert("3".into())
        );
        assert_eq!(
            classify_tps(&keypad("3", 0x63), false, false),
            ComposingKeyIntent::PassThrough
        );
    }

    /// A key that types `digit` from the key at `code`.
    fn coded_key(digit: &str, code: u16, modifiers: KeyModifiers) -> KeyEventSnapshot {
        KeyEventSnapshot::chord(Some(digit), digit, modifiers).with_key_code(code)
    }

    #[test]
    fn under_tps_the_window_picks_with_the_number_row_and_space_confirms() {
        // trace: D7 — window up: bare number-row `3` (0x33) → slot 2; Shift+`1`
        // (0x31) types ㆠ; `e` still types ㄍ (the executor shuts the window);
        // Escape closes, a second (no window) cancels.
        assert_eq!(
            classify_tps(&coded_key("3", 0x33, KeyModifiers::NONE), true, true),
            ComposingKeyIntent::SelectCandidateSlot {
                slot: 2,
                flip: false
            }
        );
        assert_eq!(
            classify_tps(&coded_key("3", 0x33, KeyModifiers::NONE), true, false),
            ComposingKeyIntent::TpsKey("\u{02ea}".into()),
            "typing: the number row types its glyph"
        );
        assert_eq!(
            classify_tps(&coded_key("!", 0x31, KeyModifiers::SHIFT), true, true),
            ComposingKeyIntent::TpsKey("ㆠ".into())
        );
        assert_eq!(
            classify_tps(&text("e"), true, true),
            ComposingKeyIntent::TpsKey("ㄍ".into())
        );
        let escape = text("\u{1b}");
        assert_eq!(
            classify_tps(&escape, true, true),
            ComposingKeyIntent::CloseCandidates
        );
        assert_eq!(
            classify_tps(&escape, true, false),
            ComposingKeyIntent::Cancel
        );
        assert_eq!(
            classify(&escape, true, true),
            ComposingKeyIntent::Cancel,
            "TL: Escape cancels with the list up"
        );
    }

    #[test]
    fn under_tps_typing_the_moving_keys_open_the_window_and_the_commit_rows_part_ways() {
        // trace: D7 — no window, composing: ↓ and the keys bound to the
        // navigation rows (Tab, `]`) open; Hanji conversion B2 — Enter
        // commits as shown (`Commit`), Shift+Enter the glyphs as typed,
        // window up or not; idle, every one is the host's.
        let down = KeyEventSnapshot::navigation(NavigationKey::DownArrow, KeyModifiers::NONE);
        let tab = text("\t");
        let page = text("]");
        for key in [&down, &tab] {
            assert_eq!(
                classify_tps(key, true, false),
                ComposingKeyIntent::OpenCandidates,
                "{key:?}"
            );
        }
        assert_eq!(
            classify_tps(&page, true, false),
            ComposingKeyIntent::OpenCandidates
        );
        let mut enter = text("\r");
        enter.is_named_special_key = true;
        let shift_enter = KeyEventSnapshot {
            modifiers: KeyModifiers::SHIFT,
            ..enter.clone()
        };
        assert_eq!(
            classify_tps(&enter, true, false),
            ComposingKeyIntent::Commit
        );
        assert_eq!(
            classify_tps(&shift_enter, true, false),
            ComposingKeyIntent::CommitAsTyped
        );
        assert_eq!(
            classify_tps(&shift_enter, true, true),
            ComposingKeyIntent::CommitAsTyped,
            "window up: Shift+Enter still commits as typed"
        );
        assert_eq!(
            classify(&shift_enter, true, false),
            ComposingKeyIntent::Commit,
            "TL: Shift+Enter is the literal commit"
        );
        assert_eq!(
            classify_tps(&down, false, false),
            ComposingKeyIntent::PassThrough
        );
        assert_eq!(
            classify_tps(&enter, false, false),
            ComposingKeyIntent::PassThrough
        );
        assert_eq!(
            classify(&down, true, false),
            ComposingKeyIntent::CommitThenPassThrough,
            "TL: no list, ↓ is the host's"
        );
    }

    #[test]
    fn under_tps_typing_plain_left_and_right_step_the_caret() {
        // trace: Hanji conversion H3/H6 — composing, no window: plain ← →
        // are `MoveCaret`, with Show Candidate Window on or off; with the
        // window up tier 1 keeps them the window's; idle they are the host's;
        // TL keeps D7's rule (no list, the host's).
        let mut window_off = tps_bindings();
        window_off.is_candidate_window_enabled = false;
        for (key, direction) in [
            (NavigationKey::LeftArrow, CaretDirection::Left),
            (NavigationKey::RightArrow, CaretDirection::Right),
        ] {
            let arrow = KeyEventSnapshot::navigation(key, KeyModifiers::NONE);
            assert_eq!(
                classify_tps(&arrow, true, false),
                ComposingKeyIntent::MoveCaret(direction)
            );
            assert_eq!(
                ComposingKeyIntent::intent(&arrow, true, false, &window_off, PLATFORM),
                ComposingKeyIntent::MoveCaret(direction),
                "window off"
            );
            assert_eq!(
                classify_tps(&arrow, true, true),
                ComposingKeyIntent::Navigate(CandidateNavigation::from(key))
            );
            assert_eq!(
                classify_tps(&arrow, false, false),
                ComposingKeyIntent::PassThrough
            );
            assert_eq!(
                classify(&arrow, true, false),
                ComposingKeyIntent::CommitThenPassThrough,
                "TL"
            );
            // Shift+← is the host's selection: no caret step.
            let shifted = KeyEventSnapshot::navigation(key, KeyModifiers::SHIFT);
            assert_ne!(
                classify_tps(&shifted, true, false),
                ComposingKeyIntent::MoveCaret(direction)
            );
        }
    }

    #[test]
    fn under_tps_a_rebound_commit_as_typed_row_commits_the_glyphs() {
        // trace: the TPS branch reads the CommitLiteral row by binding, not by
        // key: a recorded Ctrl+' commits as typed, window up or not.
        let mut stored = BTreeMap::new();
        stored.insert(
            (ComposingAction::CommitLiteral, ChordField::First),
            Some(ComposingKeyChord {
                key: "'".into(),
                modifiers: KeyModifiers::CONTROL,
            }),
        );
        let mut bindings = ComposingKeyBindings::resolve(&stored, ToneInputScheme::Standard);
        bindings.input_mode = InputMode::Tps;
        let chord = KeyEventSnapshot::chord(None, "'", KeyModifiers::CONTROL);
        for is_showing in [false, true] {
            assert_eq!(
                ComposingKeyIntent::intent(&chord, true, is_showing, &bindings, PLATFORM),
                ComposingKeyIntent::CommitAsTyped,
                "window up: {is_showing}"
            );
        }
    }

    #[test]
    fn the_commit_first_is_the_highlight_except_under_tps() {
        use ComposingKeyIntent::{Commit, CommitHighlightedCandidate};
        for mode in [InputMode::Tl, InputMode::Poj] {
            assert_eq!(
                ComposingKeyIntent::commit_first(true, mode),
                CommitHighlightedCandidate
            );
            assert_eq!(ComposingKeyIntent::commit_first(false, mode), Commit);
        }
        for has_highlight in [true, false] {
            assert_eq!(
                ComposingKeyIntent::commit_first(has_highlight, InputMode::Tps),
                Commit
            );
        }
    }

    #[test]
    fn under_tps_with_the_window_off_nothing_opens() {
        let mut bindings = tps_bindings();
        bindings.is_candidate_window_enabled = false;
        let down = KeyEventSnapshot::navigation(NavigationKey::DownArrow, KeyModifiers::NONE);
        assert_eq!(
            ComposingKeyIntent::intent(&down, true, false, &bindings, PLATFORM),
            ComposingKeyIntent::CommitThenPassThrough
        );
        assert_eq!(
            ComposingKeyIntent::intent(&text("]"), true, false, &bindings, PLATFORM),
            ComposingKeyIntent::CommitThenPassThrough
        );
        // A navigation row rebound onto another key follows it.
        let mut stored = BTreeMap::new();
        stored.insert(
            (ComposingAction::NextCandidate, ChordField::First),
            Some(ComposingKeyChord {
                key: "'".into(),
                modifiers: KeyModifiers::CONTROL,
            }),
        );
        let mut rebound = ComposingKeyBindings::resolve(&stored, ToneInputScheme::Standard);
        rebound.input_mode = InputMode::Tps;
        let chord = KeyEventSnapshot::chord(None, "'", KeyModifiers::CONTROL);
        assert_eq!(
            ComposingKeyIntent::intent(&chord, true, false, &rebound, PLATFORM),
            ComposingKeyIntent::OpenCandidates
        );
        rebound.is_candidate_window_enabled = false;
        assert_eq!(
            ComposingKeyIntent::intent(&chord, true, false, &rebound, PLATFORM),
            ComposingKeyIntent::CommitThenPassThrough
        );
    }

    #[test]
    fn under_tps_an_unassigned_key_is_document_text_and_the_shared_keys_are_unchanged() {
        let question = KeyEventSnapshot::text("?", KeyModifiers::SHIFT);
        assert_eq!(
            classify_tps(&question, true, false),
            ComposingKeyIntent::CommitThenInsert("?".into())
        );
        assert_eq!(
            classify_tps(&question, false, false),
            ComposingKeyIntent::PassThrough
        );
        let mut enter = text("\r");
        enter.is_named_special_key = true;
        assert_eq!(
            classify_tps(&enter, true, true),
            ComposingKeyIntent::CommitHighlightedCandidate
        );
        assert_eq!(
            classify_tps(&text("\u{8}"), true, false),
            ComposingKeyIntent::DeleteBackward
        );
        let ctrl_comma = KeyEventSnapshot::chord(Some(","), ",", KeyModifiers::CONTROL);
        assert_eq!(
            classify_tps(&ctrl_comma, true, false),
            ComposingKeyIntent::CommitThenInsert(",".into())
        );
    }

    #[test]
    fn under_tps_shift_space_and_keypad_non_digits_are_document_text() {
        // trace: Shift+Space is no layout key and no bare Space → tier 7 TPS;
        // keypad `.` (VK_DECIMAL 0x6E) is refused by the layout → tier 7; a
        // Shift+keypad digit is no slot (TpsDigits has no flip) → tier 7.
        let shift_space = KeyEventSnapshot::text(" ", KeyModifiers::SHIFT);
        assert_eq!(
            classify_tps(&shift_space, true, true),
            ComposingKeyIntent::CommitThenInsert(" ".into())
        );
        assert_eq!(
            classify_tps(&shift_space, false, false),
            ComposingKeyIntent::PassThrough
        );
        let keypad_dot = KeyEventSnapshot::text(".", KeyModifiers::NONE).with_key_code(0x6E);
        assert_eq!(
            classify_tps(&keypad_dot, true, true),
            ComposingKeyIntent::CommitThenInsert(".".into())
        );
        let shifted_keypad =
            KeyEventSnapshot::chord(Some("3"), "3", KeyModifiers::SHIFT).with_key_code(0x63);
        assert_eq!(
            classify_tps(&shifted_keypad, true, true),
            ComposingKeyIntent::CommitThenInsert("3".into())
        );
    }

    #[test]
    fn under_tps_paging_tab_and_the_caret_chord_keep_their_meaning() {
        let mut tab = text("\t");
        tab.is_named_special_key = true;
        assert_eq!(
            classify_tps(&tab, true, true),
            ComposingKeyIntent::Navigate(CandidateNavigation::NextCandidate)
        );
        assert_eq!(
            classify_tps(&text("]"), true, true),
            ComposingKeyIntent::Navigate(CandidateNavigation::PageDown)
        );
        assert_eq!(
            classify_tps(&text("["), true, true),
            ComposingKeyIntent::Navigate(CandidateNavigation::PageUp)
        );
        let left =
            KeyEventSnapshot::navigation(NavigationKey::LeftArrow, caret_chord_modifiers(PLATFORM));
        assert_eq!(
            classify_tps(&left, true, false),
            ComposingKeyIntent::MoveCaret(CaretDirection::Left)
        );
    }

    #[test]
    fn under_tps_a_stored_telex_scheme_changes_nothing() {
        // trace: the TPS branch runs before the Telex tier; slot set = TpsDigits.
        let mut bindings = ComposingKeyBindings::resolve(&BTreeMap::new(), ToneInputScheme::Telex);
        bindings.input_mode = InputMode::Tps;
        let classify = |key: &KeyEventSnapshot| {
            ComposingKeyIntent::intent(key, true, true, &bindings, PLATFORM)
        };
        assert_eq!(
            classify(&text("f")),
            ComposingKeyIntent::TpsKey("ㄑ".into())
        );
        assert_eq!(
            classify(&text("3")),
            ComposingKeyIntent::TpsKey("\u{02ea}".into())
        );
    }

    #[test]
    fn a_panel_press_types_its_glyph_only_under_tps() {
        // trace: KEYS — ㄅ is `1`'s glyph, ㆠ its Shift glyph; " " (the
        // separator) and "a" are no glyph of the table.
        assert_eq!(
            ComposingKeyIntent::tps_keyboard_press("ㄅ", InputMode::Tps),
            Some(ComposingKeyIntent::TpsKey("ㄅ".into()))
        );
        assert_eq!(
            ComposingKeyIntent::tps_keyboard_press("ㆠ", InputMode::Tps),
            Some(ComposingKeyIntent::TpsKey("ㆠ".into()))
        );
        assert_eq!(
            ComposingKeyIntent::tps_keyboard_press("ㄅ", InputMode::Tl),
            None
        );
        assert_eq!(
            ComposingKeyIntent::tps_keyboard_press("ㄅ", InputMode::Poj),
            None
        );
        for not_a_glyph in [" ", "a", "", "ㄅㄚ"] {
            assert_eq!(
                ComposingKeyIntent::tps_keyboard_press(not_a_glyph, InputMode::Tps),
                None,
                "{not_a_glyph:?}"
            );
        }
    }

    #[test]
    fn under_tps_output_the_other_script_is_inert_wherever_it_is_bound() {
        // trace: tier 4 filters CommitAlternateScript under TPS; an Alt+Enter
        // binding then falls to the host-chord guard (commit, pass on).
        let mut stored = BTreeMap::new();
        stored.insert(
            (ComposingAction::CommitAlternateScript, ChordField::First),
            Some(ComposingKeyChord {
                key: "\r".into(),
                modifiers: KeyModifiers::ALT,
            }),
        );
        let mut bindings = ComposingKeyBindings::resolve(&stored, ToneInputScheme::Standard);
        let mut alt_enter = KeyEventSnapshot::text("\r", KeyModifiers::ALT);
        alt_enter.is_named_special_key = true;
        assert_eq!(
            ComposingKeyIntent::intent(&alt_enter, true, true, &bindings, PLATFORM),
            ComposingKeyIntent::CommitAlternateScript
        );
        bindings.input_mode = InputMode::Tps;
        assert_eq!(
            ComposingKeyIntent::intent(&alt_enter, true, true, &bindings, PLATFORM),
            ComposingKeyIntent::CommitThenPassThrough
        );
    }
}
