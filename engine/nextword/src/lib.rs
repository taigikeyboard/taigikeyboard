#![forbid(unsafe_code)]

//! NextWord slice — Intent → Effect state machine for the v3.5.5 Rust core.
//!
//! Owns `PersistedState` (last_selected_word/roman, last_selection_time_ms,
//! is_showing, current_generation). External callers reach the engine
//! through `dispatch::handle`; the engine itself is held inside a singleton
//! `Mutex<Engine>` at the FFI boundary (see `nextword::handle::EngineHandle`).
//!
//! Plan ref: `docs/engine/nextword-slice-plan.md`.

pub mod api;
pub mod dispatch;
pub mod handle;

mod booster;
mod decide;
mod filter;
mod scorer;

pub use api::{Engine, NextWordError};
pub use handle::EngineHandle;

// Compile-time guarantee: `Engine` must remain `Send` so the static
// `OnceCell<EngineHandle>` in handle.rs can wrap it in `Mutex<Engine>`
// and satisfy `Sync`. Mirrors composing/src/lib.rs:25-28.
const _: fn() = || {
    fn assert_send<T: Send>() {}
    assert_send::<Engine>();
};
