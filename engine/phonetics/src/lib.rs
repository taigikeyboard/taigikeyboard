#![forbid(unsafe_code)]
#![doc = include_str!("../../README.md")]

// Public façade modules — the only externally-supported entry points.
// Tests and other engine crates depend on these paths; everything else
// stays mod-private per the domain↔proto boundary rule in
// rules/rust-best-practices.md §3a.
pub mod api;
pub mod dispatch;

mod case_tables;
pub mod case_transform;
mod derivation;
mod normalization;
mod poj;
mod syllable;
mod tables;
mod tl;
mod tone_variations;
mod tps;
mod tps_adjust;

// Top-level re-exports — stable native-helper surface used by the dev
// `cli` crate and integration tests. The cross-platform FFI envelope
// is `engine/dispatch::process_request`; this crate exposes only the
// in-process Rust API.
pub use api::{contains_tps, to_tone_marks, to_tone_number, InputMode, PhoneticsError, System};
pub use normalization::{has_tone_marks, normalize_input, taigi_unicode_base_form};
pub use poj::to_poj;
pub use syllable::{normalize_to_tl, strip_tone_mark};
pub use tl::to_tl;
pub use tps::from_zhuyin as tps_to_tl;
