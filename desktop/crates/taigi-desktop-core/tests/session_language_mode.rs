//! The session paths a shell opts in to: the commit before the caret leaves,
//! and the Taigi/English switch. A binary of its own, because the engine is
//! one per process and these assert what it committed; tests take turns and
//! leave it idle.

use std::sync::{Arc, Mutex, MutexGuard};

use taigi_desktop_core::composing::{ContextToken, SystemClock};
use taigi_desktop_core::keys::{KeyEventSnapshot, KeyModifiers};
use taigi_desktop_core::platform::DesktopPlatform;
use taigi_desktop_core::runtime::{DesktopRuntime, RuntimeParts};
use taigi_desktop_core::session::{
    commit_composition, end_session, is_composing, process_key, toggle_language_mode, Emit,
    EngineState,
};
use taigi_desktop_core::settings::{SettingsDocument, StaticSettingsProvider};

const TOKEN: ContextToken = ContextToken(1);

static ENGINE: Mutex<()> = Mutex::new(());

struct Fixture {
    runtime: DesktopRuntime,
    state: EngineState,
    _turn: MutexGuard<'static, ()>,
}

impl Fixture {
    fn new() -> Self {
        let turn = ENGINE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let runtime = DesktopRuntime::new(RuntimeParts {
            settings: Arc::new(StaticSettingsProvider::new(SettingsDocument::default())),
            settings_store: None,
            data_directory: None,
            dictionaries: Box::new(|| None),
            dictionary_version: 1,
            system_locale: Box::new(|| "en-US".to_owned()),
            platform: DesktopPlatform::Linux,
            clock: Arc::new(SystemClock),
        });
        Self {
            runtime,
            state: EngineState::default(),
            _turn: turn,
        }
    }

    fn press(&mut self, snapshot: &KeyEventSnapshot) -> bool {
        process_key(&self.runtime, TOKEN, &mut self.state, snapshot).handled
    }

    fn letter(&mut self, character: &str) -> bool {
        self.press(&KeyEventSnapshot::text(character, KeyModifiers::NONE))
    }

    fn toggle(&mut self) -> Vec<Emit> {
        toggle_language_mode(&self.runtime, TOKEN, &mut self.state)
    }

    fn is_composing(&self) -> bool {
        is_composing(&self.runtime, TOKEN)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        end_session(&self.runtime, TOKEN, &mut self.state);
    }
}

#[test]
fn commit_composition_writes_the_composition_and_leaves_no_arm() {
    let mut fixture = Fixture::new();
    fixture.letter("a");
    fixture.state.armed_auto_space = true;
    // trace: no lexicon, `commit_raw` writes the raw composition "a".
    let emits = commit_composition(&fixture.runtime, TOKEN, &mut fixture.state);
    assert_eq!(
        emits,
        vec![Emit::ClearPreedit, Emit::Commit("a".to_owned())]
    );
    assert!(!fixture.is_composing());
    assert!(!fixture.state.armed_auto_space);
}

#[test]
fn commit_composition_with_nothing_composing_writes_nothing() {
    let mut fixture = Fixture::new();
    fixture.letter("a");
    fixture.toggle();
    fixture.toggle();
    fixture.state.armed_auto_space = true;
    let emits = commit_composition(&fixture.runtime, TOKEN, &mut fixture.state);
    assert_eq!(emits, Vec::new());
    assert!(!fixture.state.armed_auto_space);
}

#[test]
fn toggling_to_english_commits_and_the_next_key_is_the_documents() {
    let mut fixture = Fixture::new();
    fixture.letter("a");
    let emits = fixture.toggle();
    assert!(emits.contains(&Emit::Commit("a".to_owned())), "{emits:?}");
    assert_eq!(
        emits[emits.len() - 2..],
        [Emit::ModeChanged, Emit::AnnounceMode]
    );
    assert!(fixture.state.language_mode.is_english());
    assert!(!fixture.is_composing());

    assert!(!fixture.letter("b"));
    assert!(!fixture.is_composing());

    fixture.toggle();
    assert!(fixture.letter("b"));
    assert!(fixture.is_composing());
}

#[test]
fn english_keeps_the_host_chords_and_gives_up_the_bare_one() {
    // trace: `default_chord` — ToggleTranslateSwapped = bare "`",
    // ShowSymbolPicker = Ctrl+Alt+",".
    let mut fixture = Fixture::new();
    fixture.toggle();
    assert!(!fixture.letter("`"));
    fixture.press(&KeyEventSnapshot::text(
        ",",
        KeyModifiers::CONTROL.with(KeyModifiers::ALT),
    ));
    assert!(fixture.state.symbol_picker.is_some());
}
