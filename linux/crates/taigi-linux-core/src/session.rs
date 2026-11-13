//! The Linux half of the session: a key as the framework hands it, the
//! IBus capability bit, and the two launches the shared session
//! (`taigi_desktop_core::session`) only asks for. Everything else a key does
//! happens there; the caller replays [`KeyReply::emits`] after every lock is
//! dropped.

use crate::runtime::Runtime;
use taigi_desktop_core::composing::ContextToken;
use taigi_desktop_core::keys::{LEFT_SHIFT_SCAN_CODE, RIGHT_SHIFT_SCAN_CODE, VK_SHIFT_CODE};
use taigi_desktop_core::session::process_key;
use taigi_desktop_core::session::toggle_language_mode;
use taigi_linux_platform::key_translation::state::{
    CONTROL, HYPER, META, MOD1, MOD4, MOD5, RELEASE, SHIFT, SUPER,
};
use taigi_linux_platform::{open_settings, snapshot, toggle_tps_keyboard, RawKeyEvent};

pub use taigi_desktop_core::session::{
    click_from_panel, navigate_from_panel, Emit, EngineState, KeyReply, TextBeforeCaret,
};

/// `IBUS_CAP_SURROUNDING_TEXT` (ibus `src/ibustypes.h:124`).
const CAP_SURROUNDING_TEXT: u32 = 1 << 5;

/// The client's `SetCapabilities` mask, read for the one bit the session
/// cares about. A client with surrounding text also reports it, so from here
/// the swap waits for that report rather than delete blind (#866).
pub fn set_capabilities(state: &mut EngineState, capabilities: u32) {
    state.can_delete_surrounding = capabilities & CAP_SURROUNDING_TEXT != 0;
    if state.can_delete_surrounding && state.text_before_caret == TextBeforeCaret::Unreported {
        state.text_before_caret = TextBeforeCaret::Reported(None);
    }
}

/// The client's surrounding text — IBus `SetSurroundingText`, Fcitx5
/// `InputContext::surroundingText` — with the caret and the selection's
/// anchor in characters; `None` when the client holds none valid. Both
/// frameworks' GTK modules send it before every key press
/// (`ibusimcontext.c` `_request_surrounding_text`, `fcitximcontext.cpp`
/// `fcitx_im_context_filter_keypress`).
pub fn set_surrounding_text(state: &mut EngineState, text: Option<(&str, usize, usize)>) {
    state.text_before_caret = match text {
        Some((text, cursor, anchor)) => TextBeforeCaret::from_surrounding(text, cursor, anchor),
        None => TextBeforeCaret::Reported(None),
    };
}

/// A key as the framework hands it — keysym, keycode, modifier mask with
/// the release bit. Releases and bare modifier presses are answered
/// unhandled. Fcitx5 uses this path and retains framework-managed switching.
pub fn process_raw_key(
    runtime: &Runtime,
    token: ContextToken,
    state: &mut EngineState,
    raw: RawKeyEvent,
) -> KeyReply {
    process_raw_key_with_shift_toggle(runtime, token, state, raw, false)
}

/// IBus receives the complete press/release lifecycle and opts into the
/// Windows-like bare-Shift language toggle. Fcitx5 retains its engine-switch
/// behavior through [`process_raw_key`].
pub fn process_ibus_raw_key(
    runtime: &Runtime,
    token: ContextToken,
    state: &mut EngineState,
    raw: RawKeyEvent,
) -> KeyReply {
    process_raw_key_with_shift_toggle(runtime, token, state, raw, true)
}

fn process_raw_key_with_shift_toggle(
    runtime: &Runtime,
    token: ContextToken,
    state: &mut EngineState,
    raw: RawKeyEvent,
    shift_toggle_enabled: bool,
) -> KeyReply {
    let reply = answer_raw_key(runtime, token, state, raw, shift_toggle_enabled);
    #[cfg(feature = "e2e-trace")]
    crate::trace::key(raw, &reply, state);
    reply
}

fn answer_raw_key(
    runtime: &Runtime,
    token: ContextToken,
    state: &mut EngineState,
    raw: RawKeyEvent,
    shift_toggle_enabled: bool,
) -> KeyReply {
    let unhandled = KeyReply {
        handled: false,
        emits: Vec::new(),
    };
    let now = state
        .shift_tap_clock
        .elapsed()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64;
    let is_release = raw.state & RELEASE != 0;
    let shift_scan = match raw.keyval {
        0xffe1 => Some(LEFT_SHIFT_SCAN_CODE),  // Shift_L
        0xffe2 => Some(RIGHT_SHIFT_SCAN_CODE), // Shift_R
        _ => None,
    };
    if is_release {
        state.latched_chord = None;
        if shift_toggle_enabled {
            state.pressed_keycodes.remove(&raw.keycode);
        }
        if shift_toggle_enabled {
            if let Some(scan_code) = shift_scan {
                if state
                    .shift_tap
                    .take_tap_on_release(VK_SHIFT_CODE, scan_code, now)
                {
                    let mut emits = toggle_language_mode(runtime, token, state);
                    run_launches(&mut emits);
                    // Linux has no test-key phase to claim Shift-down before its
                    // release proves to be a tap. Let both events reach the client
                    // so it sees a complete modifier lifecycle.
                    return KeyReply {
                        handled: false,
                        emits,
                    };
                }
            }
        }
        return unhandled;
    }
    if shift_toggle_enabled {
        let is_repeat = raw.keycode != 0 && !state.pressed_keycodes.insert(raw.keycode);
        if let Some(scan_code) = shift_scan {
            let other_modifier = raw.state & (CONTROL | MOD1 | MOD4 | MOD5 | SUPER | META | HYPER)
                != 0
                || (raw.state & SHIFT != 0 && !is_repeat);
            state.shift_tap.observe_key_down(
                VK_SHIFT_CODE,
                scan_code,
                is_repeat,
                other_modifier,
                now,
            );
            return unhandled;
        }
        // Any other key press, including auto-repeat, ends the tap.
        state.shift_tap.clear();
    }
    match snapshot(raw) {
        Some(snapshot) => {
            let mut reply = process_key(runtime, token, state, &snapshot);
            run_launches(&mut reply.emits);
            reply
        }
        None => unhandled,
    }
}

/// The lifecycle end of a composition: the daemon has just committed (or
/// dropped) the preedit itself. See `taigi_desktop_core::session::end_session`.
pub fn end_session(runtime: &Runtime, token: ContextToken, state: &mut EngineState) -> Vec<Emit> {
    #[cfg(feature = "e2e-trace")]
    crate::trace::session_end();
    state.shift_tap.clear();
    state.pressed_keycodes.clear();
    taigi_desktop_core::session::end_session(runtime, token, state)
}

/// Starts or resumes the session after focus enters or the client resets.
pub fn begin_session(runtime: &Runtime, state: &mut EngineState) -> Vec<Emit> {
    state.shift_tap.clear();
    state.pressed_keycodes.clear();
    taigi_desktop_core::session::begin_session(runtime, state)
}

/// Starts what the shared session asked to launch and leaves what the shells
/// replay, so both shells launch alike, before any signal goes out, and
/// never see a launch. Every path in this crate into `process_key` and
/// `perform_global` comes through here.
pub(crate) fn run_launches(emits: &mut Vec<Emit>) {
    start_launches(
        emits,
        || {
            open_settings(None);
        },
        || {
            toggle_tps_keyboard();
        },
    );
}

fn start_launches(emits: &mut Vec<Emit>, mut open: impl FnMut(), mut toggle: impl FnMut()) {
    emits.retain(|emit| match emit {
        Emit::OpenSettings => {
            open();
            false
        }
        Emit::ToggleTpsKeyboard => {
            toggle();
            false
        }
        // The key panel is a window of the settings app, with no channel here.
        Emit::TpsKeyboardCapTyped(_) => false,
        _ => true,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};
    use taigi_desktop_core::keys::LanguageMode;

    fn shift(keyval: u32, keycode: u32, mask: u32) -> RawKeyEvent {
        RawKeyEvent {
            keyval,
            keycode,
            state: mask,
        }
    }

    #[test]
    fn a_bare_shift_tap_switches_mode_and_the_next_key_passes_through() {
        let (_directory, runtime) = crate::runtime::temporary_runtime();
        let token = ContextToken(71);
        let mut state = EngineState::default();
        assert!(!process_ibus_raw_key(&runtime, token, &mut state, shift(0xffe1, 50, 0)).handled);
        let release = process_ibus_raw_key(
            &runtime,
            token,
            &mut state,
            shift(0xffe1, 50, SHIFT | RELEASE),
        );
        assert!(
            !release.handled,
            "the client receives the matching Shift release"
        );
        assert_eq!(state.language_mode, LanguageMode::English);
        assert_eq!(release.emits, [Emit::ModeChanged, Emit::AnnounceMode]);
        let letter = process_ibus_raw_key(
            &runtime,
            token,
            &mut state,
            RawKeyEvent {
                keyval: b'a' as u32,
                keycode: 38,
                state: 0,
            },
        );
        assert!(!letter.handled, "English keys go to the client unchanged");
        assert!(letter.emits.is_empty());
    }

    #[test]
    fn fcitx_raw_key_path_does_not_toggle_on_shift_tap() {
        let (_directory, runtime) = crate::runtime::temporary_runtime();
        let token = ContextToken(76);
        let mut state = EngineState::default();
        assert!(!process_raw_key(&runtime, token, &mut state, shift(0xffe1, 50, 0)).handled);
        let release = process_raw_key(
            &runtime,
            token,
            &mut state,
            shift(0xffe1, 50, SHIFT | RELEASE),
        );
        assert!(!release.handled);
        assert!(release.emits.is_empty());
        assert_eq!(state.language_mode, LanguageMode::Taigi);
    }

    #[test]
    fn both_shift_keys_toggle_but_a_chord_and_shift_letter_do_not() {
        let (_directory, runtime) = crate::runtime::temporary_runtime();
        let token = ContextToken(72);
        let mut state = EngineState::default();
        {
            let (sym, code) = (0xffe2, 62);
            process_ibus_raw_key(&runtime, token, &mut state, shift(sym, code, 0));
            let release = process_ibus_raw_key(
                &runtime,
                token,
                &mut state,
                shift(sym, code, SHIFT | RELEASE),
            );
            assert!(!release.handled);
            assert_eq!(release.emits, [Emit::ModeChanged, Emit::AnnounceMode]);
        }
        assert_eq!(state.language_mode, LanguageMode::English);
        // A chorded Shift is never a tap.
        process_ibus_raw_key(&runtime, token, &mut state, shift(0xffe1, 50, CONTROL));
        assert!(
            !process_ibus_raw_key(
                &runtime,
                token,
                &mut state,
                shift(0xffe1, 50, CONTROL | SHIFT | RELEASE),
            )
            .handled
        );
        assert_eq!(state.language_mode, LanguageMode::English);

        // Shift+A is a key sequence, not a language switch.
        process_ibus_raw_key(&runtime, token, &mut state, shift(0xffe1, 50, 0));
        process_ibus_raw_key(
            &runtime,
            token,
            &mut state,
            RawKeyEvent {
                keyval: b'A' as u32,
                keycode: 38,
                state: SHIFT,
            },
        );
        assert!(
            !process_ibus_raw_key(
                &runtime,
                token,
                &mut state,
                shift(0xffe1, 50, SHIFT | RELEASE),
            )
            .handled
        );
        assert_eq!(state.language_mode, LanguageMode::English);
    }

    #[test]
    fn holding_both_shifts_or_repeating_a_letter_disqualifies_the_tap() {
        let (_directory, runtime) = crate::runtime::temporary_runtime();
        let token = ContextToken(75);
        let mut state = EngineState::default();

        // Holding left Shift while pressing and releasing right Shift is a
        // chord, even if the release state does not report the other Shift.
        process_ibus_raw_key(&runtime, token, &mut state, shift(0xffe1, 50, 0));
        process_ibus_raw_key(&runtime, token, &mut state, shift(0xffe2, 62, SHIFT));
        let right_up = process_ibus_raw_key(
            &runtime,
            token,
            &mut state,
            shift(0xffe2, 62, SHIFT | RELEASE),
        );
        let left_up = process_ibus_raw_key(
            &runtime,
            token,
            &mut state,
            shift(0xffe1, 50, SHIFT | RELEASE),
        );
        assert!(!right_up.handled && !left_up.handled);
        assert_eq!(state.language_mode, LanguageMode::Taigi);

        // A held letter's repeat also stays a normal key sequence, never a tap.
        process_ibus_raw_key(&runtime, token, &mut state, shift(0xffe1, 50, 0));
        process_ibus_raw_key(
            &runtime,
            token,
            &mut state,
            RawKeyEvent {
                keyval: b'A' as u32,
                keycode: 38,
                state: SHIFT,
            },
        );
        process_ibus_raw_key(
            &runtime,
            token,
            &mut state,
            RawKeyEvent {
                keyval: b'A' as u32,
                keycode: 38,
                state: SHIFT,
            },
        );
        let release = process_ibus_raw_key(
            &runtime,
            token,
            &mut state,
            shift(0xffe1, 50, SHIFT | RELEASE),
        );
        assert!(!release.handled);
        assert!(release.emits.is_empty());
        assert_eq!(state.language_mode, LanguageMode::Taigi);
    }

    #[test]
    fn held_and_repeated_shift_does_not_restart_the_tap_window() {
        let (_directory, runtime) = crate::runtime::temporary_runtime();
        let token = ContextToken(73);
        let mut state = EngineState::default();
        process_ibus_raw_key(&runtime, token, &mut state, shift(0xffe1, 50, 0));
        state.shift_tap_clock = Instant::now() - Duration::from_millis(600);
        process_ibus_raw_key(&runtime, token, &mut state, shift(0xffe1, 50, 0));
        assert!(
            !process_ibus_raw_key(
                &runtime,
                token,
                &mut state,
                shift(0xffe1, 50, SHIFT | RELEASE),
            )
            .handled
        );
        assert_eq!(state.language_mode, LanguageMode::Taigi);
    }

    #[test]
    fn focus_reset_spends_a_shift_press_and_modifier_chords_are_excluded() {
        let (_directory, runtime) = crate::runtime::temporary_runtime();
        let token = ContextToken(74);
        let mut state = EngineState::default();
        for modifier in [CONTROL, MOD1, MOD4, SUPER, MOD5] {
            process_ibus_raw_key(&runtime, token, &mut state, shift(0xffe1, 50, modifier));
            assert!(
                !process_ibus_raw_key(
                    &runtime,
                    token,
                    &mut state,
                    shift(0xffe1, 50, modifier | SHIFT | RELEASE),
                )
                .handled
            );
        }
        process_ibus_raw_key(&runtime, token, &mut state, shift(0xffe1, 50, 0));
        end_session(&runtime, token, &mut state);
        assert!(
            !process_ibus_raw_key(
                &runtime,
                token,
                &mut state,
                shift(0xffe1, 50, SHIFT | RELEASE),
            )
            .handled
        );
        assert_eq!(state.language_mode, LanguageMode::Taigi);
    }

    #[test]
    fn launches_leave_the_list_and_everything_else_keeps_its_order() {
        let mut emits = vec![
            Emit::HideLookupTable,
            Emit::OpenSettings,
            Emit::ModeChanged,
            Emit::ToggleTpsKeyboard,
        ];
        let (mut opened, mut toggled) = (0, 0);
        start_launches(&mut emits, || opened += 1, || toggled += 1);
        assert_eq!((opened, toggled), (1, 1));
        assert_eq!(emits, vec![Emit::HideLookupTable, Emit::ModeChanged]);
    }

    #[test]
    fn only_the_surrounding_text_bit_of_the_mask_counts() {
        let mut state = EngineState::default();
        set_capabilities(&mut state, CAP_SURROUNDING_TEXT | 1);
        assert!(state.can_delete_surrounding);
        set_capabilities(&mut state, !CAP_SURROUNDING_TEXT);
        assert!(!state.can_delete_surrounding);
    }

    #[test]
    fn a_client_with_surrounding_text_holds_the_swap_until_it_reports() {
        let mut state = EngineState::default();
        set_capabilities(&mut state, CAP_SURROUNDING_TEXT);
        assert_eq!(state.text_before_caret, TextBeforeCaret::Reported(None));
        set_surrounding_text(&mut state, Some(("guá ", 4, 4)));
        // A repeated SetCapabilities keeps the report: the daemon sends only
        // a text that changed.
        set_capabilities(&mut state, CAP_SURROUNDING_TEXT);
        assert_eq!(
            state.text_before_caret,
            TextBeforeCaret::Reported(Some(' '))
        );
        set_surrounding_text(&mut state, None);
        assert_eq!(state.text_before_caret, TextBeforeCaret::Reported(None));
    }
}
