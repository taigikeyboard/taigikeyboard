//! What one input context remembers between keys, and what a key answers.

use super::{Emit, LookupSelection, PAGE_SIZE};
use crate::composing::CandidateSource;
use crate::keys::ShortcutAction;

/// The symbol picker while it is up: the symbols in pick order (recents
/// first) and the highlight over them (Linux roadmap L4: a lookup table, not
/// a window of its own).
#[derive(Debug)]
pub struct SymbolPicker {
    pub symbols: Vec<String>,
    pub selection: LookupSelection,
}

/// What one engine object remembers between keys.
#[derive(Debug)]
pub struct EngineState {
    /// The fetched list and its presentation (cells, scripts).
    pub candidates: CandidateSource,
    /// The highlight and page over `candidates`' cells.
    pub selection: LookupSelection,
    /// The auto-space swap promised to the next key.
    pub armed_auto_space: bool,
    /// The client can take an [`Emit::DeleteSurrounding`].
    pub can_delete_surrounding: bool,
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
    /// The Telex guide is up (as a lookup table) — the first key takes it
    /// down.
    pub telex_guide_shown: bool,
    /// The symbol picker is up; every key is its first.
    pub symbol_picker: Option<SymbolPicker>,
}

impl Default for EngineState {
    fn default() -> Self {
        Self {
            candidates: CandidateSource::default(),
            selection: LookupSelection::new(0, PAGE_SIZE),
            armed_auto_space: false,
            can_delete_surrounding: false,
            is_password_field: false,
            is_table_shown: false,
            latched_chord: None,
            telex_guide_shown: false,
            symbol_picker: None,
        }
    }
}

impl EngineState {
    pub(crate) fn clear_list(&mut self) {
        self.candidates.clear();
        self.selection = LookupSelection::new(0, PAGE_SIZE);
    }
}

/// What a key did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyReply {
    /// `true` = consumed; `false` = the client processes the key itself.
    pub handled: bool,
    pub emits: Vec<Emit>,
}
