//! The Linux half of the session: a key as the framework hands it, the
//! IBus capability bit, and the two launches the shared session
//! (`taigi_desktop_core::session`) only asks for. Everything else a key does
//! happens there; the caller replays [`KeyReply::emits`] after every lock is
//! dropped.

use crate::runtime::Runtime;
use taigi_desktop_core::composing::ContextToken;
use taigi_desktop_core::session::process_key;
use taigi_linux_platform::key_translation::state::RELEASE;
use taigi_linux_platform::{open_settings, snapshot, toggle_tps_keyboard, RawKeyEvent};

pub use taigi_desktop_core::session::{
    begin_session, click_from_panel, navigate_from_panel, Emit, EngineState, KeyReply,
};

/// `IBUS_CAP_SURROUNDING_TEXT` (ibus `src/ibustypes.h:124`).
const CAP_SURROUNDING_TEXT: u32 = 1 << 5;

/// The client's `SetCapabilities` mask, read for the one bit the session
/// cares about.
pub fn set_capabilities(state: &mut EngineState, capabilities: u32) {
    state.can_delete_surrounding = capabilities & CAP_SURROUNDING_TEXT != 0;
}

/// A key as the framework hands it — keysym, keycode, modifier mask with
/// the release bit. Releases and bare modifier presses are answered
/// unhandled; a release also unlatches a held toggle chord.
pub fn process_raw_key(
    runtime: &Runtime,
    token: ContextToken,
    state: &mut EngineState,
    raw: RawKeyEvent,
) -> KeyReply {
    let reply = answer_raw_key(runtime, token, state, raw);
    #[cfg(feature = "e2e-trace")]
    crate::trace::key(raw, &reply, state);
    reply
}

fn answer_raw_key(
    runtime: &Runtime,
    token: ContextToken,
    state: &mut EngineState,
    raw: RawKeyEvent,
) -> KeyReply {
    let unhandled = KeyReply {
        handled: false,
        emits: Vec::new(),
    };
    if raw.state & RELEASE != 0 {
        state.latched_chord = None;
        return unhandled;
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
    taigi_desktop_core::session::end_session(runtime, token, state)
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
}
