//! Stateful host-client + IME simulator wrapping `DesktopRuntime` and
//! `taigi_desktop_core::session`, checking six per-keystroke invariants on
//! every emitted signal.

use std::path::Path;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use taigi_desktop_core::composing::{Clock, ContextToken};
use taigi_desktop_core::engine::user_data;
use taigi_desktop_core::keys::{
    CandidateNavigation, CandidateSlotKeySet, ComposingKeyBindings, KeyEventSnapshot, KeyModifiers,
    LineEdgeKey, NavigationKey, ShortcutAction,
};
use taigi_desktop_core::runtime::{DesktopRuntime, RuntimeParts};
use taigi_desktop_core::session::{
    begin_session, click_from_panel, commit_composition, end_session, is_composing,
    navigate_from_panel, perform_global, process_key, toggle_language_mode, tps_keyboard_press,
    Emit, EngineState, KeyReply, LookupTableContent,
};
use taigi_desktop_core::settings::{SettingsDocument, SettingsStore};
use taigi_desktop_storage::{LiveSettings, SettingsFileStore};
use taigi_linux_platform::DESKTOP_PLATFORM;

#[derive(Debug)]
pub struct SimulatedClock {
    now_ms: AtomicI64,
}

impl SimulatedClock {
    pub const fn new(initial_ms: i64) -> Self {
        Self {
            now_ms: AtomicI64::new(initial_ms),
        }
    }

    pub fn advance_ms(&self, delta_ms: i64) {
        self.now_ms.fetch_add(delta_ms.max(0), Ordering::Relaxed);
    }

    pub fn get_ms(&self) -> i64 {
        self.now_ms.load(Ordering::Relaxed)
    }
}

impl Clock for SimulatedClock {
    fn now_ms(&self) -> i64 {
        self.get_ms()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpecialKey {
    Enter,
    ShiftEnter,
    Space,
    Backspace,
    DeleteForward,
    Escape,
    Tab,
    ShiftTab,
    PagePrevBracket,
    PageNextBracket,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NavDir {
    Left,
    Right,
    Up,
    Down,
    PageUp,
    PageDown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaretStep {
    Left,
    Right,
    Home,
    End,
    CtrlUp,
    CtrlDown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SafeGlobalShortcut {
    ToggleRomanization,
    ToggleTps,
    ToggleTranslateSwapped,
    CycleCandidateDisplayMode,
    ShowSymbolPicker,
    ShowTelexGuide,
}

impl SafeGlobalShortcut {
    pub fn to_action(self) -> ShortcutAction {
        match self {
            Self::ToggleRomanization => ShortcutAction::ToggleRomanization,
            Self::ToggleTps => ShortcutAction::ToggleTps,
            Self::ToggleTranslateSwapped => ShortcutAction::ToggleTranslateSwapped,
            Self::CycleCandidateDisplayMode => ShortcutAction::CycleCandidateDisplayMode,
            Self::ShowSymbolPicker => ShortcutAction::ShowSymbolPicker,
            Self::ShowTelexGuide => ShortcutAction::ShowTelexGuide,
        }
    }
}

/// Every user-level interaction step the simulator can drive and shrink.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimStep {
    /// Type a single character on the main keyboard (`shift` for uppercase / shifted symbols).
    KeyChar { ch: char, shift: bool },
    /// Press a special editing / confirmation key.
    Special(SpecialKey),
    /// Press the current mode's slot key for `slot` (0..9), optionally holding `Shift`
    /// to commit in the alternate script.
    PickSlot { slot: usize, alternate_script: bool },
    /// Candidate-window navigation key.
    Navigate(NavDir),
    /// Panel navigation button (no hardware key).
    PanelNavigate(NavDir),
    /// Panel candidate click (selects without committing).
    PanelClick { position: usize },
    /// Move the composing caret (`Ctrl+Arrow` or `Home`/`End`).
    MoveCaret(CaretStep),
    /// Tap bare `Shift` to toggle Taigi <-> English language mode.
    ShiftTapLanguageToggle,
    /// Trigger one of the safe global shortcuts (never launches external settings binary).
    GlobalShortcut(SafeGlobalShortcut),
    /// Click a glyph on the on-screen TPS keyboard.
    TpsPanelGlyph(String),
    /// Simulate focus leaving and returning (`ibus`/`fcitx5` commit-on-focus-out + `end_session` -> `begin_session`).
    FocusBounceLinux,
    /// Simulate macOS / Web `commit_composition` on focus loss, then refocus.
    FocusBounceCommitComposition,
    /// Switch client `can_delete_surrounding` capability (e.g. moving between terminal and GTK editor).
    SetSurroundingCap(bool),
    /// Advance virtual clock (e.g. user pauses between clauses / sentences).
    AdvanceClockMs(i64),
    /// Add a custom dictionary entry mid-session.
    SaveCustomWord { roman: String, hanji: String },
    /// Clear custom dictionary entries mid-session.
    ClearCustomWords,
    /// Toggle live settings on disk (`IS_LITERAL_ROMAN_CANDIDATE_ENABLED` / `IS_HANJI_FIRST`) via `SettingsFileStore`.
    FlipLiveSettings,
}

/// Simulated host text widget state receiving IBus / Fcitx5 signals.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HostBuffer {
    pub committed: String,
    pub preedit: Option<(String, u32)>,
    pub lookup: Option<LookupTableContent>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InvariantViolation {
    pub code: &'static str,
    pub step_index: usize,
    pub step: SimStep,
    pub message: String,
    pub committed_tail: String,
    pub preedit: Option<(String, u32)>,
    pub lookup_summary: Option<String>,
}

pub struct HostSimulator {
    _config_temp: tempfile::TempDir,
    pub settings_store: SettingsFileStore,
    pub runtime: DesktopRuntime,
    pub clock: Arc<SimulatedClock>,
    pub token: ContextToken,
    pub state: EngineState,
    pub buffer: HostBuffer,
    pub step_count: usize,
    pub key_latencies_us: Vec<u64>,
}

impl HostSimulator {
    pub fn new(
        user_data_dir: &Path,
        dictionaries_dir: &Path,
        initial_settings: &SettingsDocument,
        can_delete_surrounding: bool,
    ) -> Self {
        let config_temp = tempfile::tempdir().expect("config tempdir");
        let store = SettingsFileStore::new(config_temp.path());
        store
            .update_document(&mut |doc| {
                *doc = initial_settings.clone();
            })
            .expect("write initial settings");

        let live_settings = Arc::new(LiveSettings::new(store.clone()));
        let clock = Arc::new(SimulatedClock::new(1_760_000_000_000));
        let dicts = dictionaries_dir.to_path_buf();
        let runtime = DesktopRuntime::new(RuntimeParts {
            settings: live_settings,
            settings_store: Some(Box::new(store.clone())),
            data_directory: Some(user_data_dir.to_path_buf()),
            dictionaries: Box::new(move || Some(dicts.clone())),
            dictionary_version: taigi_linux_core::runtime::dictionary_version(),
            system_locale: Box::new(|| "en-US".to_owned()),
            platform: DESKTOP_PLATFORM,
            clock: Arc::clone(&clock) as _,
        });
        runtime.prepare_for_first_key();

        let token = ContextToken(1);
        let mut state = EngineState {
            can_delete_surrounding,
            ..EngineState::default()
        };
        super::reset_transient_engine_state();
        let mut buffer = HostBuffer::default();
        let init_emits = begin_session(&runtime, &mut state);
        for emit in init_emits {
            if let Emit::LookupTable(t) = emit {
                buffer.lookup = Some(t);
            } else if matches!(emit, Emit::HideLookupTable) {
                buffer.lookup = None;
            }
        }

        Self {
            _config_temp: config_temp,
            settings_store: store,
            runtime,
            clock,
            token,
            state,
            buffer,
            step_count: 0,
            key_latencies_us: Vec::new(),
        }
    }

    pub fn update_settings(&mut self, mutate: impl FnOnce(&mut SettingsDocument)) {
        let mut f = Some(mutate);
        self.settings_store
            .update_document(&mut |doc| {
                if let Some(cb) = f.take() {
                    cb(doc);
                }
            })
            .expect("update settings");
    }

    pub fn clear_committed_buffer(&mut self) {
        self.buffer.committed.clear();
    }

    /// Executes one user interaction step, updates `HostBuffer`, records timing,
    /// and verifies all six per-step invariants.
    pub fn step(&mut self, action: &SimStep) -> Result<(), Box<InvariantViolation>> {
        let step_idx = self.step_count;
        self.step_count += 1;
        // Default inter-key advance so timestamps move monotonically.
        self.clock.advance_ms(95);

        match action {
            SimStep::AdvanceClockMs(ms) => {
                self.clock.advance_ms(*ms);
                return Ok(());
            }
            SimStep::SetSurroundingCap(enabled) => {
                self.state.can_delete_surrounding = *enabled;
                return Ok(());
            }
            SimStep::SaveCustomWord { roman, hanji } => {
                let _ = user_data::save_custom_entry("", roman, hanji);
                return Ok(());
            }
            SimStep::ClearCustomWords => {
                let _ = user_data::delete_all_custom_entries();
                return Ok(());
            }
            SimStep::FlipLiveSettings => {
                use taigi_desktop_core::settings::keys;
                self.update_settings(|doc| {
                    let cur = doc.bool(&keys::IS_LITERAL_ROMAN_CANDIDATE_ENABLED);
                    doc.set_bool(&keys::IS_LITERAL_ROMAN_CANDIDATE_ENABLED, !cur);
                });
                return Ok(());
            }
            SimStep::FocusBounceLinux => {
                // On IBus / Fcitx5 (`mode = COMMIT`), the client commits any visible preedit on
                // focus-out before the daemon calls `end_session`.
                if let Some((preedit_text, _)) = self.buffer.preedit.take() {
                    self.buffer.committed.push_str(&preedit_text);
                }
                let end_emits = end_session(&self.runtime, self.token, &mut self.state);
                self.apply_emits_only(&end_emits, step_idx, action)?;
                let begin_emits = begin_session(&self.runtime, &mut self.state);
                self.apply_emits_only(&begin_emits, step_idx, action)?;
                return self.check_post_invariants(step_idx, action);
            }
            SimStep::FocusBounceCommitComposition => {
                let emits = commit_composition(&self.runtime, self.token, &mut self.state);
                self.apply_emits_only(&emits, step_idx, action)?;
                let begin_emits = begin_session(&self.runtime, &mut self.state);
                self.apply_emits_only(&begin_emits, step_idx, action)?;
                return self.check_post_invariants(step_idx, action);
            }
            SimStep::ShiftTapLanguageToggle => {
                let t0 = Instant::now();
                let emits = toggle_language_mode(&self.runtime, self.token, &mut self.state);
                self.key_latencies_us.push(t0.elapsed().as_micros() as u64);
                self.apply_emits_only(&emits, step_idx, action)?;
                return self.check_post_invariants(step_idx, action);
            }
            SimStep::GlobalShortcut(sc) => {
                let t0 = Instant::now();
                let emits =
                    perform_global(&self.runtime, self.token, &mut self.state, sc.to_action());
                self.key_latencies_us.push(t0.elapsed().as_micros() as u64);
                self.apply_emits_only(&emits, step_idx, action)?;
                return self.check_post_invariants(step_idx, action);
            }
            SimStep::PanelNavigate(dir) => {
                let emits = navigate_from_panel(
                    &self.runtime,
                    self.token,
                    &mut self.state,
                    to_candidate_nav(*dir),
                );
                self.apply_emits_only(&emits, step_idx, action)?;
                return self.check_post_invariants(step_idx, action);
            }
            SimStep::PanelClick { position } => {
                let emits = click_from_panel(&self.runtime, &mut self.state, *position);
                self.apply_emits_only(&emits, step_idx, action)?;
                return self.check_post_invariants(step_idx, action);
            }
            SimStep::TpsPanelGlyph(glyph) => {
                let t0 = Instant::now();
                let emits = tps_keyboard_press(&self.runtime, self.token, &mut self.state, glyph);
                self.key_latencies_us.push(t0.elapsed().as_micros() as u64);
                self.apply_emits_only(&emits, step_idx, action)?;
                return self.check_post_invariants(step_idx, action);
            }
            SimStep::KeyChar { .. }
            | SimStep::Special(_)
            | SimStep::PickSlot { .. }
            | SimStep::Navigate(_)
            | SimStep::MoveCaret(_) => {}
        }

        let snapshot = self.build_key_snapshot(action);
        // Any non-latched key clears a held toggle latch in real hardware key-release flow.
        self.state.latched_chord = None;

        let t0 = Instant::now();
        let reply = process_key(&self.runtime, self.token, &mut self.state, &snapshot);
        self.key_latencies_us.push(t0.elapsed().as_micros() as u64);

        self.apply_key_reply(&reply, &snapshot, step_idx, action)?;
        self.check_post_invariants(step_idx, action)
    }

    fn build_key_snapshot(&self, action: &SimStep) -> KeyEventSnapshot {
        match action {
            SimStep::KeyChar { ch, shift } => {
                let mods = if *shift {
                    KeyModifiers::SHIFT
                } else {
                    KeyModifiers::NONE
                };
                let s = ch.to_string();
                let mut snap = KeyEventSnapshot::text(&s, mods);
                if !*shift && ('1'..='9').contains(ch) {
                    snap.key_code = Some(0x31 + (*ch as u16 - '1' as u16));
                } else if !*shift && *ch == '0' {
                    snap.key_code = Some(0x30);
                } else if !*shift && *ch == ';' {
                    snap.key_code = Some(0xBA);
                }
                snap
            }
            SimStep::Special(spec) => match spec {
                SpecialKey::Enter => KeyEventSnapshot::text("\r", KeyModifiers::NONE),
                SpecialKey::ShiftEnter => KeyEventSnapshot::text("\r", KeyModifiers::SHIFT),
                SpecialKey::Space => KeyEventSnapshot::text(" ", KeyModifiers::NONE),
                SpecialKey::Backspace => KeyEventSnapshot::text("\u{8}", KeyModifiers::NONE),
                SpecialKey::DeleteForward => KeyEventSnapshot {
                    characters: Some("\u{7F}".to_owned()),
                    characters_ignoring_modifiers: Some("\u{7F}".to_owned()),
                    is_named_special_key: true,
                    ..KeyEventSnapshot::default()
                },
                SpecialKey::Escape => KeyEventSnapshot::text("\u{1B}", KeyModifiers::NONE),
                SpecialKey::Tab => KeyEventSnapshot::text("\t", KeyModifiers::NONE),
                SpecialKey::ShiftTab => KeyEventSnapshot::text("\t", KeyModifiers::SHIFT),
                SpecialKey::PagePrevBracket => KeyEventSnapshot::text("[", KeyModifiers::NONE),
                SpecialKey::PageNextBracket => KeyEventSnapshot::text("]", KeyModifiers::NONE),
            },
            SimStep::PickSlot {
                slot,
                alternate_script,
            } => {
                let settings = self.runtime.settings.current();
                let bindings = ComposingKeyBindings::from_document(&settings, DESKTOP_PLATFORM);
                let set = bindings.slot_key_set();
                let clamped = (*slot).min(set.page_size().saturating_sub(1));
                let mods = if *alternate_script {
                    KeyModifiers::SHIFT
                } else {
                    KeyModifiers::NONE
                };
                match set {
                    CandidateSlotKeySet::BareKeys => {
                        let key = CandidateSlotKeySet::BARE_KEY_ROW[clamped];
                        let typed = if *alternate_script && key != ";" {
                            key.to_ascii_uppercase()
                        } else if *alternate_script && key == ";" {
                            ":".to_owned()
                        } else {
                            key.to_owned()
                        };
                        let mut snap = KeyEventSnapshot::chord(Some(&typed), key, mods);
                        if key == ";" {
                            snap.key_code = Some(0xBA);
                        }
                        snap
                    }
                    CandidateSlotKeySet::BareKeysWithoutF => {
                        let key = CandidateSlotKeySet::BARE_KEY_ROW_WITHOUT_F[clamped];
                        let typed = if *alternate_script && key != ";" {
                            key.to_ascii_uppercase()
                        } else if *alternate_script && key == ";" {
                            ":".to_owned()
                        } else {
                            key.to_owned()
                        };
                        let mut snap = KeyEventSnapshot::chord(Some(&typed), key, mods);
                        if key == ";" {
                            snap.key_code = Some(0xBA);
                        }
                        snap
                    }
                    CandidateSlotKeySet::Digits | CandidateSlotKeySet::TpsDigits => {
                        let digit = (b'1' + clamped as u8) as char;
                        let s = digit.to_string();
                        KeyEventSnapshot::text(&s, mods).with_key_code(0x31 + clamped as u16)
                    }
                }
            }
            SimStep::Navigate(dir) => {
                KeyEventSnapshot::navigation(to_nav_key(*dir), KeyModifiers::NONE)
            }
            SimStep::MoveCaret(step) => match step {
                CaretStep::Left => {
                    KeyEventSnapshot::navigation(NavigationKey::LeftArrow, KeyModifiers::CONTROL)
                }
                CaretStep::Right => {
                    KeyEventSnapshot::navigation(NavigationKey::RightArrow, KeyModifiers::CONTROL)
                }
                CaretStep::CtrlUp => {
                    KeyEventSnapshot::navigation(NavigationKey::UpArrow, KeyModifiers::CONTROL)
                }
                CaretStep::CtrlDown => {
                    KeyEventSnapshot::navigation(NavigationKey::DownArrow, KeyModifiers::CONTROL)
                }
                CaretStep::Home => KeyEventSnapshot {
                    is_named_special_key: true,
                    line_edge_key: Some(LineEdgeKey::Home),
                    ..KeyEventSnapshot::default()
                },
                CaretStep::End => KeyEventSnapshot {
                    is_named_special_key: true,
                    line_edge_key: Some(LineEdgeKey::End),
                    ..KeyEventSnapshot::default()
                },
            },
            _ => KeyEventSnapshot::default(),
        }
    }

    fn apply_key_reply(
        &mut self,
        reply: &KeyReply,
        snapshot: &KeyEventSnapshot,
        step_idx: usize,
        action: &SimStep,
    ) -> Result<(), Box<InvariantViolation>> {
        self.apply_emits_only(&reply.emits, step_idx, action)?;

        if !reply.handled {
            // Invariant 1: while composing, no printable/Backspace key should leak unhandled
            // into the host document behind an active preedit.
            if self.buffer.preedit.is_some()
                && !snapshot.modifiers.has_host_chord()
                && !snapshot.is_named_special_key
            {
                if let Some(chars) = &snapshot.characters {
                    if chars == "\u{8}" || chars.chars().any(|c| !c.is_control()) {
                        return Err(self.violation(
                            "UNHANDLED_KEY_DURING_PREEDIT",
                            step_idx,
                            action,
                            format!(
                                "key {chars:?} returned handled=false while preedit {:?} was active",
                                self.buffer.preedit
                            ),
                        ));
                    }
                }
            }

            // Host widget processes unhandled plain keys.
            if !snapshot.modifiers.has_host_chord() && !snapshot.is_named_special_key {
                if let Some(chars) = &snapshot.characters {
                    match chars.as_str() {
                        "\u{8}" => {
                            let _ = self.buffer.committed.pop();
                        }
                        "\r" => self.buffer.committed.push('\n'),
                        "\t" => self.buffer.committed.push('\t'),
                        "\u{1B}" => {}
                        s if s.chars().all(|c| !c.is_control()) => {
                            self.buffer.committed.push_str(s);
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }

    fn apply_emits_only(
        &mut self,
        emits: &[Emit],
        step_idx: usize,
        action: &SimStep,
    ) -> Result<(), Box<InvariantViolation>> {
        for emit in emits {
            match emit {
                Emit::Preedit { text, caret } => {
                    let char_len = text.chars().count() as u32;
                    if *caret > char_len {
                        return Err(self.violation(
                            "PREEDIT_CARET_OUT_OF_BOUNDS",
                            step_idx,
                            action,
                            format!("preedit caret {caret} > char_len {char_len} in {text:?}"),
                        ));
                    }
                    if text.is_empty() {
                        return Err(self.violation(
                            "EMPTY_VISIBLE_PREEDIT",
                            step_idx,
                            action,
                            "Emit::Preedit carried empty text instead of Emit::ClearPreedit"
                                .to_owned(),
                        ));
                    }
                    if text.chars().any(char::is_control) {
                        return Err(self.violation(
                            "CONTROL_CHAR_IN_PREEDIT",
                            step_idx,
                            action,
                            format!("Emit::Preedit contained control character: {text:?}"),
                        ));
                    }
                    self.buffer.preedit = Some((text.clone(), *caret));
                }
                Emit::ClearPreedit => {
                    self.buffer.preedit = None;
                }
                Emit::Commit(text) => {
                    if text
                        .chars()
                        .any(|c| c.is_control() && c != '\n' && c != '\t')
                    {
                        return Err(self.violation(
                            "CONTROL_CHAR_IN_COMMIT",
                            step_idx,
                            action,
                            format!("Emit::Commit contained control character: {text:?}"),
                        ));
                    }
                    if self.buffer.preedit.is_some() {
                        return Err(self.violation(
                            "COMMIT_WHILE_PREEDIT_ACTIVE",
                            step_idx,
                            action,
                            format!(
                                "Emit::Commit({text:?}) arrived while preedit {:?} was still visible",
                                self.buffer.preedit
                            ),
                        ));
                    }
                    self.buffer.committed.push_str(text);
                }
                Emit::DeleteSurrounding { offset, count } => {
                    if !self.state.can_delete_surrounding {
                        return Err(self.violation(
                            "DELETE_SURROUNDING_WITHOUT_CAPABILITY",
                            step_idx,
                            action,
                            "Emit::DeleteSurrounding fired while can_delete_surrounding == false"
                                .to_owned(),
                        ));
                    }
                    if *offset != -1 || *count != 1 {
                        return Err(self.violation(
                            "UNEXPECTED_DELETE_SURROUNDING_RANGE",
                            step_idx,
                            action,
                            format!("unexpected DeleteSurrounding offset={offset} count={count}"),
                        ));
                    }
                    match self.buffer.committed.pop() {
                        Some(' ') => {}
                        Some(other) => {
                            return Err(self.violation(
                                "DELETE_SURROUNDING_ATE_NON_SPACE",
                                step_idx,
                                action,
                                format!(
                                    "auto-space DeleteSurrounding deleted user character {other:?} instead of ' '"
                                ),
                            ));
                        }
                        None => {
                            return Err(self.violation(
                                "DELETE_SURROUNDING_UNDERFLOW",
                                step_idx,
                                action,
                                "auto-space DeleteSurrounding fired on an empty host buffer"
                                    .to_owned(),
                            ));
                        }
                    }
                }
                Emit::LookupTable(table) => {
                    if table.cells.is_empty() {
                        return Err(self.violation(
                            "EMPTY_LOOKUP_TABLE",
                            step_idx,
                            action,
                            "Emit::LookupTable carried zero cells".to_owned(),
                        ));
                    }
                    if (table.cursor as usize) >= table.cells.len() {
                        return Err(self.violation(
                            "LOOKUP_CURSOR_OUT_OF_BOUNDS",
                            step_idx,
                            action,
                            format!(
                                "lookup cursor {} >= cells.len() {}",
                                table.cursor,
                                table.cells.len()
                            ),
                        ));
                    }
                    self.buffer.lookup = Some(table.clone());
                }
                Emit::HideLookupTable => {
                    self.buffer.lookup = None;
                }
                Emit::ModeChanged
                | Emit::AnnounceMode
                | Emit::OpenSettings
                | Emit::ToggleTpsKeyboard
                | Emit::TpsKeyboardCapTyped(_) => {}
            }
        }
        Ok(())
    }

    fn check_post_invariants(
        &self,
        step_idx: usize,
        action: &SimStep,
    ) -> Result<(), Box<InvariantViolation>> {
        let engine_composing = is_composing(&self.runtime, self.token);
        let host_has_preedit = self.buffer.preedit.is_some();

        // Invariant 2: Engine composing state and host preedit visibility must never diverge.
        if engine_composing != host_has_preedit {
            return Err(self.violation(
                "PREEDIT_COMPOSING_DESYNC",
                step_idx,
                action,
                format!(
                    "engine is_composing={engine_composing} != host_has_preedit={host_has_preedit} (preedit={:?})",
                    self.buffer.preedit
                ),
            ));
        }

        // Invariant 3: `state.is_table_shown` and host lookup visibility must stay in lockstep.
        if self.state.is_table_shown != self.buffer.lookup.is_some() {
            return Err(self.violation(
                "LOOKUP_VISIBILITY_DESYNC",
                step_idx,
                action,
                format!(
                    "state.is_table_shown={} != host.lookup.is_some()={}",
                    self.state.is_table_shown,
                    self.buffer.lookup.is_some()
                ),
            ));
        }

        // Invariant 4: When idle with no symbol picker and no Telex guide, lookup must be hidden.
        let settings = self.runtime.settings.current();
        let guide_on_table = settings.is_telex_guide_wanted() && !self.state.draws_telex_guide;
        if !engine_composing
            && self.state.symbol_picker.is_none()
            && !guide_on_table
            && self.buffer.lookup.is_some()
        {
            return Err(self.violation(
                "STUCK_LOOKUP_WHEN_IDLE",
                step_idx,
                action,
                "lookup table remained visible while idle with no picker or Telex guide".to_owned(),
            ));
        }

        Ok(())
    }

    fn violation(
        &self,
        code: &'static str,
        step_idx: usize,
        step: &SimStep,
        message: String,
    ) -> Box<InvariantViolation> {
        let tail_chars: Vec<char> = self.buffer.committed.chars().collect();
        let start = tail_chars.len().saturating_sub(60);
        let committed_tail: String = tail_chars[start..].iter().collect();
        let lookup_summary = self.buffer.lookup.as_ref().map(|t| {
            let preview: Vec<String> = t
                .cells
                .iter()
                .take(5)
                .map(|c| match &c.annotation {
                    Some(a) => format!("{}({a})", c.text),
                    None => c.text.clone(),
                })
                .collect();
            format!(
                "len={} cursor={} top={:?}",
                t.cells.len(),
                t.cursor,
                preview
            )
        });
        Box::new(InvariantViolation {
            code,
            step_index: step_idx,
            step: step.clone(),
            message,
            committed_tail,
            preedit: self.buffer.preedit.clone(),
            lookup_summary,
        })
    }
}

fn to_nav_key(dir: NavDir) -> NavigationKey {
    match dir {
        NavDir::Left => NavigationKey::LeftArrow,
        NavDir::Right => NavigationKey::RightArrow,
        NavDir::Up => NavigationKey::UpArrow,
        NavDir::Down => NavigationKey::DownArrow,
        NavDir::PageUp => NavigationKey::PageUp,
        NavDir::PageDown => NavigationKey::PageDown,
    }
}

fn to_candidate_nav(dir: NavDir) -> CandidateNavigation {
    CandidateNavigation::from(to_nav_key(dir))
}

impl Drop for HostSimulator {
    fn drop(&mut self) {
        super::reset_transient_engine_state();
    }
}
