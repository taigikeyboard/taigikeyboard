//! One key, end to end: snapshot → classifier → engine + document, inside
//! a synchronous edit session. Port of `TaigiInputController.swift`
//! `handle(_:client:)` (`:452-597`) and the commit / auto-space paths
//! (`:618-960`), with the TSF specifics the roadmap fixes: `OnTestKeyDown`
//! and `OnKeyDown` answer through one classification (terminals skip the
//! former); the engine is touched only INSIDE the session so a refused
//! session leaves everything untouched; the runtime comes up on the first
//! key the classifier CONSUMES; ownership is handed over between contexts
//! per the coordinator's contract.
//!
//! Candidates are a headless list here (PR5b): the highlighted index and
//! the slot keys work, the window arrives with PR6.

// 一個按鍵的完整路徑 — 快照→分類→(同步 edit session 內)引擎與文件;候選暫為無視窗列表。

use crate::composition::{is_password_field, is_read_only, CompositionEditor, NullExecutor};
use crate::contexts::ContextEntry;
use crate::contexts::ContextRegistry;
use crate::edit_session;
use crate::key_translation;
use crate::runtime::Runtime;
use crate::settings_launcher;
use crate::text_service::TextService_Impl;
use crate::ui::presenter::CandidatePresenter;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::MutexGuard;
use taigi_windows_core::composing::{
    CandidateCellContent, CandidateCommitOutcome, CandidateListChange, CandidateSource,
    ComposingManager, ComposingSessionCoordinator, ContextToken, ResolvedCommit,
};
use taigi_windows_core::keys::{
    CandidateNavigation, ComposingKeyBindings, ComposingKeyIntent, KeyEventSnapshot, ShortcutAction,
};
use taigi_windows_core::policies;
use taigi_windows_core::settings::{keys, AppearanceMode, InputMode, SettingsDocument};
use taigi_windows_core::strings::{StringKey, StringResolver};
use windows::core::{Interface, BOOL};
use windows::Win32::Foundation::{E_UNEXPECTED, LPARAM, POINT, RECT, WPARAM};
use windows::Win32::UI::TextServices::{
    ITfComposition, ITfCompositionSink, ITfContext, ITfDocumentMgr, ITfRange,
};
use windows_core::IUnknownImpl;

/// Whether TSF is asking (`OnTestKeyDown`) or delivering (`OnKeyDown`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum KeyPhase {
    Test,
    Deliver,
}

/// What a key did to the document, as the classifier and the session
/// agree on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KeyOutcome {
    Consumed,
    ToHost,
}

impl TextService_Impl {
    /// Both key-down entry points. `Test` answers from the classifier
    /// alone (optimistically for the conditional pass-through cases —
    /// a `TRUE` test followed by a `FALSE` delivery hands the key to the
    /// host, which is the safe disagreement); `Deliver` runs the session.
    pub(crate) fn key_down(
        &self,
        context: Option<&ITfContext>,
        wparam: WPARAM,
        lparam: LPARAM,
        phase: KeyPhase,
    ) -> BOOL {
        self.refresh_settings_if_pending();
        let Some(context) = context else {
            return BOOL::from(false);
        };
        let Some(snapshot) = key_translation::snapshot(wparam, lparam) else {
            return BOOL::from(false);
        };
        let Some((token, identity)) = self.token_for(context) else {
            return BOOL::from(false);
        };
        if is_read_only(context) {
            return BOOL::from(false);
        }
        let runtime = Runtime::shared();
        let settings = runtime.settings.current();
        // A chord recorded in the settings window takes effect at the next
        // key, whichever way the file's change was noticed.
        self.sync_preserved_keys(&settings);

        // The global chords, before the classifier — the Carbon hotkey's
        // position on the Mac, and like it independent of whether a
        // composition is running (UX decision, roadmap W5: a bare key is
        // matched here rather than registered as a preserved key). The two
        // preserved keys normally arrive through `OnPreservedKey`; this is
        // their fallback in hosts that bypass preserved keys.
        if let Some(action) = global_action_for(&snapshot, &settings) {
            if phase == KeyPhase::Deliver {
                self.perform_global(action, identity);
            }
            return BOOL::from(true);
        }

        // English mode: below the global chords, above everything that
        // composes. Every key is the document's — including the ones this
        // input method would otherwise consume outside a composition (the
        // auto-space swap, full-width punctuation, the bare 漢羅 key), which
        // is what makes the mode mean "type English here" rather than "stop
        // composing". Nothing needs ending first: entering the mode already
        // committed the composition and spent the arm
        // (`toggle_language_mode`).
        if self.state.borrow().language_mode.is_english() {
            return BOOL::from(false);
        }

        let bindings = ComposingKeyBindings::from_document(&settings);
        let (is_composing, is_showing_candidates) = self.composing_flags(runtime, token, identity);
        let intent =
            ComposingKeyIntent::intent(&snapshot, is_composing, is_showing_candidates, &bindings);
        log::debug!(
            "key.intent {intent:?} composing={is_composing} candidates={is_showing_candidates}"
        );

        let would_consume = match &intent {
            ComposingKeyIntent::PassThrough => {
                self.pass_through_may_consume(&snapshot, &settings, identity)
            }
            ComposingKeyIntent::CommitThenPassThrough => is_composing,
            _ => true,
        };
        if phase == KeyPhase::Test {
            return BOOL::from(would_consume);
        }
        if !would_consume && !matches!(intent, ComposingKeyIntent::CommitThenPassThrough) {
            // Not ours, and nothing to finish — but a character reaching the
            // document outside a composition still ends the next-word
            // context (a full stop typed here is a sentence end).
            if ComposingKeyIntent::is_document_text(&snapshot) {
                if let Some(characters) = snapshot.characters.as_deref() {
                    if let Some(mut coordinator) = runtime
                        .coordinator_if_built()
                        .and_then(|m| m.try_lock().ok())
                    {
                        if let Some(manager) = coordinator.manager(token) {
                            manager.note_character_typed_outside_composition(characters);
                        }
                    }
                }
            }
            return BOOL::from(false);
        }

        // From here the key is ours (or at least ends a composition): the
        // runtime comes up now, never for a key merely observed (W3).
        let setup = runtime.prepare_for_first_key();
        if setup.lexicon.is_none() {
            log::warn!("key.no_lexicon");
        }
        let outcome = self.run_key(context, token, identity, &snapshot, &intent, &settings);
        BOOL::from(outcome == KeyOutcome::Consumed)
    }

    /// What the engine says about this context, read without a session.
    /// A coordinator that does not exist yet (no key consumed so far) reads
    /// as idle.
    fn composing_flags(
        &self,
        runtime: &Runtime,
        token: ContextToken,
        identity: usize,
    ) -> (bool, bool) {
        let is_composing = runtime
            .coordinator_if_built()
            .and_then(|mutex| mutex.try_lock().ok())
            .and_then(|coordinator| {
                coordinator
                    .manager_ref(token)
                    .map(ComposingManager::is_composing)
            })
            .unwrap_or(false);
        let _ = identity;
        let presenter = self.state.borrow().presenter.clone();
        let is_showing = presenter.is_some_and(|presenter| presenter.borrow().is_showing(token));
        (is_composing, is_showing)
    }

    /// The two pass-through keys this input method consumes: attaching
    /// punctuation right after an auto space (the swap), and full-width
    /// punctuation in hanji-first mode. Optimistic — the swap is verified
    /// against the document only in the session.
    fn pass_through_may_consume(
        &self,
        snapshot: &KeyEventSnapshot,
        settings: &SettingsDocument,
        identity: usize,
    ) -> bool {
        if !ComposingKeyIntent::is_document_text(snapshot) {
            return false;
        }
        let Some(characters) = snapshot.characters.as_deref() else {
            return false;
        };
        let is_armed = self
            .state
            .borrow_mut()
            .contexts
            .entry_mut(identity)
            .is_some_and(|entry| entry.state.armed_auto_space.is_some());
        if is_armed
            && policies::is_attaching_punctuation(characters)
            && settings.bool(&keys::IS_AUTO_SPACE_ENABLED)
        {
            return true;
        }
        full_width_mapped(settings, characters).is_some()
    }

    /// The session: deferred engine work, ownership handover, password
    /// gate, then the intent — and one consistency rule on top: a document
    /// write that fails ABANDONS the composition on both sides (engine
    /// reset, TSF composition ended) and the key stays consumed, so the
    /// engine and the document never disagree and the host never sees the
    /// key twice.
    fn run_key(
        &self,
        context: &ITfContext,
        token: ContextToken,
        identity: usize,
        snapshot: &KeyEventSnapshot,
        intent: &ComposingKeyIntent,
        settings: &SettingsDocument,
    ) -> KeyOutcome {
        let runtime = Runtime::shared();
        // A hide a focus callback could not post (the presenter was busy
        // inside a session) lands here, before anything new is shown.
        let hide_pending = std::mem::take(&mut self.state.borrow_mut().is_ui_hide_pending);
        if hide_pending {
            let presenter = self.state.borrow().presenter.clone();
            if let Some(presenter) = presenter {
                presenter.borrow_mut().hide_for_handover();
            }
        }
        let mut coordinator = match runtime.coordinator().lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        self.run_deferred_engine_work(&mut coordinator);
        // Contract point 3: the focus generation is the re-validation — any
        // focus / context event during the handover's re-entrancy bumps it,
        // and the key then goes back to the host rather than into a
        // composition that moved.
        let focus_generation = self.state.borrow().focus_generation;
        self.hand_over_if_needed(&mut coordinator, token);
        let moved = self.state.borrow().focus_generation != focus_generation
            || self.token_for(context).map(|(t, _)| t) != Some(token);
        if moved {
            log::info!("key.dropped_after_handover reason=focus_moved");
            return KeyOutcome::ToHost;
        }
        let client_id = self.state.borrow().client_id;
        let atom = self.state.borrow().display_attribute_atom;
        let sink: ITfCompositionSink = self
            .to_object()
            .to_interface::<ITfCompositionSink>()
            .to_owned();
        // Every key gets exactly one chance at the swap: the arm is consumed
        // here, before any early return, and only the auto-space paths below
        // re-arm it.
        let (composition, armed_swap, mut list, presenter) = {
            let mut state = self.state.borrow_mut();
            let presenter = state.presenter.clone();
            match state.contexts.entry_mut(identity) {
                Some(entry) => (
                    entry.state.composition.take(),
                    entry.state.armed_auto_space.take(),
                    std::mem::take(&mut entry.state.candidates),
                    presenter,
                ),
                None => (None, None, CandidateSource::default(), presenter),
            }
        };
        let mut armed_after: Option<ITfRange> = None;
        let surface = Surface {
            presenter,
            token,
            slot_key_set: ComposingKeyBindings::from_document(settings).slot_key_set,
            actions: RefCell::new(Vec::new()),
        };

        let session = edit_session::read_write(context, client_id, |ec| {
            if is_password_field(context, ec) {
                return Ok((KeyOutcome::ToHost, composition.clone()));
            }
            let manager = coordinator.claim(token);
            let mut editor = CompositionEditor::new(context, ec, &sink, atom, composition.clone());
            let outcome = perform_intent(
                intent,
                snapshot,
                settings,
                manager,
                &mut editor,
                &mut list,
                &surface,
                armed_swap.as_ref(),
            );
            if let Some(error) = editor.failure.take() {
                log::error!("key.document_write_failed error={error} — composition abandoned");
                manager.cancel_composition(&mut NullExecutor);
                editor.abandon();
                list.clear();
                surface.hide();
                return Ok((KeyOutcome::Consumed, None));
            }
            armed_after = editor.armed.take();
            Ok((outcome, editor.composition.take()))
        });
        drop(coordinator);
        // The window is touched only now — outside the session and the
        // engine lock, so a host re-entering us from `BeginUIElement` /
        // `SetWindowPos` finds neither held.
        if let Some(caret) = surface.apply(settings) {
            self.state.borrow_mut().focused_caret = POINT {
                x: caret.left,
                y: caret.top,
            };
        }

        let (outcome, composition_after) = match session {
            Ok(answer) => answer,
            Err(error) => {
                // The session was refused (TF_E_SYNCHRONOUS, read-only,
                // teardown): engine and document untouched, key to the host.
                log::warn!("key.session_refused error={error}");
                (KeyOutcome::ToHost, composition)
            }
        };
        // Moves only, under the borrow; the handles were counted outside.
        let previous = {
            let mut state = self.state.borrow_mut();
            state.contexts.entry_mut(identity).map(|entry| {
                let previous = std::mem::replace(&mut entry.state.composition, composition_after);
                entry.state.candidates = list;
                let previous_arm =
                    std::mem::replace(&mut entry.state.armed_auto_space, armed_after);
                (previous, previous_arm)
            })
        };
        drop(previous);
        drop(armed_swap);
        outcome
    }

    /// `ITfCandidateListUIElementBehavior::Finalize`: the host commits the
    /// highlighted candidate — the key path with the commit key's intent.
    pub(crate) fn ui_element_finalize(&self) -> windows::core::Result<()> {
        self.run_from_ui_element(ComposingKeyIntent::CommitHighlightedCandidate)
    }

    /// `ITfCandidateListUIElementBehavior::Abort`: the host cancels the
    /// composition — Escape's intent.
    pub(crate) fn ui_element_abort(&self) -> windows::core::Result<()> {
        self.run_from_ui_element(ComposingKeyIntent::Cancel)
    }

    /// A host-initiated synchronous call on the TIP thread, outside any
    /// session of ours: it runs like a key, under the owning context's own
    /// edit session. Refused (`E_UNEXPECTED`) when no list is up, the
    /// owner's context is gone, or the engine is busy — which means the
    /// host re-entered us from inside our own session.
    fn run_from_ui_element(&self, intent: ComposingKeyIntent) -> windows::core::Result<()> {
        let busy = || windows::core::Error::from_hresult(E_UNEXPECTED);
        let presenter = self.state.borrow().presenter.clone();
        let owner = presenter
            .as_ref()
            .and_then(|presenter| presenter.try_borrow().ok()?.owner())
            .ok_or_else(busy)?;
        let context = {
            let mut state = self.state.borrow_mut();
            state
                .contexts
                .entry_by_token_mut(owner)
                .map(|entry| Rc::clone(&entry.context))
        }
        .ok_or_else(busy)?;
        let identity = ContextRegistry::identity(&context).ok_or_else(busy)?;
        let runtime = Runtime::shared();
        let engine_free = runtime
            .coordinator_if_built()
            .is_some_and(|mutex| mutex.try_lock().is_ok());
        if !engine_free {
            log::warn!("ui_element.reentered intent={intent:?}");
            return Err(busy());
        }
        let settings = runtime.settings.current();
        let outcome = self.run_key(
            &context,
            owner,
            identity,
            &KeyEventSnapshot::default(),
            &intent,
            &settings,
        );
        log::debug!("ui_element.intent {intent:?} outcome={outcome:?}");
        Ok(())
    }

    /// Engine work a callback could not do because the engine was busy
    /// (contract: a callback never waits on the coordinator): applied at the
    /// next key, under the lock the key already holds.
    fn run_deferred_engine_work(
        &self,
        coordinator: &mut MutexGuard<'_, ComposingSessionCoordinator>,
    ) {
        let (releases, resets) = {
            let mut state = self.state.borrow_mut();
            let releases = std::mem::take(&mut state.deferred_releases);
            let resets: Vec<ContextToken> = state
                .contexts
                .entries_mut()
                .filter_map(|entry| {
                    std::mem::take(&mut entry.state.is_engine_reset_pending).then_some(entry.token)
                })
                .collect();
            (releases, resets)
        };
        for token in resets {
            if let Some(manager) = coordinator.manager(token) {
                manager.cancel_composition(&mut NullExecutor);
            }
        }
        for token in releases {
            coordinator.release(token);
        }
    }

    /// A key from context B while A owns the engine: A's composition is
    /// finished into A's own document under A's own session, and ownership
    /// moves ONLY when that session ran and every document write in it
    /// succeeded (contract point 2). If A is gone, refuses, or the write
    /// fails, the engine is reset instead — B starts fresh, and A's
    /// document keeps whatever the host still shows.
    fn hand_over_if_needed(
        &self,
        coordinator: &mut MutexGuard<'_, ComposingSessionCoordinator>,
        incoming: ContextToken,
    ) {
        let Some(owner) = coordinator.current_owner() else {
            return;
        };
        if owner == incoming {
            return;
        }
        let (previous_context, previous_composition) = {
            let mut state = self.state.borrow_mut();
            match state.contexts.entry_by_token_mut(owner) {
                Some(entry) => (
                    Some(std::rc::Rc::clone(&entry.context)),
                    entry.state.composition.take(),
                ),
                None => (None, None),
            }
        };
        let client_id = self.state.borrow().client_id;
        let atom = self.state.borrow().display_attribute_atom;
        let is_composing = coordinator
            .manager_ref(owner)
            .is_some_and(|m| m.is_composing());
        let mut finished = !is_composing;
        if let (Some(previous_context), true) = (&previous_context, is_composing) {
            let previous_context: &ITfContext = previous_context;
            let sink: ITfCompositionSink = self
                .to_object()
                .to_interface::<ITfCompositionSink>()
                .to_owned();
            let session = edit_session::read_write(previous_context, client_id, |ec| {
                let mut editor = CompositionEditor::new(
                    previous_context,
                    ec,
                    &sink,
                    atom,
                    previous_composition.clone(),
                );
                if let Some(manager) = coordinator.manager(owner) {
                    manager.commit_composition(&mut editor);
                }
                let wrote_cleanly = editor.failure.is_none() && editor.composition.is_none();
                Ok(wrote_cleanly)
            });
            finished = matches!(session, Ok(true));
        }
        if !finished {
            // The previous document could not take its composition: the
            // engine forgets it rather than carrying it into another window.
            if let Some(manager) = coordinator.manager(owner) {
                manager.cancel_composition(&mut NullExecutor);
            }
        }
        log::info!("key.handover finished_previous={finished}");
        // The previous context's candidates and window are gone with its
        // ownership (`hideForHandover`).
        let presenter = self.state.borrow().presenter.clone();
        if let Some(presenter) = presenter {
            presenter.borrow_mut().hide_for_handover();
        }
        let stale = {
            let mut state = self.state.borrow_mut();
            state.contexts.entry_by_token_mut(owner).map(|entry| {
                entry.state.candidates.clear();
                let arm = entry.state.armed_auto_space.take();
                (arm, entry.state.composition.take())
            })
        };
        drop(stale);
        drop(previous_context);
        drop(previous_composition);
        // `claim` by the caller starts the fresh session for `incoming`.
    }

    /// The host ended a composition itself (a click elsewhere, focus loss):
    /// the text stays as the host left it; only the engine is reset.
    pub(crate) fn composition_terminated(&self, composition: &ITfComposition) {
        let candidate_identity = composition
            .cast::<windows::core::IUnknown>()
            .ok()
            .map(|u| u.as_raw() as usize);
        let (token, held) = {
            let mut state = self.state.borrow_mut();
            let found = state.contexts.entries_mut().find(|entry| {
                entry
                    .state
                    .composition
                    .as_ref()
                    .and_then(|held| held.cast::<windows::core::IUnknown>().ok())
                    .map(|u| u.as_raw() as usize)
                    == candidate_identity
            });
            match found {
                Some(entry) => {
                    entry.state.candidates.clear();
                    entry.state.armed_auto_space = None;
                    (Some(entry.token), entry.state.composition.take())
                }
                None => (None, None),
            }
        };
        drop(held);
        let Some(token) = token else { return };
        let presenter = self.state.borrow().presenter.clone();
        if let Some(presenter) = presenter {
            presenter.borrow_mut().hide(token);
        }
        let reset_now = Runtime::shared()
            .coordinator_if_built()
            .is_some_and(|mutex| match mutex.try_lock() {
                Ok(mut coordinator) => {
                    if let Some(manager) = coordinator.manager(token) {
                        manager.cancel_composition(&mut NullExecutor);
                    }
                    true
                }
                Err(_) => false,
            });
        if !reset_now {
            // The engine is busy (a session in flight re-entered us): the
            // reset is applied at the next key rather than skipped.
            let mut state = self.state.borrow_mut();
            if let Some(entry) = state.contexts.entry_by_token_mut(token) {
                entry.state.is_engine_reset_pending = true;
            }
        }
    }

    /// Finishes every composition this service still holds (deactivation):
    /// best-effort commits under each context's own session, then the
    /// engine is released for every token.
    pub(crate) fn finish_all_compositions(&self, entries: &mut [ContextEntry]) {
        let Some(mutex) = Runtime::shared().coordinator_if_built() else {
            return;
        };
        let Ok(mut coordinator) = mutex.try_lock() else {
            return;
        };
        let client_id = self.state.borrow().client_id;
        let atom = self.state.borrow().display_attribute_atom;
        for entry in entries.iter_mut() {
            if coordinator
                .manager_ref(entry.token)
                .is_some_and(|m| m.is_composing())
            {
                let sink: ITfCompositionSink = self
                    .to_object()
                    .to_interface::<ITfCompositionSink>()
                    .to_owned();
                let composition = entry.state.composition.take();
                let context: &ITfContext = &entry.context;
                let _ = edit_session::read_write(context, client_id, |ec| {
                    let mut editor =
                        CompositionEditor::new(context, ec, &sink, atom, composition.clone());
                    if let Some(manager) = coordinator.manager(entry.token) {
                        manager.commit_composition(&mut editor);
                    }
                    Ok(())
                });
            }
            coordinator.release(entry.token);
        }
    }

    /// The mode HUD: the name of the mode just switched into, on the monitor
    /// of the last caret this service anchored to. Every mode switch ends
    /// this way — the chord or the tap fires from anywhere, and a mode that
    /// changed with no notice reads as the keyboard breaking (USER
    /// 2026-08-26).
    fn flash_mode_label(&self, runtime: &Runtime, settings: &SettingsDocument, label: StringKey) {
        let Some(flash) = self.state.borrow().mode_flash.clone() else {
            return;
        };
        let text = StringResolver::new(runtime.display_language())
            .resolve(label)
            .to_owned();
        let anchor = self.state.borrow().focused_caret;
        let appearance: AppearanceMode = settings.choice(&keys::APPEARANCE_MODE);
        flash.borrow_mut().flash(&text, anchor, appearance);
    }

    /// A Shift tap switched 中/英. Runs on the TIP thread from `OnKeyUp`,
    /// outside any session of ours, and in this order: what is half-typed is
    /// written to the document under the mode it was typed in, the state that
    /// mode left behind is spent, and only then does the mode flip and the
    /// three indicators (compartment, tray letter, flash) follow it.
    ///
    /// A busy engine means a host re-entered us from inside our own session;
    /// the switch is skipped entirely rather than half-applied, and the next
    /// tap does it.
    pub(crate) fn toggle_language_mode(
        &self,
        context: &ITfContext,
        token: ContextToken,
        identity: usize,
    ) {
        let runtime = Runtime::shared();
        // A host that re-entered us from inside our own session: the switch
        // is skipped whole rather than half-applied, and the next tap does it.
        // Same probe `run_from_ui_element` makes, same polarity.
        let engine_free = runtime
            .coordinator_if_built()
            .is_some_and(|mutex| mutex.try_lock().is_ok());
        if !engine_free {
            log::warn!("language_mode.reentered — switch skipped");
            return;
        }
        // One intent, one snapshot (`settings/mod.rs`): the commit and the
        // flash must not straddle a settings change.
        let settings = runtime.settings.current();
        let next = self.state.borrow().language_mode.toggled();
        let (is_composing, _) = self.composing_flags(runtime, token, identity);
        if is_composing {
            let outcome = self.run_key(
                context,
                token,
                identity,
                &KeyEventSnapshot::default(),
                &ComposingKeyIntent::Commit,
                &settings,
            );
            // The commit is the switch's precondition, not a courtesy: a
            // refused edit session, a focus that moved under the handover or a
            // password field all leave the composition standing, and flipping
            // anyway would reset the engine under text the document still
            // shows. Nothing has been touched yet at this point, so giving up
            // here leaves the whole switch un-run — the next tap does it.
            if outcome != KeyOutcome::Consumed {
                log::warn!("language_mode.commit_refused — switch skipped");
                return;
            }
        }
        // The auto-space arm promises the NEXT key a swap; that key now
        // belongs to the other mode, so the promise is spent here rather than
        // left to move a space the user typed in English. The candidate list
        // goes with the composition it described.
        let presenter = {
            let mut state = self.state.borrow_mut();
            if let Some(entry) = state.contexts.entry_mut(identity) {
                entry.state.armed_auto_space = None;
                entry.state.candidates.clear();
            }
            state.presenter.clone()
        };
        if let Some(presenter) = presenter {
            presenter.borrow_mut().hide(token);
        }
        // The next-word context is Taiwanese. English typed after the switch
        // is not the predecessor of the word typed after the switch back, and
        // the engine is what would otherwise keep believing it is.
        if let Some(mut coordinator) = runtime
            .coordinator_if_built()
            .and_then(|mutex| mutex.try_lock().ok())
        {
            if let Some(manager) = coordinator.manager(token) {
                manager.start_new_session();
            }
        }

        self.set_language_mode(next);
        self.flash_mode_label(runtime, &settings, next.flash_label_key());
        log::info!("language_mode.switched mode={next:?}");
    }

    /// A global shortcut fired (preserved key or the key sink's match).
    pub(crate) fn perform_global(&self, action: ShortcutAction, identity: usize) {
        let runtime = Runtime::shared();
        match action {
            ShortcutAction::OpenLastSettingsPane => settings_launcher::open_settings(),
            ShortcutAction::ToggleRomanization => {
                let Some(store) = runtime.settings_store() else {
                    log::warn!("shortcut.no_settings_store");
                    return;
                };
                if let Err(error) = store.update(|document| {
                    let next = match document.choice(&keys::INPUT_MODE) {
                        InputMode::Tl => InputMode::Poj,
                        _ => InputMode::Tl,
                    };
                    document.set_choice(&keys::INPUT_MODE, next);
                }) {
                    log::error!("shortcut.toggle_romanization_failed error={error}");
                }
                // The candidates on screen were fetched under the old
                // romanization; they go with the mode that produced them —
                // then the HUD, because the chord fires from anywhere and a
                // romanization that changed with no notice reads as the
                // keyboard breaking (USER 2026-08-26).
                let (token, presenter) = {
                    let mut state = self.state.borrow_mut();
                    let token = state.contexts.entry_mut(identity).map(|entry| {
                        entry.state.candidates.clear();
                        entry.token
                    });
                    (token, state.presenter.clone())
                };
                if let (Some(token), Some(presenter)) = (token, presenter) {
                    presenter.borrow_mut().hide(token);
                }
                let settings = runtime.settings.current();
                let mode: InputMode = settings.choice(&keys::INPUT_MODE);
                let label = match mode {
                    InputMode::Poj => StringKey::SettingsPojMode,
                    _ => StringKey::SettingsTlMode,
                };
                self.flash_mode_label(runtime, &settings, label);
            }
            ShortcutAction::ToggleTranslateSwapped => {
                let Some(store) = runtime.settings_store() else {
                    log::warn!("shortcut.no_settings_store");
                    return;
                };
                // Inert unless side-by-side (`allows_swap_toggle`): no write,
                // no flash, the stored swap waits for the way back. Read off
                // the same snapshot every other consumer uses.
                let display_mode = runtime
                    .settings
                    .current()
                    .engine_settings()
                    .candidate_display_mode;
                if !display_mode.allows_swap_toggle() {
                    return;
                }
                if let Err(error) = store.update(|document| {
                    let swapped = document.bool(&keys::IS_TRANSLATE_SWAPPED);
                    document.set_bool(&keys::IS_TRANSLATE_SWAPPED, !swapped);
                }) {
                    log::error!("shortcut.toggle_translate_swapped_failed error={error}");
                }
                // The list STAYS: the swap changes how a candidate displays,
                // never which exist — re-presented in place, selection kept
                // (dismissing read as the window vanishing, 2026-08-21).
                self.represent_open_list(identity, runtime, false);
            }
            ShortcutAction::CycleCandidateDisplayMode => {
                let Some(store) = runtime.settings_store() else {
                    log::warn!("shortcut.no_settings_store");
                    return;
                };
                if let Err(error) = store.update(|document| {
                    let next = document.choice(&keys::CANDIDATE_DISPLAY_MODE).next();
                    document.set_choice(&keys::CANDIDATE_DISPLAY_MODE, next);
                }) {
                    log::error!("shortcut.cycle_candidate_display_mode_failed error={error}");
                }
                // The mode changes which candidates exist (invariants §44),
                // not only how they draw — so the open list is re-fetched
                // under the new mode and re-rendered in place, the pane's own
                // behaviour. Then the HUD with the new mode's name, as the
                // romanization switch does — the chord fires from anywhere.
                self.represent_open_list(identity, runtime, true);
                let settings = runtime.settings.current();
                let mode = settings.engine_settings().candidate_display_mode;
                self.flash_mode_label(runtime, &settings, mode.label_key());
            }
        }
    }

    /// Re-presents the open list for `identity` under the settings in force
    /// right now: re-fetched first when the change alters which candidates
    /// exist (`refetch`), where an empty answer takes the window down —
    /// otherwise the same list re-rendered in place. Never hidden first:
    /// from mid-composition that reads as the window vanishing.
    fn represent_open_list(&self, identity: usize, runtime: &Runtime, refetch: bool) {
        let (token, presenter) = {
            let mut state = self.state.borrow_mut();
            let token = state.contexts.entry_mut(identity).map(|entry| entry.token);
            (token, state.presenter.clone())
        };
        let (Some(token), Some(presenter), Some(mutex)) =
            (token, presenter, runtime.coordinator_if_built())
        else {
            return;
        };
        let Ok(mut coordinator) = mutex.try_lock() else {
            return;
        };
        let Some(manager) = coordinator.manager(token) else {
            return;
        };
        let settings = runtime.settings.current();
        let cells = {
            let mut state = self.state.borrow_mut();
            let Some(entry) = state.contexts.entry_mut(identity) else {
                return;
            };
            let source = &mut entry.state.candidates;
            if !refetch {
                source.refresh_presentation(manager);
                Some(source.cells())
            } else {
                match manager.fetch_candidates().list_change() {
                    CandidateListChange::Replace(candidates) => {
                        source.set(candidates, manager);
                        Some(source.cells())
                    }
                    CandidateListChange::Clear => {
                        source.clear();
                        None
                    }
                }
            }
        };
        match cells {
            Some(cells) => presenter.borrow_mut().update_cells(cells, &settings, token),
            None => presenter.borrow_mut().hide(token),
        }
    }
}

/// One thing the key decided the window should do, replayed after the
/// session: reading the caret needs the edit cookie, showing a window does
/// not — and showing one inside the session would run the host's
/// `BeginUIElement` / `SetWindowPos` re-entry with the engine lock held.
enum SurfaceAction {
    Show {
        cells: Vec<CandidateCellContent>,
        caret: RECT,
        document: Option<ITfDocumentMgr>,
    },
    Hide,
    Navigate(CandidateNavigation),
}

/// The window, as one key sees it: the presenter (if this host got one)
/// plus the owner token and the slot keys the list was classified against.
/// Reads answer from the presenter at once; writes are queued for `apply`.
struct Surface {
    presenter: Option<Rc<RefCell<CandidatePresenter>>>,
    token: ContextToken,
    slot_key_set: taigi_windows_core::keys::CandidateSlotKeySet,
    actions: RefCell<Vec<SurfaceAction>>,
}

impl Surface {
    /// Queues the list for the screen anchored to the caret; a host that
    /// cannot say where its caret is gets no window and no list (as on the
    /// Mac). The caret is read HERE, under the session's cookie.
    fn present(&self, source: &mut CandidateSource, editor: &CompositionEditor<'_>) {
        if self.presenter.is_none() {
            source.clear();
            return;
        }
        if source.is_empty() {
            self.hide();
            return;
        }
        let Some(caret) = editor.caret_rect() else {
            log::debug!("candidates.no_caret_rect — list dropped");
            source.clear();
            self.hide();
            return;
        };
        let cells = source.cells();
        let document = editor.document();
        self.actions.borrow_mut().push(SurfaceAction::Show {
            cells,
            caret,
            document,
        });
    }

    fn hide(&self) {
        self.actions.borrow_mut().push(SurfaceAction::Hide);
    }

    fn navigate(&self, direction: CandidateNavigation) {
        self.actions
            .borrow_mut()
            .push(SurfaceAction::Navigate(direction));
    }

    /// Replays the queued actions on the presenter, in order. Answers the
    /// caret the list was anchored to when one was shown.
    fn apply(&self, settings: &SettingsDocument) -> Option<RECT> {
        let presenter = self.presenter.as_ref()?;
        let actions = std::mem::take(&mut *self.actions.borrow_mut());
        let mut shown_at = None;
        for action in actions {
            let mut presenter = presenter.borrow_mut();
            match action {
                SurfaceAction::Show {
                    cells,
                    caret,
                    document,
                } => {
                    presenter.show(
                        cells,
                        self.slot_key_set,
                        caret,
                        settings,
                        self.token,
                        document,
                    );
                    shown_at = Some(caret);
                }
                SurfaceAction::Hide => presenter.hide(self.token),
                SurfaceAction::Navigate(direction) => presenter.navigate(direction, self.token),
            }
        }
        shown_at
    }

    // Both answer the window's absolute index, which is a CELL index into
    // the source's presentation (合用 shows two cells per candidate) — the
    // commit resolves it through `CandidateSource::resolve`, never by
    // indexing the fetched list.
    fn selected_index(&self) -> Option<usize> {
        self.presenter.as_ref()?.borrow().selected_index(self.token)
    }

    fn candidate_index_for_slot(&self, slot: usize) -> Option<usize> {
        self.presenter
            .as_ref()?
            .borrow()
            .candidate_index_for_slot(slot, self.token)
    }
}

/// The intent, performed against the engine and the document in one
/// session (`handle(_:client:)`'s switch).
#[allow(clippy::too_many_arguments)]
fn perform_intent(
    intent: &ComposingKeyIntent,
    snapshot: &KeyEventSnapshot,
    settings: &SettingsDocument,
    manager: &mut ComposingManager,
    editor: &mut CompositionEditor<'_>,
    list: &mut CandidateSource,
    surface: &Surface,
    armed_swap: Option<&ITfRange>,
) -> KeyOutcome {
    match intent {
        ComposingKeyIntent::Input(text) => {
            manager.append(text, editor);
            refresh_candidates(manager, list);
            surface.present(list, editor);
            KeyOutcome::Consumed
        }
        ComposingKeyIntent::DeleteBackward => {
            manager.delete_backward(editor);
            refresh_candidates(manager, list);
            surface.present(list, editor);
            KeyOutcome::Consumed
        }
        ComposingKeyIntent::Commit => {
            // The preedit AS TYPED: romanization on a platform shipping TL and
            // POJ only, whichever script the candidate list led with.
            let committed = manager
                .commit_composition(editor)
                .map(|text| ResolvedCommit {
                    text,
                    wrote_romanization: raw_preedit_wrote_romanization(settings),
                });
            list.clear();
            surface.hide();
            append_auto_space(committed.as_ref(), settings, editor);
            KeyOutcome::Consumed
        }
        ComposingKeyIntent::Cancel => {
            manager.cancel_composition(editor);
            list.clear();
            surface.hide();
            KeyOutcome::Consumed
        }
        ComposingKeyIntent::CommitThenInsert(text) => {
            // Mapped before the auto-space augmentation so the full-width
            // character rides the same single mutation as the commit. Both
            // rewrites CAN fire: this path commits the preedit as typed,
            // which is romanization under every mode, while the full-width
            // map still answers to the output MODE — so 漢字優先 gets
            // `taigi？ `. That approximation is a 全形標點 policy question,
            // left standing (macOS pins the same pair).
            let document_text = full_width_mapped(settings, text).unwrap_or_else(|| text.clone());
            let gate = auto_space_gate(settings, raw_preedit_wrote_romanization(settings));
            let insert = policies::augment_insert(&document_text, manager.display_text(), gate);
            let committed = manager.commit_composition_then_insert(&insert.text, editor);
            list.clear();
            surface.hide();
            if insert.leaves_trailing_auto_space && committed.is_some() {
                editor.arm_swap();
            }
            KeyOutcome::Consumed
        }
        ComposingKeyIntent::CommitThenPassThrough => {
            manager.commit_composition(editor);
            list.clear();
            surface.hide();
            KeyOutcome::ToHost
        }
        ComposingKeyIntent::PassThrough => {
            let Some(characters) = snapshot.characters.as_deref() else {
                return KeyOutcome::ToHost;
            };
            if let Some(anchor) = armed_swap {
                // The arm's EXISTENCE is the verdict — it is only ever set
                // after a commit that wrote romanization earned its space — so
                // only 自動空白 itself is re-read live here.
                if ComposingKeyIntent::is_document_text(snapshot)
                    && policies::is_attaching_punctuation(characters)
                    && settings.bool(&keys::IS_AUTO_SPACE_ENABLED)
                    && editor.swap_preceding_space(&format!("{characters} "), anchor)
                {
                    manager.note_character_typed_outside_composition(characters);
                    // Re-armed at the caret the rewrite left, re-verified
                    // against the document on the next key (`?!` chains).
                    editor.arm_swap();
                    return KeyOutcome::Consumed;
                }
            }
            if ComposingKeyIntent::is_document_text(snapshot) {
                if let Some(mapped) = full_width_mapped(settings, characters) {
                    editor.insert_external(&mapped);
                    manager.note_character_typed_outside_composition(&mapped);
                    return KeyOutcome::Consumed;
                }
                manager.note_character_typed_outside_composition(characters);
            }
            KeyOutcome::ToHost
        }
        ComposingKeyIntent::CommitHighlightedCandidate => {
            commit_candidate(
                surface.selected_index(),
                false,
                settings,
                manager,
                editor,
                list,
                surface,
            );
            KeyOutcome::Consumed
        }
        // Space: the highlighted cell's OTHER script.
        ComposingKeyIntent::CommitAlternateScript => {
            commit_candidate(
                surface.selected_index(),
                true,
                settings,
                manager,
                editor,
                list,
                surface,
            );
            KeyOutcome::Consumed
        }
        ComposingKeyIntent::SelectCandidateSlot(slot) => {
            // A chord aimed at an empty slot is consumed all the same.
            commit_candidate(
                surface.candidate_index_for_slot(*slot),
                false,
                settings,
                manager,
                editor,
                list,
                surface,
            );
            KeyOutcome::Consumed
        }
        ComposingKeyIntent::Navigate(direction) => {
            surface.navigate(*direction);
            KeyOutcome::Consumed
        }
    }
}

fn refresh_candidates(manager: &mut ComposingManager, list: &mut CandidateSource) {
    match manager.fetch_candidates().list_change() {
        CandidateListChange::Replace(candidates) => list.set(candidates, manager),
        CandidateListChange::Clear => list.clear(),
    }
}

/// Commits the candidate behind window cell `cell_index`, in the cell's own
/// script or (`flip`, Space) the other one. The script is resolved BEFORE the
/// commit and the same one decides the auto space, so a 合用 roman cell earns
/// it as `Alternate` under the derived swap.
#[allow(clippy::too_many_arguments)]
fn commit_candidate(
    cell_index: Option<usize>,
    flip: bool,
    settings: &SettingsDocument,
    manager: &mut ComposingManager,
    editor: &mut CompositionEditor<'_>,
    list: &mut CandidateSource,
    surface: &Surface,
) {
    // Nil (no window) and an index past the list both mean nothing to
    // commit; the key is consumed either way.
    let Some((candidate, script)) = cell_index.and_then(|index| list.resolve(index, flip)) else {
        return;
    };
    let candidate = candidate.clone();
    let (outcome, committed) = manager.commit_candidate(&candidate, script, editor);
    log::debug!("candidate.commit {outcome:?}");
    match outcome {
        CandidateCommitOutcome::Finalized => {
            list.clear();
            surface.hide();
            append_auto_space(committed.as_ref(), settings, editor);
        }
        CandidateCommitOutcome::Nailed
        | CandidateCommitOutcome::Ignored
        | CandidateCommitOutcome::Unavailable => {
            refresh_candidates(manager, list);
            surface.present(list, editor);
        }
    }
}

/// The gate every auto-space site reads — 自動空白 live
/// (`isAutoSpaceGateActive`), and `wrote_romanization` from whatever
/// resolved the string this commit wrote. Never re-derived from the output
/// mode here: a candidate commit gets it from `composing::resolved_commit`,
/// a preedit commit from [`raw_preedit_wrote_romanization`], and the swap
/// from the armed record of the commit that wrote the space.
fn auto_space_gate(settings: &SettingsDocument, wrote_romanization: bool) -> bool {
    policies::is_gate_active(
        settings.bool(&keys::IS_AUTO_SPACE_ENABLED),
        wrote_romanization,
    )
}

/// Whether committing the preedit AS TYPED writes romanization — the
/// literal-commit chord and the mid-composition punctuation key. One key read,
/// not a whole `engine_settings()` snapshot: this runs per keystroke.
fn raw_preedit_wrote_romanization(settings: &SettingsDocument) -> bool {
    policies::raw_preedit_writes_romanization(settings.choice(&keys::INPUT_MODE))
}

/// The trailing auto space after an explicit commit, and the swap armed on
/// it. Lifecycle commits never come here.
fn append_auto_space(
    committed: Option<&ResolvedCommit>,
    settings: &SettingsDocument,
    editor: &mut CompositionEditor<'_>,
) {
    let Some(committed) = committed else { return };
    if !auto_space_gate(settings, committed.wrote_romanization)
        || !policies::should_append_space(&committed.text)
    {
        return;
    }
    editor.insert_external(" ");
    if editor.failure.is_none() {
        editor.arm_swap();
    }
}

/// The full-width form of a typed character in hanji-first mode — the
/// DERIVED swap, so roman-only stays half-width.
fn full_width_mapped(settings: &SettingsDocument, text: &str) -> Option<String> {
    if !settings.engine_settings().is_translate_swapped {
        return None;
    }
    policies::full_width_mapped(text)
}

/// The global action `snapshot` is, if its recorded chord matches.
fn global_action_for(
    snapshot: &KeyEventSnapshot,
    settings: &SettingsDocument,
) -> Option<ShortcutAction> {
    ShortcutAction::ALL.into_iter().find(|action| {
        action
            .chord_in(settings)
            .is_some_and(|chord| chord.matches(snapshot))
    })
}
