#![forbid(unsafe_code)]

//! Composing slice — Intent → Effect state machine for the v3.5.4 Rust core.
//!
//! Owns `Phase::Idle | Continuous { .. }`.
//! External callers reach the engine through `requests::handle`; the engine
//! itself is held inside a singleton `Mutex<Engine>` at the FFI boundary
//! (see `engine/dispatch::EngineHandle`).
//!
//! Spec: `docs/engine/composing.md`.

pub mod api;
pub mod handle;
pub mod requests;
pub mod syllabifier;

mod commit_text;
mod continuous;
mod conversion;
mod derived;
mod external_insert;
mod lattice;
mod shadow;
pub mod telex;
mod transition;

pub use api::{
    Applied, CommitScript, ComposingError, ConversionFrequency, Engine, EngineState, Intent,
    NailedSegment, Phase, Usage, UserRows,
};
pub use conversion::WordListStart;
pub use handle::{EngineHandle, ListContext, PendingSnapshot};

// Compile-time guarantee: `Engine` must remain `Send` so the static
// `OnceCell<EngineHandle>` in dispatch can wrap it in `Mutex<Engine>` and
// satisfy `Sync`. See plan §3.4.
const _: fn() = || {
    fn assert_send<T: Send>() {}
    assert_send::<Engine>();
};
