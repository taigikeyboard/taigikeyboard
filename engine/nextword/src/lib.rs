#![forbid(unsafe_code)]

//! NextWord slice — Intent → Effect state machine for the v3.5.5 Rust core.
//!
//! Owns `PersistedState` (last_selected_word/roman, last_selection_time_ms,
//! predictions_visible, current_generation). External callers reach the engine
//! through the re-exported `EngineHandle`, which holds it in a singleton
//! `Mutex<Engine>` and routes to the private `requests::handle`.
//!
//! Spec: `docs/engine/nextword.md`.

pub mod api;
mod handle;
mod requests;

mod decide;
mod filter;
mod scorer;

pub use api::{Association, Engine, Handled, NextWordError};
pub use handle::EngineHandle;

// Compile-time guarantee: `Engine` must remain `Send` so the static
// `OnceCell<EngineHandle>` in handle.rs can wrap it in `Mutex<Engine>`
// and satisfy `Sync`. Mirrors composing/src/lib.rs:25-28.
const _: fn() = || {
    fn assert_send<T: Send>() {}
    assert_send::<Engine>();
};
