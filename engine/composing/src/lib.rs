#![forbid(unsafe_code)]

//! Composing slice — Intent → Effect state machine for the v3.5.4 Rust core.
//!
//! Owns `Phase::Idle | Composing { raw }` plus `selected_candidate_index`.
//! External callers reach the engine through `dispatch::handle`; the engine
//! itself is held inside a singleton `Mutex<Engine>` at the FFI boundary
//! (see `engine/dispatch::EngineHandle`).
//!
//! Plan ref: `docs/engine/composing-slice-plan.md`.

pub mod api;
pub mod dispatch;
pub mod handle;

mod derived;
mod transition;

pub use api::{ComposingError, Engine, Intent, Phase};
pub use handle::EngineHandle;

// Compile-time guarantee: `Engine` must remain `Send` so the static
// `OnceCell<EngineHandle>` in dispatch can wrap it in `Mutex<Engine>` and
// satisfy `Sync`. See plan §3.4.
const _: fn() = || {
    fn assert_send<T: Send>() {}
    assert_send::<Engine>();
};
