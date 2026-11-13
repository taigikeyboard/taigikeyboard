//! What one input context remembers between keys, and what a key answers.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use super::{Emit, ListLayout, ListSelection};
use crate::composing::CandidateSource;
use crate::keys::{LanguageMode, ShortcutAction};

/// The symbol picker while it is up: the symbols in pick order (recents
/// first) and the highlight over them (Linux roadmap L4: a lookup table, not
/// a window of its own).
#[derive(Debug)]
pub struct SymbolPicker {
    pub symbols: Vec<String>,
    pub selection: ListSelection,
}

/// What the shell knows of the document just before the caret. The swap's
/// `DeleteSurrounding` deletes blind, so a shell that can read the client's
/// surrounding text has the swap check it first — the Mac's and Windows'
/// verification against the document, with the client's report standing in
/// for the document (#866).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextBeforeCaret {
    /// The shell reports nothing; it keeps the arm from going stale itself
    /// (web: `ime.js` disarms on any change the page did not make).
    #[default]
    Unreported,
    /// The character before a collapsed caret as the client last reported
    /// it. `None`: not reported yet, out of date, at the start of the text,
    /// or a selection — the swap does not run.
    Reported(Option<char>),
}

impl TextBeforeCaret {
    /// From a client's surrounding text; `cursor` and `anchor` count
    /// characters, as IBus and Fcitx5 both do.
    pub fn from_surrounding(text: &str, cursor: usize, anchor: usize) -> Self {
        let before = cursor
            .checked_sub(1)
            .filter(|_| cursor == anchor)
            .and_then(|index| text.chars().nth(index));
        Self::Reported(before)
    }

    /// Whether the swap may take the character before the caret for the
    /// auto space it left.
    pub fn may_be_the_auto_space(self) -> bool {
        match self {
            Self::Unreported => true,
            Self::Reported(before) => before == Some(' '),
        }
    }

    /// A write of ours moved the text: the report waits for the client's next.
    pub(crate) fn outdate(&mut self) {
        if let Self::Reported(before) = self {
            *before = None;
        }
    }
}

/// What one engine object remembers between keys.
#[derive(Debug)]
pub struct EngineState {
    /// The fetched list and its presentation (cells, scripts).
    pub candidates: CandidateSource,
    /// The highlight over `candidates`' cells.
    pub selection: ListSelection,
    /// What lays a list out, for a shell that draws its own window. `None`:
    /// the shell's framework pages it.
    pub list_layout: Option<Arc<dyn ListLayout>>,
    /// The shell draws the Telex guide in a card of its own, apart from the
    /// list, as macOS does (`TelexGuidePanel.swift`): the table never carries
    /// it. `false`: the framework has one table, and the guide is that table.
    pub draws_telex_guide: bool,
    /// The auto-space swap promised to the next key.
    pub armed_auto_space: bool,
    /// The client can take an [`Emit::DeleteSurrounding`].
    pub can_delete_surrounding: bool,
    /// What the client last said precedes the caret, which the swap checks
    /// before it deletes.
    pub text_before_caret: TextBeforeCaret,
    /// The focused field hides what is typed (IBus content purpose
    /// `PASSWORD` / `PIN`, Fcitx5 `CapabilityFlag::Password`): nothing
    /// composes and nothing is learned there — the Windows
    /// `is_password_field` gate.
    pub is_password_field: bool,
    /// Whether the shell currently shows a lookup table of ours.
    pub is_table_shown: bool,
    /// A toggle chord held down: auto-repeat is invisible on the wire, so
    /// the chord is latched until any other key or a release arrives
    /// (Windows `release_toggle_chord_on_other_key`).
    pub latched_chord: Option<ShortcutAction>,
    /// The symbol picker is up; every key is its first.
    pub symbol_picker: Option<SymbolPicker>,
    /// Taigi or English. Windows and IBus opt in through their native key
    /// event paths; Fcitx5 and macOS keep their input source selection outside
    /// this state.
    pub language_mode: LanguageMode,
    /// IBus's bare-Shift recognizer. Windows owns this in its TSF key sink;
    /// Fcitx5 keeps its existing panel-based engine switching.
    pub shift_tap: crate::keys::ShiftTapTracker,
    /// Monotonic origin for the tracker timestamps.
    pub shift_tap_clock: Instant,
    /// Held keycodes let Linux distinguish a fresh press from an auto-repeat,
    /// which IBus does not mark separately.
    pub pressed_keycodes: HashSet<u32>,
}

impl Default for EngineState {
    fn default() -> Self {
        Self {
            candidates: CandidateSource::default(),
            selection: ListSelection::new(0),
            list_layout: None,
            draws_telex_guide: false,
            armed_auto_space: false,
            can_delete_surrounding: false,
            text_before_caret: TextBeforeCaret::default(),
            is_password_field: false,
            is_table_shown: false,
            latched_chord: None,
            symbol_picker: None,
            language_mode: LanguageMode::default(),
            shift_tap: crate::keys::ShiftTapTracker::default(),
            shift_tap_clock: Instant::now(),
            pressed_keycodes: HashSet::new(),
        }
    }
}

impl EngineState {
    pub(crate) fn clear_list(&mut self) {
        self.candidates.clear();
        self.selection = ListSelection::new(0);
    }
}

/// What a key did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyReply {
    /// `true` = consumed; `false` = the client processes the key itself.
    pub handled: bool,
    pub emits: Vec<Emit>,
}
