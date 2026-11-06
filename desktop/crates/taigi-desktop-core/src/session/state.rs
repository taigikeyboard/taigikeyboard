//! What one input context remembers between keys, and what a key answers.

use std::sync::Arc;

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
    /// The symbol picker is up; every key is its first.
    pub symbol_picker: Option<SymbolPicker>,
    /// Taigi or English. A shell opts in by calling
    /// [`super::toggle_language_mode`]; Linux never does, its desktop switches
    /// engines itself (`taigi_linux_core::chrome::mode_indicator`).
    pub language_mode: LanguageMode,
}

impl Default for EngineState {
    fn default() -> Self {
        Self {
            candidates: CandidateSource::default(),
            selection: ListSelection::new(0),
            list_layout: None,
            armed_auto_space: false,
            can_delete_surrounding: false,
            is_password_field: false,
            is_table_shown: false,
            latched_chord: None,
            symbol_picker: None,
            language_mode: LanguageMode::default(),
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
