//! Input classification primitives for the IME and Tab3 search.
//!
//! Two consumers, both reach this module via the lexicon proto dispatch:
//!
//! - `classify_input` — the IME autocomplete classifier. Replaces iOS
//!   `AutocompleteInputClassifier.classify(rawInput:)` and Android
//!   `AutocompleteInputClassifier.determineInputType` per-keystroke ladder.
//! - `is_hanzi` — the Tab3 search short-circuit predicate. Replaces iOS
//!   `CandidateProcessor.isHanzi` and Android `DictionarySearchViewModel`'s
//!   inline 16-bit `Char.code` check (parity correction — that inline check
//!   silently missed Extensions B/C/D/E because Kotlin `Char.code` tops out
//!   at 0xFFFF).
//!
//! Pure functions — no I/O, no engine handle. INVARIANT contracts live in
//! `docs/architecture/behavioral-invariants.md` under the umbrella label
//! `INVARIANT_LEX_INPUT_CLASSIFICATION`.

// 中文: IME 與 Tab3 共用的輸入分類純函式 — 判斷漢字 vs 有聲調 / 無聲調羅馬字。
// 中文: C-1 後 search_key 維持原樣 (identity);TPS 查詢直接走 SearchRequest{input_mode=Tps},不再前置轉成 TL。

use phonetics::has_tone_marks;
use protos::engine::InputType;

/// Returns `true` iff `text` contains at least one CJK Unified Ideograph
/// (Unified block + Extensions A–E).
///
/// Range coverage matches the canonical platform set captured by
/// `INVARIANT_LEX_INPUT_CLASSIFICATION_HANZI_RANGE`. Extensions F/G/H/I/J
/// are intentionally excluded — including them would be a behavior
/// expansion beyond the v3.5.7 parity correction scope.
// 中文: 偵測文字是否含至少一個 CJK 漢字 (Unified + Extensions A-E,刻意不含 F-J)。
pub fn is_hanzi(text: &str) -> bool {
    text.chars().any(|c| {
        let cp = c as u32;
        (0x4E00..=0x9FFF).contains(&cp)        // CJK Unified Ideographs
            || (0x3400..=0x4DBF).contains(&cp)  // Extension A
            || (0x20000..=0x2A6DF).contains(&cp) // Extension B
            || (0x2A700..=0x2B73F).contains(&cp) // Extension C
            || (0x2B740..=0x2B81F).contains(&cp) // Extension D
            || (0x2B820..=0x2CEAF).contains(&cp) // Extension E
    })
}

/// Returns `true` iff `text` contains an ASCII numeric tone digit.
///
/// ASCII digits 2, 3, 5, 6, 7, 8, 9 are numeric tone markers; 1, 4, and 0
/// are not. See `INVARIANT_LEX_INPUT_CLASSIFICATION_NUMERIC_TONE_SET`.
// 中文: 偵測文字是否含數字聲調 (2/3/5/6/7/8/9);0/1/4 不算聲調。
pub fn contains_numeric_tone(text: &str) -> bool {
    text.chars()
        .any(|c| c.is_ascii_digit() && !matches!(c, '0' | '1' | '4'))
}

/// Result of `classify_input`. `input_type` is the typed proto enum;
/// callers at the proto boundary (`api::classify_input`) convert to `i32`.
// 中文: classify_input 的結果 — 輸入類型 + 原樣 search_key (C-1 後 TPS 不再前置轉 TL)。
pub struct Classification {
    // 中文: 偵測到的輸入類型 (漢字 / 有聲調羅馬字 / 無聲調羅馬字)。
    pub input_type: InputType,
    // 中文: 用於詞庫查詢的字串 — 原樣傳遞;TPS 查詢由 SearchRequest{input_mode=Tps} 經 key_normalizer 直接命中 tps: 族群。
    pub search_key: String,
}

/// Classify `raw` into `(input_type, search_key)`.
///
/// Precedence — see `INVARIANT_LEX_INPUT_CLASSIFICATION_PRECEDENCE`.
/// Search key — see `INVARIANT_LEX_INPUT_CLASSIFICATION_SEARCH_KEY`.
// 中文: 將原始輸入分類為 (input_type, search_key) — 漢字優先、再判斷聲調;search_key 一律原樣回傳 (C-1)。
pub fn classify_input(raw: &str) -> Classification {
    let input_type = if is_hanzi(raw) {
        InputType::Hanzi
    } else if has_tone_marks(raw) || contains_numeric_tone(raw) {
        InputType::RomanWithTone
    } else {
        InputType::RomanNoTone
    };

    Classification {
        input_type,
        search_key: raw.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // INVARIANT_LEX_INPUT_CLASSIFICATION_HANZI_RANGE
    #[test]
    fn is_hanzi_unified_block() {
        assert!(is_hanzi("我"));
    }
    #[test]
    fn is_hanzi_extension_a_lower_bound() {
        assert!(is_hanzi("\u{3400}"));
    }
    #[test]
    fn is_hanzi_extension_a_upper_bound() {
        assert!(is_hanzi("\u{4DBF}"));
    }
    #[test]
    fn is_hanzi_extension_b_lower_bound() {
        assert!(is_hanzi("\u{20000}"));
    }
    #[test]
    fn is_hanzi_extension_c_lower_bound() {
        assert!(is_hanzi("\u{2A700}"));
    }
    #[test]
    fn is_hanzi_extension_d_lower_bound() {
        assert!(is_hanzi("\u{2B740}"));
    }
    #[test]
    fn is_hanzi_extension_e_lower_bound() {
        assert!(is_hanzi("\u{2B820}"));
    }
    #[test]
    fn is_hanzi_extension_e_upper_bound() {
        assert!(is_hanzi("\u{2CEAF}"));
    }
    #[test]
    fn is_hanzi_extension_f_excluded() {
        // 0x2CEB0 is the start of Extension F — must NOT match (out of slice scope)
        assert!(!is_hanzi("\u{2CEB0}"));
    }
    #[test]
    fn is_hanzi_roman_letters_no_match() {
        assert!(!is_hanzi("gua"));
    }
    #[test]
    fn is_hanzi_empty_no_match() {
        assert!(!is_hanzi(""));
    }
    #[test]
    fn is_hanzi_mixed_substring_match() {
        assert!(is_hanzi("a好b"));
    }
    #[test]
    fn is_hanzi_just_below_unified_block_no_match() {
        // 0x4DFF is in Yijing Hexagram Symbols block, NOT CJK
        assert!(!is_hanzi("\u{4DFF}"));
    }

    // INVARIANT_LEX_INPUT_CLASSIFICATION_NUMERIC_TONE_SET
    #[test]
    fn numeric_tone_2_yes() {
        assert!(contains_numeric_tone("gua2"));
    }
    #[test]
    fn numeric_tone_9_yes() {
        assert!(contains_numeric_tone("gua9"));
    }
    #[test]
    fn numeric_tone_0_no() {
        assert!(!contains_numeric_tone("gua0"));
    }
    #[test]
    fn numeric_tone_1_no() {
        assert!(!contains_numeric_tone("gua1"));
    }
    #[test]
    fn numeric_tone_4_no() {
        assert!(!contains_numeric_tone("gua4"));
    }
    #[test]
    fn numeric_tone_no_digit_no() {
        assert!(!contains_numeric_tone("gua"));
    }
    #[test]
    fn numeric_tone_only_excluded_digits_no() {
        assert!(!contains_numeric_tone("0140"));
    }

    // INVARIANT_LEX_INPUT_CLASSIFICATION_PRECEDENCE
    #[test]
    fn classify_hanzi_short_circuits_over_numeric_tone() {
        // hanzi takes precedence over a trailing numeric-tone digit
        let r = classify_input("好2");
        assert_eq!(r.input_type, InputType::Hanzi);
    }
    #[test]
    fn classify_tone_mark_before_numeric_check() {
        // tone-mark detection runs before numeric-tone scan
        let r = classify_input("hó");
        assert_eq!(r.input_type, InputType::RomanWithTone);
    }
    #[test]
    fn classify_numeric_tone_yields_with_tone() {
        let r = classify_input("gua2");
        assert_eq!(r.input_type, InputType::RomanWithTone);
    }
    #[test]
    fn classify_no_tone_yields_no_tone() {
        let r = classify_input("gua");
        assert_eq!(r.input_type, InputType::RomanNoTone);
    }
    #[test]
    fn classify_empty_yields_no_tone() {
        let r = classify_input("");
        assert_eq!(r.input_type, InputType::RomanNoTone);
        assert_eq!(r.search_key, "");
    }

    // INVARIANT_LEX_INPUT_CLASSIFICATION_SEARCH_KEY
    #[test]
    fn search_key_passthrough_when_not_tps() {
        let r = classify_input("gua2");
        assert_eq!(r.search_key, "gua2");
    }
    #[test]
    fn search_key_passthrough_for_hanzi() {
        let r = classify_input("好");
        assert_eq!(r.search_key, "好");
    }
    #[test]
    fn search_key_tps_passes_through_raw() {
        // C-1: TPS input is no longer pre-converted to TL. `search_key`
        // mirrors `raw` so the per-keystroke search path can hit the
        // `tps:` FST family directly via `SearchRequest{input_mode=Tps}`.
        let tps_input = "\u{310d}\u{3128}\u{311a}\u{02cb}";
        let r = classify_input(tps_input);
        assert_eq!(r.search_key, tps_input);
    }
}
