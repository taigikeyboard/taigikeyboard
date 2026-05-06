//! TPSAdjustmentBundle port — collapsed 4-fn TPS keystroke adjustment.
//!
//! Mirrors:
//! - iOS `Input/TPS/TPSInputAdjuster.swift` (4 functions).
//! - iOS `Input/CharacterInputPipeline.swift` (orchestration: collapsed
//!   single entry-point landed in commit 1).
//! - Android `ime/text/CharacterInputPipeline.kt` (commit 1 mirror).
//!
//! Caller (platform) MUST gate by TPS layout. Engine does not gate because
//! Android `InputMode` (POJ/TL) has no `.tps` case — TPS is a layout, not
//! a mode.
//!
//! Trigger sets for syllabic-nasal (`{ㄇ, ㄫ}`) and palatalization
//! (`{ㄗ, ㄘ, ㄙ, ㆡ}`) are disjoint by `lastChar`, so the `?:` short-circuit
//! is observationally equivalent to running both checks unconditionally.

// 中文: TPS 鍵入即時調整,把 4 個調整函式收成單一入口;處理音節初聲鍵的初/終形切換、自動更正、鼻化音節、ㄗ/ㄘ/ㄙ/ㆡ 顎化等規則。平台端必須先用 TPS 配置才呼叫。

use crate::tps::{ZHUYIN_TONES, ZHUYIN_TONES_ENCODE_SAFE};
use once_cell::sync::Lazy;
use std::collections::HashSet;

// =========================================================================
// Tone-mark predicate (used by IsTpsToneMark op + syllabic-nasal trigger)
// =========================================================================

/// Set of TPS non-entering tone marks (`ˋ ˊ ˇ ˫ ˙ ˆ ˪ ˫`).
///
/// Excludes entering-tone finals (`ㆴ ㆵ ㆻ ㆷ`) which are CONSONANTS not
/// tone marks. Excludes the literal " " mapped from tone "1" (no-op).
// 中文: TPS 非入聲的聲調符號集合;排除入聲韻尾 (那些是子音不是聲調) 與 1 聲的空白佔位。
static TONE_MARK_CHARS: Lazy<HashSet<char>> = Lazy::new(|| {
    let mut set = HashSet::new();
    let mut collect = |table: &[(&str, &str)]| {
        for (key, tps) in table {
            // Filter out entering-tone finals (key starts with consonant + digit).
            if key.len() > 1 {
                continue;
            }
            for ch in tps.chars() {
                // Ignore the placeholder " " for tone 1 + combining marks like
                // U+0307 / U+02D9 (those attach to entering-tone finals).
                if ch.is_whitespace() {
                    continue;
                }
                if matches!(ch as u32, 0x0300..=0x036F) {
                    continue;
                }
                set.insert(ch);
            }
        }
    };
    collect(ZHUYIN_TONES);
    collect(ZHUYIN_TONES_ENCODE_SAFE);
    set
});

fn is_tps_tone_mark(c: char) -> bool {
    TONE_MARK_CHARS.contains(&c)
}

/// String-overload for the `Method::IsTpsToneMark` op which takes a string
/// (single grapheme expected). Returns false on empty / multi-grapheme.
// 中文: 字串版聲調符號判定 (給 op 用);空字串或多字元一律回 false。
pub(crate) fn is_tps_tone_mark_str(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(c) = chars.next() else { return false };
    if chars.next().is_some() {
        return false;
    }
    is_tps_tone_mark(c)
}

// =========================================================================
// Syllable-boundary set (used by adjustInitialKey)
// =========================================================================

static SYLLABLE_BOUNDARY_CHARS: Lazy<HashSet<char>> = Lazy::new(|| {
    let mut set: HashSet<char> = TONE_MARK_CHARS.iter().copied().collect();
    // Checked-tone finals (entering-tone consonants — end syllable).
    for c in ['ㆴ', 'ㆵ', 'ㆻ', 'ㆷ'] {
        set.insert(c);
    }
    // Nasal finals (end syllable; next consonant starts new syllable).
    for c in ['ㆬ', 'ㄣ', 'ㆭ', 'ㄥ'] {
        set.insert(c);
    }
    set
});

// =========================================================================
// Per-function adjustments (mirror TPSInputAdjuster.swift)
// =========================================================================

/// Returns context-adjusted TPS character for keys with dual initial/final
/// forms. At syllable start → keep initial form. Not at syllable start →
/// final form (with ㄫ context-aware: after ㄧ → ㄥ, otherwise → ㆭ).
// 中文: 處理同時兼具初聲/終聲形的注音鍵;音節起始保留初聲形,否則改成終聲形 (ㄫ 視前文決定 ㄥ 或 ㆭ)。
fn adjust_initial_key(char_str: &str, raw_input: &str) -> String {
    let Some(first) = char_str.chars().next() else {
        return char_str.to_string();
    };
    if !matches!(first, 'ㄇ' | 'ㄋ' | 'ㄫ' | 'ㄅ' | 'ㄉ' | 'ㄍ' | 'ㄏ') {
        return char_str.to_string();
    }
    if raw_input.is_empty() {
        return char_str.to_string();
    }
    let last = raw_input.chars().last().unwrap();
    if last == ' ' || SYLLABLE_BOUNDARY_CHARS.contains(&last) {
        return char_str.to_string();
    }
    match first {
        'ㄇ' => "ㆬ".to_string(),
        'ㄋ' => "ㄣ".to_string(),
        'ㄅ' => "ㆴ".to_string(),
        'ㄉ' => "ㆵ".to_string(),
        'ㄍ' => "ㆻ".to_string(),
        'ㄏ' => "ㆷ".to_string(),
        'ㄫ' => {
            if last == 'ㄧ' {
                "ㄥ".to_string()
            } else {
                "ㆭ".to_string()
            }
        }
        _ => char_str.to_string(),
    }
}

/// Auto-correct `ㆮ` → `ㆯ` when preceded by `ㄧ`. "iainn" is invalid;
/// only "iaunn" exists.
// 中文: ㄧ 之後的 ㆮ 自動更正成 ㆯ ("iainn" 不存在,只有 "iaunn")。
fn adjust_nasalized_vowel_key(char_str: &str, raw_input: &str) -> String {
    if char_str != "ㆮ" {
        return char_str.to_string();
    }
    let Some(last) = raw_input.chars().last() else {
        return char_str.to_string();
    };
    if last == 'ㄧ' {
        "ㆯ".to_string()
    } else {
        char_str.to_string()
    }
}

/// Returns syllabic replacement for `lastRawChar`, or None.
// 中文: 鼻化抽象音節觸發:在聲調符號接續下,把前一個 ㄇ/ㄫ 改成 ㆬ/ㆭ。
fn syllabic_nasal_replacement(incoming: &str, last_raw_char: Option<char>) -> Option<String> {
    let last = last_raw_char?;
    let first = incoming.chars().next()?;
    if !is_tps_tone_mark(first) {
        return None;
    }
    match last {
        'ㄇ' => Some("ㆬ".to_string()),
        'ㄫ' => Some("ㆭ".to_string()),
        _ => None,
    }
}

/// Returns palatalized replacement for `lastRawChar`, or None.
// 中文: 顎化觸發:在 ㄧ/ㆪ 接續下,把前一個 ㄗ/ㄘ/ㄙ/ㆡ 改成顎化版本 ㄐ/ㄑ/ㄒ/ㆢ。
fn palatalization_replacement(incoming: &str, last_raw_char: Option<char>) -> Option<String> {
    let last = last_raw_char?;
    let first = incoming.chars().next()?;
    if !matches!(first, 'ㄧ' | 'ㆪ') {
        return None;
    }
    match last {
        'ㄗ' => Some("ㄐ".to_string()),
        'ㄘ' => Some("ㄑ".to_string()),
        'ㄙ' => Some("ㄒ".to_string()),
        'ㆡ' => Some("ㆢ".to_string()),
        _ => None,
    }
}

// =========================================================================
// `Method::TpsInputAdjust` — collapsed entry point
// =========================================================================

/// Collapse-equivalent of iOS `CharacterInputPipeline.adjust(_, .tps, raw)`.
/// Returns `(adjusted, replace_last?)`. Caller MUST gate by TPS layout.
// 中文: TPS 鍵入即時調整單一入口,回傳 (調整後字元, 是否要替換最後一字)。呼叫端必須先確認是 TPS 配置。
pub(crate) fn adjust(incoming: &str, raw_input: &str) -> (String, Option<String>) {
    let mut adjusted = adjust_initial_key(incoming, raw_input);
    adjusted = adjust_nasalized_vowel_key(&adjusted, raw_input);

    let last_char = raw_input.chars().last();
    let replace_last = syllabic_nasal_replacement(&adjusted, last_char)
        .or_else(|| palatalization_replacement(&adjusted, last_char));

    (adjusted, replace_last)
}
