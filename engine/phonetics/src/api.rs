//! High-level Phonetics API for direct in-process callers (the dev
//! `cli` crate, integration tests, and `phonetics::dispatch::handle`).
//! The cross-platform FFI envelope lives in `engine/dispatch` per
//! `rules/rust-best-practices.md §3a`; this module never decodes a
//! top-level `taigi.engine.Request` or owns a panic boundary.

use crate::poj::to_poj;
use crate::syllable::{
    is_stop_tone, normalize_to_tl, parse_syllable, split_initial_final, strip_tone_mark,
};
use crate::tl::to_tl;
use crate::tps::to_zhuyin;
use once_cell::sync::Lazy;
use regex::Regex;
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
    #[error("unsupported op (TPS→TL/POJ word segmentation belongs with the Lexicon slice — not implemented here)")]
    UnsupportedOp,
}

static SYLLABLE_RE: Lazy<Regex> = Lazy::new(|| {
    // Mirrors taigi-converter/src/converter.js SYLLABLE_RE. The class covers:
    //   - ASCII A-Za-z
    //   - Latin-1 Supplement U+00C0-U+00FF (lower + UPPER precomposed
    //     acute/grave/circumflex on a/e/i/o/u — the earlier narrower class
    //     missed all uppercase forms and lowercase í/û)
    //   - Latin Extended-A U+0100-U+017F (macrons + breves — earlier class
    //     missed `ă` U+0103, the POJ tone 9 precomposed form)
    //   - Latin Extended-B U+01CD-U+01DC (carons — earlier class missed
    //     `ǐ ǒ ǔ`)
    //   - Combining diacritics U+0300-U+036F (full block — covers tone 8
    //     U+030D, TL tone 9 U+030B, combining `oo` dot U+0358 in one range)
    //   - POJ nasal markers ⁿ (U+207F) and ᴺ (U+1D3A)
    // Over-permissive on non-TL Latin chars; parse_syllable silently
    // fails for non-TL syllables and the original token is returned, so the
    // observable behaviour is unchanged for non-TL input but correct for
    // previously-bypassed TL/POJ input (e.g. Ká, kă, CHÂN). Found by Codex
    // review on PR #183 (discussion r3143631196).
    Regex::new(concat!(
        "([A-Za-z\u{00c0}-\u{00ff}\u{0100}-\u{017f}\u{01cd}-\u{01dc}",
        "\u{0300}-\u{036f}\u{207f}\u{1d3a}",
        "]+[0-9]?)"
    ))
    .unwrap()
});

#[derive(Debug, Clone, Copy)]
enum Case {
    Lower,
    Upper,
    Title,
}

fn detect_case(text: &str) -> Case {
    let alpha: String = text.chars().filter(|c| c.is_alphabetic()).collect();
    if alpha.is_empty() {
        return Case::Lower;
    }
    if alpha == alpha.to_uppercase() {
        return Case::Upper;
    }
    let first = alpha.chars().next().unwrap();
    if first.to_uppercase().next() == Some(first) {
        return Case::Title;
    }
    Case::Lower
}

fn apply_case(text: &str, case: Case) -> String {
    match case {
        Case::Upper => text.to_uppercase().replace('\u{207f}', "\u{1d3a}"),
        Case::Title => capitalize_first(text),
        Case::Lower => text.to_string(),
    }
}

fn capitalize_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Convert text from one phonetic system to another. The TPS→TL / TPS→POJ paths
/// in the JS `convert` function are not implemented here — they require
/// word-level segmentation (`segmenter.js` + 1.6 MB dictionary trie), which
/// belongs with the Lexicon slice.
pub fn convert(text: &str, from: System, to: System) -> Result<String, PhoneticsError> {
    if from == to {
        return Ok(text.to_string());
    }
    if from == System::Tps {
        return Err(PhoneticsError::UnsupportedOp);
    }

    if to == System::Tps {
        // text → tone-numbered → per-token to_zhuyin (mirrors converter.js:25-42).
        let numbered = to_tone_number(text);
        let lines: Vec<String> = numbered
            .split('\n')
            .map(|line| {
                let mut parts: Vec<String> = Vec::new();
                for word in line.split(' ') {
                    let (prefix, bare) = if let Some(rest) = word.strip_prefix("--") {
                        ("--", rest)
                    } else {
                        ("", word)
                    };
                    for (i, tok) in bare.split('-').enumerate() {
                        if tok.is_empty() {
                            continue;
                        }
                        let with_prefix = if i == 0 {
                            format!("{prefix}{tok}")
                        } else {
                            tok.to_string()
                        };
                        let tps = to_zhuyin(&with_prefix, false, false).trim_end().to_string();
                        for s in split_keep_punct(&tps) {
                            if !s.is_empty() {
                                parts.push(s);
                            }
                        }
                    }
                }
                parts.join(" ")
            })
            .collect();
        return Ok(lines.join("\n"));
    }

    // TL ↔ POJ via syllable-level rewrite.
    let assembler: fn(&str, &str, &str) -> String = match to {
        System::Tl => to_tl,
        System::Poj => to_poj,
        System::Tps => unreachable!(),
    };
    Ok(syllable_rewrite(text, assembler))
}

fn split_keep_punct(input: &str) -> Vec<String> {
    static PUNCT: Lazy<Regex> =
        Lazy::new(|| Regex::new("([\u{3002}\u{ff0c}\u{ff1f}\u{ff0e}「」]+)").unwrap());
    let mut out = Vec::new();
    let mut last = 0;
    for m in PUNCT.find_iter(input) {
        if m.start() > last {
            out.push(input[last..m.start()].to_string());
        }
        out.push(m.as_str().to_string());
        last = m.end();
    }
    if last < input.len() {
        out.push(input[last..].to_string());
    }
    out
}

fn syllable_rewrite(text: &str, assembler: fn(&str, &str, &str) -> String) -> String {
    SYLLABLE_RE
        .replace_all(text, |caps: &regex::Captures| {
            let m = &caps[0];
            match parse_syllable(m) {
                Some((initial, final_str, tone)) => {
                    let case = detect_case(m);
                    apply_case(&assembler(&initial, &final_str, &tone), case)
                }
                None => m.to_string(),
            }
        })
        .into_owned()
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
