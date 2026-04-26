#![forbid(unsafe_code)]
#![doc = include_str!("../../README.md")]

pub mod api;
pub mod parser;
pub mod poj;
pub mod tables;
pub mod tl;
pub mod tps;

pub use api::{
    convert, process_request, to_tone_marks, to_tone_number, InputMode, PhoneticsError, System,
};
pub use parser::{
    is_stop_tone, normalize_to_tl, parse_syllable, split_initial_final, strip_tone_mark,
};
pub use poj::to_poj;
pub use tl::to_tl;
