//! POJ + TL lowercase ↔ uppercase mapping tables for tone-letter case
//! conversion. Adopted verbatim from Android `ToneUtilities.kt` (the
//! cross-platform canonical) because Kotlin/Swift stdlib `String.uppercase()`
//! / `lowercase()` do NOT round-trip combining-mark sequences such as
//! `a̍`, `o̍͘`, `n̂`, `m̀` — they decompose or drop the combining mark.
//!
//! Tables are crate-private and consumed exclusively by `case_transform`.
//! POJ and TL diverge on:
//!   - 8th tone marker (`̍` U+030D vs `̋` U+030B for tones 8 vs 9)
//!   - `o͘` (POJ) vs `oo` (TL) — POJ uses `O\u{0358}` combining; TL doubles
//!
//! Every entry is sourced from `android/app/src/main/java/com/siansiansu/
//! taigikeyboard/ime/dictionary/ToneUtilities.kt::pojLowercaseToUppercaseMapping`
//! and `tlLowercaseToUppercaseMapping`. iOS Swift relied on
//! `String.uppercased()` directly which silently dropped the combining marks
//! in some edge cases — Rust adopts the explicit-table form to eliminate the
//! divergence. See case-transform-slice-audit.md for the cross-platform
//! resolution rationale.

// 中文: POJ + TL 聲調字母大小寫對應表 (純資料)。stdlib 的 uppercase/lowercase 處理組合符號會掉字,所以這邊用顯式對應表來避免跨平台分歧。

use once_cell::sync::Lazy;
use std::collections::HashMap;

use crate::api::InputMode;

/// POJ lowercase → uppercase tone-letter map.
// 中文: POJ 小寫 → 大寫聲調字母對應 (含組合符號完整音節)。
static POJ_LOWER_TO_UPPER: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let pairs: &[(&str, &str)] = &[
        // a
        ("á", "Á"),
        ("à", "À"),
        ("â", "Â"),
        ("ǎ", "Ǎ"),
        ("ā", "Ā"),
        ("a̍", "A̍"),
        ("ă", "Ă"),
        // e
        ("é", "É"),
        ("è", "È"),
        ("ê", "Ê"),
        ("ě", "Ě"),
        ("ē", "Ē"),
        ("e̍", "E̍"),
        ("ĕ", "Ĕ"),
        // i
        ("í", "Í"),
        ("ì", "Ì"),
        ("î", "Î"),
        ("ǐ", "Ǐ"),
        ("ī", "Ī"),
        ("i̍", "I̍"),
        ("ĭ", "Ĭ"),
        // o
        ("ó", "Ó"),
        ("ò", "Ò"),
        ("ô", "Ô"),
        ("ǒ", "Ǒ"),
        ("ō", "Ō"),
        ("o̍", "O̍"),
        ("ŏ", "Ŏ"),
        // o͘ (POJ combining U+0358)
        ("ó͘", "Ó͘"),
        ("ò͘", "Ò͘"),
        ("ô͘", "Ô͘"),
        ("ǒ͘", "Ǒ͘"),
        ("ō͘", "Ō͘"),
        ("o̍͘", "O̍͘"),
        ("ŏ͘", "Ŏ͘"),
        // u
        ("ú", "Ú"),
        ("ù", "Ù"),
        ("û", "Û"),
        ("ǔ", "Ǔ"),
        ("ū", "Ū"),
        ("u̍", "U̍"),
        ("ŭ", "Ŭ"),
        // n
        ("ń", "Ń"),
        ("ǹ", "Ǹ"),
        ("n̂", "N̂"),
        ("ň", "Ň"),
        ("n̄", "N̄"),
        ("n̍", "N̍"),
        ("n̋", "N̋"),
        // m
        ("ḿ", "Ḿ"),
        ("m̀", "M̀"),
        ("m̂", "M̂"),
        ("m̌", "M̌"),
        ("m̄", "M̄"),
        ("m̍", "M̍"),
        ("m̋", "M̋"),
    ];
    pairs.iter().copied().collect()
});

/// TL lowercase → uppercase tone-letter map. Differs from POJ on the
/// 8th-tone diacritic (`̍` U+030D unchanged but tone-9 `̋` U+030B added)
/// and on `oo` (TL doubles instead of POJ's `o͘`).
// 中文: TL 小寫 → 大寫聲調字母對應;與 POJ 差在第 9 聲符號 (U+030B) 及 `oo` 雙寫 (POJ 用 `o͘`)。
static TL_LOWER_TO_UPPER: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let pairs: &[(&str, &str)] = &[
        // a
        ("á", "Á"),
        ("à", "À"),
        ("â", "Â"),
        ("ǎ", "Ǎ"),
        ("ā", "Ā"),
        ("a̍", "A̍"),
        ("a̋", "A̋"),
        // e
        ("é", "É"),
        ("è", "È"),
        ("ê", "Ê"),
        ("ě", "Ě"),
        ("ē", "Ē"),
        ("e̍", "E̍"),
        ("e̋", "E̋"),
        // i
        ("í", "Í"),
        ("ì", "Ì"),
        ("î", "Î"),
        ("ǐ", "Ǐ"),
        ("ī", "Ī"),
        ("i̍", "I̍"),
        ("i̋", "I̋"),
        // o
        ("ó", "Ó"),
        ("ò", "Ò"),
        ("ô", "Ô"),
        ("ǒ", "Ǒ"),
        ("ō", "Ō"),
        ("o̍", "O̍"),
        ("ő", "Ő"),
        // oo (TL — doubled vowel form)
        ("óo", "Óo"),
        ("òo", "Òo"),
        ("ôo", "Ôo"),
        ("ǒo", "Ǒo"),
        ("ōo", "Ōo"),
        ("o̍o", "O̍o"),
        ("őo", "Őo"),
        // u
        ("ú", "Ú"),
        ("ù", "Ù"),
        ("û", "Û"),
        ("ǔ", "Ǔ"),
        ("ū", "Ū"),
        ("u̍", "U̍"),
        ("ű", "Ű"),
        // n
        ("ń", "Ń"),
        ("ǹ", "Ǹ"),
        ("n̂", "N̂"),
        ("ň", "Ň"),
        ("n̄", "N̄"),
        ("n̍", "N̍"),
        ("n̋", "N̋"),
        // m
        ("ḿ", "Ḿ"),
        ("m̀", "M̀"),
        ("m̂", "M̂"),
        ("m̌", "M̌"),
        ("m̄", "M̄"),
        ("m̍", "M̍"),
        ("m̋", "M̋"),
    ];
    pairs.iter().copied().collect()
});

/// Reverse maps — populated by inverting the lowercase→uppercase map at
/// first access. POJ and TL each have a 1:1 inverse since the entries are
/// unique grapheme cluster strings.
// 中文: 反向對應 (大寫 → 小寫),由正向表反轉而成;POJ/TL 各自為 1:1。
static POJ_UPPER_TO_LOWER: Lazy<HashMap<&'static str, &'static str>> =
    Lazy::new(|| POJ_LOWER_TO_UPPER.iter().map(|(k, v)| (*v, *k)).collect());

static TL_UPPER_TO_LOWER: Lazy<HashMap<&'static str, &'static str>> =
    Lazy::new(|| TL_LOWER_TO_UPPER.iter().map(|(k, v)| (*v, *k)).collect());

/// Returns the mode-specific lowercase→uppercase map, or `None` for English /
/// unsupported modes (in which case the caller falls back to stdlib casing).
// 中文: 依模式取對應的小寫→大寫表;English 模式回 None,呼叫端就退回 stdlib 預設轉換。
pub(crate) fn lower_to_upper(
    mode: InputMode,
) -> Option<&'static HashMap<&'static str, &'static str>> {
    match mode {
        InputMode::Poj => Some(&POJ_LOWER_TO_UPPER),
        InputMode::Tl => Some(&TL_LOWER_TO_UPPER),
        InputMode::English => None,
    }
}

/// Returns the mode-specific uppercase→lowercase map, or `None` for English /
/// unsupported modes (in which case the caller falls back to stdlib casing).
// 中文: 依模式取對應的大寫→小寫表;English 模式回 None,呼叫端就退回 stdlib 預設轉換。
pub(crate) fn upper_to_lower(
    mode: InputMode,
) -> Option<&'static HashMap<&'static str, &'static str>> {
    match mode {
        InputMode::Poj => Some(&POJ_UPPER_TO_LOWER),
        InputMode::Tl => Some(&TL_UPPER_TO_LOWER),
        InputMode::English => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poj_round_trip_combining_marks() {
        let pairs: &[(&str, &str)] = &[("a̍", "A̍"), ("o̍͘", "O̍͘"), ("n̂", "N̂"), ("m̀", "M̀")];
        for (lower, upper) in pairs {
            assert_eq!(POJ_LOWER_TO_UPPER.get(lower), Some(upper));
            assert_eq!(POJ_UPPER_TO_LOWER.get(upper), Some(lower));
        }
    }

    #[test]
    fn tl_doubled_oo_distinct_from_poj_combining() {
        // TL uses doubled vowel "oo" form; POJ table must not contain it.
        assert_eq!(TL_LOWER_TO_UPPER.get("óo"), Some(&"Óo"));
        assert!(POJ_LOWER_TO_UPPER.get("óo").is_none());
        // POJ uses combining "o͘" form; TL table must not contain it.
        assert_eq!(POJ_LOWER_TO_UPPER.get("ó͘"), Some(&"Ó͘"));
        assert!(TL_LOWER_TO_UPPER.get("ó͘").is_none());
    }

    #[test]
    fn english_mode_returns_no_map() {
        assert!(lower_to_upper(InputMode::English).is_none());
        assert!(upper_to_lower(InputMode::English).is_none());
    }
}
