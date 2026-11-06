//! The composing effects, recorded as the signals a shell will replay. The
//! engine never emits while it holds the engine lock: one key produces a
//! list of [`Emit`]s under the lock, and the shell replays them after the
//! lock is dropped — the Windows rule that the candidate window is touched
//! only after the edit session returned.
//!
//! Counterpart of `taigi-windows-tsf::composition::CompositionEditor`, minus
//! the document: the preedit is the shell's, a commit is the one write, and
//! a preedit character never lives in the document.

use super::ListSelection;
use crate::composing::{
    CandidateCellContent, CandidateSource, ComposingEffectExecutor, IntentSurface,
};
use crate::engine::Effect;
use crate::keys::{CandidateNavigation, TpsKeyCapIndex};

/// One thing for the shell to do, in order. The IBus signal each maps to
/// names the contract; Fcitx5 and other shells render the same meaning.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Emit {
    /// `UpdatePreeditText(text, cursor, visible = true, mode = COMMIT)`;
    /// `caret` in Unicode scalar values, not UTF-16 units.
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
    /// The indicator changed: Fcitx5 re-reads `subModeIconImpl` /
    /// `subModeLabelImpl`, IBus gets `UpdateProperty` with the new symbol
    /// and icon. No payload — each shell asks the runtime for what it draws.
    ModeChanged,
    /// Show the mode briefly — the macOS / Windows HUD flash after a
    /// switch. Fcitx5 `showInputMethodInformation`; IBus has no equivalent
    /// and only the label changes (NAMED DIVERGENCE, Linux roadmap L4).
    AnnounceMode,
    /// Open the settings window where the user left it. On Linux the core
    /// starts this itself, before the replay
    /// (`taigi_linux_core::session::run_launches`).
    OpenSettings,
    /// Show the on-screen TPS key panel, or close it when it is up. Started
    /// like [`Emit::OpenSettings`].
    ToggleTpsKeyboard,
    /// A key the engine took under TPS: the on-screen key panel lights its
    /// cap. The web page draws that panel. On Linux it is a window of the
    /// settings app, another process this side has no channel to (desktop
    /// TPS roadmap D6), so `taigi_linux_core::session::run_launches` drops
    /// this before any replay.
    TpsKeyboardCapTyped(TpsKeyCapIndex),
}

/// What the panel draws: the cells as strings, the per-position labels, the
/// highlighted absolute index, the orientation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LookupTableContent {
    pub cells: Vec<CandidateCellContent>,
    pub labels: Vec<String>,
    pub cursor: u32,
    pub cursor_visible: bool,
    pub page_size: u32,
    pub vertical: bool,
}

impl LookupTableContent {
    /// Each cell as one string, for a panel with one text column: the leading
    /// script, then the other (Pairing, Combined) after a space.
    pub fn joined_candidates(&self) -> Vec<String> {
        self.cells
            .iter()
            .map(|cell| match &cell.annotation {
                Some(annotation) => format!("{} {annotation}", cell.text),
                None => cell.text.clone(),
            })
            .collect()
    }
}

/// Records what one key's composing work asks of the shell.
#[derive(Debug, Default)]
pub struct Recorder {
    pub emits: Vec<Emit>,
    /// Whether the auto-space swap is armed for the next key (the Windows
    /// `armed` range, minus the range: the swap reads the client's
    /// surrounding text, not a caret of ours).
    pub armed_swap: bool,
    /// Whether the client can take a `DeleteSurroundingText` — the shell
    /// reads it from the client.
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
    /// the shell its surrounding text; elsewhere the mark is written after
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
            // No autocomplete surface; the learning handshakes never reach
            // an executor (`ComposingManager` reports them to next word).
            Effect::ClearCandidates
            | Effect::RefreshCandidates
            | Effect::ResetCandidateContext
            | Effect::NextWordUpdateLastSelectedWord { .. }
            | Effect::NextWordWordSelected { .. }
            | Effect::NextWordClearForNewComposing => {}
        }
    }
}

/// One key's surface for the shared executor
/// (`crate::composing::perform_intent`): the recorder, whether
/// this key may swap the auto space, and the selection over the list.
pub(crate) struct KeySurface<'a> {
    pub recorder: Recorder,
    /// The arm this key took over (every key gets exactly one chance at the
    /// swap); the recorder's own `armed_swap` is the arm it leaves.
    pub is_swap_armed: bool,
    pub selection: &'a mut ListSelection,
    pub is_vertical: bool,
}

impl ComposingEffectExecutor for KeySurface<'_> {
    fn execute(&mut self, effect: &Effect) {
        self.recorder.execute(effect);
    }
}

impl IntentSurface for KeySurface<'_> {
    fn insert_external(&mut self, text: &str) {
        self.recorder.insert_external(text);
    }

    fn swap_preceding_space(&mut self, replacement: &str) -> bool {
        self.is_swap_armed && self.recorder.swap_preceding_space(replacement)
    }

    fn arm_swap(&mut self) {
        self.recorder.arm_swap();
    }

    /// A recorded emit cannot fail: the shell replays it after the lock.
    fn has_write_failed(&self) -> bool {
        false
    }

    fn list_changed(&mut self, list: &mut CandidateSource) {
        *self.selection = ListSelection::new(list.len());
        self.selection
            .set_unkeyed_lead(list.leads_with_literal_roman());
    }

    fn list_closed(&mut self) {
        *self.selection = ListSelection::new(0);
    }

    fn selected_index(&self) -> Option<usize> {
        self.selection.selected_index()
    }

    fn index_for_key_slot(&self, slot: usize) -> Option<usize> {
        self.selection.candidate_index_for_key_slot(slot)
    }

    fn navigate(&mut self, direction: CandidateNavigation) {
        self.selection.navigate(direction, self.is_vertical);
    }

    fn tps_keyboard_cap_typed(&mut self, cap: TpsKeyCapIndex) {
        self.recorder.emits.push(Emit::TpsKeyboardCapTyped(cap));
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
