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

// 中文: 一個按鍵的完整路徑 — 快照→分類→(同步 edit session 內)引擎與文件;候選暫為無視窗列表。

use crate::composition::{is_password_field, is_read_only, CompositionEditor, NullExecutor};
use crate::contexts::ContextEntry;
use crate::edit_session;
use crate::key_translation;
use crate::runtime::Runtime;
use crate::settings_launcher;
use crate::text_service::TextService_Impl;
use std::sync::MutexGuard;
use taigi_windows_core::composing::{
    CandidateCommitOutcome, CandidateFetchOutcome, CandidateScript, ComposingManager,
    ComposingSessionCoordinator, ContextToken,
};
use taigi_windows_core::engine::ContinuousCandidate;
use taigi_windows_core::keys::{
    CandidateNavigation, ComposingKeyBindings, ComposingKeyIntent, KeyEventSnapshot, ShortcutAction,
};
use taigi_windows_core::policies;
use taigi_windows_core::settings::{keys, InputMode, SettingsDocument};
use windows::core::{Interface, BOOL};
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::UI::TextServices::{ITfComposition, ITfCompositionSink, ITfContext, ITfRange};
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
        let is_showing = self
            .state
            .borrow_mut()
            .contexts
            .entry_mut(identity)
            .is_some_and(|entry| !entry.state.candidates.is_empty());
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
        let armed = self
            .state
            .borrow_mut()
            .contexts
            .entry_mut(identity)
            .and_then(|entry| {
                entry
                    .state
                    .armed_auto_space
                    .as_ref()
                    .map(|(script, _)| *script)
            });
        if let Some(script) = armed {
            if policies::is_attaching_punctuation(characters) && auto_space_gate(settings, script) {
                return true;
            }
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
        let (composition, armed_swap, mut candidates, selected) = {
            let mut state = self.state.borrow_mut();
            match state.contexts.entry_mut(identity) {
                Some(entry) => (
                    entry.state.composition.take(),
                    entry.state.armed_auto_space.take(),
                    std::mem::take(&mut entry.state.candidates),
                    entry.state.selected,
                ),
                None => (None, None, Vec::new(), 0),
            }
        };
        let mut list = CandidateList {
            candidates: std::mem::take(&mut candidates),
            selected,
        };
        let mut armed_after: Option<(CandidateScript, ITfRange)> = None;

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
                armed_swap.as_ref(),
            );
            if let Some(error) = editor.failure.take() {
                log::error!("key.document_write_failed error={error} — composition abandoned");
                manager.cancel_composition(&mut NullExecutor);
                editor.abandon();
                list.clear();
                return Ok((KeyOutcome::Consumed, None));
            }
            armed_after = editor.armed.take();
            Ok((outcome, editor.composition.take()))
        });
        drop(coordinator);

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
                entry.state.candidates = list.candidates;
                entry.state.selected = list.selected;
                let previous_arm =
                    std::mem::replace(&mut entry.state.armed_auto_space, armed_after);
                (previous, previous_arm)
            })
        };
        drop(previous);
        drop(armed_swap);
        outcome
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
        // The previous context's candidates are gone with its ownership.
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
                // romanization; they go with the mode that produced them.
                let mut state = self.state.borrow_mut();
                if let Some(entry) = state.contexts.entry_mut(identity) {
                    entry.state.candidates.clear();
                }
            }
            ShortcutAction::ToggleTranslateSwapped => {
                let Some(store) = runtime.settings_store() else {
                    log::warn!("shortcut.no_settings_store");
                    return;
                };
                if let Err(error) = store.update(|document| {
                    let swapped = document.bool(&keys::IS_TRANSLATE_SWAPPED);
                    document.set_bool(&keys::IS_TRANSLATE_SWAPPED, !swapped);
                }) {
                    log::error!("shortcut.toggle_translate_swapped_failed error={error}");
                }
                // The list stays (the swap changes how a candidate displays,
                // never which exist); PR6 re-renders it.
            }
        }
        runtime.settings.current();
    }
}

/// The headless candidate list for one context (PR5b).
struct CandidateList {
    candidates: Vec<ContinuousCandidate>,
    selected: usize,
}

impl CandidateList {
    fn clear(&mut self) {
        self.candidates.clear();
        self.selected = 0;
    }

    fn replace(&mut self, candidates: Vec<ContinuousCandidate>) {
        self.candidates = candidates;
        self.selected = 0;
    }

    fn navigate(&mut self, direction: CandidateNavigation) {
        if self.candidates.is_empty() {
            return;
        }
        match direction {
            CandidateNavigation::Right
            | CandidateNavigation::Down
            | CandidateNavigation::NextCandidate => {
                self.selected = (self.selected + 1).min(self.candidates.len() - 1);
            }
            CandidateNavigation::Left
            | CandidateNavigation::Up
            | CandidateNavigation::PreviousCandidate => {
                self.selected = self.selected.saturating_sub(1);
            }
            // Paging needs a layout — the window's (PR6).
            CandidateNavigation::PageUp | CandidateNavigation::PageDown => {}
        }
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
    list: &mut CandidateList,
    armed_swap: Option<&(CandidateScript, ITfRange)>,
) -> KeyOutcome {
    match intent {
        ComposingKeyIntent::Input(text) => {
            manager.append(text, editor);
            refresh_candidates(manager, list);
            KeyOutcome::Consumed
        }
        ComposingKeyIntent::DeleteBackward => {
            manager.delete_backward(editor);
            refresh_candidates(manager, list);
            KeyOutcome::Consumed
        }
        ComposingKeyIntent::Commit => {
            let committed = manager.commit_composition(editor);
            list.clear();
            append_auto_space(
                committed.as_deref(),
                CandidateScript::Primary,
                settings,
                editor,
            );
            KeyOutcome::Consumed
        }
        ComposingKeyIntent::Cancel => {
            manager.cancel_composition(editor);
            list.clear();
            KeyOutcome::Consumed
        }
        ComposingKeyIntent::CommitThenInsert(text) => {
            // Mapped before the auto-space augmentation so the full-width
            // character rides the same single mutation as the commit.
            let document_text = full_width_mapped(settings, text).unwrap_or_else(|| text.clone());
            let gate = auto_space_gate(settings, CandidateScript::Primary);
            let insert = policies::augment_insert(&document_text, manager.display_text(), gate);
            let committed = manager.commit_composition_then_insert(&insert.text, editor);
            list.clear();
            if insert.leaves_trailing_auto_space && committed.is_some() {
                editor.arm_swap(CandidateScript::Primary);
            }
            KeyOutcome::Consumed
        }
        ComposingKeyIntent::CommitThenPassThrough => {
            manager.commit_composition(editor);
            list.clear();
            KeyOutcome::ToHost
        }
        ComposingKeyIntent::PassThrough => {
            let Some(characters) = snapshot.characters.as_deref() else {
                return KeyOutcome::ToHost;
            };
            if let Some((script, anchor)) = armed_swap {
                if ComposingKeyIntent::is_document_text(snapshot)
                    && policies::is_attaching_punctuation(characters)
                    && auto_space_gate(settings, *script)
                    && editor.swap_preceding_space(&format!("{characters} "), anchor)
                {
                    manager.note_character_typed_outside_composition(characters);
                    // Re-armed at the caret the rewrite left, re-verified
                    // against the document on the next key (`?!` chains).
                    editor.arm_swap(*script);
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
                list.selected,
                CandidateScript::Primary,
                settings,
                manager,
                editor,
                list,
            );
            KeyOutcome::Consumed
        }
        ComposingKeyIntent::CommitAlternateScript => {
            commit_candidate(
                list.selected,
                CandidateScript::Alternate,
                settings,
                manager,
                editor,
                list,
            );
            KeyOutcome::Consumed
        }
        ComposingKeyIntent::SelectCandidateSlot(slot) => {
            // A chord aimed at an empty slot is consumed all the same.
            commit_candidate(
                *slot,
                CandidateScript::Primary,
                settings,
                manager,
                editor,
                list,
            );
            KeyOutcome::Consumed
        }
        ComposingKeyIntent::Navigate(direction) => {
            list.navigate(*direction);
            KeyOutcome::Consumed
        }
    }
}

fn refresh_candidates(manager: &mut ComposingManager, list: &mut CandidateList) {
    match manager.fetch_candidates() {
        CandidateFetchOutcome::Unavailable | CandidateFetchOutcome::NotComposing => list.clear(),
        CandidateFetchOutcome::Found(candidates) => list.replace(candidates),
    }
}

fn commit_candidate(
    index: usize,
    script: CandidateScript,
    settings: &SettingsDocument,
    manager: &mut ComposingManager,
    editor: &mut CompositionEditor<'_>,
    list: &mut CandidateList,
) {
    let Some(candidate) = list.candidates.get(index).cloned() else {
        return;
    };
    let (outcome, committed) = manager.commit_candidate(&candidate, script, editor);
    log::debug!("candidate.commit {outcome:?}");
    match outcome {
        CandidateCommitOutcome::Finalized => {
            list.clear();
            append_auto_space(committed.as_deref(), script, settings, editor);
        }
        CandidateCommitOutcome::Nailed
        | CandidateCommitOutcome::Ignored
        | CandidateCommitOutcome::Unavailable => {
            refresh_candidates(manager, list);
        }
    }
}

/// The gate every auto-space site reads — live (`isAutoSpaceGateActive`).
fn auto_space_gate(settings: &SettingsDocument, script: CandidateScript) -> bool {
    policies::is_gate_active(
        settings.bool(&keys::IS_AUTO_SPACE_ENABLED),
        policies::writes_romanization(
            script,
            settings.bool(&keys::IS_TRANSLATE_SWAPPED),
            settings.bool(&keys::IS_OUTPUT_BOTH_SCRIPTS),
        ),
    )
}

/// The trailing auto space after an explicit commit, and the swap armed on
/// it. Lifecycle commits never come here.
fn append_auto_space(
    committed: Option<&str>,
    script: CandidateScript,
    settings: &SettingsDocument,
    editor: &mut CompositionEditor<'_>,
) {
    let Some(committed) = committed else { return };
    if !auto_space_gate(settings, script) || !policies::should_append_space(committed) {
        return;
    }
    editor.insert_external(" ");
    if editor.failure.is_none() {
        editor.arm_swap(script);
    }
}

/// The full-width form of a typed character in hanji-first mode.
fn full_width_mapped(settings: &SettingsDocument, text: &str) -> Option<String> {
    if !settings.bool(&keys::IS_TRANSLATE_SWAPPED) {
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
