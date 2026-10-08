//! What one key does through the whole Linux session path —
//! `process_raw_key` over a runtime on the repo's real dictionaries — pinned
//! before the key-intent executor moves into `taigi-desktop-core`
//! (maintainability roadmap R3(c)). Every expected value here was read by
//! running HEAD, not derived.
//!
//! Hermetic: nothing is learned (the data directory cannot be created), so a
//! commit never reorders a later test's list; and the tests take one lock,
//! because the lexicon and the engine's user data are one per process.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use taigi_desktop_core::composing::ContextToken;
use taigi_desktop_core::settings::keys;
use taigi_desktop_storage::SettingsFileStore;
use taigi_linux_core::session::{end_session, process_raw_key, EngineState, CAP_SURROUNDING_TEXT};
use taigi_linux_core::{Emit, LookupTableContent, Runtime};
use taigi_linux_platform::key_translation::state;
use taigi_linux_platform::{RawKeyEvent, UserDirectories};

const SPACE: u32 = 0x20;
const COMMA: u32 = 0x2c;
const PERIOD: u32 = 0x2e;
const RETURN: u32 = 0xff0d;
const ESCAPE: u32 = 0xff1b;
const BACKSPACE: u32 = 0xff08;

/// A session over the real dictionaries with auto-space on or off, and a
/// client that can (or cannot) delete surrounding text.
struct Session {
    _directory: tempfile::TempDir,
    runtime: Runtime,
    token: ContextToken,
    state: EngineState,
}

/// Ends the composition like a focus-out, so the engine singleton carries
/// nothing into the next test.
impl Drop for Session {
    fn drop(&mut self) {
        end_session(&self.runtime, self.token, &mut self.state);
    }
}

/// One test at a time: the lexicon and the user data are one per process.
/// Held for the whole test, so a test may open several sessions.
fn serial() -> MutexGuard<'static, ()> {
    static SERIAL: Mutex<()> = Mutex::new(());
    SERIAL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl Session {
    fn new(is_auto_space_enabled: bool, can_delete_surrounding: bool) -> Self {
        let directory = tempfile::tempdir().expect("tempdir");
        let config = directory.path().join("config");
        std::fs::create_dir_all(&config).expect("config directory");
        SettingsFileStore::new(&config)
            .update(|document| {
                document.set_bool(&keys::IS_AUTO_SPACE_ENABLED, is_auto_space_enabled);
                // These characterisations are written around the §34 literal
                // leading the list; Show Typed Text First ships OFF since
                // 2026-10-02, so it is pinned ON here.
                document.set_bool(&keys::IS_LITERAL_ROMAN_CANDIDATE_ENABLED, true);
            })
            .expect("settings written");
        // A data directory under a FILE cannot be created: no learning.
        let blocker = directory.path().join("not-a-directory");
        std::fs::write(&blocker, "").expect("blocker file");
        let runtime = Runtime::from_directories(
            Some(UserDirectories {
                config,
                data: blocker.join("data"),
            }),
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../assets/dictionaries"),
        );
        let state = EngineState {
            capabilities: if can_delete_surrounding {
                CAP_SURROUNDING_TEXT
            } else {
                0
            },
            ..EngineState::default()
        };
        let token = runtime.allocate_token();
        Self {
            _directory: directory,
            runtime,
            token,
            state,
        }
    }

    fn press_with(&mut self, keyval: u32, modifiers: u32) -> (bool, Vec<Emit>) {
        self.press_key(keyval, 0, modifiers)
    }

    fn press_key(&mut self, keyval: u32, keycode: u32, modifiers: u32) -> (bool, Vec<Emit>) {
        let reply = process_raw_key(
            &self.runtime,
            self.token,
            &mut self.state,
            RawKeyEvent {
                keyval,
                keycode,
                state: modifiers,
            },
        );
        (reply.handled, reply.emits)
    }

    /// A consumed key's emits.
    fn press(&mut self, keyval: u32) -> Vec<Emit> {
        let (is_handled, emits) = self.press_with(keyval, 0);
        assert!(is_handled, "key {keyval:#x} is consumed");
        emits
    }

    /// Types `letters` into a composition and answers the last key's list.
    fn type_word(&mut self, letters: &str) -> LookupTableContent {
        let mut last = Vec::new();
        for letter in letters.chars() {
            last = self.press(letter as u32);
        }
        match last.as_slice() {
            [Emit::Preedit { text, .. }, Emit::LookupTable(table)] => {
                assert_eq!(text, letters);
                table.clone()
            }
            other => panic!("typing shows the preedit and its list, got {other:?}"),
        }
    }
}

fn commit(text: &str) -> Emit {
    Emit::Commit(text.to_owned())
}

#[test]
fn typing_shows_the_preedit_and_a_labelled_list_led_by_the_literal() {
    let _serial = serial();
    let mut session = Session::new(false, false);
    let table = session.type_word("ho");
    // trace: cell 0 is the literal roman; the first hanji cell is 好 hó; the
    // literal takes no key (§34, as on macOS / Windows), so the Standard slot
    // keys label the page from cell 1; vertical, cursor on the literal.
    assert_eq!(&table.candidates[..2], ["ho", "好 hó"]);
    assert_eq!(table.labels, ["", "q", "w", "d", "f", "z", "x", "v", "y"]);
    assert_eq!((table.cursor, table.cursor_visible), (0, true));
    assert_eq!((table.page_size, table.vertical), (9, true));
}

#[test]
fn a_slot_key_commits_its_cell_and_closes_the_list() {
    let _serial = serial();
    for is_auto_space_enabled in [false, true] {
        let mut session = Session::new(is_auto_space_enabled, true);
        session.type_word("ho");
        // The first slot key picks the first dictionary cell, not the literal.
        assert_eq!(
            session.press('q' as u32),
            [Emit::ClearPreedit, commit("好"), Emit::HideLookupTable],
            "auto_space={is_auto_space_enabled}: a Hanji commit takes no space"
        );
        assert!(!session.state.armed_auto_space);
    }
}

#[test]
fn return_commits_the_literal_and_auto_space_follows_a_romanization() {
    let _serial = serial();
    let mut session = Session::new(false, true);
    session.type_word("ho");
    assert_eq!(
        session.press(RETURN),
        [Emit::ClearPreedit, commit("ho"), Emit::HideLookupTable]
    );
    assert!(!session.state.armed_auto_space);

    let mut session = Session::new(true, true);
    session.type_word("ho");
    assert_eq!(
        session.press(RETURN),
        [
            Emit::ClearPreedit,
            commit("ho"),
            commit(" "),
            Emit::HideLookupTable
        ]
    );
    assert!(session.state.armed_auto_space, "the space is swappable");
}

#[test]
fn escape_abandons_the_composition() {
    let _serial = serial();
    let mut session = Session::new(true, true);
    session.type_word("ho");
    assert_eq!(
        session.press(ESCAPE),
        [Emit::ClearPreedit, Emit::HideLookupTable]
    );
    assert!(!session.state.armed_auto_space);
}

#[test]
fn space_inside_a_composition_only_redraws_the_list() {
    let _serial = serial();
    let mut session = Session::new(true, true);
    let typed = session.type_word("tsiah");
    assert_eq!(session.press(SPACE), [Emit::LookupTable(typed)]);
    assert!(!session.state.armed_auto_space);
}

#[test]
fn punctuation_inside_a_composition_commits_the_literal_with_the_full_width_mark() {
    let _serial = serial();
    let mut session = Session::new(false, true);
    session.type_word("tsiah");
    assert_eq!(
        session.press(COMMA),
        [Emit::ClearPreedit, commit("tsiah，"), Emit::HideLookupTable]
    );
    assert!(!session.state.armed_auto_space);

    let mut session = Session::new(true, true);
    session.type_word("tsiah");
    assert_eq!(
        session.press(COMMA),
        [
            Emit::ClearPreedit,
            commit("tsiah， "),
            Emit::HideLookupTable
        ]
    );
    assert!(session.state.armed_auto_space);
}

#[test]
fn bare_punctuation_after_hanji_is_full_width() {
    let _serial = serial();
    let mut session = Session::new(true, true);
    session.type_word("ho");
    session.press('w' as u32);
    assert_eq!(session.press(PERIOD), [commit("。")]);
}

#[test]
fn the_auto_space_swap_needs_the_surrounding_text_capability() {
    let _serial = serial();
    // Without it the space stays and the mark is typed after it, full width.
    let mut session = Session::new(true, false);
    session.type_word("ho");
    session.press(RETURN);
    assert!(session.state.armed_auto_space);
    assert_eq!(session.press(COMMA), [commit("，")]);
    assert!(!session.state.armed_auto_space);

    // With it the space goes and the mark takes its half-width romanized form
    // plus the space.
    let mut session = Session::new(true, true);
    session.type_word("ho");
    session.press(RETURN);
    assert_eq!(
        session.press(COMMA),
        [
            Emit::DeleteSurrounding {
                offset: -1,
                count: 1
            },
            commit(", ")
        ]
    );
    // trace (read by running HEAD): the swapped mark ends in a space of its
    // own, so the next mark may swap again.
    assert!(session.state.armed_auto_space);
}

#[test]
fn the_armed_swap_lasts_exactly_one_key() {
    let _serial = serial();
    let mut session = Session::new(true, true);
    session.type_word("ho");
    session.press(RETURN);
    assert!(session.state.armed_auto_space);
    session.press('x' as u32);
    assert!(!session.state.armed_auto_space, "any key spends the arm");
    session.press(ESCAPE);
    assert_eq!(session.press(COMMA), [commit("，")], "no swap two keys on");
}

#[test]
fn the_symbol_picker_neither_swaps_nor_keeps_the_arm() {
    let _serial = serial();
    let mut session = Session::new(true, true);
    session.type_word("ho");
    session.press(RETURN);
    assert!(session.state.armed_auto_space);
    let (is_handled, emits) = session.press_with(COMMA, state::CONTROL | state::MOD1);
    assert!(is_handled);
    let [Emit::LookupTable(picker)] = emits.as_slice() else {
        panic!("Ctrl+Alt+, opens the picker, got {emits:?}");
    };
    let first = picker.candidates[0].clone();
    assert_eq!(
        session.press(RETURN),
        [commit(&first), Emit::HideLookupTable]
    );
    assert!(!session.state.armed_auto_space);
}

#[test]
fn backspace_with_nothing_composed_is_the_clients() {
    let _serial = serial();
    let mut session = Session::new(true, true);
    assert_eq!(session.press_with(BACKSPACE, 0), (false, Vec::new()));
}

#[test]
fn a_refetching_switch_obeys_the_candidate_window_setting() {
    let _serial = serial();
    // trace (read by running): Ctrl+Alt+H = CycleCandidateDisplayMode, which
    // re-fetches the open list under the new mode.
    let mut session = Session::new(true, true);
    session.type_word("ho");
    let (is_handled, emits) = session.press_with('h' as u32, state::CONTROL | state::MOD1);
    assert!(is_handled);
    assert!(
        matches!(emits.first(), Some(Emit::LookupTable(_))),
        "window on: the refetched list is shown, got {emits:?}"
    );

    // The window switched off (in the settings window) while the list was
    // up: the next refetch fetches nothing and takes the list down.
    drop(session);
    let mut session = Session::new(true, true);
    session.type_word("ho");
    let config = session._directory.path().join("config");
    SettingsFileStore::new(&config)
        .update(|document| document.set_bool(&keys::IS_CANDIDATE_WINDOW_ENABLED, false))
        .expect("settings written");
    let (is_handled, emits) = session.press_with('h' as u32, state::CONTROL | state::MOD1);
    assert!(is_handled);
    assert_eq!(
        emits,
        [Emit::HideLookupTable, Emit::ModeChanged, Emit::AnnounceMode]
    );
    assert!(session.state.candidates.is_empty());
}

#[test]
fn the_literal_stays_unkeyed_when_a_switch_presents_the_list_again() {
    let _serial = serial();
    // Ctrl+Alt+H re-fetches and re-presents the open list outside the key
    // path that builds a list; the literal must still take no key there.
    let mut session = Session::new(false, false);
    session.type_word("ho");
    let (_, emits) = session.press_with('h' as u32, state::CONTROL | state::MOD1);
    let Some(Emit::LookupTable(table)) = emits.first() else {
        panic!("the refetched list is shown, got {emits:?}");
    };
    assert_eq!(table.candidates[0], "ho");
    assert_eq!(table.labels[..2], ["", "q"]);

    // Show Typed Text First OFF: the list leads with a dictionary cell, which
    // takes the first key.
    let config = session._directory.path().join("config");
    SettingsFileStore::new(&config)
        .update(|document| document.set_bool(&keys::IS_LITERAL_ROMAN_CANDIDATE_ENABLED, false))
        .expect("settings written");
    let (_, emits) = session.press_with('h' as u32, state::CONTROL | state::MOD1);
    let Some(Emit::LookupTable(table)) = emits.first() else {
        panic!("the refetched list is shown, got {emits:?}");
    };
    assert_ne!(table.candidates[0], "ho");
    assert_eq!(table.labels[0], "q");
}

const CTRL_ALT: u32 = state::CONTROL | state::MOD1;

/// Desktop TPS P3: Ctrl+Alt+P enters TPS from TL; a second press returns to
/// TL with the glyphs still composing, and the next key commits them as
/// shown before it starts the TL composition. Read by running HEAD
/// (2026-10-03): `e` ㄍ, `8` ㄚ (D2 table).
#[test]
fn switch_tps_round_trip_commits_the_glyphs_on_the_next_key() {
    let _serial = serial();
    let mut session = Session::new(false, false);
    let (_, emits) = session.press_with('p' as u32, CTRL_ALT);
    assert_eq!(emits, [Emit::ModeChanged, Emit::AnnounceMode]);
    session.press('e' as u32);
    session.press('8' as u32);
    let (_, emits) = session.press_with('p' as u32, CTRL_ALT);
    assert_eq!(
        emits,
        [Emit::ModeChanged, Emit::AnnounceMode],
        "the switch commits nothing itself; TPS typing put no table up (D7)"
    );
    let emits = session.press('a' as u32);
    assert_eq!(
        emits[..3],
        [
            Emit::ClearPreedit,
            commit("ㄍㄚ"),
            Emit::Preedit {
                text: "a".to_owned(),
                caret: 1
            }
        ]
    );
}

/// Negative control: TL ↔ POJ crosses no TPS, so the composition carries on.
#[test]
fn switch_romanization_keeps_the_composition() {
    let _serial = serial();
    let mut session = Session::new(false, false);
    session.press('t' as u32);
    session.press('a' as u32);
    session.press_with('c' as u32, CTRL_ALT);
    let emits = session.press('i' as u32);
    assert!(
        matches!(emits.as_slice(), [Emit::Preedit { text, .. }, Emit::LookupTable(_)] if text == "tai"),
        "{emits:?}"
    );
}

/// The other way: a TL composition left under TPS is committed by the first
/// glyph key, which then starts the TPS composition.
#[test]
fn switch_into_tps_commits_the_romanization_on_the_next_key() {
    let _serial = serial();
    let mut session = Session::new(false, false);
    session.type_word("tai");
    session.press_with('p' as u32, CTRL_ALT);
    let emits = session.press('e' as u32);
    // trace, read by running: the commit, then `e` as ㄍ (D2).
    assert_eq!(
        emits[..3],
        [
            Emit::ClearPreedit,
            commit("tai"),
            Emit::Preedit {
                text: "ㄍ".to_owned(),
                caret: 1
            }
        ]
    );
}

/// Desktop TPS D7 on Linux: typing puts no table up; ↓ opens it; the
/// number-row `2` (X keycode 11) then picks the second cell; Escape over it
/// closes it and keeps the glyphs. The pick nails the word and closes the
/// table, the composition stays up; Enter writes it (Hanji conversion H4,
/// B2).
#[test]
fn tps_opens_the_table_on_demand_and_the_number_row_picks() {
    const DOWN: u32 = 0xff54;
    let _serial = serial();
    let mut session = Session::new(false, false);
    session.press_with('p' as u32, CTRL_ALT);
    session.press('e' as u32);
    let emits = session.press('8' as u32);
    assert!(
        !emits
            .iter()
            .any(|emit| matches!(emit, Emit::LookupTable(_))),
        "no table while typing: {emits:?}"
    );
    let opened = session.press(DOWN);
    let [Emit::LookupTable(table)] = opened.as_slice() else {
        panic!("↓ opens the table: {opened:?}");
    };
    let second = table.candidates[1].clone();
    assert_eq!(session.press(ESCAPE), [Emit::HideLookupTable]);
    let reopened = session.press(DOWN);
    assert!(
        matches!(reopened.as_slice(), [Emit::LookupTable(_)]),
        "↓ opens it again: {reopened:?}"
    );
    let (is_handled, emits) = session.press_key('2' as u32, 11, 0);
    assert!(is_handled);
    assert_eq!(
        emits,
        [
            Emit::Preedit {
                text: second.clone(),
                caret: 1
            },
            Emit::HideLookupTable
        ],
        "the number row picks the second cell"
    );
    assert_eq!(session.press(RETURN), [Emit::ClearPreedit, commit(&second)]);
}

/// Hanji conversion on Linux: `e` `8` `4` (ㄍㄚˋ) shows converted; plain ←
/// steps the caret over the word; Shift+Enter writes the glyphs, Enter the
/// Hanji. trace, read by running: ㄍㄚˋ → 絞 (E1 P5a corpus pick: 絞/ká 66 > 假/ká 25).
#[test]
fn tps_converts_a_closed_reading_and_the_commit_keys_part_ways() {
    const LEFT: u32 = 0xff51;
    let _serial = serial();
    let mut session = Session::new(false, false);
    session.press_with('p' as u32, CTRL_ALT);
    session.press('e' as u32);
    session.press('8' as u32);
    let typed = session.press('4' as u32);
    assert_eq!(
        typed,
        [Emit::Preedit {
            text: "絞".to_owned(),
            caret: 1
        }]
    );
    assert_eq!(
        session.press(LEFT),
        [Emit::Preedit {
            text: "絞".to_owned(),
            caret: 0
        }]
    );
    let (is_handled, emits) = session.press_with(RETURN, state::SHIFT);
    assert!(is_handled);
    assert_eq!(emits, [Emit::ClearPreedit, commit("ㄍㄚˋ")]);
    session.press('e' as u32);
    session.press('8' as u32);
    session.press('4' as u32);
    assert_eq!(session.press(RETURN), [Emit::ClearPreedit, commit("絞")]);
}
