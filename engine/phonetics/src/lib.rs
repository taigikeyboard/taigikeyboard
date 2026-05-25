#![forbid(unsafe_code)]
#![doc = include_str!("../../README.md")]

// 中文: 表音 (phonetics) crate 入口,負責 TL/POJ/TPS 羅馬字轉換、聲調正規化、字母大小寫處理、音節解析等跨平台演算法。

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
pub use syllable::{
    canonicalize_poj_syllable, canonicalize_syllable, is_valid_syllable, normalize_to_poj,
    normalize_to_tl, strip_tone_mark, NORMALIZE_TO_POJ_GLYPH_RULES, NORMALIZE_TO_POJ_RULES,
    NORMALIZE_TO_TL_RULES,
};
pub use tl::to_tl;
pub use tps::{
    canonicalize_tps_syllable, from_zhuyin as tps_to_tl, is_tps_char, is_tps_initial,
    is_tps_tone_mark, tps_notone_from_tl, tps_notone_or_variant,
};
