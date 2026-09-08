#![forbid(unsafe_code)]

//! Composing slice — Intent → Effect state machine for the v3.5.4 Rust core.
//!
//! Owns `Phase::Idle | Composing { raw }` plus `selected_candidate_index`.
//! External callers reach the engine through `dispatch::handle`; the engine
//! itself is held inside a singleton `Mutex<Engine>` at the FFI boundary
//! (see `engine/dispatch::EngineHandle`).
//!
//! Spec: `docs/engine/composing.md`.

pub mod api;
pub mod dispatch;
pub mod handle;
pub mod syllabifier;

mod continuous;
mod derived;
mod lattice;
mod shadow;
pub mod telex;
mod transition;

pub use api::{ComposingError, Engine, EngineState, Intent, NailedSegment, Phase};
pub use handle::EngineHandle;

// Compile-time guarantee: `Engine` must remain `Send` so the static
// `OnceCell<EngineHandle>` in dispatch can wrap it in `Mutex<Engine>` and
// satisfy `Sync`. See plan §3.4.
const _: fn() = || {
    fn assert_send<T: Send>() {}
    assert_send::<Engine>();
};
