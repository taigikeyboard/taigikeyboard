//! One key, end to end: snapshot → classifier → engine + document, inside
//! a synchronous edit session — the TSF shell over desktop-core's key path
//! (`desktop/crates/taigi-desktop-core/src/composing/manager.rs`,
//! `composing/intent_executor.rs`, `policies/auto_space.rs`), with the TSF
//! specifics the roadmap fixes: `OnTestKeyDown`
//! and `OnKeyDown` answer through one classification (terminals skip the
//! former); the engine is touched only INSIDE the session so a refused
//! session leaves everything untouched; the runtime comes up on the first
//! key the classifier CONSUMES; ownership is handed over between contexts
//! per the coordinator's contract.
//!
//! The candidate list belongs to the presenter: a key only QUEUES what the
//! window should do (`Surface`), and the queue is replayed after the
//! session has returned — never inside it, where showing a window would run
//! the host's `BeginUIElement` / `SetWindowPos` re-entry under the engine
//! lock.

use crate::composition::{is_password_field, is_read_only, CompositionEditor, NullExecutor};
use crate::contexts::ContextEntry;
use crate::contexts::ContextRegistry;
use crate::edit_session;
use crate::key_translation;
use crate::runtime::Runtime;
use crate::settings_launcher;
use crate::text_service::TextService_Impl;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::MutexGuard;
use taigi_desktop_core::composing::{
    pass_through_may_consume, represent_list_following, CandidateSource, ComposingManager,
    ComposingSessionCoordinator, ContextToken,
};
use taigi_desktop_core::global_action::{apply_global_action, ListEffect, ShellRequest};
use taigi_desktop_core::keys::{
    ComposingKeyBindings, ComposingKeyIntent, KeyEventSnapshot, ShortcutAction,
};
use taigi_desktop_core::settings::{keys, AppearanceMode, InputMode, SettingsDocument};
use taigi_desktop_core::strings::{StringKey, StringResolver};
use taigi_windows_platform::tps_keyboard_click::{click_intent, TPS_KEYBOARD_CLICK_VIRTUAL_KEY};
use taigi_windows_platform::DESKTOP_PLATFORM;
use windows::core::{Interface, BOOL};
use windows::Win32::Foundation::{E_UNEXPECTED, LPARAM, POINT, RECT, WPARAM};
use windows::Win32::UI::TextServices::{ITfComposition, ITfCompositionSink, ITfContext, ITfRange};
use windows_core::IUnknownImpl;

mod surface;
mod symbol_picker;

use surface::{perform_work, window_content, Surface};

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

/// What one edit session is asked to do: a key's composing intent, or a
/// symbol the picker chose — text that arrived as no key at all, written
/// outside any composition (the picker opens only once the composition
/// has ended, `toggle_symbol_picker`).
enum KeyWork {
    Compose(ComposingKeyIntent),
    InsertSymbol(String),
}

/// Everything a [`CompositionEditor`] is built from, snapshotted together.
/// One value rather than three loose arguments so that the sink's COM
/// reference has ONE well-defined lifetime at each call site: it must
/// outlive every `ITfComposition` handle the editors built from it produce,
/// which is what the declaration order at each site pins.
struct EditorInputs {
    client_id: u32,
    display_attribute_atom: u32,
    sink: ITfCompositionSink,
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
        // Before any bail: a hide a callback could not post must not wait
        // for a key this context will never consume.
        self.drain_pending_ui_hide();
        self.release_toggle_chord_on_other_key(wparam, lparam);
        let Some(context) = context else {
            return BOOL::from(false);
        };
        // A click on the TPS key panel: read before the key translation, which
        // has no character for it and would drop it.
        if key_translation::virtual_key(wparam) == TPS_KEYBOARD_CLICK_VIRTUAL_KEY {
            return BOOL::from(self.tps_keyboard_click(context, lparam, phase));
        }
        // A key in this activation: it is where the user types, whatever
        // focus events this host did or did not send (the floating panels).
        self.note_panel_focus(true);
        // Modifiers and synthetic keys build no snapshot: they leave before
        // the settings read the key's layout needs.
        if !key_translation::builds_snapshot(key_translation::virtual_key(wparam)) {
            return BOOL::from(false);
        }
        let Some((token, identity)) = self.token_for(context) else {
            return BOOL::from(false);
        };
        let runtime = Runtime::shared();
        let settings = runtime.settings.current();
        // Read in the user's layout (Dvorak / Colemak over the US base the
        // TIP runs on; QWERTY under TPS) before anything reads the key.
        let Some(snapshot) =
            key_translation::snapshot(wparam, lparam, settings.key_reading_layout())
        else {
            return BOOL::from(false);
        };
        // A chord recorded in the settings window takes effect at the next
        // key, whichever way the file's change was noticed.
        self.sync_preserved_keys(&settings);
        let global_action = ShortcutAction::matching_in(
            &snapshot,
            &settings,
            DESKTOP_PLATFORM,
            self.state.borrow().language_mode,
        );

        if is_read_only(context) {
            return BOOL::from(false);
        }

        // The global chords, before the classifier — the Carbon hotkey's
        // position on the Mac, and like it independent of whether a
        // composition is running (UX decision, roadmap W5: a bare key is
        // matched here rather than registered as a preserved key). The
        // preserved keys normally arrive through `OnPreservedKey`; this is
        // their fallback in hosts that bypass preserved keys.
        if let Some(action) = global_action {
            // A held toggle chord would otherwise flip on every auto-repeat
            // (`fires_once_per_press`). This is the fallback path only:
            // `OnPreservedKey` carries no repeat flag and keeps its own latch.
            let is_toggle_repeat =
                action.fires_once_per_press() && key_translation::is_repeat(lparam);
            if phase == KeyPhase::Deliver && !is_toggle_repeat {
                // The picker runs against this key event's context
                // (`ShortcutAction::needs_key_context`): a pick writes into
                // the document and anchors to the caret.
                if action.needs_key_context() {
                    self.toggle_symbol_picker(context, token, identity, &settings);
                } else {
                    self.perform_global(action, identity);
                }
            }
            return BOOL::from(true);
        }

        // Resolved once per key: the picker and the composing contract below
        // read the same value.
        let bindings = ComposingKeyBindings::from_document(&settings, DESKTOP_PLATFORM);
        // With the picker up, every key is the picker's first — read before
        // the composing contract so the slot keys and the arrows reach it
        // rather than a list that is not showing. A key the picker has no
        // use for takes it down and goes on below.
        if self.symbol_picker_takes_key(context, token, identity, &snapshot, phase, &bindings) {
            return BOOL::from(true);
        }

        // English mode: below the global chords, above everything that
        // composes. Every key is the document's — including the ones this
        // input method would otherwise consume outside a composition (the
        // auto-space swap, full-width punctuation, the bare Hanji/romanization key), which
        // is what makes the mode mean "type English here" rather than "stop
        // composing". Nothing needs ending first: entering the mode already
        // committed the composition and spent the arm
        // (`toggle_language_mode`).
        if self.state.borrow().language_mode.is_english() {
            return BOOL::from(false);
        }

        // A window the user switched off since the last key comes down HERE,
        // before the key is read (mirrors `TaigiInputController.handle`):
        // the TIP has no settings observer inside the host, and a Return
        // classified against a window still up would pick a candidate the
        // user asked never to see.
        if !settings.bool(&keys::IS_CANDIDATE_WINDOW_ENABLED) {
            self.hide_candidates(token);
        }
        // A composition a switch across TPS left behind — from a chord, the
        // tray menu or the settings window — is committed as shown before
        // this key is read; the key is then the new mode's first
        // (`commit_composition_left_by_mode_change`, which `perform_intent`
        // runs first, so the `Commit` here then finds nothing left). Claimed
        // from the test phase, which runs no session; at delivery a refused
        // commit hands the key to the host rather than type into the old
        // buffer.
        let is_left_by_mode_change = runtime
            .try_coordinator()
            .and_then(|coordinator| {
                coordinator.manager_ref(token).map(|manager| {
                    manager.is_left_by_mode_change(settings.choice(&keys::INPUT_MODE))
                })
            })
            .unwrap_or(false);
        if is_left_by_mode_change {
            if phase == KeyPhase::Test {
                return BOOL::from(true);
            }
            let outcome = self.run_key(
                context,
                token,
                identity,
                &KeyEventSnapshot::default(),
                &KeyWork::Compose(ComposingKeyIntent::Commit),
                &settings,
            );
            if outcome != KeyOutcome::Consumed {
                return BOOL::from(false);
            }
        }
        let (is_composing, is_showing_candidates) = self.composing_flags(runtime, token);
        let intent = ComposingKeyIntent::intent(
            &snapshot,
            is_composing,
            is_showing_candidates,
            &bindings,
            DESKTOP_PLATFORM,
        );
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
        // A key the host gets — refused here at the test, when TSF never
        // delivers it, or handed back below — acts on the document where
        // the swap cannot see, so the arm is spent with it (#866).
        if !would_consume {
            self.spend_auto_space_arm(identity);
        }
        if phase == KeyPhase::Test {
            return BOOL::from(would_consume);
        }
        if !would_consume && !matches!(intent, ComposingKeyIntent::CommitThenPassThrough) {
            // Not ours, and nothing to finish — but a character reaching the
            // document outside a composition still ends the next-word
            // context (a full stop typed here is a sentence end).
            if let Some(typed) = ComposingKeyIntent::document_text(&snapshot, bindings.input_mode) {
                if let Some(mut coordinator) = runtime.try_coordinator() {
                    if let Some(manager) = coordinator.manager(token) {
                        manager.note_character_typed_outside_composition(&typed);
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
        let outcome = self.run_key(
            context,
            token,
            identity,
            &snapshot,
            &KeyWork::Compose(intent),
            &settings,
        );
        BOOL::from(outcome == KeyOutcome::Consumed)
    }

    /// A click on the TPS key panel, injected as the key that names its
    /// glyph (`ui/tps_keyboard.rs`): typed through the engine's `TpsKey` as
    /// the layout key would be, but whatever the window or the slot keys
    /// would make of that key (`ComposingKeyIntent::tps_keyboard_press`).
    /// Not taken when this context cannot be typed in (read-only, English
    /// mode), the mode is no longer TPS, or the scan code names no glyph —
    /// and a delivery can still hand it on after a TRUE test (a password
    /// field, focus moving in between). Either way the host receives a key
    /// no layout gives a character, and the click carries its glyph, so
    /// nothing is left waiting.
    fn tps_keyboard_click(&self, context: &ITfContext, lparam: LPARAM, phase: KeyPhase) -> bool {
        let Some((token, identity)) = self.token_for(context) else {
            return false;
        };
        if is_read_only(context) || self.state.borrow().language_mode.is_english() {
            return false;
        }
        let runtime = Runtime::shared();
        let settings = runtime.settings.current();
        let scan_code = key_translation::scan_code(lparam);
        let Some(intent) = click_intent(scan_code, settings.choice(&keys::INPUT_MODE)) else {
            return false;
        };
        if phase == KeyPhase::Test {
            return true;
        }
        log::debug!("key.tps_keyboard_click scan_code={scan_code}");
        // The picker goes down first, as for any key.
        self.hide_symbol_picker_of(token);
        runtime.prepare_for_first_key();
        let outcome = self.run_key(
            context,
            token,
            identity,
            &KeyEventSnapshot::default(),
            &KeyWork::Compose(intent),
            &settings,
        );
        outcome == KeyOutcome::Consumed
    }

    /// What the engine says about this context, read without a session.
    /// A coordinator that does not exist yet (no key consumed so far) reads
    /// as idle.
    fn composing_flags(&self, runtime: &Runtime, token: ContextToken) -> (bool, bool) {
        let is_composing = runtime
            .try_coordinator()
            .and_then(|coordinator| {
                coordinator
                    .manager_ref(token)
                    .map(ComposingManager::is_composing)
            })
            .unwrap_or(false);
        let is_showing = self
            .presenter()
            .is_some_and(|presenter| presenter.borrow().is_showing(token));
        (is_composing, is_showing)
    }

    /// Drops the auto-space arm of this context: a key it did not take
    /// reached the document. Moved out under the borrow, released after it.
    fn spend_auto_space_arm(&self, identity: usize) {
        let arm = self
            .state
            .borrow_mut()
            .contexts
            .entry_mut(identity)
            .and_then(|entry| entry.state.armed_auto_space.take());
        drop(arm);
    }

    /// The pass-through keys this input method consumes: attaching
    /// punctuation right after an auto space (the swap), full-width
    /// punctuation in hanji-first mode and under TPS, its Ctrl punctuation
    /// chord included. Optimistic — the swap is verified against the document only
    /// in the session. Mirrors the session's `PassThrough` arm.
    fn pass_through_may_consume(
        &self,
        snapshot: &KeyEventSnapshot,
        settings: &SettingsDocument,
        identity: usize,
    ) -> bool {
        let is_armed = self
            .state
            .borrow_mut()
            .contexts
            .entry_mut(identity)
            .is_some_and(|entry| entry.state.armed_auto_space.is_some());
        pass_through_may_consume(snapshot, settings, is_armed)
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
        work: &KeyWork,
        settings: &SettingsDocument,
    ) -> KeyOutcome {
        let runtime = Runtime::shared();
        // A hide a focus callback could not post (a window was busy inside
        // a session) lands here, before anything new is shown.
        self.drain_pending_ui_hide();
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
        let EditorInputs {
            client_id,
            display_attribute_atom: atom,
            sink,
        } = self.editor_inputs();
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
            slot_key_set: ComposingKeyBindings::from_document(settings, DESKTOP_PLATFORM)
                .slot_key_set(),
            actions: RefCell::new(Vec::new()),
            typed_tps_cap: Cell::new(None),
        };

        let session = edit_session::read_write(context, client_id, |ec| {
            if is_password_field(context, ec) {
                return Ok((KeyOutcome::ToHost, composition.clone()));
            }
            let manager = coordinator.claim(token);
            let mut editor = CompositionEditor::new(context, ec, &sink, atom, composition.clone());
            let outcome = perform_work(
                work,
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
                // The glyph never reached the document: nothing to flash.
                surface.typed_tps_cap.take();
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
            self.record_focused_caret(caret);
        }
        if let Some(cap) = surface.typed_tps_cap.take() {
            if let Some(tps_keyboard) = self.tps_keyboard() {
                tps_keyboard.borrow().flash(cap);
            }
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

    /// Everything a [`CompositionEditor`] is built from: the edit-session
    /// client id, the preedit's display-attribute atom, and this service as
    /// the composition sink. The two state fields are read under their own
    /// short borrows and the sink's `AddRef` happens with NO borrow held —
    /// the module rule (`text_service.rs` header) is that no COM call, not
    /// even a reference count, runs under one.
    fn editor_inputs(&self) -> EditorInputs {
        let client_id = self.state.borrow().client_id;
        let display_attribute_atom = self.state.borrow().display_attribute_atom;
        let sink: ITfCompositionSink = self
            .to_object()
            .to_interface::<ITfCompositionSink>()
            .to_owned();
        EditorInputs {
            client_id,
            display_attribute_atom,
            sink,
        }
    }

    /// Commits `token`'s composition into `context`'s own document under
    /// that document's OWN synchronous session — the lifecycle commit, with
    /// no candidates, no auto space and no swap arm. Answers whether the
    /// session ran AND every document write inside it succeeded, which is
    /// the handover's condition for moving engine ownership.
    fn commit_under_session(
        &self,
        context: &ITfContext,
        token: ContextToken,
        composition: &Option<ITfComposition>,
        coordinator: &mut MutexGuard<'_, ComposingSessionCoordinator>,
        inputs: &EditorInputs,
    ) -> bool {
        let session = edit_session::read_write(context, inputs.client_id, |ec| {
            let mut editor = CompositionEditor::new(
                context,
                ec,
                &inputs.sink,
                inputs.display_attribute_atom,
                composition.clone(),
            );
            if let Some(manager) = coordinator.manager(token) {
                manager.commit_composition(&mut editor);
            }
            Ok(editor.failure.is_none() && editor.composition.is_none())
        });
        matches!(session, Ok(true))
    }

    /// Takes `token`'s candidate list off the screen. The presenter is
    /// cloned out of the state first (see
    /// [`TextService_Impl::presenter`]) — the `borrow_mut` here is the
    /// presenter's own, held only for the call.
    fn hide_candidates(&self, token: ContextToken) {
        if let Some(presenter) = self.presenter() {
            presenter.borrow_mut().hide(token);
        }
    }

    /// `ITfCandidateListUIElementBehavior::Finalize`: the host commits the
    /// highlighted candidate — the key path with the commit key's intent;
    /// under TPS the composition as shown (`ComposingKeyIntent::commit_first`).
    pub(crate) fn ui_element_finalize(&self) -> windows::core::Result<()> {
        self.run_from_ui_element(|input_mode| ComposingKeyIntent::commit_first(true, input_mode))
    }

    /// `ITfCandidateListUIElementBehavior::Abort`: the host cancels the
    /// composition — Escape's intent.
    pub(crate) fn ui_element_abort(&self) -> windows::core::Result<()> {
        self.run_from_ui_element(|_| ComposingKeyIntent::Cancel)
    }

    /// A host-initiated synchronous call on the TIP thread, outside any
    /// session of ours: it runs like a key, under the owning context's own
    /// edit session. Refused (`E_UNEXPECTED`) when no list is up, the
    /// owner's context is gone, or the engine is busy — which means the
    /// host re-entered us from inside our own session.
    fn run_from_ui_element(
        &self,
        intent: impl FnOnce(InputMode) -> ComposingKeyIntent,
    ) -> windows::core::Result<()> {
        let busy = || windows::core::Error::from_hresult(E_UNEXPECTED);
        let presenter = self.presenter();
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
        let settings = runtime.settings.current();
        let intent = intent(settings.choice(&keys::INPUT_MODE));
        let engine_free = runtime.try_coordinator().is_some();
        if !engine_free {
            log::warn!("ui_element.reentered intent={intent:?}");
            return Err(busy());
        }
        log::debug!("ui_element.intent {intent:?}");
        let outcome = self.run_key(
            &context,
            owner,
            identity,
            &KeyEventSnapshot::default(),
            &KeyWork::Compose(intent),
            &settings,
        );
        log::debug!("ui_element.outcome {outcome:?}");
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
        let is_composing = coordinator
            .manager_ref(owner)
            .is_some_and(|m| m.is_composing());
        let mut finished = !is_composing;
        if let (Some(previous_context), true) = (&previous_context, is_composing) {
            // Scoped to the branch, as the sink always was here: it is
            // released at the end of this block, BEFORE the explicit
            // `drop(previous_composition)` at the end of the function.
            let inputs = self.editor_inputs();
            finished = self.commit_under_session(
                previous_context,
                owner,
                &previous_composition,
                coordinator,
                &inputs,
            );
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
        if let Some(presenter) = self.presenter() {
            presenter.borrow_mut().hide_for_handover();
        }
        self.hide_symbol_picker_now();
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
        self.hide_candidates(token);
        // The picker belongs to the context whose composition the host just
        // ended; a click elsewhere or a focus loss takes it down with the
        // list (`TaigiInputController.endSession`).
        self.hide_symbol_picker_of(token);
        let reset_now = match Runtime::shared().try_coordinator() {
            Some(mut coordinator) => {
                if let Some(manager) = coordinator.manager(token) {
                    manager.cancel_composition(&mut NullExecutor);
                }
                true
            }
            None => false,
        };
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
        let Some(mut coordinator) = Runtime::shared().try_coordinator() else {
            return;
        };
        // Snapshotted ONCE for the whole teardown, and declared before the
        // loop so the sink outlives every composition handle taken inside it
        // (an editor's `ITfComposition` must not outlive the sink it was
        // built against).
        let inputs = self.editor_inputs();
        for entry in entries.iter_mut() {
            if coordinator
                .manager_ref(entry.token)
                .is_some_and(|m| m.is_composing())
            {
                let composition = entry.state.composition.take();
                let context: &ITfContext = &entry.context;
                let _ = self.commit_under_session(
                    context,
                    entry.token,
                    &composition,
                    &mut coordinator,
                    &inputs,
                );
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

    /// A Shift tap switched Chinese/English. Runs on the TIP thread from `OnKeyUp`,
    /// outside any session of ours, and in this order: what is half-typed is
    /// written to the document under the mode it was typed in, the state that
    /// mode left behind is spent, and only then does the mode flip and the
    /// three indicators (compartment, tray icon, flash) follow it.
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
        let engine_free = runtime.try_coordinator().is_some();
        if !engine_free {
            log::warn!("language_mode.reentered — switch skipped");
            return;
        }
        // One intent, one snapshot (`settings/mod.rs`): the commit and the
        // flash must not straddle a settings change.
        let settings = runtime.settings.current();
        let next = self.state.borrow().language_mode.toggled();
        let (is_composing, _) = self.composing_flags(runtime, token);
        if is_composing {
            let outcome = self.run_key(
                context,
                token,
                identity,
                &KeyEventSnapshot::default(),
                &KeyWork::Compose(ComposingKeyIntent::Commit),
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
        {
            let mut state = self.state.borrow_mut();
            if let Some(entry) = state.contexts.entry_mut(identity) {
                entry.state.armed_auto_space = None;
                entry.state.candidates.clear();
            }
        }
        self.hide_candidates(token);
        // A list of symbols is not English either.
        self.hide_symbol_picker_of(token);
        // The next-word context is Taiwanese. English typed after the switch
        // is not the predecessor of the word typed after the switch back, and
        // the engine is what would otherwise keep believing it is.
        if let Some(mut coordinator) = runtime.try_coordinator() {
            if let Some(manager) = coordinator.manager(token) {
                manager.start_new_session();
            }
        }

        self.set_language_mode(next);
        self.flash_mode_label(runtime, &settings, next.flash_label_key());
        log::info!("language_mode.switched mode={next:?}");
    }

    /// A global shortcut fired (preserved key or the key sink's match).
    /// `identity` is 0 when the host named no context (`OnPreservedKey`).
    /// What the action decides is the core's (`apply_global_action`); this
    /// carries it out with the service's own windows. A composition across
    /// TPS stays until the next key commits it
    /// (`commit_composition_left_by_mode_change`) — no edit session is
    /// requested here, outside a key. The auto-space arm stays: it names a
    /// space in the document, whatever the mode.
    pub(crate) fn perform_global(&self, action: ShortcutAction, identity: usize) {
        let runtime = Runtime::shared();
        if action.is_inert_under(runtime.settings.current().choice(&keys::INPUT_MODE)) {
            return;
        }
        let token = self
            .state
            .borrow_mut()
            .contexts
            .entry_mut(identity)
            .map(|entry| entry.token);
        // The picker, unconditionally: no global action can reach the
        // key path that would otherwise take it down, and a romanization
        // switched under an open list is a list the user is no longer
        // looking at (`TaigiInputController.performShortcutAction`).
        self.hide_symbol_picker_now();
        let Some(outcome) = apply_global_action(runtime, action) else {
            return;
        };
        match outcome.shell {
            Some(ShellRequest::OpenSettings) => settings_launcher::open_settings(),
            // Routed to `toggle_symbol_picker` by both doorways before this
            // is reached (`needs_key_context`).
            Some(ShellRequest::ToggleSymbolPicker) => {}
            Some(ShellRequest::TelexGuideChanged) => self.sync_panels(),
            Some(ShellRequest::ToggleTpsKeyboard) => {
                // The user's wish, stored — every host process reads it at
                // its next focus — then this one's panel follows. The
                // composition and its list are not touched.
                if !runtime.update_settings("toggle_tps_keyboard", |document| {
                    let shown = document.bool(&keys::TPS_KEYBOARD_SHOWN);
                    document.set_bool(&keys::TPS_KEYBOARD_SHOWN, !shown);
                }) {
                    return;
                }
                self.sync_panels();
            }
            None => {}
        }
        match outcome.list {
            ListEffect::Untouched => {}
            // Fetched under the old mode: the candidates go with the mode
            // that produced them. The TPS key panel and the Telex guide come
            // and go with TPS, their stored keys staying; the guide is
            // respelled for a romanization.
            ListEffect::Clear => {
                if let Some(entry) = self.state.borrow_mut().contexts.entry_mut(identity) {
                    entry.state.candidates.clear();
                }
                if let Some(token) = token {
                    self.hide_candidates(token);
                }
                self.sync_panels();
            }
            // Never hidden first: from mid-composition that reads as the
            // window vanishing (2026-08-21).
            ListEffect::Represent { refetch } => {
                self.represent_open_list(identity, runtime, refetch);
            }
        }
        // The HUD names only what changed, because the chord fires from
        // anywhere and a mode that changed with no notice reads as the
        // keyboard breaking (USER 2026-08-26).
        if outcome.announces {
            let settings = runtime.settings.current();
            if let Some(label) = action.switched_label_key(&settings) {
                self.flash_mode_label(runtime, &settings, label);
            }
        }
        // The script, the swap and the display mode are all on the tray
        // icon (`ModeIndicator`); the paths above that changed nothing
        // returned early.
        self.notify_lang_bar();
    }

    /// Where the last window this service anchored to was — the point the
    /// mode flash and the guide centre on.
    fn record_focused_caret(&self, caret: RECT) {
        self.state.borrow_mut().focused_caret = POINT {
            x: caret.left,
            y: caret.top,
        };
    }

    /// Re-presents the open list for `identity` under the settings in force
    /// right now (`represent_list`): re-fetched when the change alters which
    /// candidates exist, where an empty answer — or the Show Candidate
    /// Window setting turned off since — takes the window down; otherwise
    /// the same list re-rendered in place. Never hidden first: from
    /// mid-composition that reads as the window vanishing.
    fn represent_open_list(&self, identity: usize, runtime: &Runtime, refetch: bool) {
        let (token, presenter) = {
            let mut state = self.state.borrow_mut();
            let token = state.contexts.entry_mut(identity).map(|entry| entry.token);
            (token, state.presenter.clone())
        };
        let (Some(token), Some(presenter)) = (token, presenter) else {
            return;
        };
        let Some(mut coordinator) = runtime.try_coordinator() else {
            return;
        };
        let Some(manager) = coordinator.manager(token) else {
            return;
        };
        let settings = runtime.settings.current();
        let slot_key_set =
            ComposingKeyBindings::from_document(&settings, DESKTOP_PLATFORM).slot_key_set();
        let selected = presenter.borrow().selected_index(token).unwrap_or(0);
        let cells = {
            let mut state = self.state.borrow_mut();
            let Some(entry) = state.contexts.entry_mut(identity) else {
                return;
            };
            let source = &mut entry.state.candidates;
            // No list open: a switch never opens one.
            if source.is_empty() {
                return;
            }
            represent_list_following(&settings, manager, source, refetch, selected)
                .map(|cell| (window_content(source, slot_key_set), cell))
        };
        match cells {
            // A list fetched again keeps the selection on its word, which may
            // sit on another cell now (`CandidateSource::cell_following`).
            Some((content, cell)) => presenter.borrow_mut().update_cells(
                content,
                &settings,
                token,
                refetch.then_some(cell),
            ),
            None => presenter.borrow_mut().hide(token),
        }
    }
}
