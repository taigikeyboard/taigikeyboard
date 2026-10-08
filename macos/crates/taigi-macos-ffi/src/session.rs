//! The composing session behind the seam
//! (docs/architecture/macos-desktop-core-roadmap.md D3): `Activate` / `Key` /
//! `CommitComposition` / `Cancel` / `CommitForSymbolPicker` /
//! `InsertSymbol` / `TpsKeyboardPress` / `Represent` / `Release` on the process's one
//! coordinator, answered as the effects Swift replays after the call
//! returns: desktop-core's classifier (`ComposingKeyIntent`) and executor
//! (`perform_intent`), hosted for the macOS controller. The per-key
//! preamble — the Telex guide, the symbol picker — runs
//! in the Swift controller (`TaigiInputController.swift`) before a key
//! reaches this seam (the global chords never reach it: Carbon hotkeys);
//! the picker's key reading is asked of `key_rules.rs`.
//!
//! Record, then replay: [`RecordingSurface`] only records, so no client call
//! happens while the coordinator is locked. The window's state travels in
//! with each request (`PanelState`) and is read, never kept.

use taigi_desktop_core::composing::{
    insert_symbol, perform_intent, represent_list, CandidateSource, ComposingEffectExecutor,
    ComposingManager, ContextToken, IntentSurface,
};
use taigi_desktop_core::engine::Effect as EngineEffect;
use taigi_desktop_core::keys::{
    CandidateNavigation, ComposingKeyBindings, ComposingKeyIntent, KeyEventSnapshot, TpsKeyCapIndex,
};
use taigi_desktop_core::runtime::DesktopRuntime;
use taigi_desktop_core::settings::keys;

use crate::key_translation;
use crate::proto::{
    self, effect, ActivateRequest, CancelRequest, CandidateCell, CandidatesChanged,
    CommitCompositionRequest, CommitForSymbolPickerRequest, Effect, InsertSymbolRequest,
    KeyRequest, PanelState, ReleaseRequest, RepresentRequest, SessionReply,
    TpsKeyboardPressRequest,
};
use crate::runtime::{Refusal, DESKTOP_PLATFORM};

/// What the session keeps between requests: the list the last fetch
/// returned and the cells shown for it. One per process, like the
/// composition it describes — only the session that owns the engine reaches
/// it, and `Activate` drops it on every handover, so a list a released
/// session left behind is never read.
///
/// A stale token — one released, or superseded by another `Activate` — is
/// refused by ownership alone: every request but `Activate` from a token
/// that does not own the engine is answered `ignored`. `Activate` always
/// claims, because a stale token is not a dead one: Swift releases on every
/// `deactivateServer` and claims again with the same token when the field
/// takes the focus back. A closed controller sends nothing more.
#[derive(Default)]
pub(crate) struct Session {
    candidates: CandidateSource,
}

impl Session {
    /// `token` takes the engine; a token that already holds it keeps its
    /// composition. The list goes either way: the window went down with the
    /// handover.
    pub(crate) fn activate(
        &mut self,
        runtime: &DesktopRuntime,
        request: &ActivateRequest,
    ) -> Result<SessionReply, Refusal> {
        let token = token(request.token)?;
        let mut coordinator = runtime.lock_coordinator();
        let manager = coordinator.claim(token);
        self.candidates.clear();
        Ok(owner_reply(false, Vec::new(), manager))
    }

    /// One key: classified under the settings in force for this request,
    /// then run through the core's executor.
    pub(crate) fn key(
        &mut self,
        runtime: &DesktopRuntime,
        request: &KeyRequest,
    ) -> Result<SessionReply, Refusal> {
        let event = required(&request.event, "key.event")?;
        let panel = required(&request.panel, "key.panel")?;
        self.run_owned(
            runtime,
            request.token,
            panel,
            |manager, candidates, surface| {
                // One read for the whole key: the window check, the bindings and
                // the executor see the same settings.
                let settings = runtime.settings.current();
                let bindings = ComposingKeyBindings::from_document(&settings, DESKTOP_PLATFORM);
                // A window the user switched off since the last key comes down
                // HERE, before the key is read: a Return classified against a
                // list still up would pick a candidate the user asked never to
                // see.
                if !bindings.is_candidate_window_enabled && !candidates.is_empty() {
                    candidates.clear();
                    surface.list_closed();
                }
                // A composition a switch across TPS left behind — from a
                // chord, the input-method menu or the settings window — is
                // committed as shown before this key is read, which is then
                // the new mode's first (`commit_composition_left_by_mode_change`,
                // which `perform_intent` runs first: the `Commit` itself then
                // finds nothing left). Keyless, as on Windows and Linux.
                if manager.is_left_by_mode_change(settings.choice(&keys::INPUT_MODE)) {
                    let no_key = KeyEventSnapshot::default();
                    perform_intent(
                        &ComposingKeyIntent::Commit,
                        &no_key,
                        &settings,
                        manager,
                        candidates,
                        surface,
                    );
                }
                let key = key_translation::snapshot(event);
                let intent = ComposingKeyIntent::intent(
                    &key,
                    manager.is_composing(),
                    !candidates.is_empty(),
                    &bindings,
                    DESKTOP_PLATFORM,
                );
                log::debug!("key.intent {intent:?}");
                perform_intent(&intent, &key, &settings, manager, candidates, surface)
            },
        )
    }

    /// The lifecycle commit: the composition as typed, no auto space — the
    /// user did not finish a word there — and no list effect: Swift took the
    /// window down first, and says so in the panel.
    pub(crate) fn commit_composition(
        &mut self,
        runtime: &DesktopRuntime,
        request: &CommitCompositionRequest,
    ) -> Result<SessionReply, Refusal> {
        let panel = required(&request.panel, "commit_composition.panel")?;
        self.run_owned(runtime, request.token, panel, |manager, _, surface| {
            manager.commit_composition(surface);
            false
        })
    }

    /// Drops the composition and the list — what Swift sends after a
    /// request answered FAIL_INTERNAL (roadmap D4).
    pub(crate) fn cancel(
        &mut self,
        runtime: &DesktopRuntime,
        request: &CancelRequest,
    ) -> Result<SessionReply, Refusal> {
        let panel = required(&request.panel, "cancel.panel")?;
        self.run_owned(
            runtime,
            request.token,
            panel,
            |manager, candidates, surface| {
                manager.cancel_composition(surface);
                candidates.clear();
                surface.list_closed();
                false
            },
        )
    }

    /// The commit the symbol-picker chord runs before the picker opens: the
    /// highlighted cell while the window shows one, the composition as
    /// rendered with its auto space otherwise; under TPS always the
    /// composition (`ComposingKeyIntent::commit_first`). Chosen by the
    /// highlight the window reports (the Linux shell asks its list instead,
    /// `commit_for_picker`).
    pub(crate) fn commit_for_symbol_picker(
        &mut self,
        runtime: &DesktopRuntime,
        request: &CommitForSymbolPickerRequest,
    ) -> Result<SessionReply, Refusal> {
        let panel = required(&request.panel, "commit_for_symbol_picker.panel")?;
        let has_highlight = panel.selected_index.is_some();
        self.run_owned(
            runtime,
            request.token,
            panel,
            |manager, candidates, surface| {
                let settings = runtime.settings.current();
                let intent = ComposingKeyIntent::commit_first(
                    has_highlight,
                    settings.choice(&keys::INPUT_MODE),
                );
                // No key: neither commit reads one.
                let key = KeyEventSnapshot::default();
                perform_intent(&intent, &key, &settings, manager, candidates, surface);
                false
            },
        )
    }

    /// A symbol the picker wrote (the core's `insert_symbol`).
    pub(crate) fn insert_symbol(
        &mut self,
        runtime: &DesktopRuntime,
        request: &InsertSymbolRequest,
    ) -> Result<SessionReply, Refusal> {
        if request.symbol.is_empty() {
            return Err(Refusal::Missing("insert_symbol.symbol"));
        }
        let panel = required(&request.panel, "insert_symbol.panel")?;
        self.run_owned(runtime, request.token, panel, |manager, _, surface| {
            let settings = runtime.settings.current();
            insert_symbol(&request.symbol, &settings, manager, surface);
            false
        })
    }

    /// A click on a cap of the TPS key panel: its glyph typed through the
    /// engine's `TpsKey`, as the layout key would type it — but whatever the
    /// window or the slot keys would make of that key
    /// (`ComposingKeyIntent::tps_keyboard_press`). Not handled outside TPS
    /// or for a glyph the layout does not type: the panel may have been
    /// clicked after a switch Swift has not yet taken it down for.
    pub(crate) fn tps_keyboard_press(
        &mut self,
        runtime: &DesktopRuntime,
        request: &TpsKeyboardPressRequest,
    ) -> Result<SessionReply, Refusal> {
        if request.glyph.is_empty() {
            return Err(Refusal::Missing("tps_keyboard_press.glyph"));
        }
        let panel = required(&request.panel, "tps_keyboard_press.panel")?;
        self.run_owned(
            runtime,
            request.token,
            panel,
            |manager, candidates, surface| {
                let settings = runtime.settings.current();
                let input_mode = settings.choice(&keys::INPUT_MODE);
                let Some(intent) =
                    ComposingKeyIntent::tps_keyboard_press(&request.glyph, input_mode)
                else {
                    return false;
                };
                // No key: the click names its glyph, and `perform_intent`
                // commits a composition a switch across TPS left behind
                // before it types.
                let no_key = KeyEventSnapshot::default();
                perform_intent(&intent, &no_key, &settings, manager, candidates, surface)
            },
        )
    }

    /// The list on screen again under the settings this request carries
    /// (`represent_list`). No list — none held, or the window says none is
    /// up — records nothing; otherwise the list is shown again, or closed
    /// when the refetch emptied it.
    pub(crate) fn represent(
        &mut self,
        runtime: &DesktopRuntime,
        request: &RepresentRequest,
    ) -> Result<SessionReply, Refusal> {
        let panel = required(&request.panel, "represent.panel")?;
        self.run_owned(
            runtime,
            request.token,
            panel,
            |manager, candidates, surface| {
                // Checked here, not left to `list_changed`: an empty list there
                // records a close, and no list to present is "unchanged".
                if candidates.is_empty() {
                    return false;
                }
                let settings = runtime.settings.current();
                represent_list(&settings, manager, candidates, request.refetch);
                surface.list_changed(candidates);
                false
            },
        )
    }

    /// `token` gives the engine up. The list is left as it is; the next
    /// `Activate` drops it.
    pub(crate) fn release(
        &mut self,
        runtime: &DesktopRuntime,
        request: &ReleaseRequest,
    ) -> Result<SessionReply, Refusal> {
        let token = token(request.token)?;
        let mut coordinator = runtime.lock_coordinator();
        if coordinator.current_owner() != Some(token) {
            return Ok(SessionReply::ignored());
        }
        coordinator.release(token);
        Ok(SessionReply::default())
    }

    /// Runs `work` — which answers `handled` — when `raw_token` owns the
    /// engine, and replies with what it recorded. The list is brought into
    /// line with the window first, so nothing in `work` reads a list the
    /// user cannot see (a list the client gave no caret rectangle for, a
    /// dismissal outside the key path). A non-owner is answered `ignored`
    /// and touches nothing.
    fn run_owned(
        &mut self,
        runtime: &DesktopRuntime,
        raw_token: u64,
        panel: &PanelState,
        work: impl FnOnce(
            &mut ComposingManager,
            &mut CandidateSource,
            &mut RecordingSurface<'_>,
        ) -> bool,
    ) -> Result<SessionReply, Refusal> {
        let token = token(raw_token)?;
        let mut coordinator = runtime.lock_coordinator();
        let Some(manager) = coordinator.manager(token) else {
            return Ok(SessionReply::ignored());
        };
        if !panel.is_list_on_screen {
            self.candidates.clear();
        }
        let mut surface = RecordingSurface::new(panel);
        let handled = work(manager, &mut self.candidates, &mut surface);
        let effects = surface.finish(manager);
        Ok(owner_reply(handled, effects, manager))
    }
}

/// A field the request cannot run without; proto3 would otherwise read an
/// unset one as empty and run anyway.
fn required<'a, T>(field: &'a Option<T>, name: &'static str) -> Result<&'a T, Refusal> {
    field.as_ref().ok_or(Refusal::Missing(name))
}

/// The session a request names. 0 is never a token: it is what a request
/// that left the field unset decodes to.
fn token(raw: u64) -> Result<ContextToken, Refusal> {
    match usize::try_from(raw) {
        Ok(0) | Err(_) => Err(Refusal::Missing("token")),
        Ok(token) => Ok(ContextToken(token)),
    }
}

impl SessionReply {
    /// The token does not own the engine: nothing was done.
    fn ignored() -> Self {
        Self {
            ignored: true,
            ..Self::default()
        }
    }
}

fn owner_reply(handled: bool, effects: Vec<Effect>, manager: &ComposingManager) -> SessionReply {
    SessionReply {
        ignored: false,
        handled,
        effects,
        is_composing: manager.is_composing(),
    }
}

/// One request's surface for the core's executor: records every effect in
/// order and answers the window's questions from the panel state Swift sent.
pub(crate) struct RecordingSurface<'a> {
    panel: &'a PanelState,
    effects: Vec<Effect>,
}

impl<'a> RecordingSurface<'a> {
    pub(crate) fn new(panel: &'a PanelState) -> Self {
        Self {
            panel,
            effects: Vec::new(),
        }
    }

    fn record(&mut self, effect: effect::Effect) {
        self.effects.push(Effect {
            effect: Some(effect),
        });
    }

    /// The recorded effects, with the list's anchor filled in: a
    /// `CandidatesChanged` carries where on screen its list belongs, which
    /// the surface cannot read while the executor holds the manager. Every
    /// arm that refreshes the list does so as its last step
    /// (`intent_executor.rs` `refresh`), so the composition now is the one
    /// the list was fetched for. A TPS list is usually the word before the
    /// caret's (desktop-tps-hanji-conversion-roadmap H4, As built in H-P5),
    /// so it anchors at the caret; any other list covers the composition to
    /// its end.
    pub(crate) fn finish(mut self, manager: &ComposingManager) -> Vec<Effect> {
        let anchor_end = if manager.is_tps_composition() {
            manager.display_caret_utf16()
        } else {
            utf16_len(manager.display_text())
        };
        let mut lists = 0;
        for effect in &mut self.effects {
            if let Some(effect::Effect::CandidatesChanged(list)) = &mut effect.effect {
                list.anchor_end_utf16 = anchor_end;
                lists += 1;
            }
        }
        debug_assert!(lists <= 1, "one refresh per request");
        self.effects
    }
}

impl ComposingEffectExecutor for RecordingSurface<'_> {
    /// The three engine effects that reach a client; the rest no client
    /// sees.
    fn execute(&mut self, effect: &EngineEffect) {
        match effect {
            EngineEffect::UpdatePreedit { text, caret_utf16 } => {
                self.record(effect::Effect::SetMarkedText(proto::SetMarkedText {
                    text: text.clone(),
                    caret_utf16: *caret_utf16,
                }));
            }
            EngineEffect::ClearPreeditWithoutCommit => {
                self.record(effect::Effect::ClearMarkedText(proto::ClearMarkedText {}));
            }
            EngineEffect::CommitTextReplacingPreedit(text) => {
                self.record(effect::Effect::InsertText(proto::InsertText {
                    text: text.clone(),
                }));
            }
            EngineEffect::ClearCandidates
            | EngineEffect::RefreshCandidates
            | EngineEffect::ResetCandidateContext
            | EngineEffect::NextWordUpdateLastSelectedWord { .. }
            | EngineEffect::NextWordWordSelected { .. }
            | EngineEffect::NextWordClearForNewComposing => {}
        }
    }
}

impl IntentSurface for RecordingSurface<'_> {
    fn insert_external(&mut self, text: &str) {
        self.record(effect::Effect::InsertText(proto::InsertText {
            text: text.to_owned(),
        }));
    }

    /// Swift made the client checks before the call (`swap_available`);
    /// it keeps the replacement range and the re-arm arithmetic.
    fn swap_preceding_space(&mut self, replacement: &str) -> bool {
        if !self.panel.swap_available {
            return false;
        }
        self.record(effect::Effect::SwapPrecedingSpace(
            proto::SwapPrecedingSpace {
                replacement: replacement.to_owned(),
            },
        ));
        true
    }

    fn arm_swap(&mut self) {
        self.record(effect::Effect::ArmSwap(proto::ArmSwap {}));
    }

    /// IMKit's `insertText` reports no failure (roadmap D3).
    fn has_write_failed(&self) -> bool {
        false
    }

    /// An emptied list closes the window — an empty, unavailable or
    /// switched-off fetch alike.
    fn list_changed(&mut self, list: &mut CandidateSource) {
        if list.is_empty() {
            self.list_closed();
            return;
        }
        let cells = list
            .cells()
            .into_iter()
            .map(|cell| CandidateCell {
                text: cell.text,
                annotation: cell.annotation,
            })
            .collect();
        self.record(effect::Effect::CandidatesChanged(CandidatesChanged {
            cells,
            leads_with_literal_roman: list.leads_with_literal_roman(),
            // Filled in by `finish`.
            anchor_end_utf16: 0,
        }));
    }

    fn list_closed(&mut self) {
        self.record(effect::Effect::CandidatesClosed(proto::CandidatesClosed {}));
    }

    fn selected_index(&self) -> Option<usize> {
        self.panel.selected_index.map(|index| index as usize)
    }

    fn index_for_key_slot(&self, slot: usize) -> Option<usize> {
        let slot = u32::try_from(slot).ok()?;
        self.panel
            .slot_indices
            .get(&slot)
            .map(|index| *index as usize)
    }

    /// The window reads the direction for its layout and repaints itself.
    fn navigate(&mut self, direction: CandidateNavigation) {
        self.record(effect::Effect::Navigate(proto::Navigate {
            direction: navigation(direction) as i32,
        }));
    }

    /// Swift's key panel flashes the cap.
    fn tps_keyboard_cap_typed(&mut self, cap: TpsKeyCapIndex) {
        self.record(effect::Effect::TpsKeyboardKeyTyped(
            proto::TpsKeyboardKeyTyped {
                row: cap.row as u32,
                cap: cap.cap as u32,
            },
        ));
    }
}

pub(crate) fn navigation(direction: CandidateNavigation) -> proto::CandidateNavigation {
    match direction {
        CandidateNavigation::Left => proto::CandidateNavigation::Left,
        CandidateNavigation::Right => proto::CandidateNavigation::Right,
        CandidateNavigation::Up => proto::CandidateNavigation::Up,
        CandidateNavigation::Down => proto::CandidateNavigation::Down,
        CandidateNavigation::PageUp => proto::CandidateNavigation::PageUp,
        CandidateNavigation::PageDown => proto::CandidateNavigation::PageDown,
        CandidateNavigation::NextCandidate => proto::CandidateNavigation::NextCandidate,
        CandidateNavigation::PreviousCandidate => proto::CandidateNavigation::PreviousCandidate,
    }
}

fn utf16_len(text: &str) -> u32 {
    text.encode_utf16().count() as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key_translation::{COMMAND, CONTROL, OPTION, SHIFT};
    use crate::proto::desktop_request::Request;
    use crate::proto::desktop_response::Reply;
    use crate::proto::{KeyEvent, SettingEntry, SettingsSnapshot};
    use crate::runtime::Shell;
    use crate::test_support::{
        boolean, engine_shell, key_event, next_token, text, FUNCTION, NUMERIC_PAD,
    };
    use std::collections::HashMap;

    // The rest of NSEvent.SpecialKey (`key_translation.rs`).
    const CARRIAGE_RETURN: u32 = 0xD;
    const LEFT_ARROW: u32 = 0xF702;

    fn typed(characters: &str) -> KeyEvent {
        chord(characters, 0, None)
    }

    fn chord(characters: &str, modifier_flags: u64, special_key: Option<u32>) -> KeyEvent {
        key_event(characters, modifier_flags, special_key)
    }

    /// No list on screen, nothing highlighted, no swap armed.
    fn no_list() -> PanelState {
        PanelState::default()
    }

    /// A list on screen with `selected` highlighted and the first slots
    /// addressing the first cells.
    fn list(selected: Option<u32>) -> PanelState {
        PanelState {
            is_list_on_screen: true,
            selected_index: selected,
            slot_indices: (0..9).map(|slot| (slot, slot)).collect(),
            swap_available: false,
        }
    }

    fn marked(text: &str, caret_utf16: u32) -> effect::Effect {
        effect::Effect::SetMarkedText(proto::SetMarkedText {
            text: text.to_owned(),
            caret_utf16,
        })
    }

    fn insert(text: &str) -> effect::Effect {
        effect::Effect::InsertText(proto::InsertText {
            text: text.to_owned(),
        })
    }

    fn cleared() -> effect::Effect {
        effect::Effect::ClearMarkedText(proto::ClearMarkedText {})
    }

    fn closed() -> effect::Effect {
        effect::Effect::CandidatesClosed(proto::CandidatesClosed {})
    }

    /// The key panel's flash of the cap at `row`, `cap` (counted from 0).
    fn flashed(row: u32, cap: u32) -> effect::Effect {
        effect::Effect::TpsKeyboardKeyTyped(proto::TpsKeyboardKeyTyped { row, cap })
    }

    fn arm() -> effect::Effect {
        effect::Effect::ArmSwap(proto::ArmSwap {})
    }

    fn unwrapped(effects: Vec<Effect>) -> Vec<effect::Effect> {
        effects
            .into_iter()
            .map(|effect| effect.effect.expect("every effect is set"))
            .collect()
    }

    fn effects(reply: &SessionReply) -> Vec<effect::Effect> {
        unwrapped(reply.effects.clone())
    }

    /// The list a reply showed.
    fn shown_list(reply: &SessionReply) -> CandidatesChanged {
        effects(reply)
            .into_iter()
            .find_map(|effect| match effect {
                effect::Effect::CandidatesChanged(list) => Some(list),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no list in {reply:?}"))
    }

    /// One test's session on the shared engine, every request carrying the
    /// same snapshot so nothing leaks in from another test.
    struct Typist<'s> {
        shell: &'s Shell,
        token: u64,
        settings: Vec<SettingEntry>,
    }

    impl<'s> Typist<'s> {
        fn activated(shell: &'s Shell, settings: Vec<SettingEntry>) -> Self {
            let typist = Self {
                shell,
                token: next_token(),
                settings,
            };
            typist.activate();
            typist
        }

        fn serve(&self, request: Request) -> SessionReply {
            let snapshot = SettingsSnapshot {
                entries: self.settings.clone(),
            };
            match self.shell.serve(request, Some(&snapshot)) {
                Ok(Reply::Session(reply)) => reply,
                other => panic!("expected a session reply, got {other:?}"),
            }
        }

        fn activate(&self) -> SessionReply {
            self.serve(Request::Activate(ActivateRequest { token: self.token }))
        }

        fn key(&self, event: KeyEvent, panel: PanelState) -> SessionReply {
            self.serve(Request::Key(KeyRequest {
                token: self.token,
                event: Some(event),
                panel: Some(panel),
            }))
        }

        /// Types `text` one letter at a time, the list on screen after the
        /// first; answers the last key's reply.
        fn type_text(&self, text: &str) -> SessionReply {
            let mut reply = None;
            for (index, letter) in text.chars().enumerate() {
                let panel = if index == 0 { no_list() } else { list(Some(0)) };
                reply = Some(self.key(typed(&letter.to_string()), panel));
            }
            reply.expect("some text")
        }

        fn commit_composition(&self) -> SessionReply {
            self.serve(Request::CommitComposition(CommitCompositionRequest {
                token: self.token,
                panel: Some(no_list()),
            }))
        }

        fn cancel(&self) -> SessionReply {
            self.serve(Request::Cancel(CancelRequest {
                token: self.token,
                panel: Some(no_list()),
            }))
        }

        fn release(&self) -> SessionReply {
            self.serve(Request::Release(ReleaseRequest { token: self.token }))
        }

        fn commit_for_symbol_picker(&self, panel: PanelState) -> SessionReply {
            self.serve(Request::CommitForSymbolPicker(
                CommitForSymbolPickerRequest {
                    token: self.token,
                    panel: Some(panel),
                },
            ))
        }

        fn insert_symbol(&self, symbol: &str, panel: PanelState) -> SessionReply {
            self.serve(Request::InsertSymbol(InsertSymbolRequest {
                token: self.token,
                symbol: symbol.to_owned(),
                panel: Some(panel),
            }))
        }

        fn tps_keyboard_press(&self, glyph: &str, panel: PanelState) -> SessionReply {
            self.serve(Request::TpsKeyboardPress(TpsKeyboardPressRequest {
                token: self.token,
                glyph: glyph.to_owned(),
                panel: Some(panel),
            }))
        }

        fn represent(&self, refetch: bool, panel: PanelState) -> SessionReply {
            self.serve(Request::Represent(RepresentRequest {
                token: self.token,
                refetch,
                panel: Some(panel),
            }))
        }

        /// The same session, its requests carrying `settings` from now on.
        fn with_settings(&self, settings: Vec<SettingEntry>) -> Self {
            Self {
                shell: self.shell,
                token: self.token,
                settings,
            }
        }
    }

    fn auto_space() -> Vec<SettingEntry> {
        vec![boolean("autoSpaceEnabled", true)]
    }

    // ---- The recording surface, without the engine ----

    fn recorded(
        panel: &PanelState,
        run: impl FnOnce(&mut RecordingSurface),
    ) -> Vec<effect::Effect> {
        let mut surface = RecordingSurface::new(panel);
        run(&mut surface);
        unwrapped(surface.effects)
    }

    /// The three engine effects a client sees, translated; the rest are
    /// not recorded.
    #[test]
    fn engine_effects_translate_to_client_effects() {
        let effects = recorded(&no_list(), |surface| {
            for effect in [
                EngineEffect::UpdatePreedit {
                    text: "tâi".to_owned(),
                    caret_utf16: 2,
                },
                EngineEffect::ClearPreeditWithoutCommit,
                EngineEffect::CommitTextReplacingPreedit("台".to_owned()),
                EngineEffect::ClearCandidates,
                EngineEffect::RefreshCandidates,
                EngineEffect::ResetCandidateContext,
                EngineEffect::NextWordClearForNewComposing,
            ] {
                surface.execute(&effect);
            }
        });
        assert_eq!(effects, vec![marked("tâi", 2), cleared(), insert("台"),]);
    }

    /// The surface's own effects, and the swap only where Swift said it can.
    #[test]
    fn surface_writes_are_recorded_in_order() {
        let swap = |available| {
            let panel = PanelState {
                swap_available: available,
                ..no_list()
            };
            recorded(&panel, |surface| {
                surface.insert_external("，");
                let swapped = surface.swap_preceding_space("? ");
                surface.arm_swap();
                surface.list_closed();
                assert_eq!(swapped, available);
                assert!(!surface.has_write_failed());
            })
        };
        let replacement = effect::Effect::SwapPrecedingSpace(proto::SwapPrecedingSpace {
            replacement: "? ".to_owned(),
        });
        assert_eq!(swap(true), vec![insert("，"), replacement, arm(), closed()]);
        assert_eq!(swap(false), vec![insert("，"), arm(), closed()]);
    }

    /// An empty list closes the window.
    #[test]
    fn an_emptied_list_closes_the_window() {
        let effects = recorded(&no_list(), |surface| {
            surface.list_changed(&mut CandidateSource::default());
        });
        assert_eq!(effects, vec![closed()]);
    }

    #[test]
    fn every_direction_translates() {
        let directions = [
            (CandidateNavigation::Left, proto::CandidateNavigation::Left),
            (
                CandidateNavigation::Right,
                proto::CandidateNavigation::Right,
            ),
            (CandidateNavigation::Up, proto::CandidateNavigation::Up),
            (CandidateNavigation::Down, proto::CandidateNavigation::Down),
            (
                CandidateNavigation::PageUp,
                proto::CandidateNavigation::PageUp,
            ),
            (
                CandidateNavigation::PageDown,
                proto::CandidateNavigation::PageDown,
            ),
            (
                CandidateNavigation::NextCandidate,
                proto::CandidateNavigation::NextCandidate,
            ),
            (
                CandidateNavigation::PreviousCandidate,
                proto::CandidateNavigation::PreviousCandidate,
            ),
        ];
        for (direction, wire) in directions {
            let effects = recorded(&no_list(), |surface| surface.navigate(direction));
            assert_eq!(
                effects,
                vec![effect::Effect::Navigate(proto::Navigate {
                    direction: wire as i32
                })],
                "{direction:?}"
            );
        }
    }

    /// The window's questions answer from the panel: the highlight, and a
    /// slot only where the page maps it.
    #[test]
    fn the_panel_answers_the_windows_questions() {
        let panel = PanelState {
            is_list_on_screen: true,
            selected_index: Some(4),
            slot_indices: HashMap::from([(0, 9), (2, 11)]),
            swap_available: false,
        };
        let surface = RecordingSurface::new(&panel);
        assert_eq!(surface.selected_index(), Some(4));
        assert_eq!(surface.index_for_key_slot(0), Some(9));
        assert_eq!(surface.index_for_key_slot(1), None);
        assert_eq!(surface.index_for_key_slot(2), Some(11));
        assert_eq!(RecordingSurface::new(&no_list()).selected_index(), None);
    }

    // ---- Key: golden replies for the executor's arms ----

    /// trace: TL; `tai5` → engine display "tâi" (tone 5 = circumflex), 3
    /// UTF-16 units against 4 typed — the list anchors on what is on screen.
    #[test]
    fn typing_composes_and_anchors_the_list_on_the_marked_text() {
        let (_engine, shell) = engine_shell();
        let typist = Typist::activated(shell, vec![]);
        let reply = typist.type_text("tai5");
        assert!(reply.handled && reply.is_composing && !reply.ignored);
        let list = shown_list(&reply);
        assert_eq!(
            effects(&reply),
            vec![
                marked("tâi", 3),
                effect::Effect::CandidatesChanged(list.clone())
            ]
        );
        assert_eq!(list.anchor_end_utf16, 3);
        assert!(!list.cells.is_empty());
    }

    /// Backspace (AppKit names `\u{7F}` `delete`) mid-composition deletes
    /// one character and refetches.
    #[test]
    fn delete_backward_shortens_the_composition_and_refetches() {
        let (_engine, shell) = engine_shell();
        let typist = Typist::activated(shell, vec![]);
        typist.type_text("ta");
        let reply = typist.key(chord("\u{7F}", 0, Some(0x7F)), list(Some(0)));
        assert!(reply.handled && reply.is_composing);
        let list = shown_list(&reply);
        assert_eq!(
            effects(&reply),
            vec![
                marked("t", 1),
                effect::Effect::CandidatesChanged(list.clone())
            ]
        );
        assert_eq!(list.anchor_end_utf16, 1);
    }

    /// trace: Telex keys `vydwxqzf`; `y` after `tai` is the engine's tone 3
    /// → "tài".
    #[test]
    fn a_telex_key_reaches_the_engine_as_a_tone() {
        let (_engine, shell) = engine_shell();
        let typist = Typist::activated(shell, vec![text("toneInputScheme", "telex")]);
        typist.type_text("tai");
        let reply = typist.key(typed("y"), list(Some(0)));
        assert!(reply.handled && reply.is_composing);
        assert_eq!(effects(&reply)[0], marked("tài", 3));
        assert_eq!(shown_list(&reply).anchor_end_utf16, 3);
    }

    /// Return over a list commits the highlighted cell in its own script —
    /// a Hanji cell, so no auto space.
    #[test]
    fn return_commits_the_highlighted_cell() {
        let (_engine, shell) = engine_shell();
        let typist = Typist::activated(shell, auto_space());
        let cells = shown_list(&typist.type_text("ti")).cells;
        let reply = typist.key(chord("\r", 0, Some(CARRIAGE_RETURN)), list(Some(0)));
        assert!(reply.handled && !reply.is_composing);
        assert_eq!(effects(&reply), vec![insert(&cells[0].text), closed()]);
    }

    /// trace: Standard tone scheme → bare slot keys `q w d …`; `w` = slot 1,
    /// which the panel maps to cell 1. A Hanji cell earns no auto space.
    #[test]
    fn a_slot_key_commits_the_cell_the_panel_maps_it_to() {
        let (_engine, shell) = engine_shell();
        let typist = Typist::activated(shell, auto_space());
        let cells = shown_list(&typist.type_text("tai")).cells;
        let mut panel = list(None);
        panel.slot_indices = HashMap::from([(1, 1)]);
        let reply = typist.key(typed("w"), panel);
        assert!(reply.handled && !reply.is_composing);
        assert_eq!(effects(&reply), vec![insert(&cells[1].text), closed()]);
    }

    /// A slot the page leaves empty is consumed and commits nothing.
    #[test]
    fn a_slot_key_on_an_empty_slot_is_consumed() {
        let (_engine, shell) = engine_shell();
        let typist = Typist::activated(shell, vec![]);
        typist.type_text("tai");
        let mut panel = list(None);
        panel.slot_indices.clear();
        let reply = typist.key(typed("q"), panel);
        assert!(reply.handled && reply.is_composing);
        assert_eq!(effects(&reply), vec![]);
    }

    /// trace: Space = the highlighted cell's other script; cell 0 leads
    /// with Hanji, so its annotation (romanization) is written, which earns
    /// the auto space and its arm.
    #[test]
    fn space_commits_the_other_script_with_its_auto_space() {
        let (_engine, shell) = engine_shell();
        let typist = Typist::activated(shell, auto_space());
        let cells = shown_list(&typist.type_text("ka")).cells;
        let romanization = cells[0].annotation.clone().expect("a Hanji cell");
        let reply = typist.key(typed(" "), list(Some(0)));
        assert!(reply.handled && !reply.is_composing);
        assert_eq!(
            effects(&reply),
            vec![insert(&romanization), closed(), insert(" "), arm()]
        );
    }

    /// ⇧Return: the composition as typed, spaced under TL.
    #[test]
    fn shift_return_commits_as_typed_with_its_auto_space() {
        let (_engine, shell) = engine_shell();
        let typist = Typist::activated(shell, auto_space());
        typist.type_text("g");
        let reply = typist.key(chord("\r", SHIFT, Some(CARRIAGE_RETURN)), list(Some(0)));
        assert!(reply.handled && !reply.is_composing);
        assert_eq!(
            effects(&reply),
            vec![insert("g"), closed(), insert(" "), arm()]
        );
    }

    /// trace: the default display is Hanji-first, whose punctuation is full
    /// width: `,` mid-composition commits `l` and writes `，` with the auto
    /// space in one write, then arms. ⌃`,` types no other width (USER
    /// 2026-10-07): under TL it is the host's chord, like ⌘ below — `l` is
    /// committed as typed, no mark, no space, the key handed back.
    #[test]
    fn punctuation_commits_then_inserts_and_ctrl_punctuation_is_the_hosts() {
        let (_engine, shell) = engine_shell();
        let typist = Typist::activated(shell, auto_space());
        typist.type_text("l");
        let reply = typist.key(typed(","), list(Some(0)));
        assert!(reply.handled && !reply.is_composing);
        assert_eq!(effects(&reply), vec![insert("l， "), closed(), arm()]);

        typist.type_text("l");
        let chorded = typist.key(chord(",", CONTROL, None), list(Some(0)));
        assert!(!chorded.handled && !chorded.is_composing);
        assert_eq!(effects(&chorded), vec![insert("l"), closed()]);
    }

    /// ⌘ is the host's: the composition is finished, the key handed back.
    #[test]
    fn a_host_chord_commits_then_passes_the_key_through() {
        let (_engine, shell) = engine_shell();
        let typist = Typist::activated(shell, auto_space());
        typist.type_text("h");
        let reply = typist.key(chord("a", COMMAND, None), list(Some(0)));
        assert!(!reply.handled && !reply.is_composing);
        assert_eq!(effects(&reply), vec![insert("h"), closed()]);
    }

    /// Outside a composition: an attaching mark swaps with the armed space
    /// when Swift says it can; otherwise it is written full width; a digit
    /// goes to the host untouched.
    #[test]
    fn pass_through_swaps_maps_or_hands_the_key_back() {
        let (_engine, shell) = engine_shell();
        let typist = Typist::activated(shell, auto_space());
        let swappable = PanelState {
            swap_available: true,
            ..no_list()
        };
        let swapped = typist.key(typed("?"), swappable);
        assert!(swapped.handled);
        assert_eq!(
            effects(&swapped),
            vec![
                effect::Effect::SwapPrecedingSpace(proto::SwapPrecedingSpace {
                    replacement: "? ".to_owned()
                }),
                arm()
            ]
        );

        let mapped = typist.key(typed("?"), no_list());
        assert!(mapped.handled);
        assert_eq!(effects(&mapped), vec![insert("？")]);

        let plain = typist.key(typed("1"), no_list());
        assert!(!plain.handled && !plain.ignored);
        assert_eq!(effects(&plain), vec![]);
    }

    /// ⌥← steps the caret; the list is not refetched. AppKit's `.function`
    /// and `.numericPad` on an arrow do not make it another key.
    #[test]
    fn the_caret_chord_moves_the_caret_without_a_refetch() {
        let (_engine, shell) = engine_shell();
        let typist = Typist::activated(shell, vec![]);
        typist.type_text("t");
        let arrow = chord(
            "\u{F702}",
            OPTION | FUNCTION | NUMERIC_PAD,
            Some(LEFT_ARROW),
        );
        let reply = typist.key(arrow, list(Some(0)));
        assert!(reply.handled && reply.is_composing);
        assert_eq!(effects(&reply), vec![marked("t", 0)]);
    }

    /// The arrows move the window's selection; nothing else happens.
    #[test]
    fn an_arrow_over_the_list_navigates_it() {
        let (_engine, shell) = engine_shell();
        let typist = Typist::activated(shell, vec![]);
        typist.type_text("t");
        let arrow = chord("\u{F701}", FUNCTION | NUMERIC_PAD, Some(0xF701));
        let reply = typist.key(arrow, list(Some(0)));
        assert!(reply.handled);
        assert_eq!(
            effects(&reply),
            vec![effect::Effect::Navigate(proto::Navigate {
                direction: proto::CandidateNavigation::Down as i32
            })]
        );
    }

    #[test]
    fn escape_cancels_the_composition_and_the_list() {
        let (_engine, shell) = engine_shell();
        let typist = Typist::activated(shell, vec![]);
        typist.type_text("s");
        let reply = typist.key(typed("\u{1B}"), list(Some(0)));
        assert!(reply.handled && !reply.is_composing);
        assert_eq!(effects(&reply), vec![cleared(), closed()]);
    }

    // ---- The symbol picker ----

    /// trace: `ka` → cell 0 = 共 (annotation kā). With the window showing a
    /// highlight, the picker commits that cell in its own script — a Hanji
    /// cell earns no auto space.
    #[test]
    fn the_picker_commits_the_highlighted_cell() {
        let (_engine, shell) = engine_shell();
        let typist = Typist::activated(shell, auto_space());
        let cells = shown_list(&typist.type_text("ka")).cells;
        assert_eq!(cells[0].text, "共");
        let reply = typist.commit_for_symbol_picker(list(Some(0)));
        assert!(!reply.handled && !reply.is_composing && !reply.ignored);
        assert_eq!(effects(&reply), vec![insert(&cells[0].text), closed()]);
    }

    /// Under TPS the picker commits the composition as shown even over a
    /// highlight: a pick would only nail a word and keep composing (Hanji
    /// conversion H6). trace, read by running: ㄍㄚˋ → 絞 (E1 P5a corpus pick), no space.
    #[test]
    fn under_tps_the_picker_commits_the_composition_as_shown() {
        let (_engine, shell) = engine_shell();
        let typist = Typist::activated(shell, vec![text("inputMode", "tps")]);
        for key in ["e", "8", "4"] {
            typist.key(typed(key), no_list());
        }
        let down = chord("\u{F701}", FUNCTION | NUMERIC_PAD, Some(0xF701));
        assert!(!shown_list(&typist.key(down, no_list())).cells.is_empty());
        let reply = typist.commit_for_symbol_picker(list(Some(0)));
        assert!(!reply.handled && !reply.is_composing);
        assert_eq!(effects(&reply), vec![insert("絞"), closed()]);
    }

    /// No highlight: the composition as typed, spaced under TL
    /// (`commitAsTyped`) — what the picker writes after it.
    #[test]
    fn the_picker_commits_as_typed_without_a_highlight() {
        let (_engine, shell) = engine_shell();
        let typist = Typist::activated(shell, auto_space());
        typist.type_text("g");
        let reply = typist.commit_for_symbol_picker(list(None));
        assert!(!reply.handled && !reply.is_composing);
        assert_eq!(
            effects(&reply),
            vec![insert("g"), closed(), insert(" "), arm()]
        );
        // Nothing composing (Swift does not send it then): the as-typed
        // commit writes nothing and closes the list, as `commitAsTyped`.
        assert_eq!(
            effects(&typist.commit_for_symbol_picker(no_list())),
            vec![closed()]
        );
    }

    /// A highlight the window reports with no list up: the list is dropped
    /// first, so the highlight resolves to no cell, and nothing is committed.
    #[test]
    fn the_picker_commits_nothing_for_a_highlight_without_a_list() {
        let (_engine, shell) = engine_shell();
        let typist = Typist::activated(shell, auto_space());
        typist.type_text("ka");
        let panel = PanelState {
            selected_index: Some(0),
            ..no_list()
        };
        let reply = typist.commit_for_symbol_picker(panel);
        assert!(reply.is_composing);
        assert_eq!(effects(&reply), vec![]);
    }

    /// The picker's symbol: an attaching mark swaps with the armed space
    /// and re-arms; with no swap it is written as it is, and a mark that
    /// does not attach is written even when a swap is armed.
    #[test]
    fn a_picked_symbol_swaps_or_is_written_as_picked() {
        let (_engine, shell) = engine_shell();
        let typist = Typist::activated(shell, auto_space());
        let swappable = PanelState {
            swap_available: true,
            ..no_list()
        };
        let swapped = typist.insert_symbol("？", swappable.clone());
        assert!(!swapped.handled && !swapped.ignored);
        assert_eq!(
            effects(&swapped),
            vec![
                effect::Effect::SwapPrecedingSpace(proto::SwapPrecedingSpace {
                    replacement: "？ ".to_owned()
                }),
                arm()
            ]
        );
        assert_eq!(
            effects(&typist.insert_symbol("？", no_list())),
            vec![insert("？")]
        );
        assert_eq!(
            effects(&typist.insert_symbol("（）", swappable.clone())),
            vec![insert("（）")]
        );
        let no_auto_space = typist.with_settings(vec![]);
        assert_eq!(
            effects(&no_auto_space.insert_symbol("？", swappable)),
            vec![insert("？")]
        );
    }

    /// trace: `guahoo` → cell 1 = 我 (guá), one syllable of two: the commit
    /// nails it and the rest stays composing — the composition shown again
    /// and the list refetched for it, anchored on the 4-unit `我hoo`
    /// (`commit_candidate` → `refresh`).
    #[test]
    fn the_picker_nails_a_highlighted_segment_and_keeps_composing() {
        let (_engine, shell) = engine_shell();
        let typist = Typist::activated(shell, auto_space());
        let cells = shown_list(&typist.type_text("guahoo")).cells;
        assert_eq!(cells[1].text, "我");
        let reply = typist.commit_for_symbol_picker(list(Some(1)));
        assert!(!reply.handled && reply.is_composing);
        let list = shown_list(&reply);
        assert_eq!(
            effects(&reply),
            vec![
                marked("我hoo", 4),
                effect::Effect::CandidatesChanged(list.clone())
            ]
        );
        assert_eq!(list.anchor_end_utf16, 4);
        // trace: the refetch is for what is left, `hoo` → 予 (hōo) first.
        assert_eq!(list.cells[0].annotation.as_deref(), Some("hōo"));
    }

    // ---- Represent ----

    /// No list held, or one the window no longer shows: nothing to do.
    #[test]
    fn represent_without_a_list_does_nothing() {
        let (_engine, shell) = engine_shell();
        let typist = Typist::activated(shell, vec![]);
        for refetch in [false, true] {
            let reply = typist.represent(refetch, no_list());
            assert!(!reply.ignored);
            assert_eq!(effects(&reply), vec![]);
        }
        typist.type_text("ka");
        assert_eq!(effects(&typist.represent(true, no_list())), vec![]);
        // trace: the reconcile dropped the list, so the next represent with
        // the window reported up has nothing either.
        assert_eq!(effects(&typist.represent(false, list(Some(0)))), vec![]);
    }

    /// trace: `ka` Hanji-first → 共 / kā; after the Hanji/romanization swap
    /// (`isTranslateSwapped` false) the same list leads with the
    /// romanization, no refetch; the anchor is the 2-unit composition.
    #[test]
    fn represent_presents_the_same_list_under_the_new_settings() {
        let (_engine, shell) = engine_shell();
        let typist = Typist::activated(shell, vec![]);
        let before = shown_list(&typist.type_text("ka"));
        let reply = typist
            .with_settings(vec![boolean("isTranslateSwapped", false)])
            .represent(false, list(Some(0)));
        assert!(reply.is_composing);
        let after = shown_list(&reply);
        assert_eq!(effects(&reply).len(), 1);
        assert_eq!(after.cells.len(), before.cells.len());
        assert_eq!(
            after.cells[0],
            CandidateCell {
                text: before.cells[0].annotation.clone().expect("a Hanji cell"),
                annotation: Some(before.cells[0].text.clone()),
            }
        );
        assert_eq!(after.anchor_end_utf16, 2);
    }

    /// trace (2026-10-02 dictionaries): Candidate Display → Romanization
    /// Only refetches `ka`: 123 side-by-side cells collapse to 49
    /// romanizations, none annotated.
    #[test]
    fn represent_refetches_under_the_new_display() {
        let (_engine, shell) = engine_shell();
        let typist = Typist::activated(shell, vec![]);
        let before = shown_list(&typist.type_text("ka"));
        let roman_only = typist.with_settings(vec![text("candidateDisplayMode", "romanOnly")]);
        // Control: presented again without a refetch, the same rows stay —
        // the collapse happens at fetch time.
        let repainted = shown_list(&roman_only.represent(false, list(Some(0))));
        assert_eq!(repainted.cells.len(), before.cells.len());
        let reply = roman_only.represent(true, list(Some(0)));
        let after = shown_list(&reply);
        assert_eq!(effects(&reply).len(), 1);
        assert!(after.cells.len() < before.cells.len());
        assert!(after.cells.iter().all(|cell| cell.annotation.is_none()));
        assert_eq!(after.cells[0].text, "kā");
    }

    /// The refetch obeys Show Candidate Window: off, the list closes. The
    /// deleted Swift back end (P13, #361) refetched without reading it
    /// (inventory C4, characterised in P10); this is the core's rule.
    #[test]
    fn a_refetch_with_the_window_off_closes_the_list() {
        let (_engine, shell) = engine_shell();
        let typist = Typist::activated(shell, vec![]);
        typist.type_text("ka");
        let off = typist.with_settings(vec![boolean("candidateWindowEnabled", false)]);
        let reply = off.represent(true, list(Some(0)));
        assert!(reply.is_composing);
        assert_eq!(effects(&reply), vec![closed()]);
        assert_eq!(effects(&off.represent(false, list(Some(0)))), vec![]);
    }

    // ---- The panel is the truth about the list ----

    /// A list the window no longer shows is dropped before the key is
    /// read: Space then commits the composition as typed with the space
    /// (CommitThenInsert), not a candidate the user cannot see.
    #[test]
    fn a_list_the_window_dropped_is_not_read() {
        let (_engine, shell) = engine_shell();
        let typist = Typist::activated(shell, auto_space());
        typist.type_text("ta");
        let reply = typist.key(typed(" "), no_list());
        assert!(reply.handled && !reply.is_composing);
        assert_eq!(effects(&reply), vec![insert("ta "), closed(), arm()]);
    }

    /// The window switched off since the last key: the list comes down
    /// before the key, and the refresh fetches nothing. The two closes are
    /// `key`'s window-off check and the executor's refresh, each closing on
    /// its own — not a duplicate to fold.
    #[test]
    fn a_switched_off_window_closes_the_list_before_the_key() {
        let (_engine, shell) = engine_shell();
        let typist = Typist::activated(shell, vec![]);
        typist.type_text("t");
        let off = Typist {
            settings: vec![boolean("candidateWindowEnabled", false)],
            ..typist
        };
        let reply = off.key(typed("a"), list(Some(0)));
        assert!(reply.handled && reply.is_composing);
        assert_eq!(effects(&reply), vec![closed(), marked("ta", 2), closed()]);
    }

    /// A switch across TPS leaves the composition on screen; the next key
    /// commits it as shown, then is read as the new mode's first — a TPS
    /// layout key (`1` = ㄅ, `tps_layout.rs`), not a TL digit.
    #[test]
    fn the_first_key_after_a_switch_across_tps_commits_the_composition_first() {
        let (_engine, shell) = engine_shell();
        let typist = Typist::activated(shell, vec![]);
        typist.type_text("tai");
        let tps = Typist {
            settings: vec![text("inputMode", "tps")],
            ..typist
        };
        let reply = tps.key(typed("1"), no_list());
        assert!(reply.handled && reply.is_composing);
        // trace: left-behind commit writes the preedit as shown (`tai`) and
        // closes the list; the keyless `Commit` closes it again with nothing
        // to write; `1` then starts a glyph composition and fetches.
        assert_eq!(
            effects(&reply)[..4],
            [insert("tai"), closed(), closed(), marked("ㄅ", 1)]
        );
    }

    /// Desktop TPS D7: typing opens no list; ↓ opens it; the number-row `2`
    /// (`kVK_ANSI_2`) then picks — over a list the window reports up. The
    /// pick nails the word and closes the list, the composition stays marked;
    /// Return writes it (Hanji conversion H4, B2).
    #[test]
    fn under_tps_the_list_opens_on_demand_and_the_number_row_picks() {
        let (_engine, shell) = engine_shell();
        let typist = Typist::activated(shell, vec![text("inputMode", "tps")]);
        typist.key(typed("e"), no_list());
        let reply = typist.key(typed("8"), no_list());
        // trace: `8` is the number row's eighth cap (row 0, cap 7).
        assert_eq!(
            effects(&reply),
            vec![marked("ㄍㄚ", 2), flashed(0, 7)],
            "no list while typing"
        );
        let down = chord("\u{F701}", FUNCTION | NUMERIC_PAD, Some(0xF701));
        let opened = typist.key(down.clone(), no_list());
        assert!(opened.handled && opened.is_composing);
        let second = shown_list(&opened).cells[1].text.clone();
        // Escape closes it and keeps the glyphs; ↓ opens it again.
        let closed_again = typist.key(typed("\u{1b}"), list(Some(0)));
        assert!(closed_again.is_composing);
        assert_eq!(effects(&closed_again), vec![closed()]);
        let reopened = typist.key(down, no_list());
        assert_eq!(shown_list(&reopened).cells[1].text, second);
        let two = KeyEvent {
            key_code: Some(0x13),
            ..typed("2")
        };
        let picked = typist.key(two, list(Some(0)));
        assert!(picked.handled && picked.is_composing);
        // A pick types no glyph, so no cap flashes.
        assert_eq!(effects(&picked), vec![marked(&second, 1), closed()]);
        let enter = typist.key(chord("\r", 0, Some(CARRIAGE_RETURN)), no_list());
        assert!(enter.handled && !enter.is_composing);
        assert_eq!(effects(&enter).first(), Some(&insert(&second)));
    }

    /// Hanji conversion on macOS: `e` `8` `4` (ㄍㄚˋ) is marked converted; a
    /// plain ← steps the caret over the word; ⇧Return writes the glyphs.
    /// trace, read by running: ㄍㄚˋ → 絞 (E1 P5a corpus pick: 絞/ká 66 > 假/ká 25).
    #[test]
    fn under_tps_a_closed_reading_is_marked_converted_and_shift_return_writes_the_glyphs() {
        let (_engine, shell) = engine_shell();
        let typist = Typist::activated(shell, vec![text("inputMode", "tps")]);
        typist.key(typed("e"), no_list());
        typist.key(typed("8"), no_list());
        let reply = typist.key(typed("4"), no_list());
        assert_eq!(effects(&reply), vec![marked("絞", 1), flashed(0, 3)]);
        let left = chord("\u{F702}", FUNCTION | NUMERIC_PAD, Some(LEFT_ARROW));
        let stepped = typist.key(left, no_list());
        assert!(stepped.handled && stepped.is_composing);
        assert_eq!(effects(&stepped), vec![marked("絞", 0)]);
        let shift_enter = typist.key(chord("\r", SHIFT, Some(CARRIAGE_RETURN)), no_list());
        assert!(shift_enter.handled && !shift_enter.is_composing);
        assert_eq!(effects(&shift_enter).first(), Some(&insert("ㄍㄚˋ")));
    }

    /// INVARIANT_TPS_PREEDIT_HANJI_CONVERSION (§59): a TPS list is the word
    /// before the caret's, so the window anchors at the caret; TL anchors at
    /// the end (`typing_composes_and_anchors_the_list_on_the_marked_text`).
    /// trace, read by running: `e` `8` → ㄍㄚ (open reading); Space → 家;
    /// `1` `8` `5` (ㄅㄚ˫) → 家罷, caret 2.
    #[test]
    fn under_tps_the_list_anchors_at_the_caret() {
        let (_engine, shell) = engine_shell();
        let typist = Typist::activated(shell, vec![text("inputMode", "tps")]);
        let down = chord("\u{F701}", FUNCTION | NUMERIC_PAD, Some(0xF701));
        let left = chord("\u{F702}", FUNCTION | NUMERIC_PAD, Some(LEFT_ARROW));
        let escape = typed("\u{1b}");
        typist.key(typed("e"), no_list());
        typist.key(typed("8"), no_list());
        // An open reading at the end: the end, as before.
        let open = shown_list(&typist.key(down.clone(), no_list()));
        assert_eq!(open.anchor_end_utf16, 2);
        typist.key(escape.clone(), list(Some(0)));
        for key in [" ", "1", "8", "5"] {
            typist.key(typed(key), no_list());
        }
        // Caret at the end: 罷's list, under 罷.
        let last = shown_list(&typist.key(down.clone(), no_list()));
        assert_eq!(last.cells[0].text, "罷");
        assert_eq!(last.anchor_end_utf16, 2);
        typist.key(escape, list(Some(0)));
        // ← steps over 罷: 家's list, under 家 — not the end.
        assert_eq!(
            effects(&typist.key(left.clone(), no_list())),
            vec![marked("家罷", 1)]
        );
        let first = shown_list(&typist.key(down.clone(), no_list()));
        assert_eq!(first.cells[0].text, "家");
        assert_eq!(first.anchor_end_utf16, 1);
        // Pick 家 (the caret goes to the end), ← to the start of the tail:
        // the list is 罷's, the word after the caret, and the window stays at
        // the caret — under the nailed 家 (roadmap H8, as built in H-P5).
        let one = KeyEvent {
            key_code: Some(0x12),
            ..typed("1")
        };
        assert_eq!(
            effects(&typist.key(one, list(Some(0)))),
            vec![marked("家罷", 2), closed()]
        );
        typist.key(left, no_list());
        let tail_start = shown_list(&typist.key(down, no_list()));
        assert_eq!(tail_start.cells[0].text, "罷");
        assert_eq!(tail_start.anchor_end_utf16, 1);
    }

    /// A slot key after a switch across TPS picks nothing from the old
    /// list — even one still reported up: keypad `2` (`kVK_ANSI_Keypad2`)
    /// commits `tai` as shown, then, with nothing composing, is document text
    /// the host types.
    #[test]
    fn a_keypad_slot_key_after_a_switch_across_tps_picks_nothing() {
        let (_engine, shell) = engine_shell();
        let keypad_two = KeyEvent {
            key_code: Some(0x54),
            ..chord("2", NUMERIC_PAD, None)
        };
        for panel in [no_list(), list(Some(0))] {
            let typist = Typist::activated(shell, vec![]);
            typist.type_text("tai");
            let tps = Typist {
                settings: vec![text("inputMode", "tps")],
                ..typist
            };
            let reply = tps.key(keypad_two.clone(), panel);
            assert!(!reply.handled && !reply.is_composing);
            assert_eq!(effects(&reply), vec![insert("tai"), closed(), closed()]);
        }
    }

    /// The other way: glyphs left behind under TL are written as shown
    /// before the next key — Space here, which then has nothing to separate
    /// and passes through as document text.
    #[test]
    fn the_first_key_after_leaving_tps_commits_the_glyphs_first() {
        let (_engine, shell) = engine_shell();
        let typist = Typist::activated(shell, vec![text("inputMode", "tps")]);
        typist.key(typed("e"), no_list());
        typist.key(typed("8"), list(Some(0)));
        let tl = Typist {
            settings: vec![text("inputMode", "tl")],
            ..typist
        };
        let reply = tl.key(typed(" "), no_list());
        // trace: `ㄍㄚ` written as shown; the keyless `Commit` finds nothing;
        // an idle Space is not taken, so the host types it.
        assert!(!reply.handled && !reply.is_composing);
        assert_eq!(effects(&reply), vec![insert("ㄍㄚ"), closed(), closed()]);
    }

    /// Desktop TPS D6: a panel click types its glyph as the layout key
    /// would — and with the window up, a click on the `4` cap types ˋ
    /// (closing the window, as a glyph key does) rather than picking slot 4.
    #[test]
    fn a_tps_keyboard_press_types_its_glyph_even_over_an_open_list() {
        let (_engine, shell) = engine_shell();
        let typist = Typist::activated(shell, vec![text("inputMode", "tps")]);
        let first = typist.tps_keyboard_press("ㄍ", no_list());
        assert!(first.handled && first.is_composing);
        // trace: ㄍ is `e`, row 1 cap 2 — the clicked cap flashes as a key's.
        assert_eq!(effects(&first), vec![marked("ㄍ", 1), flashed(1, 2)]);
        typist.tps_keyboard_press("ㄚ", no_list());
        let down = chord("\u{F701}", FUNCTION | NUMERIC_PAD, Some(0xF701));
        assert!(!shown_list(&typist.key(down, no_list())).cells.is_empty());
        // trace: KEYS `4` = ˋ (U+02CB, tone 2); after ㄚ the adjuster keeps it,
        // and the closed reading shows converted (read by running: 絞).
        let over_list = typist.tps_keyboard_press("\u{02cb}", list(Some(0)));
        assert!(over_list.handled && over_list.is_composing);
        // The flash comes before the window goes, as the executor runs them.
        assert_eq!(
            effects(&over_list),
            vec![marked("絞", 1), flashed(0, 3), closed()]
        );
    }

    /// A press is not handled outside TPS — a click that raced a switch —
    /// nor for anything the layout does not type, the separator included.
    #[test]
    fn a_tps_keyboard_press_outside_tps_or_off_the_layout_does_nothing() {
        let (_engine, shell) = engine_shell();
        let tl = Typist::activated(shell, vec![]);
        let reply = tl.tps_keyboard_press("ㄅ", no_list());
        assert!(!reply.handled && !reply.ignored && !reply.is_composing);
        assert_eq!(effects(&reply), vec![]);
        let tps = tl.with_settings(vec![text("inputMode", "tps")]);
        for not_a_glyph in [" ", "a", "ㄅㄚ"] {
            let reply = tps.tps_keyboard_press(not_a_glyph, no_list());
            assert!(!reply.handled && !reply.is_composing, "{not_a_glyph:?}");
            assert_eq!(effects(&reply), vec![], "{not_a_glyph:?}");
        }
    }

    /// TL ↔ POJ crosses no TPS: the composition carries on under the new
    /// romanization, nothing is committed.
    #[test]
    fn a_switch_between_romanizations_keeps_the_composition() {
        let (_engine, shell) = engine_shell();
        let typist = Typist::activated(shell, vec![]);
        typist.type_text("tai");
        let poj = Typist {
            settings: vec![text("inputMode", "poj")],
            ..typist
        };
        let reply = poj.key(typed("n"), list(Some(0)));
        assert!(reply.handled && reply.is_composing);
        assert!(!effects(&reply)
            .iter()
            .any(|effect| matches!(effect, effect::Effect::InsertText(_))));
    }

    // ---- Lifecycle ----

    /// The lifecycle commit writes the composition as typed — no auto
    /// space, no list effect.
    #[test]
    fn the_lifecycle_commit_writes_the_composition_as_typed() {
        let (_engine, shell) = engine_shell();
        let typist = Typist::activated(shell, auto_space());
        typist.type_text("tai");
        let reply = typist.commit_composition();
        assert!(!reply.handled && !reply.is_composing && !reply.ignored);
        assert_eq!(effects(&reply), vec![insert("tai")]);
    }

    /// With the window still reported up, the lifecycle commit keeps the
    /// list — no list effect; the window, which Swift takes down first,
    /// is what drops it.
    #[test]
    fn the_lifecycle_commit_leaves_the_list_to_the_window() {
        let (_engine, shell) = engine_shell();
        let typist = Typist::activated(shell, vec![]);
        typist.type_text("tai");
        let reply = typist.serve(Request::CommitComposition(CommitCompositionRequest {
            token: typist.token,
            panel: Some(list(Some(0))),
        }));
        assert_eq!(effects(&reply), vec![insert("tai")]);
        // trace: the list kept, `q` is still slot 0 — its commit finds no
        // composition, and the refresh that follows closes the list
        // (`commit_candidate` → `refresh`); a dropped list would have made
        // `q` composition text instead.
        let next = typist.key(typed("q"), list(None));
        assert!(next.handled && !next.is_composing);
        assert_eq!(effects(&next), vec![closed()]);
    }

    #[test]
    fn cancel_drops_the_composition_and_the_list() {
        let (_engine, shell) = engine_shell();
        let typist = Typist::activated(shell, vec![]);
        typist.type_text("t");
        let reply = typist.cancel();
        assert!(!reply.is_composing && !reply.ignored);
        assert_eq!(effects(&reply), vec![cleared(), closed()]);
    }

    // ---- Tokens ----

    /// A token that does not own the engine is answered `ignored` and
    /// touches nothing — not the owner's composition, not its list (the
    /// non-owner's "no list on screen" is not reconciled).
    #[test]
    fn a_non_owner_is_ignored_and_changes_nothing() {
        let (_engine, shell) = engine_shell();
        let owner = Typist::activated(shell, vec![]);
        let cells = shown_list(&owner.type_text("tai")).cells;
        let other = Typist {
            shell,
            token: next_token(),
            settings: vec![],
        };
        for reply in [
            other.key(typed("q"), no_list()),
            other.commit_composition(),
            other.cancel(),
            other.commit_for_symbol_picker(list(Some(0))),
            other.insert_symbol("，", no_list()),
            other.represent(true, no_list()),
            other.release(),
        ] {
            assert_eq!(reply, SessionReply::ignored());
        }
        let picked = owner.key(typed("q"), list(None));
        assert_eq!(effects(&picked), vec![insert(&cells[0].text), closed()]);
    }

    /// An ignored request's snapshot is not put in force either: the
    /// owner's next request without one still reads its own settings.
    #[test]
    fn an_ignored_request_leaves_the_settings_alone() {
        let (_engine, shell) = engine_shell();
        let owner = Typist::activated(shell, vec![]);
        owner.type_text("t");
        let other = Typist {
            shell,
            token: next_token(),
            settings: vec![boolean("candidateWindowEnabled", false)],
        };
        assert!(other.key(typed("a"), no_list()).ignored);
        let request = Request::Key(KeyRequest {
            token: owner.token,
            event: Some(typed("a")),
            panel: Some(list(Some(0))),
        });
        let Ok(Reply::Session(reply)) = shell.serve(request, None) else {
            panic!("expected a session reply");
        };
        assert!(
            !shown_list(&reply).cells.is_empty(),
            "the window is still on"
        );
    }

    /// The same for the picker's requests: neither an ignored one nor a
    /// refused one (no panel) puts its snapshot in force, and neither
    /// touches the owner's composition or list.
    #[test]
    fn ignored_and_refused_picker_requests_leave_the_owner_alone() {
        let (_engine, shell) = engine_shell();
        let owner = Typist::activated(shell, vec![]);
        let cells = shown_list(&owner.type_text("ka")).cells;
        let window_off = vec![boolean("candidateWindowEnabled", false)];
        let other = Typist {
            shell,
            token: next_token(),
            settings: window_off.clone(),
        };
        let snapshot = SettingsSnapshot {
            entries: window_off,
        };
        // The owner's next request carries no snapshot: it reads whatever
        // is in force, which must still be its own (window on).
        let owner_list_is_intact = || {
            let request = Request::Represent(RepresentRequest {
                token: owner.token,
                refetch: true,
                panel: Some(list(Some(0))),
            });
            let Ok(Reply::Session(reply)) = shell.serve(request, None) else {
                panic!("expected a session reply");
            };
            assert!(reply.is_composing);
            assert_eq!(shown_list(&reply).cells, cells, "the window is still on");
        };
        assert!(other.commit_for_symbol_picker(list(Some(0))).ignored);
        owner_list_is_intact();
        assert!(other.insert_symbol("，", no_list()).ignored);
        owner_list_is_intact();
        assert!(other.represent(true, no_list()).ignored);
        owner_list_is_intact();
        assert!(other.tps_keyboard_press("ㄅ", list(Some(0))).ignored);
        owner_list_is_intact();
        for request in [
            Request::CommitForSymbolPicker(CommitForSymbolPickerRequest {
                token: owner.token,
                panel: None,
            }),
            Request::InsertSymbol(InsertSymbolRequest {
                token: owner.token,
                symbol: String::new(),
                panel: Some(no_list()),
            }),
            Request::Represent(RepresentRequest {
                token: owner.token,
                refetch: true,
                panel: None,
            }),
        ] {
            assert!(matches!(
                shell.serve(request, Some(&snapshot)),
                Err(Refusal::Missing(_))
            ));
            owner_list_is_intact();
        }
    }

    /// Activating again keeps the composition and drops the list: `q` is
    /// then typed, not a slot.
    #[test]
    fn activating_again_keeps_the_composition_and_drops_the_list() {
        let (_engine, shell) = engine_shell();
        let typist = Typist::activated(shell, vec![]);
        typist.type_text("ta");
        let again = typist.activate();
        assert!(again.is_composing && !again.ignored);
        assert_eq!(effects(&again), vec![]);
        let reply = typist.key(typed("q"), list(None));
        assert_eq!(effects(&reply)[0], marked("taq", 3));
    }

    /// Another session's Activate takes the engine and drops the first
    /// one's composition; the first is ignored from then on.
    #[test]
    fn another_activation_takes_the_engine() {
        let (_engine, shell) = engine_shell();
        let first = Typist::activated(shell, vec![]);
        first.type_text("ta");
        let second = Typist::activated(shell, vec![]);
        assert!(!second.activate().is_composing);
        assert!(first.key(typed("i"), list(Some(0))).ignored);
        assert!(second.key(typed("i"), no_list()).is_composing);
    }

    /// Release gives the engine up: the owner's next key is ignored, and
    /// so is a second release.
    #[test]
    fn a_released_session_is_ignored() {
        let (_engine, shell) = engine_shell();
        let typist = Typist::activated(shell, vec![]);
        typist.type_text("t");
        let released = typist.release();
        assert_eq!(released, SessionReply::default());
        assert!(typist.key(typed("a"), no_list()).ignored);
        assert!(typist.release().ignored);
    }

    /// The focus leaving (Swift's `deactivateServer` releases) and coming
    /// back (`activateServer` claims with the same token): while released
    /// and superseded, every request the token sends is ignored and leaves
    /// the owner's composition alone; activated again, it owns the engine.
    #[test]
    fn a_released_session_activates_again_and_is_ignored_meanwhile() {
        let (_engine, shell) = engine_shell();
        let first = Typist::activated(shell, vec![]);
        first.type_text("ka");
        first.commit_composition();
        assert_eq!(first.release(), SessionReply::default());
        let second = Typist::activated(shell, vec![]);
        second.type_text("ta");
        for reply in [
            first.key(typed("i"), list(Some(0))),
            first.commit_composition(),
            first.cancel(),
            first.commit_for_symbol_picker(list(Some(0))),
            first.insert_symbol("，", no_list()),
            first.represent(true, list(Some(0))),
            first.release(),
        ] {
            assert_eq!(reply, SessionReply::ignored());
        }
        assert_eq!(
            effects(&second.key(typed("i"), list(Some(0))))[0],
            marked("tai", 3)
        );

        let back = first.activate();
        assert!(!back.ignored && !back.is_composing);
        assert!(second.key(typed("i"), no_list()).ignored);
        assert_eq!(
            effects(&first.key(typed("a"), no_list()))[0],
            marked("a", 1)
        );
    }

    // ---- Refusals and recovery ----

    /// Each session request is refused before Configure, and a request
    /// missing what it cannot run without is refused (not read as zero /
    /// empty): token 0, a key with no event or no panel, a lifecycle
    /// request with no panel.
    #[test]
    fn requests_that_cannot_run_are_refused() {
        let unconfigured = Shell::default();
        let panel = Some(no_list());
        let key = |token, event: Option<KeyEvent>, panel: Option<PanelState>| {
            Request::Key(KeyRequest {
                token,
                event,
                panel,
            })
        };
        for request in [
            Request::Activate(ActivateRequest { token: 1 }),
            key(1, Some(typed("a")), panel.clone()),
            Request::CommitComposition(CommitCompositionRequest {
                token: 1,
                panel: panel.clone(),
            }),
            Request::Cancel(CancelRequest {
                token: 1,
                panel: panel.clone(),
            }),
            Request::Release(ReleaseRequest { token: 1 }),
            Request::CommitForSymbolPicker(CommitForSymbolPickerRequest {
                token: 1,
                panel: panel.clone(),
            }),
            Request::InsertSymbol(InsertSymbolRequest {
                token: 1,
                symbol: "，".to_owned(),
                panel: panel.clone(),
            }),
            Request::Represent(RepresentRequest {
                token: 1,
                refetch: false,
                panel: panel.clone(),
            }),
        ] {
            assert_eq!(
                unconfigured.serve(request, None),
                Err(Refusal::NotConfigured)
            );
        }

        let (_engine, shell) = engine_shell();
        let token = next_token();
        let missing = |request| match shell.serve(request, None) {
            Err(Refusal::Missing(field)) => field,
            other => panic!("expected a refusal, got {other:?}"),
        };
        assert_eq!(
            missing(Request::Activate(ActivateRequest { token: 0 })),
            "token"
        );
        assert_eq!(missing(key(0, Some(typed("a")), panel.clone())), "token");
        assert_eq!(missing(key(token, None, panel.clone())), "key.event");
        assert_eq!(missing(key(token, Some(typed("a")), None)), "key.panel");
        assert_eq!(
            missing(Request::CommitComposition(CommitCompositionRequest {
                token,
                panel: None
            })),
            "commit_composition.panel"
        );
        assert_eq!(
            missing(Request::Cancel(CancelRequest { token, panel: None })),
            "cancel.panel"
        );
        assert_eq!(
            missing(Request::CommitForSymbolPicker(
                CommitForSymbolPickerRequest { token, panel: None }
            )),
            "commit_for_symbol_picker.panel"
        );
        assert_eq!(
            missing(Request::InsertSymbol(InsertSymbolRequest {
                token,
                symbol: String::new(),
                panel: panel.clone(),
            })),
            "insert_symbol.symbol"
        );
        assert_eq!(
            missing(Request::InsertSymbol(InsertSymbolRequest {
                token,
                symbol: "，".to_owned(),
                panel: None,
            })),
            "insert_symbol.panel"
        );
        assert_eq!(
            missing(Request::Represent(RepresentRequest {
                token,
                refetch: true,
                panel: None,
            })),
            "represent.panel"
        );
        assert_eq!(
            missing(Request::TpsKeyboardPress(TpsKeyboardPressRequest {
                token,
                glyph: String::new(),
                panel: panel.clone(),
            })),
            "tps_keyboard_press.glyph"
        );
        assert_eq!(
            missing(Request::TpsKeyboardPress(TpsKeyboardPressRequest {
                token,
                glyph: "ㄅ".to_owned(),
                panel: None,
            })),
            "tps_keyboard_press.panel"
        );
    }

    /// rust-ffi-safety.md §6 T3 on the session: eight threads type into one
    /// session with the window switched on and off. Each request runs under
    /// the snapshot it carried — a switched-off key never shows a list.
    #[test]
    fn concurrent_keys_each_run_under_their_own_snapshot() {
        let (_engine, shell) = engine_shell();
        let token = Typist::activated(shell, vec![]).token;
        std::thread::scope(|scope| {
            let workers: Vec<_> = (0..8)
                .map(|index| {
                    scope.spawn(move || {
                        let window = index % 2 == 0;
                        let typist = Typist {
                            shell,
                            token,
                            settings: vec![boolean("candidateWindowEnabled", window)],
                        };
                        (window, typist.key(typed("t"), no_list()))
                    })
                })
                .collect();
            for worker in workers {
                let (window, reply) = worker.join().expect("no panic escapes");
                assert!(reply.handled && !reply.ignored);
                let shows_a_list = effects(&reply)
                    .iter()
                    .any(|effect| matches!(effect, effect::Effect::CandidatesChanged(_)));
                assert!(window || !shows_a_list, "{reply:?}");
            }
        });
    }
}
