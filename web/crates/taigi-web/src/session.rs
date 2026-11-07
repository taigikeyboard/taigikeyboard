//! One tab's input session: the shared `taigi_desktop_core::session` with the
//! browser's clock, its settings document and its key events. One textarea,
//! so one context token.

use std::sync::Arc;

use taigi_desktop_core::composing::{Clock, ContextToken};
use taigi_desktop_core::keys::{
    CandidateNavigation, ShiftTapTracker, ShortcutAction, VK_SHIFT_CODE,
};
use taigi_desktop_core::mode_indicator::ModeIndicator;
use taigi_desktop_core::platform::DesktopPlatform;
use taigi_desktop_core::runtime::{DesktopRuntime, RuntimeParts};
use taigi_desktop_core::session::{
    begin_session, click_from_panel, commit_composition, end_session, navigate_from_panel,
    process_key, toggle_language_mode, tps_keyboard_press, Emit, EngineState, ListSelection,
};
use taigi_desktop_core::settings::presentation::output_script_label;
use taigi_desktop_core::settings::{keys, InputMode, SettingsProvider};

use crate::key::{core_tier_snapshot, is_mac_core_tier, shift_index, BrowserKey, SHIFT_SCAN_CODES};
use crate::layout::{PageListLayout, TextRuler};
use crate::page_settings::PageSettings;
use crate::reply::{wire_emit, wire_layout, ModeText, Reply};
use crate::shortcuts::shortcuts;

const CONTEXT: ContextToken = ContextToken(1);
/// A stamp for the log only, until a page ships dictionaries of a version.
pub(crate) const DICTIONARY_VERSION: u32 = 1;

pub(crate) struct WebSession {
    runtime: DesktopRuntime,
    settings: PageSettings,
    clock: Arc<dyn Clock>,
    state: EngineState,
    shift_tap: ShiftTapTracker,
    /// Which Shift keys are down, left then right. `shiftKey` cannot say:
    /// it is true for the press of the first one too.
    shifts_down: [bool; 2],
}

impl WebSession {
    pub(crate) fn new(
        settings_json: &str,
        platform: DesktopPlatform,
        locale: String,
        clock: Arc<dyn Clock>,
        ruler: Arc<dyn TextRuler>,
    ) -> Result<Self, serde_json::Error> {
        let settings = PageSettings::from_json(settings_json)?;
        let runtime = DesktopRuntime::new(RuntimeParts {
            settings: Arc::new(settings.clone()),
            settings_store: Some(Box::new(settings.clone())),
            data_directory: None,
            // `DictionaryArtifacts::locate` needs a file system; the bytes
            // arrive after the page starts, and `Session.install` names them.
            dictionaries: Box::new(|| None),
            dictionary_version: DICTIONARY_VERSION,
            system_locale: Box::new(move || locale.clone()),
            platform,
            clock: Arc::clone(&clock),
        });
        Ok(Self {
            runtime,
            settings,
            clock,
            state: EngineState {
                // The page can reach the character before the caret.
                can_delete_surrounding: true,
                list_layout: Some(Arc::new(PageListLayout::new(ruler))),
                ..EngineState::default()
            },
            shift_tap: ShiftTapTracker::default(),
            shifts_down: [false; 2],
        })
    }

    pub(crate) fn key_down(&mut self, key: &BrowserKey) -> String {
        self.observe_press(key);
        let Some(snapshot) = key.snapshot(self.runtime.platform()) else {
            return self.reply(false, &[], None);
        };
        let platform = self.runtime.platform();
        let document = self.runtime.settings.current();
        // A chord as pressed comes first, so a stored ⌃⌘ row keeps its action.
        // The rewrite only stands for a chord that exists: ⌃⌘F and the like
        // stay the browser's.
        let as_pressed = ShortcutAction::matching(&snapshot, &document, platform);
        // The Mac app has no ⌃⌥ tier, so the page has none there either.
        if as_pressed.is_some() && is_mac_core_tier(&snapshot, platform) {
            return self.reply(false, &[], None);
        }
        let rewritten = core_tier_snapshot(&snapshot, platform)
            .filter(|_| as_pressed.is_none())
            .and_then(|rewritten| {
                let action = ShortcutAction::matching(&rewritten, &document, platform)?;
                Some((action, rewritten))
            });
        let (action, snapshot) = match rewritten {
            Some((action, rewritten)) => (Some(action), rewritten),
            None => (as_pressed, snapshot),
        };
        // A toggle here, where the desktops open a window: a repeat would flip
        // it back. The core latches only its own toggles.
        if action == Some(ShortcutAction::OpenLastSettingsPane) && key.repeat {
            return self.reply(true, &[], None);
        }
        // The event says what the wire elsewhere cannot: this is a new press,
        // so a held chord's latch is over even if its release went to another
        // window.
        if !key.repeat {
            self.state.latched_chord = None;
        }
        let reply = process_key(&self.runtime, CONTEXT, &mut self.state, &snapshot);
        let announced = action
            .filter(|_| reply.emits.contains(&Emit::AnnounceMode))
            .and_then(|action| self.switched_label(action));
        self.reply(reply.handled, &reply.emits, announced)
    }

    /// What a chord just switched, and only that: the macOS and Windows flash
    /// (`TaigiInputController.performShortcutAction`, `perform_global`), not
    /// the whole mode line Linux shows in its panel (`mode_label`).
    fn switched_label(&self, action: ShortcutAction) -> Option<String> {
        let document = self.runtime.settings.current();
        let key = match action {
            ShortcutAction::ToggleRomanization | ShortcutAction::ToggleTps => {
                document.choice::<InputMode>(&keys::INPUT_MODE).label_key()
            }
            ShortcutAction::ToggleTranslateSwapped => {
                output_script_label(document.bool(&keys::IS_HANJI_FIRST))
            }
            ShortcutAction::CycleCandidateDisplayMode => document
                .engine_settings()
                .candidate_display_mode
                .label_key(),
            ShortcutAction::OpenLastSettingsPane
            | ShortcutAction::ShowSymbolPicker
            | ShortcutAction::ShowTelexGuide
            | ShortcutAction::ShowTpsKeyboard => return None,
        };
        Some(self.runtime.strings().resolve(key).to_owned())
    }

    /// The release: it ends a held chord, and may complete a Shift tap. Only
    /// a Shift release reaches the tracker: another key let go during the tap
    /// was pressed before it, and on Windows that release is never delivered
    /// (`OnTestKeyUp` answers false).
    pub(crate) fn key_up(&mut self, key: &BrowserKey) -> String {
        self.state.latched_chord = None;
        let Some(index) = shift_index(&key.code) else {
            return self.reply(false, &[], None);
        };
        self.shifts_down[index] = false;
        let now = self.now();
        let is_tap =
            self.shift_tap
                .take_tap_on_release(VK_SHIFT_CODE, SHIFT_SCAN_CODES[index], now);
        if !is_tap {
            return self.reply(false, &[], None);
        }
        let emits = toggle_language_mode(&self.runtime, CONTEXT, &mut self.state);
        let label = self
            .runtime
            .strings()
            .resolve(self.state.language_mode.flash_label_key())
            .to_owned();
        self.reply(true, &emits, Some(label))
    }

    pub(crate) fn click_candidate(&mut self, position: usize) -> String {
        let emits = click_from_panel(&self.runtime, &mut self.state, position);
        self.reply(true, &emits, None)
    }

    /// A click on the panel's page arrows.
    pub(crate) fn turn_page(&mut self, is_forward: bool) -> String {
        let direction = if is_forward {
            CandidateNavigation::PageDown
        } else {
            CandidateNavigation::PageUp
        };
        let emits = navigate_from_panel(&self.runtime, CONTEXT, &mut self.state, direction);
        self.reply(true, &emits, None)
    }

    pub(crate) fn viewport_scrolled(&mut self, offset: f32) {
        if let Some(model) = self.selection_mut().expandable_mut() {
            model.on_viewport_scrolled(offset);
        }
    }

    pub(crate) fn commit_composition(&mut self) -> String {
        let emits = commit_composition(&self.runtime, CONTEXT, &mut self.state);
        self.reply(true, &emits, None)
    }

    /// The host's text field took the composition as it stood (ChromeOS
    /// confirms the preedit itself on blur, as Linux frameworks do), so the
    /// engine drops its copy without a commit.
    pub(crate) fn end_session(&mut self) -> String {
        let emits = end_session(&self.runtime, CONTEXT, &mut self.state);
        self.reply(true, &emits, None)
    }

    /// A text field gained focus: the guide a mode keeps on screen comes back.
    pub(crate) fn begin_session(&mut self) -> String {
        let emits = begin_session(&self.runtime, &mut self.state);
        self.reply(true, &emits, None)
    }

    /// A click on a cap of the TPS key panel, naming the glyph it types.
    pub(crate) fn tps_key_press(&mut self, glyph: &str) -> String {
        let emits = tps_keyboard_press(&self.runtime, CONTEXT, &mut self.state, glyph);
        self.reply(true, &emits, None)
    }

    /// The caret or the text moved by anything but this page's own write, or
    /// a pointer went down. The auto-space swap deletes the character before
    /// the caret unseen, so the arm cannot outlive the caret it was set at. A
    /// Shift-click is a click, not a tap.
    pub(crate) fn caret_moved(&mut self) {
        self.state.armed_auto_space = false;
        self.shift_tap.clear();
    }

    /// The window lost the keyboard: a key held across that is released
    /// somewhere else.
    pub(crate) fn focus_lost(&mut self) {
        self.state.latched_chord = None;
        self.shift_tap.clear();
        self.shifts_down = [false; 2];
    }

    /// The sidebar's Shortcuts section under the current document.
    pub(crate) fn shortcuts(&self) -> String {
        let wire = shortcuts(
            &self.settings.current(),
            &self.runtime.strings(),
            self.runtime.platform(),
        );
        serde_json::to_string(&wire).expect("plain strings")
    }

    pub(crate) fn set_settings(&self, json: &str) -> Result<(), serde_json::Error> {
        self.settings.replace_from_json(json)
    }

    /// The selection over the list in view.
    fn selection(&self) -> &ListSelection {
        match &self.state.symbol_picker {
            Some(picker) => &picker.selection,
            None => &self.state.selection,
        }
    }

    fn selection_mut(&mut self) -> &mut ListSelection {
        match &mut self.state.symbol_picker {
            Some(picker) => &mut picker.selection,
            None => &mut self.state.selection,
        }
    }

    /// The indicator as things stand.
    pub(crate) fn mode_symbol(&self) -> &'static str {
        let settings = self.runtime.settings.current();
        ModeIndicator::of(&settings.engine_settings(), self.state.language_mode).symbol()
    }

    /// Every press reaches the tracker, before anything can give up on the
    /// key: the tap is made of a key the classifier never sees.
    fn observe_press(&mut self, key: &BrowserKey) {
        let shift = shift_index(&key.code);
        // Any other key is `(0, 0)`, which only has to differ from Shift.
        let (virtual_key, scan_code) =
            shift.map_or((0, 0), |index| (VK_SHIFT_CODE, SHIFT_SCAN_CODES[index]));
        let is_other_shift_down = shift.is_some_and(|index| self.shifts_down[1 - index]);
        let is_other_modifier_held = key.ctrl || key.alt || key.meta || is_other_shift_down;
        if let Some(index) = shift {
            self.shifts_down[index] = true;
        }
        let now = self.now();
        self.shift_tap.observe_key_down(
            virtual_key,
            scan_code,
            key.repeat,
            is_other_modifier_held,
            now,
        );
    }

    fn now(&self) -> u64 {
        u64::try_from(self.clock.now_ms()).unwrap_or(0)
    }

    /// `announced` is what the flash says, for a reply that announces.
    fn reply(&self, handled: bool, emits: &[Emit], announced: Option<String>) -> String {
        let mode = ModeText {
            symbol: self.mode_symbol(),
            label: announced.unwrap_or_default(),
        };
        // One reply can carry two tables (a picker going down, then the list),
        // and the model belongs to the one left on screen. The Telex guide
        // has no highlight and no model.
        let laid_out = emits
            .iter()
            .rposition(|emit| matches!(emit, Emit::LookupTable(table) if table.cursor_visible));
        let reply = Reply {
            handled,
            emits: emits
                .iter()
                .enumerate()
                .map(|(index, emit)| {
                    let layout = if laid_out == Some(index) {
                        wire_layout(self.selection())
                    } else {
                        None
                    };
                    wire_emit(emit, &mode, layout)
                })
                .collect(),
            settings: self.settings.take_unsent(),
        };
        // Strings and numbers only: this cannot fail.
        serde_json::to_string(&reply).unwrap_or_else(|_| String::from("{}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use std::ops::{Deref, DerefMut};
    use std::sync::atomic::{AtomicI64, Ordering};
    use std::sync::{Mutex, MutexGuard};
    use taigi_desktop_core::session::end_session;
    use taigi_desktop_core::settings::keys;
    use taigi_desktop_core::settings::{InputMode, SettingsProvider};

    use crate::layout::Measured;

    /// A clock the test moves: the tap window is what is under test.
    #[derive(Default)]
    struct TestClock(AtomicI64);

    impl Clock for TestClock {
        fn now_ms(&self) -> i64 {
            self.0.load(Ordering::Relaxed)
        }
    }

    /// The default layout never measures.
    #[derive(Debug)]
    struct NoRuler;

    impl TextRuler for NoRuler {
        fn measure(&self, _runs: &[(&str, f32)]) -> Option<Measured> {
            None
        }
    }

    /// The engine is one per process, whichever session drives it: tests take
    /// turns, and each leaves it idle for the next.
    static ENGINE: Mutex<()> = Mutex::new(());

    struct Fixture {
        session: WebSession,
        _turn: MutexGuard<'static, ()>,
    }

    impl Deref for Fixture {
        type Target = WebSession;

        fn deref(&self) -> &WebSession {
            &self.session
        }
    }

    impl DerefMut for Fixture {
        fn deref_mut(&mut self) -> &mut WebSession {
            &mut self.session
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let session = &mut self.session;
            end_session(&session.runtime, CONTEXT, &mut session.state);
        }
    }

    fn session_on(clock: Arc<TestClock>) -> Fixture {
        session_for(DesktopPlatform::Linux, clock)
    }

    fn session_for(platform: DesktopPlatform, clock: Arc<TestClock>) -> Fixture {
        let turn = ENGINE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let session =
            WebSession::new("{}", platform, "en".to_owned(), clock, Arc::new(NoRuler)).unwrap();
        Fixture {
            session,
            _turn: turn,
        }
    }

    fn session() -> Fixture {
        session_on(Arc::default())
    }

    fn press(key: &str, code: &str) -> BrowserKey {
        BrowserKey {
            key: key.to_owned(),
            code: code.to_owned(),
            ..BrowserKey::default()
        }
    }

    fn chord(key: &str, code: &str) -> BrowserKey {
        BrowserKey {
            ctrl: true,
            alt: true,
            ..press(key, code)
        }
    }

    fn parsed(reply: String) -> Value {
        serde_json::from_str(&reply).unwrap()
    }

    fn emit_types(reply: &Value) -> Vec<&str> {
        reply["emits"]
            .as_array()
            .unwrap()
            .iter()
            .map(|emit| emit["type"].as_str().unwrap())
            .collect()
    }

    fn shift_tap(session: &mut WebSession) -> Value {
        let shift = BrowserKey {
            shift: true,
            ..press("Shift", "ShiftLeft")
        };
        session.key_down(&shift);
        parsed(session.key_up(&press("Shift", "ShiftLeft")))
    }

    #[test]
    fn a_letter_composes_and_a_modifier_is_nothing() {
        let mut session = session();
        let reply = parsed(session.key_down(&press("a", "KeyA")));
        assert_eq!(reply["handled"], true);
        assert_eq!(reply["emits"][0]["type"], "preedit");
        assert_eq!(reply["emits"][0]["text"], "a");
        let reply = parsed(session.key_down(&press("Control", "ControlLeft")));
        assert_eq!(reply["handled"], false);
        assert_eq!(emit_types(&reply), Vec::<&str>::new());
    }

    #[test]
    fn a_shift_tap_switches_to_english_and_says_so() {
        // trace: `flash_label_key` → SettingsEnglishMode, en "English";
        // `ModeIndicator::English` symbol 英.
        let mut session = session();
        session.key_down(&press("a", "KeyA"));
        let reply = shift_tap(&mut session);
        let types = emit_types(&reply);
        assert!(types.contains(&"commit"), "{types:?}");
        let emits = reply["emits"].as_array().unwrap();
        let mode = &emits[emits.len() - 2];
        assert_eq!(mode["symbol"], "英");
        let announced = &emits[emits.len() - 1];
        assert_eq!(announced["type"], "announceMode");
        assert!(!announced["label"].as_str().unwrap().contains(" · "));
        assert_eq!(
            parsed(session.key_down(&press("b", "KeyB")))["handled"],
            false
        );
    }

    #[test]
    fn a_key_released_during_the_tap_leaves_it_a_tap() {
        // trace: a↓ Shift↓ a↑ Shift↑, typing rollover. Only a press
        // disqualifies (`shift_tap.rs`), and Windows never delivers the `a↑`.
        let mut session = session();
        session.key_down(&press("a", "KeyA"));
        session.key_down(&press("Shift", "ShiftLeft"));
        session.key_up(&press("a", "KeyA"));
        session.key_up(&press("Shift", "ShiftLeft"));
        assert!(session.state.language_mode.is_english());
    }

    #[test]
    fn the_symbol_follows_the_settings_and_the_language() {
        // trace: `ModeIndicator::symbol`: Tps 方, English 英.
        let mut session = typing_tps();
        assert_eq!(session.mode_symbol(), "方");
        shift_tap(&mut session);
        assert_eq!(session.mode_symbol(), "英");
    }

    #[test]
    fn shift_held_past_the_window_is_no_tap() {
        let clock = Arc::new(TestClock::default());
        let mut session = session_on(Arc::clone(&clock));
        session.key_down(&press("Shift", "ShiftLeft"));
        clock.0.store(500, Ordering::Relaxed);
        let reply = parsed(session.key_up(&press("Shift", "ShiftLeft")));
        assert_eq!(reply["handled"], false);
        assert!(!session.state.language_mode.is_english());
    }

    #[test]
    fn a_shift_under_a_chord_or_a_held_shift_is_no_tap() {
        let mut session = session();
        session.key_down(&press("Control", "ControlLeft"));
        session.key_down(&BrowserKey {
            ctrl: true,
            shift: true,
            ..press("Shift", "ShiftLeft")
        });
        session.key_up(&press("Shift", "ShiftLeft"));
        assert!(!session.state.language_mode.is_english());

        // Left held, a letter clears the tracker, right tapped.
        session.key_down(&press("Shift", "ShiftLeft"));
        session.key_down(&press("A", "KeyA"));
        session.key_down(&press("Shift", "ShiftRight"));
        session.key_up(&press("Shift", "ShiftRight"));
        assert!(!session.state.language_mode.is_english());
    }

    #[test]
    fn a_shift_click_is_no_tap() {
        let mut session = session();
        session.key_down(&press("Shift", "ShiftLeft"));
        session.caret_moved();
        session.key_up(&press("Shift", "ShiftLeft"));
        assert!(!session.state.language_mode.is_english());
    }

    #[test]
    fn the_window_taking_the_keyboard_forgets_what_was_held() {
        let mut session = session();
        session.key_down(&press("Shift", "ShiftLeft"));
        session.focus_lost();
        session.key_up(&press("Shift", "ShiftLeft"));
        assert!(!session.state.language_mode.is_english());
        assert_eq!(session.shifts_down, [false; 2]);
    }

    #[test]
    fn a_chord_that_writes_hands_the_document_back_once() {
        // trace: Ctrl+Alt+C = ToggleRomanization; TL → POJ.
        let mut session = session();
        let reply = parsed(session.key_down(&chord("c", "KeyC")));
        assert_eq!(reply["handled"], true);
        let document = reply["settings"].as_str().expect("the written document");
        assert!(document.contains("poj"), "{document}");
        let types = emit_types(&reply);
        assert_eq!(types.last(), Some(&"announceMode"));
        session.key_up(&chord("c", "KeyC"));
        assert!(parsed(session.key_down(&press("a", "KeyA")))
            .get("settings")
            .is_none());
    }

    fn announced(reply: &Value) -> &str {
        let emits = reply["emits"].as_array().unwrap();
        let last = &emits[emits.len() - 1];
        assert_eq!(last["type"], "announceMode");
        last["label"].as_str().unwrap()
    }

    #[test]
    fn a_chord_announces_only_what_it_switched() {
        // trace: `label_key`, en: POJ "Pe̍h-ōe-jī"; ` under side by side,
        // stored swap true → false = SettingsOutputScriptRoman; Ctrl+Alt+H from
        // side by side = the next display mode, resolved the same way.
        let mut session = session();
        let strings = session.runtime.strings();
        let romanization = strings.resolve(InputMode::Poj.label_key()).to_owned();
        let output_script = strings.resolve(output_script_label(false)).to_owned();
        let reply = parsed(session.key_down(&chord("c", "KeyC")));
        assert_eq!(announced(&reply), romanization);
        session.key_up(&chord("c", "KeyC"));

        let reply = parsed(session.key_down(&press("`", "Backquote")));
        assert_eq!(announced(&reply), output_script);
        session.key_up(&press("`", "Backquote"));

        let reply = parsed(session.key_down(&chord("h", "KeyH")));
        let display_mode = session
            .runtime
            .settings
            .current()
            .engine_settings()
            .candidate_display_mode;
        let expected = session.runtime.strings().resolve(display_mode.label_key());
        assert_eq!(announced(&reply), expected);
    }

    #[test]
    fn the_mac_app_chord_fires_and_a_browser_chord_stays_the_browsers() {
        // trace: Ctrl+Cmd+C → ⌃⌥C in the core tier = ToggleRomanization; TL →
        // POJ. Ctrl+Cmd+F matches nothing, so it goes out as it came.
        let mut session = session_for(DesktopPlatform::MacOS, Arc::default());
        let app_chord = |key: &str, code: &str| BrowserKey {
            ctrl: true,
            meta: true,
            ..press(key, code)
        };
        let reply = parsed(session.key_down(&app_chord("c", "KeyC")));
        assert_eq!(reply["handled"], true);
        assert_eq!(emit_types(&reply).last(), Some(&"announceMode"));
        session.key_up(&app_chord("c", "KeyC"));
        let reply = parsed(session.key_down(&app_chord("f", "KeyF")));
        assert_eq!(reply["handled"], false);
    }

    #[test]
    fn a_stored_control_command_row_outranks_the_rewrite() {
        // trace: raw `dc|0063` = ⌃⌘C on the Mac (`modifier_letters` d, c), on
        // ToggleTps; the rewrite would have read ⌃⌥C = ToggleRomanization.
        let mut session = session_for(DesktopPlatform::MacOS, Arc::default());
        session
            .settings
            .replace_from_json(r#"{"values":{"globalShortcut.toggleTps":"dc|0063"}}"#)
            .unwrap();
        let press = BrowserKey {
            ctrl: true,
            meta: true,
            ..press("c", "KeyC")
        };
        session.key_down(&press);
        let document = session.runtime.settings.current();
        assert_eq!(
            document.choice::<InputMode>(&keys::INPUT_MODE),
            InputMode::Tps
        );
    }

    #[test]
    fn a_held_toggle_fires_once_and_a_new_press_fires_again() {
        // trace: Ctrl+Alt+/ = ShowTelexGuide, `fires_once_per_press`.
        let mut session = session();
        let guide = chord("/", "Slash");
        assert_eq!(
            emit_types(&parsed(session.key_down(&guide))),
            ["lookupTable"]
        );
        let repeat = BrowserKey {
            repeat: true,
            ..guide.clone()
        };
        assert_eq!(parsed(session.key_down(&repeat))["handled"], true);
        assert_eq!(
            emit_types(&parsed(session.key_down(&repeat))),
            Vec::<&str>::new()
        );
        // Its release went to another window: this press is still a new one.
        assert_ne!(
            emit_types(&parsed(session.key_down(&guide))),
            Vec::<&str>::new()
        );
    }

    #[test]
    fn the_settings_chord_asks_for_the_sidebar_once_per_press() {
        let mut session = session();
        let reply = parsed(session.key_down(&chord("s", "KeyS")));
        assert_eq!(reply["handled"], true);
        assert_eq!(emit_types(&reply), ["openSettings"]);
        let held = BrowserKey {
            repeat: true,
            ..chord("s", "KeyS")
        };
        let reply = parsed(session.key_down(&held));
        assert_eq!(reply["handled"], true);
        assert_eq!(emit_types(&reply), Vec::<&str>::new());
    }

    #[test]
    fn the_core_tier_does_nothing_on_a_mac() {
        // trace: ⌃⌥C reports `ç`, read back as `c` = ToggleRomanization in the
        // core's table; the Mac app has only ⌃⌘C.
        let mut session = session_for(DesktopPlatform::MacOS, Arc::default());
        let reply = parsed(session.key_down(&chord("ç", "KeyC")));
        assert_eq!(reply["handled"], false);
        assert_eq!(emit_types(&reply), Vec::<&str>::new());
        let document = session.runtime.settings.current();
        assert_eq!(
            document.choice::<InputMode>(&keys::INPUT_MODE),
            InputMode::Tl
        );
    }

    fn typing_tps() -> Fixture {
        let session = session();
        session
            .set_settings(r#"{"values":{"inputMode":"tps"}}"#)
            .unwrap();
        session
    }

    #[test]
    fn a_key_cap_click_types_its_glyph_and_lights_its_cap() {
        // trace: tps_keyboard_rows ROWS[0] "1234567890-=", '8' → ㄚ at cap 7.
        let mut session = typing_tps();
        let reply = parsed(session.tps_key_press("ㄚ"));
        let emits = reply["emits"].as_array().unwrap();
        let lit = emits
            .iter()
            .find(|emit| emit["type"] == "tpsKeyboardCapTyped")
            .expect("the cap lights");
        assert_eq!((&lit["row"], &lit["cap"]), (&json!(0), &json!(7)));
        assert!(emit_types(&reply).contains(&"preedit"), "{reply}");
    }

    #[test]
    fn a_typed_key_lights_the_same_cap_a_click_does() {
        let mut session = typing_tps();
        let reply = parsed(session.key_down(&press("8", "Digit8")));
        assert!(
            emit_types(&reply).contains(&"tpsKeyboardCapTyped"),
            "{reply}"
        );
    }

    #[test]
    fn a_key_cap_click_outside_tps_does_nothing() {
        let reply = parsed(session().tps_key_press("ㄚ"));
        assert_eq!(emit_types(&reply), Vec::<&str>::new());
    }

    #[test]
    fn a_key_cap_click_off_the_layout_does_nothing() {
        let reply = parsed(typing_tps().tps_key_press("a"));
        assert_eq!(emit_types(&reply), Vec::<&str>::new());
    }

    #[test]
    fn a_key_cap_click_in_english_mode_does_nothing() {
        let mut session = typing_tps();
        let typist = &mut *session;
        toggle_language_mode(&typist.runtime, CONTEXT, &mut typist.state);
        let reply = parsed(session.tps_key_press("ㄚ"));
        assert_eq!(emit_types(&reply), Vec::<&str>::new());
    }

    #[test]
    fn a_key_cap_click_takes_the_picker_down_first() {
        let mut session = typing_tps();
        session.key_down(&chord(",", "Comma"));
        assert!(session.state.symbol_picker.is_some());
        let reply = parsed(session.tps_key_press("ㄚ"));
        assert!(session.state.symbol_picker.is_none());
        assert_eq!(emit_types(&reply).first(), Some(&"hideLookupTable"));
        assert!(emit_types(&reply).contains(&"preedit"), "{reply}");
    }

    #[test]
    fn a_click_with_no_list_up_does_nothing() {
        let reply = parsed(session().click_candidate(0));
        assert_eq!(emit_types(&reply), Vec::<&str>::new());
    }

    #[test]
    fn a_page_arrow_with_no_list_up_does_nothing() {
        let reply = parsed(session().turn_page(true));
        assert_eq!(emit_types(&reply), Vec::<&str>::new());
    }

    #[test]
    fn the_caret_moving_spends_the_auto_space_arm() {
        let mut session = session();
        session.state.armed_auto_space = true;
        session.caret_moved();
        assert!(!session.state.armed_auto_space);
    }

    #[test]
    fn commit_composition_answers_the_commit() {
        let mut session = session();
        session.key_down(&press("a", "KeyA"));
        let reply = parsed(session.commit_composition());
        assert_eq!(emit_types(&reply), ["clearPreedit", "commit"]);
        assert_eq!(
            emit_types(&parsed(session.commit_composition())),
            Vec::<&str>::new()
        );
    }

    #[test]
    fn end_session_drops_the_composition_without_a_commit() {
        let mut session = session();
        session.key_down(&press("a", "KeyA"));
        let ended = parsed(session.end_session());
        assert!(!emit_types(&ended).contains(&"commit"));
        let reply = parsed(session.key_down(&press("b", "KeyB")));
        let preedit = reply["emits"]
            .as_array()
            .unwrap()
            .iter()
            .find(|emit| emit["type"] == "preedit")
            .unwrap();
        // trace: "a" cancelled by end_session, so "b" opens a new composition.
        assert_eq!(preedit["text"], "b");
    }

    #[test]
    fn begin_session_answers_with_no_commit() {
        let mut session = session();
        let reply = parsed(session.begin_session());
        assert!(!emit_types(&reply).contains(&"commit"));
    }

    #[test]
    fn the_pages_settings_are_read_at_the_next_key() {
        let session = session();
        session
            .set_settings(r#"{"values":{"inputMode":"poj"}}"#)
            .unwrap();
        let settings = session.runtime.settings.current();
        assert_eq!(settings.choice(&keys::INPUT_MODE), InputMode::Poj);
        assert!(session.set_settings("{").is_err());
    }

    #[test]
    fn a_malformed_document_is_refused_at_the_start() {
        let clock: Arc<TestClock> = Arc::default();
        let ruler = Arc::new(NoRuler);
        assert!(
            WebSession::new("{", DesktopPlatform::Linux, "en".to_owned(), clock, ruler).is_err()
        );
    }
}
