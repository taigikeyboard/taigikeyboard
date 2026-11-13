//! High-level Phonetics API for direct in-process callers (the dev
//! `cli` crate, integration tests, and `phonetics::requests::handle`).
//! The cross-platform FFI envelope lives in `engine/dispatch` per
//! `docs/contributing/rust-ffi-safety.md` §2; this module never decodes a
//! top-level `taigi.engine.Request` or owns a panic boundary.

use crate::tps::is_zhuyin;
use protos::engine::AppConfig;
use thiserror::Error;

// Keyboard input mode, mirroring the `AppConfig.input_mode` string. The TPS layout composes with
// the TL tables ([`composing_mode`]); `composing::requests` upgrades to `Tps` once
// `contains_tps(raw)` fires, so the `Tps` variant mostly circulates inside the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Tl,
    Poj,
    Tps,
    English,
}

// Target writing system of a conversion: TL / POJ romanization, or TPS Bopomofo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum System {
    Tl,
    Poj,
    Tps,
}

// Public error type; the single unsupported-op variant usually means proto schema drift.
#[derive(Debug, Error)]
pub enum PhoneticsError {
    #[error(
        "unsupported op (e.g. PhoneticsRequest.method is None — likely proto schema mismatch)"
    )]
    UnsupportedOp,
}

/// Whether text is punctuation that attaches before an auto space (§23).
pub fn is_attaching_punctuation(text: &str) -> bool {
    crate::punctuation::is_attaching_punctuation(text)
}

/// Whether attaching text keeps the auto space after itself — the
/// half-width marks; a full-width one takes the space's place (§23).
pub fn is_spaced_punctuation(text: &str) -> bool {
    crate::punctuation::is_spaced_punctuation(text)
}

/// Whether text is a joiner, which takes the place of an auto space (§23).
pub fn is_joiner_punctuation(text: &str) -> bool {
    crate::punctuation::is_joiner_punctuation(text)
}

pub(crate) fn capitalize_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Translate the proto `AppConfig.input_mode` string into the typed enum.
/// Unknown / empty / "tl" → `Tl`. Mirrors `phonetics::requests::parse_input_mode`.
pub fn parse_input_mode(mode: &str) -> InputMode {
    match mode {
        "poj" | "POJ" => InputMode::Poj,
        "tps" | "TPS" => InputMode::Tps,
        "english" | "English" | "EN" => InputMode::English,
        _ => InputMode::Tl,
    }
}

/// The mode a request composes and cases under: [`parse_input_mode`], except
/// that the TPS layout (`AppConfig::is_tps_layout`) reads as `Tl` — a
/// Bopomofo-free TPS buffer (`-`, a lone tone mark) takes the TL tables, as it
/// did while platforms sent TPS as `"tl"`. Bopomofo content still upgrades to
/// `Tps` at the composing seam (`contains_tps`).
pub fn composing_mode(config: &AppConfig) -> InputMode {
    if config.is_tps_layout() {
        InputMode::Tl
    } else {
        parse_input_mode(&config.input_mode)
    }
}

/// `true` if the text contains TPS (Taiwanese Phonetic Symbols / Zhuyin)
/// codepoints. Used by composing-derived display to skip POJ/TL tone-mark
/// conversion (TPS strings are already display-ready).
pub fn contains_tps(text: &str) -> bool {
    is_zhuyin(text)
}

/// Numeric-tone TL (`"hoo2"`, `"tai5-gi2"`) → TPS Bopomofo, one syllable per
/// `-`-separated token joined by a single space (the platforms'
/// `joined(separator: " ")` / `joinToString(" ")`). Empty tokens (`--`, a
/// leading or trailing `-`) are dropped so nothing double-spaces.
pub fn tl_numeric_to_tps(text: &str, or_maps_to_er: bool) -> String {
    text.split('-')
        .filter(|tok| !tok.is_empty())
        .map(|tok| {
            crate::tps::to_zhuyin(tok, false, or_maps_to_er)
                .trim_end()
                .to_string()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Display TL with tone marks (`"hóo"`, `"tâi-gí"`) → TPS Bopomofo: the tone
/// marks become digits, then [`tl_numeric_to_tps`]. What a TPS-layout cell
/// shows for a candidate with no Hanji, and what picking it writes.
pub fn tl_display_to_tps(text: &str, or_maps_to_er: bool) -> String {
    tl_numeric_to_tps(&to_tone_number(text), or_maps_to_er)
}

pub(crate) fn is_letter_like(c: char) -> bool {
    c.is_alphabetic() || c == '\u{0358}' || is_nasal_marker(c)
}

/// True when `c` is the POJ nasal marker in either case (`ⁿ` / `ᴺ`).
pub fn is_nasal_marker(c: char) -> bool {
    c == crate::case_transform::NASAL_LOWER || c == crate::case_transform::NASAL_UPPER
}

/// True when `c` could be part of a Taiwanese word, as opposed to a mark the
/// user happened to commit alongside one.
///
/// `is_alphabetic` covers everything this IME writes: Latin for TL / POJ —
/// including the decomposed forms, since `ji̍t` / `m̄` / `o͘` all keep a Latin
/// base under their combining marks — Hanji (`Lo`), and both Bopomofo blocks for
/// TPS (`Lo`). Digits, punctuation, symbols, emoji, and bare combining marks
/// are not.
///
/// The two exclusions are characters Unicode calls alphabetic that cannot be a
/// word by themselves: four of the seven TPS tone marks are modifier letters
/// (`Lm` — `ˆ ˇ ˊ ˋ`; the other three are `Sk` and never qualified), and so is
/// the POJ nasal. Attached to a syllable they are harmless — the syllable
/// already brought its own base letter — but alone they are a diacritic.
///
/// Distinct from [`is_letter_like`], which asks whether a character *carries*
/// case or a diacritic and so counts a bare nasal in; this asks whether a
/// character could stand in a word, and a bare nasal cannot.
pub fn is_word_material(c: char) -> bool {
    c.is_alphabetic() && !crate::tps::is_tps_tone_mark(c) && !is_nasal_marker(c)
}

// In-process search-key derivation for the engine's own user-data stores
// (`userdata`): the write-side bundle plus the read-side query key. No wire
// op: the read side answers `SearchCustomEntries` (the `DeriveCustomQueryKey`
// op went with its last caller on 2026-09-30).
pub use crate::custom_search::{
    custom_search_reading, derive_custom_query_key, derive_custom_search_keys,
    tps_toneless_edge_key, CustomSearchKey,
};

pub use crate::display_rewrite::{
    canonical_tl_form, poj_display_to_tl_display, tl_display_to_poj_display,
};
pub use crate::hanji::{is_hanji, is_roman_only_spelling, CJK_RANGES};
pub use crate::reading_key::{roman_reading_eq, stored_roman_mode, toneless_reading_key};
pub use crate::separators::{
    push_syllable_joiner_run, render_typed_separators, syllable_joiner_display,
};
pub use crate::tone_render::{
    consumed_tone_digit_offsets, literal_learning_text, normalize_tone, to_tone_marks,
    to_tone_number, tone_digits_to_marks,
};

/// The syllables of a canonical TL string — `-` inside a word, ` ` between
/// the words of a multi-word entry (`iā sī`); an empty piece (the khinsiann
/// `--`) is not a syllable. The one tokenizer `tps` already keys records by.
pub fn tl_syllables(canonical_tl: &str) -> impl Iterator<Item = &str> {
    crate::tps::tl_syllable_tokens(canonical_tl)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composing_mode_reads_the_tps_layout_as_tl() {
        let mode = |input_mode: &str| {
            composing_mode(&AppConfig {
                input_mode: input_mode.to_string(),
                ..AppConfig::default()
            })
        };
        assert_eq!(mode("tps"), InputMode::Tl);
        assert_eq!(mode("TPS"), InputMode::Tl);
        assert_eq!(mode("tl"), InputMode::Tl);
        assert_eq!(mode("poj"), InputMode::Poj);
        assert_eq!(mode("english"), InputMode::English);
        assert_eq!(mode(""), InputMode::Tl);
        // `parse_input_mode` itself still knows TPS (the content upgrade's target).
        assert_eq!(parse_input_mode("tps"), InputMode::Tps);
    }
}
