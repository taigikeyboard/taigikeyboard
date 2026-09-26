//! The composing effects, recorded as the signals the daemon will get
//! (roadmap L4 / L13). The engine never emits while it holds the engine
//! lock: one key produces a list of [`Emit`]s under the lock, and the D-Bus
//! method replays them after the lock is dropped — the Windows rule that
//! the candidate window is touched only after the edit session returned.
//!
//! Counterpart of `taigi-windows-tsf::composition::CompositionEditor`, minus
//! the document: on IBus the preedit is the daemon's, `CommitText` is the
//! one write, and a preedit character never lives in the document.

use taigi_desktop_core::composing::ComposingEffectExecutor;
use taigi_desktop_core::engine::Effect;

/// One signal to send the daemon, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Emit {
    /// `UpdatePreeditText(text, cursor, visible = true, mode = COMMIT)`;
    /// `caret` in characters.
    Preedit { text: String, caret: u32 },
    /// `UpdatePreeditText("", 0, visible = false, mode = COMMIT)`.
    ClearPreedit,
    /// `CommitText(text)`.
    Commit(String),
    /// `DeleteSurroundingText(offset, count)` — the auto-space swap, for a
    /// client that declared `IBUS_CAP_SURROUNDING_TEXT`.
    DeleteSurrounding { offset: i32, count: u32 },
    /// `UpdateLookupTable(table, visible = true)`.
    LookupTable(LookupTableContent),
    /// `UpdateLookupTable(<empty>, visible = false)`.
    HideLookupTable,
    /// The mode beside the input method's icon changed (`chrome::mode_label`
    /// / `mode_symbol`): Fcitx5 re-reads `subModeLabelImpl`, IBus gets
    /// `UpdateProperty` with the new symbol. No payload — each shell asks
    /// the runtime for the text it draws.
    ModeChanged,
    /// Show the mode briefly — the macOS / Windows HUD flash after a
    /// switch. Fcitx5 `showInputMethodInformation`; IBus has no equivalent
    /// and only the label changes (NAMED DIVERGENCE, roadmap L4).
    AnnounceMode,
}

/// What the panel draws: the cells as strings, the per-position labels, the
/// highlighted absolute index, the orientation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LookupTableContent {
    pub candidates: Vec<String>,
    pub labels: Vec<String>,
    pub cursor: u32,
    pub cursor_visible: bool,
    pub page_size: u32,
    pub vertical: bool,
}

/// Records what one key's composing work asks of the daemon.
#[derive(Debug, Default)]
pub struct Recorder {
    pub emits: Vec<Emit>,
    /// Whether the auto-space swap is armed for the next key (the Windows
    /// `armed` range, minus the range: the swap reads the daemon's
    /// surrounding text, not a caret of ours).
    pub armed_swap: bool,
    /// Whether the client can take a `DeleteSurroundingText` — read from
    /// `SetCapabilities`.
    pub can_delete_surrounding: bool,
}

impl Recorder {
    pub fn new(can_delete_surrounding: bool) -> Self {
        Self {
            can_delete_surrounding,
            ..Self::default()
        }
    }

    /// Text written outside any composition (the picker's symbol, a mapped
    /// punctuation, the auto space).
    pub fn insert_external(&mut self, text: &str) {
        self.emits.push(Emit::Commit(text.to_owned()));
    }

    pub fn arm_swap(&mut self) {
        self.armed_swap = true;
    }

    /// The §23 swap: the space the last commit left, replaced by
    /// `replacement` (`text + " "`). Possible only where the client hands
    /// the daemon its surrounding text; elsewhere the mark is written after
    /// the space, as typed — NAMED DIVERGENCE, logged once per key.
    pub fn swap_preceding_space(&mut self, replacement: &str) -> bool {
        if !self.can_delete_surrounding {
            log::debug!("auto_space.swap_unavailable — client has no surrounding text");
            return false;
        }
        self.emits.push(Emit::DeleteSurrounding {
            offset: -1,
            count: 1,
        });
        self.emits.push(Emit::Commit(replacement.to_owned()));
        true
    }
}

impl ComposingEffectExecutor for Recorder {
    fn execute(&mut self, effect: &Effect) {
        match effect {
            Effect::UpdatePreedit { text, caret_utf16 } => self.emits.push(Emit::Preedit {
                text: text.clone(),
                caret: char_index_at_utf16(text, *caret_utf16),
            }),
            Effect::ClearPreeditWithoutCommit => self.emits.push(Emit::ClearPreedit),
            Effect::CommitTextReplacingPreedit(text) => {
                self.emits.push(Emit::ClearPreedit);
                self.emits.push(Emit::Commit(text.clone()));
            }
            // NAMED DIVERGENCE (as macOS `ClientEffectExecutor.swift:56-63`
            // and Windows): the preedit never lived in the document, so a
            // delete there would eat a real character.
            Effect::DeleteBackwardFromDocument => {}
            // No autocomplete surface; the learning handshakes never reach
            // an executor (`ComposingManager` reports them to next word).
            Effect::ResetAutocomplete
            | Effect::PerformAutocomplete
            | Effect::ResetAutocompleteContext
            | Effect::NextWordUpdateLastSelectedWord { .. }
            | Effect::NextWordWordSelected { .. }
            | Effect::NextWordClearForNewComposing => {}
        }
    }
}

/// The engine measures the caret in UTF-16 units (the iOS / macOS / Windows
/// document unit); IBus counts characters.
fn char_index_at_utf16(text: &str, caret_utf16: u32) -> u32 {
    let mut units = 0u32;
    let mut chars = 0u32;
    for character in text.chars() {
        if units >= caret_utf16 {
            break;
        }
        units += character.len_utf16() as u32;
        chars += 1;
    }
    chars
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_caret_is_converted_from_utf16_units_to_characters() {
        // trace: "tâi" = 3 chars, 3 units; "𝄞a" = 2 chars, 3 units.
        assert_eq!(char_index_at_utf16("tâi", 0), 0);
        assert_eq!(char_index_at_utf16("tâi", 2), 2);
        assert_eq!(char_index_at_utf16("tâi", 3), 3);
        assert_eq!(char_index_at_utf16("tâi", 9), 3);
        assert_eq!(char_index_at_utf16("𝄞a", 2), 1);
        assert_eq!(char_index_at_utf16("𝄞a", 3), 2);
    }

    #[test]
    fn a_commit_replacing_the_preedit_clears_it_first() {
        let mut recorder = Recorder::new(false);
        recorder.execute(&Effect::UpdatePreedit {
            text: "tai5".into(),
            caret_utf16: 4,
        });
        recorder.execute(&Effect::CommitTextReplacingPreedit("台".into()));
        assert_eq!(
            recorder.emits,
            vec![
                Emit::Preedit {
                    text: "tai5".into(),
                    caret: 4
                },
                Emit::ClearPreedit,
                Emit::Commit("台".into()),
            ]
        );
    }

    #[test]
    fn the_swap_needs_surrounding_text_support() {
        let mut without = Recorder::new(false);
        assert!(!without.swap_preceding_space("， "));
        assert!(without.emits.is_empty());
        let mut with = Recorder::new(true);
        assert!(with.swap_preceding_space("， "));
        assert_eq!(
            with.emits,
            vec![
                Emit::DeleteSurrounding {
                    offset: -1,
                    count: 1
                },
                Emit::Commit("， ".into())
            ]
        );
    }
}
