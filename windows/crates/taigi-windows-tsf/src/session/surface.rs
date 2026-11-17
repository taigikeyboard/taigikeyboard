//! The candidate window as one key sees it (`Surface`): reads answer from
//! the presenter at once, writes are queued and replayed after the edit
//! session (`Surface::apply`), never inside it. Also the shared executor's
//! view of this key's editor and window (`perform_work`).

use super::{KeyOutcome, KeyWork};
use crate::composition::CompositionEditor;
use crate::ui::candidate_window::CandidateWindowContent;
use crate::ui::caret::Caret;
use crate::ui::presenter::CandidatePresenter;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use taigi_desktop_core::composing::{
    insert_symbol, perform_intent, CandidateSource, ComposingEffectExecutor, ComposingManager,
    ContextToken, IntentSurface,
};
use taigi_desktop_core::engine::Effect;
use taigi_desktop_core::keys::{CandidateNavigation, KeyEventSnapshot, TpsKeyCapIndex};
use taigi_desktop_core::settings::SettingsDocument;
use windows::Win32::Foundation::RECT;
use windows::Win32::UI::TextServices::{ITfDocumentMgr, ITfRange};

/// The list as the window takes it: the cells, the keys that pick them, and
/// whether the first cell is the §34 literal, which takes no key
/// (`CandidateSource::leads_with_literal_roman`). Read by both the fresh-list
/// path and the in-place update, so a repaint can never draw a different key
/// row than a `show` of the same list would.
pub(super) fn window_content(
    source: &CandidateSource,
    slot_key_set: taigi_desktop_core::keys::CandidateSlotKeySet,
) -> CandidateWindowContent {
    CandidateWindowContent {
        cells: source.cells(),
        slot_key_set,
        lead_cell_is_unkeyed: source.leads_with_literal_roman(),
    }
}

/// One thing the key decided the window should do, replayed after the
/// session: reading the caret needs the edit cookie, showing a window does
/// not — and showing one inside the session would run the host's
/// `BeginUIElement` / `SetWindowPos` re-entry with the engine lock held.
pub(super) enum SurfaceAction {
    Show {
        content: CandidateWindowContent,
        caret: RECT,
        document: Option<ITfDocumentMgr>,
    },
    Hide,
    Navigate(CandidateNavigation),
}

/// The window, as one key sees it: the presenter (if this host got one)
/// plus the owner token and the slot keys the list was classified against.
/// Reads answer from the presenter at once; writes are queued for `apply`.
pub(super) struct Surface {
    pub(super) presenter: Option<Rc<RefCell<CandidatePresenter>>>,
    pub(super) token: ContextToken,
    pub(super) slot_key_set: taigi_desktop_core::keys::CandidateSlotKeySet,
    pub(super) actions: RefCell<Vec<SurfaceAction>>,
    /// The cap of the TPS glyph the engine took, for the key panel to flash
    /// once the session is over (`TpsKeyboard::flash`).
    pub(super) typed_tps_cap: Cell<Option<TpsKeyCapIndex>>,
}

impl Surface {
    /// Queues the list for the screen anchored to the caret; a host that
    /// cannot say where its caret is gets no window and no list (as on the
    /// Mac) — unless it has only not laid this edit out yet and a window is
    /// already up, which then shows the new list where it stands (mozc keeps
    /// its window on `TF_E_NOLAYOUT` too, `tip_ui_handler_conventional.cc:388`).
    /// The caret is read HERE, under the session's cookie.
    fn present(&self, source: &mut CandidateSource, editor: &CompositionEditor<'_>) {
        if self.presenter.is_none() {
            source.clear();
            return;
        }
        if source.is_empty() {
            self.hide();
            return;
        }
        let caret = match editor.caret() {
            Caret::At(caret) => Some(caret),
            Caret::NotLaidOut => self.anchored_caret(),
            Caret::Unknown => None,
        };
        let Some(caret) = caret else {
            log::debug!("candidates.no_caret_rect — list dropped");
            source.clear();
            self.hide();
            return;
        };
        let content = window_content(source, self.slot_key_set);
        let document = editor.document();
        self.actions.borrow_mut().push(SurfaceAction::Show {
            content,
            caret,
            document,
        });
    }

    pub(super) fn hide(&self) {
        self.actions.borrow_mut().push(SurfaceAction::Hide);
    }

    /// Where this context's window will stand once the queue is applied:
    /// the latest queued show or hide decides, else the window as it is.
    fn anchored_caret(&self) -> Option<RECT> {
        for action in self.actions.borrow().iter().rev() {
            match action {
                SurfaceAction::Show { caret, .. } => return Some(*caret),
                SurfaceAction::Hide => return None,
                SurfaceAction::Navigate(_) => {}
            }
        }
        self.presenter.as_ref()?.borrow().anchored_caret(self.token)
    }

    fn navigate(&self, direction: CandidateNavigation) {
        self.actions
            .borrow_mut()
            .push(SurfaceAction::Navigate(direction));
    }

    /// Replays the queued actions on the presenter, in order. Answers the
    /// caret the list was anchored to when one was shown.
    pub(super) fn apply(&self, settings: &SettingsDocument) -> Option<RECT> {
        let presenter = self.presenter.as_ref()?;
        let actions = std::mem::take(&mut *self.actions.borrow_mut());
        let mut shown_at = None;
        for action in actions {
            let mut presenter = presenter.borrow_mut();
            match action {
                SurfaceAction::Show {
                    content,
                    caret,
                    document,
                } => {
                    presenter.show(content, caret, settings, self.token, document);
                    shown_at = Some(caret);
                }
                SurfaceAction::Hide => presenter.hide(self.token),
                SurfaceAction::Navigate(direction) => presenter.navigate(direction, self.token),
            }
        }
        shown_at
    }

    // Both answer the window's absolute index — a row of the fetched list, one
    // cell per engine row (under Combined the engine lists a candidate as two
    // rows); the commit resolves it through `CandidateSource::resolve`, which
    // also picks the script the cell commits.
    fn selected_index(&self) -> Option<usize> {
        self.presenter.as_ref()?.borrow().selected_index(self.token)
    }

    /// The index the `slot`-th selection KEY addresses — shifted past the §34
    /// literal when that cell leads the keyed row and takes no key.
    fn candidate_index_for_key_slot(&self, slot: usize) -> Option<usize> {
        self.presenter
            .as_ref()?
            .borrow()
            .candidate_index_for_key_slot(slot, self.token)
    }
}

/// The session's work, performed against the engine and the document: the
/// shared executor (`taigi_desktop_core::composing::perform_intent`,
/// `insert_symbol`) over this key's editor and window.
#[allow(clippy::too_many_arguments)]
pub(super) fn perform_work(
    work: &KeyWork,
    snapshot: &KeyEventSnapshot,
    settings: &SettingsDocument,
    manager: &mut ComposingManager,
    editor: &mut CompositionEditor<'_>,
    list: &mut CandidateSource,
    surface: &Surface,
    armed_swap: Option<&ITfRange>,
) -> KeyOutcome {
    let mut intent_surface = TsfIntentSurface {
        editor,
        surface,
        armed_swap,
    };
    match work {
        KeyWork::InsertSymbol(symbol) => {
            insert_symbol(symbol, settings, manager, &mut intent_surface);
            KeyOutcome::Consumed
        }
        KeyWork::Compose(intent) => {
            if perform_intent(
                intent,
                snapshot,
                settings,
                manager,
                list,
                &mut intent_surface,
            ) {
                KeyOutcome::Consumed
            } else {
                KeyOutcome::ToHost
            }
        }
    }
}

/// One key's editor and window as the shared executor reaches them. The
/// list is shown and hidden through `Surface`'s queue — replayed after the
/// session — and the caret it anchors to is read here, under the cookie.
struct TsfIntentSurface<'k, 'e> {
    editor: &'k mut CompositionEditor<'e>,
    surface: &'k Surface,
    /// The auto space this key may swap: the caret the last commit armed.
    /// The swap re-verifies it against the document (`?!` chains).
    armed_swap: Option<&'k ITfRange>,
}

impl ComposingEffectExecutor for TsfIntentSurface<'_, '_> {
    fn execute(&mut self, effect: &Effect) {
        self.editor.execute(effect);
    }
}

impl IntentSurface for TsfIntentSurface<'_, '_> {
    fn insert_external(&mut self, text: &str) {
        self.editor.insert_external(text);
    }

    fn swap_preceding_space(&mut self, replacement: &str) -> bool {
        let Some(anchor) = self.armed_swap else {
            return false;
        };
        self.editor.swap_preceding_space(replacement, anchor)
    }

    fn arm_swap(&mut self) {
        self.editor.arm_swap();
    }

    /// The editor keeps its first failure; `run_key` abandons the
    /// composition on it after the executor returns.
    fn has_write_failed(&self) -> bool {
        self.editor.failure.is_some()
    }

    fn list_changed(&mut self, list: &mut CandidateSource) {
        self.surface.present(list, self.editor);
    }

    fn list_closed(&mut self) {
        self.surface.hide();
    }

    fn selected_index(&self) -> Option<usize> {
        self.surface.selected_index()
    }

    fn index_for_key_slot(&self, slot: usize) -> Option<usize> {
        self.surface.candidate_index_for_key_slot(slot)
    }

    fn navigate(&mut self, direction: CandidateNavigation) {
        self.surface.navigate(direction);
    }

    /// Kept for after the session: the panel is a window, touched only once
    /// the engine lock and the edit session are released, as the list is.
    fn tps_keyboard_cap_typed(&mut self, cap: TpsKeyCapIndex) {
        self.surface.typed_tps_cap.set(Some(cap));
    }
}
