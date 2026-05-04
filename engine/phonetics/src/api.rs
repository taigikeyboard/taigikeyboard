//! High-level Phonetics API for direct in-process callers (the dev
//! `cli` crate, integration tests, and `phonetics::dispatch::handle`).
//! The cross-platform FFI envelope lives in `engine/dispatch` per
//! `rules/rust-best-practices.md §3a`; this module never decodes a
//! top-level `taigi.engine.Request` or owns a panic boundary.

use crate::case_transform::adjust_nasal_marker_case;
use crate::poj::to_poj;
use crate::syllable::{is_stop_tone, normalize_to_tl, split_initial_final, strip_tone_mark};
use crate::tl::to_tl;
use crate::tps::is_zhuyin;
use protos::engine::AppConfig;
use thiserror::Error;
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Tl,
    Poj,
    English,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum System {
    Tl,
    Poj,
    Tps,
}

#[derive(Debug, Error)]
pub enum PhoneticsError {
    #[error(
        "unsupported op (e.g. PhoneticsRequest.method is None — likely proto schema mismatch)"
    )]
    UnsupportedOp,
}

fn capitalize_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Translate the proto `AppConfig.input_mode` string into the typed enum.
/// Unknown / empty / "tl" → `Tl`. Mirrors `phonetics::dispatch::parse_input_mode`.
pub fn parse_input_mode(mode: &str) -> InputMode {
    match mode {
        "poj" | "POJ" => InputMode::Poj,
        "english" | "English" | "EN" => InputMode::English,
        _ => InputMode::Tl,
    }
}

/// POJ doubletap preprocessing: `oo`→`o\u{0358}` + `nn`→nasal marker, gated
/// by `AppConfig.{oo,nn}_doubletap_enabled`. No-op for non-POJ modes.
pub(crate) fn preprocess_for_normalize_tone(
    input: &str,
    mode: InputMode,
    config: &AppConfig,
) -> String {
    if !matches!(mode, InputMode::Poj) {
        return input.to_string();
    }
    let mut s = input.to_string();
    if config.oo_doubletap_enabled {
        s = s.replace("oo", "o\u{0358}");
        s = s.replace("Oo", "O\u{0358}");
        s = s.replace("OO", "O\u{0358}");
    }
    if config.nn_doubletap_enabled {
        s = convert_nasal_double_n(&s);
    }
    s
}

/// Mirrors iOS ToneConverter `convertNasalDoubleN`: vowel + "nn" → vowel + "ⁿ".
fn convert_nasal_double_n(input: &str) -> String {
    const NASAL_VOWELS: &str = "aeiouAEIOU";
    let chars: Vec<char> = input.chars().collect();
    let mut result = String::new();
    let mut i = 0;
    while i < chars.len() {
        if i + 1 < chars.len()
            && (chars[i] == 'n' || chars[i] == 'N')
            && (chars[i + 1] == 'n' || chars[i + 1] == 'N')
            && i > 0
            && NASAL_VOWELS.contains(chars[i - 1])
        {
            result.push('\u{207F}');
            i += 2;
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }
    result
}

/// Full normalize-tone chain: parse mode → POJ doubletap preprocessing →
/// tone-mark application → nasal-marker case adjustment. The `Method::NormalizeTone`
/// dispatch arm and `composing::derived` both call this directly. Plan §3.2a.
pub fn normalize_tone(input: &str, config: &AppConfig) -> String {
    let mode = parse_input_mode(&config.input_mode);
    let preprocessed = preprocess_for_normalize_tone(input, mode, config);
    let tone_marked = to_tone_marks(&preprocessed, mode);
    adjust_nasal_marker_case(&tone_marked)
}

/// `true` if the text contains TPS (Taiwanese Phonetic Symbols / Zhuyin)
/// codepoints. Used by composing-derived display to skip POJ/TL tone-mark
/// conversion (TPS strings are already display-ready).
pub fn contains_tps(text: &str) -> bool {
    is_zhuyin(text)
}

/// Convert hyphen-separated syllables to tone marks. Tone digits 1 and 4 are
/// kept as-is — matches the keyboard convention in iOS `convertSyllable` and
/// Android `convertSyllable`.
pub fn to_tone_marks(input: &str, mode: InputMode) -> String {
    if input.is_empty() {
        return String::new();
    }
    input
        .split('-')
        .map(|syl| convert_syllable(syl, mode))
        .collect::<Vec<_>>()
        .join("-")
}

fn convert_syllable(syllable: &str, mode: InputMode) -> String {
    if syllable.is_empty() {
        return String::new();
    }
    let last = syllable.chars().last().unwrap();
    let Some(tone_digit) = last.to_digit(10) else {
        return syllable.to_string();
    };
    if !(1..=9).contains(&tone_digit) {
        return syllable.to_string();
    }
    let base: String = syllable
        .chars()
        .take(syllable.chars().count() - 1)
        .collect();
    if base.is_empty() {
        return syllable.to_string();
    }
    if matches!(mode, InputMode::English) {
        return syllable.to_string();
    }
    if tone_digit == 1 || tone_digit == 4 {
        return syllable.to_string();
    }

    let normalized = normalize_to_tl(&base.to_lowercase());
    let Some((initial, final_str)) = split_initial_final(&normalized) else {
        return syllable.to_string();
    };
    let tone = tone_digit.to_string();
    let assembled = match mode {
        InputMode::Poj => to_poj(&initial, &final_str, &tone),
        InputMode::Tl => to_tl(&initial, &final_str, &tone),
        InputMode::English => return syllable.to_string(),
    };
    let first = base.chars().next().unwrap();
    if first.is_uppercase() {
        capitalize_first(&assembled)
    } else {
        assembled
    }
}

/// Convert tone-marked text to numeric-tone form. Mirrors `toToneNumber` in
/// `converter.js`, including the NFD / per-syllable boundary scan.
pub fn to_tone_number(text: &str) -> String {
    let decomposed: Vec<char> = text.nfd().collect();
    let mut result = String::new();
    let mut i = 0;
    while i < decomposed.len() {
        let ch = decomposed[i];
        if is_letter_like(ch) {
            let start = i;
            while i < decomposed.len()
                && (is_letter_like(decomposed[i]) || is_combining(decomposed[i]))
            {
                i += 1;
            }
            if i < decomposed.len() && decomposed[i].is_ascii_digit() {
                let chunk: String = decomposed[start..=i].iter().collect();
                result.push_str(&chunk);
                i += 1;
                continue;
            }
            let chunk: String = decomposed[start..i].iter().collect();
            let nfc_chunk: String = chunk.nfc().collect();
            let (bare, tone) = strip_tone_mark(&nfc_chunk);
            if !tone.is_empty() {
                let bare_nfc: String = bare.nfc().collect();
                result.push_str(&bare_nfc);
                result.push_str(&tone);
            } else {
                let bare_nfc: String = bare.nfc().collect();
                let normalized = normalize_to_tl(&bare_nfc.to_lowercase());
                if let Some((_, final_str)) = split_initial_final(&normalized) {
                    result.push_str(&bare_nfc);
                    result.push_str(if is_stop_tone(&final_str) { "4" } else { "1" });
                } else {
                    result.push_str(&bare_nfc);
                }
            }
        } else {
            result.push(ch);
            i += 1;
        }
    }
    result
}

fn is_letter_like(c: char) -> bool {
    c.is_alphabetic() || c == '\u{0358}' || c == '\u{207f}' || c == '\u{1d3a}'
}

fn is_combining(c: char) -> bool {
    matches!(c, '\u{0300}'..='\u{036f}')
}

// MARK: - Display-level helpers (iOS / Android `pojDisplayToTLDisplay` / `tlDisplayToPOJDisplay`).
//        Exposed so the iOS+Android fixture suite can exercise them.

pub fn poj_display_to_tl_display(text: &str) -> String {
    rewrite_display(text, System::Tl)
}

pub fn tl_display_to_poj_display(text: &str) -> String {
    rewrite_display(text, System::Poj)
}

fn rewrite_display(text: &str, target: System) -> String {
    if text.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch == '-' || ch == ' ' {
            out.push_str(&rewrite_token(&current, target));
            out.push(ch);
            current.clear();
        } else {
            current.push(ch);
        }
    }
    out.push_str(&rewrite_token(&current, target));
    out
}

fn rewrite_token(token: &str, target: System) -> String {
    if token.is_empty() {
        return String::new();
    }
    let (bare, tone) = strip_tone_mark(token);
    if bare.is_empty() {
        return token.to_string();
    }
    let normalized = normalize_to_tl(&bare.to_lowercase());
    let Some((initial, final_str)) = split_initial_final(&normalized) else {
        return token.to_string();
    };
    let resolved_tone = if tone.is_empty() {
        if is_stop_tone(&final_str) { "4" } else { "1" }.to_string()
    } else {
        tone
    };
    let assembled = match target {
        System::Tl => to_tl(&initial, &final_str, &resolved_tone),
        System::Poj => to_poj(&initial, &final_str, &resolved_tone),
        System::Tps => return token.to_string(),
    };
    let first = token.chars().next().unwrap();
    if first.is_uppercase() {
        capitalize_first(&assembled)
    } else {
        assembled
    }
}
