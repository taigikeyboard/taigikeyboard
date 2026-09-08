//! What one key event means to a composition. The whole key contract of the
//! input method in one readable, testable table. Port of
//! `ComposingKeyIntent.swift:93-413`.

use super::bindings::ComposingKeyBindings;
use super::snapshot::{KeyEventSnapshot, NavigationKey};
use super::tone_input_scheme::ToneInputScheme;

/// A move in the candidate window. The six physical keys are handed through
/// raw because what each does depends on the layout (`↓` pages a horizontal
/// window and walks a vertical list); `NextCandidate` / `PreviousCandidate`
/// name an OUTCOME — one step along the list in every layout
/// (`CandidatePresenter.swift:42-53`).
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
    DeleteBackward,
    /// Finish the composition as rendered (the literal commit).
    Commit,
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
    /// Commit whichever candidate the window has highlighted, as the output
    /// settings render it.
    CommitHighlightedCandidate,
    /// Commit the highlighted candidate in the script the output settings do
    /// NOT lead with — the 漢羅 key.
    CommitAlternateScript,
    /// Commit the candidate in this slot of the visible page, counting from
    /// zero — what the slot keys address (`CandidateSlotKeySet`: the bare
    /// letters under Standard, the bare digits under Telex).
    SelectCandidateSlot(usize),
}

impl ComposingKeyIntent {
    /// Classifies `key` for a session whose composition is or is not active,
    /// and whose candidate window is or is not on screen.
    ///
    /// `is_composing` changes the meaning of most keys: Return, Escape, Space
    /// and the digits are the composition's while it runs and the host's the
    /// rest of the time. A bare digit never starts a composition — under
    /// Standard digits are the numeric tone markers of TL and POJ (`tai5`),
    /// and under Telex a tone letter has nothing to mark when idle.
    ///
    /// `is_showing_candidates` is the second state a key turns on: the
    /// arrows, the paging keys, Space and the slot keys belong to the window
    /// while it is up and to the host or the document the rest of the time.
    pub fn intent(
        key: &KeyEventSnapshot,
        is_composing: bool,
        is_showing_candidates: bool,
        bindings: &ComposingKeyBindings,
    ) -> Self {
        let modifiers = key.modifiers;

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
                    '\u{1B}' => return Self::composition_or_host(is_composing, Self::Cancel),
                    '\u{8}' | '\u{7F}' => {
                        return Self::composition_or_host(is_composing, Self::DeleteBackward)
                    }
                    _ => {}
                }
            }
        }
        // Tier 3 — the slot keys, read before the user's bindings so no
        // binding can shadow it. Which keys pick follows from the tone scheme
        // (`ToneInputScheme::slot_key_set`); both sets are bare keys, so a
        // Ctrl+3 keeps falling through to the host-chord guard below.
        if is_showing_candidates {
            if let Some(slot) = bindings.slot_key_set().slot_for_event(key) {
                return Self::SelectCandidateSlot(slot);
            }
        }
        // Tier 4 — what the user put on this key, read before the host-chord
        // guard so a chord they deliberately recorded reaches its action.
        if is_composing {
            if let Some(action) = bindings.action_for(key) {
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
                // through to the host below (`ComposingKeyIntent.swift`).
                if !bindings.is_candidate_window_enabled {
                    return Self::Commit;
                }
            }
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
        // Tier 7 — text. Under Telex the tone letters and `f` are the
        // engine's, not the composition's text. A tone letter or `f` typed
        // outside a composition is document text (like an idle digit): there
        // is no syllable for it to mark. `z` types an initial, so it starts
        // one. One scalar only, for the same grapheme reason as the
        // romanization rule below (`ComposingKeyIntent.swift` reads `first`).
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
        // Swift's `characters.first` is a grapheme, so `a` + a combining mark
        // reads as one non-ASCII character there and goes to the document.
        // Rust's first `char` would be the bare `a`, and the engine would be
        // handed a string it cannot parse (Codex PR2b review).
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

    /// True when `key` is text the host will put into its document, rather
    /// than a key it will act on. Asked by the pass-through path so
    /// punctuation typed outside a composition can be reported to the engine
    /// as the end of a context — while Escape, Return and the arrows are not.
    pub fn is_document_text(key: &KeyEventSnapshot) -> bool {
        if key.modifiers.has_host_chord() || key.is_named_special_key {
            return false;
        }
        match key.characters.as_deref() {
            Some(characters) if !characters.is_empty() => {
                characters.chars().all(Self::is_text_scalar)
            }
            _ => false,
        }
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
        Self::Navigate(match navigation {
            NavigationKey::LeftArrow => CandidateNavigation::Left,
            NavigationKey::RightArrow => CandidateNavigation::Right,
            NavigationKey::UpArrow => CandidateNavigation::Up,
            NavigationKey::DownArrow => CandidateNavigation::Down,
            NavigationKey::PageUp => CandidateNavigation::PageUp,
            NavigationKey::PageDown => CandidateNavigation::PageDown,
        })
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
    use crate::keys::KeyModifiers;

    fn classify(
        key: &KeyEventSnapshot,
        is_composing: bool,
        is_showing: bool,
    ) -> ComposingKeyIntent {
        ComposingKeyIntent::intent(
            key,
            is_composing,
            is_showing,
            &ComposingKeyBindings::default(),
        )
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
        // trace: ComposingKeyIntentTests.swift:47-72.
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
            // classifies it (`ComposingKeyIntent.swift:271`).
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
        // cannot parse — document text, as on macOS.
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
        // trace: ComposingKeyIntentTests.swift (P2) — `v` composing →
        // telexKey, capital too; idle `v` passes through; idle `z` starts.
        let telex = telex_bindings();
        assert_eq!(
            ComposingKeyIntent::intent(&text("v"), true, false, &telex),
            ComposingKeyIntent::TelexKey("v".into())
        );
        assert_eq!(
            ComposingKeyIntent::intent(&text("V"), true, false, &telex),
            ComposingKeyIntent::TelexKey("V".into())
        );
        assert_eq!(
            ComposingKeyIntent::intent(&text("f"), true, true, &telex),
            ComposingKeyIntent::TelexKey("f".into())
        );
        assert_eq!(
            ComposingKeyIntent::intent(&text("v"), false, false, &telex),
            ComposingKeyIntent::PassThrough,
            "a tone letter has nothing to mark when idle"
        );
        assert_eq!(
            ComposingKeyIntent::intent(&text("f"), false, false, &telex),
            ComposingKeyIntent::PassThrough
        );
        assert_eq!(
            ComposingKeyIntent::intent(&text("z"), false, false, &telex),
            ComposingKeyIntent::TelexKey("z".into()),
            "`z` types an initial, so it starts a composition"
        );
        // A letter neither scheme claims is still text.
        assert_eq!(
            ComposingKeyIntent::intent(&text("t"), true, false, &telex),
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
            ComposingKeyIntent::intent(&text("3"), true, true, &telex),
            ComposingKeyIntent::SelectCandidateSlot(2)
        );
        assert_eq!(
            ComposingKeyIntent::intent(&text("3"), true, false, &telex),
            ComposingKeyIntent::CommitThenInsert("3".into()),
            "with no window a digit is punctuation: commit, then insert"
        );
        assert_eq!(
            ComposingKeyIntent::intent(&text("3"), false, false, &telex),
            ComposingKeyIntent::PassThrough
        );
        assert_eq!(
            ComposingKeyIntent::intent(&text("q"), true, true, &telex),
            ComposingKeyIntent::TelexKey("q".into()),
            "`q` is tone 9, not slot 0"
        );
        assert_eq!(
            ComposingKeyIntent::intent(&text(";"), true, true, &telex),
            ComposingKeyIntent::CommitThenInsert(";".into())
        );
        let control_three = KeyEventSnapshot::chord(Some("\u{1B}"), "3", KeyModifiers::CONTROL);
        assert_eq!(
            ComposingKeyIntent::intent(&control_three, true, true, &telex),
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
    fn bare_slot_keys_pick_while_the_window_is_up_and_type_otherwise() {
        assert_eq!(
            classify(&text("q"), true, true),
            ComposingKeyIntent::SelectCandidateSlot(0)
        );
        assert_eq!(
            classify(&text(";"), true, true),
            ComposingKeyIntent::SelectCandidateSlot(8)
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
        // trace: ComposingKeyIntentTests.swift
        // `testCandidateKeys_commitTheTypedText_whenTheWindowIsOff` — S33.
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
                ComposingKeyIntent::intent(&text(characters), true, false, &window_off),
                ComposingKeyIntent::Commit,
                "{name} writes the romanization as typed when there is no window to act on"
            );
            assert_eq!(
                ComposingKeyIntent::intent(&text(characters), false, false, &window_off),
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
    fn is_document_text_accepts_printable_and_rejects_host_keys() {
        for t in ["。", "、", "!", "?", " ", "台", "x"] {
            assert!(ComposingKeyIntent::is_document_text(&text(t)), "{t}");
        }
        for t in ["\u{1B}", "\r", "\u{8}", "\u{7F}", "", "\u{F702}"] {
            assert!(!ComposingKeyIntent::is_document_text(&text(t)), "{t:?}");
        }
        for modifiers in [KeyModifiers::CONTROL, KeyModifiers::ALT, KeyModifiers::WIN] {
            assert!(!ComposingKeyIntent::is_document_text(
                &KeyEventSnapshot::text(".", modifiers)
            ));
        }
        assert!(ComposingKeyIntent::is_document_text(
            &KeyEventSnapshot::text(".", KeyModifiers::SHIFT)
        ));
        assert!(!ComposingKeyIntent::is_document_text(
            &KeyEventSnapshot::named_special(KeyModifiers::NONE)
        ));
        assert!(!ComposingKeyIntent::is_document_text(
            &KeyEventSnapshot::default()
        ));
    }
}
