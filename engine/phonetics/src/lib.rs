#![forbid(unsafe_code)]
#![doc = include_str!("../../README.md")]

pub mod api;
pub mod case_adjust;
pub mod derivation;
pub mod dispatch;
pub mod parser;
pub mod poj;
pub mod tables;
pub mod tl;
pub mod tone_variations;
pub mod tps;
pub mod tps_adjust;

pub use api::{
    convert, process_request, to_tone_marks, to_tone_number, InputMode, PhoneticsError, System,
};
pub use parser::{normalize_to_tl, strip_tone_mark};
pub use poj::to_poj;
pub use tl::to_tl;
