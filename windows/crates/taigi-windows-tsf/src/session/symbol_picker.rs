//! The symbol picker: its claim on a key while it is up, the chord that
//! opens and closes it, and the pick that writes a symbol and records it
//! among the recents. Shown through its own presenter, outside the edit
//! session, like the composing list.

use super::{KeyOutcome, KeyPhase, KeyWork};
use crate::composition::{is_password_field, is_read_only};
use crate::edit_session;
use crate::runtime::Runtime;
use crate::text_service::TextService_Impl;
use crate::ui::candidate_window::CandidateWindowContent;
use crate::ui::presenter::CandidatePresenter;
use std::cell::RefCell;
use std::rc::Rc;
use taigi_desktop_core::composing::{CandidateCellContent, ContextToken};
use taigi_desktop_core::keys::{
    ComposingKeyBindings, ComposingKeyIntent, KeyEventSnapshot, SymbolPickerIntent,
};
use taigi_desktop_core::settings::{keys, SettingsDocument};
use taigi_desktop_core::symbols::symbol_picker_symbols;
use taigi_windows_platform::DESKTOP_PLATFORM;
use windows::Win32::UI::TextServices::{ITfContext, TF_ES_READ};

impl TextService_Impl {
    /// The open symbol picker's claim on `snapshot`: TRUE from the test
    /// phase for every key while the list shows, so the delivery is
    /// guaranteed to arrive; at delivery the key is the picker's
    /// (`SymbolPickerIntent`) — or the picker closes and the key goes on
    /// through the composing contract as if it had never been there
    /// (answers `false`, the "TRUE test, FALSE delivery" disagreement).
    pub(super) fn symbol_picker_takes_key(
        &self,
        context: &ITfContext,
        token: ContextToken,
        identity: usize,
        snapshot: &KeyEventSnapshot,
        phase: KeyPhase,
        bindings: &ComposingKeyBindings,
    ) -> bool {
        let Some(picker) = self.live_symbol_picker(token) else {
            return false;
        };
        if phase == KeyPhase::Test {
            return true;
        }
        match SymbolPickerIntent::intent(snapshot, bindings, DESKTOP_PLATFORM) {
            SymbolPickerIntent::Close => {
                self.hide_symbol_picker_of(token);
                true
            }
            SymbolPickerIntent::Navigate(direction) => {
                picker.borrow_mut().navigate(direction, token);
                true
            }
            SymbolPickerIntent::PickSlot(slot) => {
                // An empty slot on a short last page is consumed all the
                // same, as it is on the list: the key is the picker's while
                // it is up.
                let index = picker.borrow().candidate_index_for_key_slot(slot, token);
                self.pick_symbol_cell(context, token, identity, index);
                true
            }
            SymbolPickerIntent::Confirm => {
                let index = picker.borrow().selected_index(token);
                self.pick_symbol_cell(context, token, identity, index);
                true
            }
            SymbolPickerIntent::CloseAndPassThrough => {
                self.hide_symbol_picker_of(token);
                false
            }
        }
    }

    /// The picker, if its popup is really up for `token`. The window is the
    /// one owner: a posted focus hide takes it down behind the key path's
    /// back, and the flag left in the state is simply never read again
    /// until the next show overwrites it.
    fn live_symbol_picker(&self, token: ContextToken) -> Option<Rc<RefCell<CandidatePresenter>>> {
        if self.state.borrow().symbol_picker_cells.is_empty() {
            return None;
        }
        let picker = self.symbol_picker()?;
        let is_up = {
            let picker = picker.borrow();
            picker.is_showing(token) && picker.is_popup_visible()
        };
        is_up.then_some(picker)
    }

    /// The context a preserved picker chord runs against, after the
    /// housekeeping every key gets first (`key_down`): `None` for a context
    /// this service does not know or a read-only one — the key sink's own
    /// bail, so `OnPreservedKey` hands the key back the same way.
    pub(crate) fn symbol_picker_target(
        &self,
        context: &ITfContext,
    ) -> Option<(ContextToken, usize)> {
        self.refresh_settings_if_pending();
        self.drain_pending_ui_hide();
        let target = self.token_for(context)?;
        (!is_read_only(context)).then_some(target)
    }

    /// The picker chord: down if up; otherwise the composition is ended
    /// first — commit first, as vChewing does
    /// (`InputHandler_HandleStates.swift:1110`): the picker writes into the
    /// document, and a composition still marked there would have the symbol
    /// land inside it. A visible highlight commits what is highlighted, the
    /// way Enter does; a composition with no window commits as typed. A
    /// commit that only NAILED a segment leaves the composition running,
    /// and the picker waits for a key that ends it.
    pub(crate) fn toggle_symbol_picker(
        &self,
        context: &ITfContext,
        token: ContextToken,
        identity: usize,
        settings: &SettingsDocument,
    ) {
        if self.live_symbol_picker(token).is_some() {
            self.hide_symbol_picker_of(token);
            return;
        }
        let bindings = ComposingKeyBindings::from_document(settings, DESKTOP_PLATFORM);
        let runtime = Runtime::shared();
        let (is_composing, is_showing) = self.composing_flags(runtime, token);
        if is_composing {
            let intent =
                ComposingKeyIntent::commit_first(is_showing, settings.choice(&keys::INPUT_MODE));
            let outcome = self.run_key(
                context,
                token,
                identity,
                &KeyEventSnapshot::default(),
                &KeyWork::Compose(intent),
                settings,
            );
            let (still_composing, _) = self.composing_flags(runtime, token);
            if outcome != KeyOutcome::Consumed || still_composing {
                return;
            }
        }
        self.present_symbol_picker(context, token, &bindings);
    }

    /// Shows the whole table anchored to the caret — one list, in file
    /// order, so the first pick is the symbol itself (USER 2026-09-09: a
    /// category to choose first "would interrupt the user's experience") — and records the
    /// picker as open. The caret is read under a read-only session (no
    /// composition is open by now, so it is the insertion point); the window
    /// is shown OUTSIDE it, like the composing list (`Surface::apply`). A
    /// list that did not reach the screen leaves nothing behind.
    fn present_symbol_picker(
        &self,
        context: &ITfContext,
        token: ContextToken,
        bindings: &ComposingKeyBindings,
    ) {
        let Some(picker) = self.symbol_picker() else {
            return;
        };
        let (client_id, focus_generation) = {
            let state = self.state.borrow();
            (state.client_id, state.focus_generation)
        };
        // The field is read in the same session as the caret: the picker
        // writes into the document and learns the pick, neither of which
        // belongs in a password field (`symbol_picker_symbols`).
        let read = edit_session::run_sync(context, client_id, TF_ES_READ, |ec| {
            Ok((
                crate::ui::caret::caret(context, ec, None).rect(),
                is_password_field(context, ec),
            ))
        });
        // The session can re-enter the focus callbacks, and a window that
        // has not been created yet has no HWND for their posted hide to
        // reach: a focus that moved under the session gets no picker (the
        // handover's own rule in `run_key`).
        let focus_moved = self.state.borrow().focus_generation != focus_generation
            || self.token_for(context).map(|(t, _)| t) != Some(token);
        let (Ok((Some(caret), is_password)), false) = (read, focus_moved) else {
            log::debug!("symbol_picker.not_shown focus_moved={focus_moved}");
            self.hide_symbol_picker_of(token);
            return;
        };
        // The recents lead (`RecentSymbols`), read once: this is the list
        // the pick will index.
        let settings = Runtime::shared().settings.current();
        let Some(ordered) = symbol_picker_symbols(&settings, is_password) else {
            log::debug!("symbol_picker.not_shown is_password={is_password}");
            self.hide_symbol_picker_of(token);
            return;
        };
        let cells = ordered
            .iter()
            .map(|symbol| CandidateCellContent::new(symbol, None))
            .collect();
        let content = CandidateWindowContent {
            cells,
            slot_key_set: bindings.slot_key_set(),
            lead_cell_is_unkeyed: false,
        };
        // No document: the picker registers no UI-less element
        // (`CandidatePresenter::attach_popup_only`).
        picker
            .borrow_mut()
            .show(content, caret, &settings, token, None);
        // `show` takes the owner before it creates the window, so the level
        // is recorded only for a popup that really reached the screen — an
        // owned list nobody can see would swallow the slot keys.
        let shown = {
            let picker = picker.borrow();
            picker.is_showing(token) && picker.is_popup_visible()
        };
        if !shown {
            picker.borrow_mut().hide(token);
            self.state.borrow_mut().symbol_picker_cells.clear();
            return;
        }
        self.state.borrow_mut().symbol_picker_cells = ordered;
        self.record_focused_caret(caret);
    }

    /// Writes the symbol at `index`, closes the picker and moves the symbol
    /// to the front of the recents, for the next opening. `None` — a slot
    /// with no cell — does nothing, and keeps the picker up.
    fn pick_symbol_cell(
        &self,
        context: &ITfContext,
        token: ContextToken,
        identity: usize,
        index: Option<usize>,
    ) {
        let Some(symbol) =
            index.and_then(|index| self.state.borrow().symbol_picker_cells.get(index).cloned())
        else {
            return;
        };
        self.hide_symbol_picker_of(token);
        // A pick is a key this input method consumes, and the runtime comes
        // up on the first such key (W3): the engine `run_key` claims for the
        // context has to exist, and it hears about the character as the end
        // of a next-word context.
        let runtime = Runtime::shared();
        runtime.prepare_for_first_key();
        let settings = runtime.settings.current();
        let outcome = self.run_key(
            context,
            token,
            identity,
            &KeyEventSnapshot::default(),
            &KeyWork::InsertSymbol(symbol.clone()),
            &settings,
        );
        // Not recorded when the key went to the host: a refused session, or
        // a field that turned into a password field under the open picker
        // (`run_key` answers ToHost for both).
        if outcome != KeyOutcome::Consumed {
            return;
        }
        // Recorded AFTER the write, so the symbol does not wait on the disk;
        // a store that cannot be written still got its symbol.
        runtime.update_settings("record_recent_symbol", |document| {
            document.note_recent_symbol(&symbol);
        });
    }
}
