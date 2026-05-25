//! Trie key construction.
//!
//! Mirrors iOS `Lexicon/Trie/InputNormalizer` + `DictionaryRepository`'s
//! prefix logic and Android `LexiconService::buildSearchKey` (the latter
//! drops in this slice per audit D-1 — see plan §10.2).
//!
//! Four prefix families:
//! - `tl:` — TL romanization (numeric tone or no-tone or abbrev)
//! - `poj:` — POJ romanization (parallel to TL)
//! - `tps:` — TPS Bopomofo (parallel to TL/POJ; C-1 onward)
//! - `hanzi:` — hanji prefix
//!
//! TL/POJ paths run through `phonetics::normalize_input` so input ↔
//! stored key match regardless of diacritic vs numeric tone form. TPS
//! keeps Bopomofo literal (Approach A — see [[project_v359_d_tps_triindex_plan]]):
//! strip hyphens + whitespace and substitute the standalone tone-8 dot
//! `U+02D9` for the combining `U+0307` the build pipeline emits.

// 中文: 建立 FST 查詢用的前綴 key — 四大族群:tl: / poj: / tps: / hanzi:。
// 中文: TL/POJ 走 phonetics::normalize_input 規範化;TPS 維持注音原樣,僅剝除連字號/空白並把獨立 tone-8 點 (U+02D9) 換成組合形式 (U+0307) 以對齊 build pipeline。

use phonetics::normalize_input;

/// Build the trie lookup key for an input + input-type/mode pair.
///
/// `input_type` decides the prefix family. `input_mode` decides POJ vs
/// TL vs TPS for romanization paths. Returns the full prefix-qualified
/// key (e.g. `tl:gua2`, `tps:ㄍㄨㄚˋ`).
// 中文: 依輸入類型與模式組出 FST 查詢用的完整前綴 key (如 tl:gua2 / poj:goa2 / tps:ㆤˊ / hanzi:好)。
pub fn build(input: &str, input_type: KeyType, mode: KeyMode) -> String {
    match input_type {
        KeyType::Hanzi => format!("hanzi:{input}"),
        KeyType::Romanization => match mode {
            KeyMode::Tl => format!("tl:{}", normalize_input(input)),
            KeyMode::Poj => format!("poj:{}", normalize_input(input)),
            KeyMode::Tps => format!("tps:{}", normalize_tps_key_body(input)),
        },
    }
}

/// Normalize raw TPS input into the body of a `tps:` FST key.
///
/// The build pipeline emits fused Bopomofo with no separators and tone-8
/// as combining dot above `U+0307` (see `dictionary/build/merge_csv.py`
/// `convert_tl_to_tps_strict(...).replace(" ", "")` and
/// `taigi-converter/src/tables.js` `ZHUYIN_TONES`). Platform keyboards
/// type the standalone modifier-letter dot `U+02D9` (see iOS
/// `TaigiLayouts.swift` + Android `tps.json`), so substitute one for the
/// other and drop user-visible separators.
// 中文: 把使用者輸入的 TPS(注音 + 調號)收斂成 FST `tps:` 鍵主體 — 去連字號/空白,並把鍵盤輸入的獨立 tone-8 點 (U+02D9) 換成 build pipeline 使用的組合形式 (U+0307)。
fn normalize_tps_key_body(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '-' | ' ' | '\t' => {}
            '\u{02D9}' => out.push('\u{0307}'),
            _ => out.push(ch),
        }
    }
    out
}

/// Lexicon-internal key family selector. Mapped from
/// `engine/protos::InputType` at the dispatch boundary.
// 中文: 內部 key 族群選擇器;由 dispatch 將 proto InputType 轉換得來。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyType {
    // 中文: 羅馬字輸入 (TL / POJ / TPS 皆走此分支)。
    Romanization,
    // 中文: 漢字輸入。
    Hanzi,
}

/// Lexicon-internal mode selector. Mapped from `engine/protos::InputMode`.
// 中文: 內部模式選擇器;由 dispatch 將 proto InputMode 轉換得來。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyMode {
    // 中文: TL 羅馬字模式。
    Tl,
    // 中文: POJ 羅馬字模式。
    Poj,
    // 中文: TPS Bopomofo 模式 (查詢 tps: 族群)。
    Tps,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn romanization_tl_uses_tl_prefix() {
        assert_eq!(build("gua2", KeyType::Romanization, KeyMode::Tl), "tl:gua2");
    }

    #[test]
    fn romanization_poj_uses_poj_prefix() {
        assert_eq!(
            build("goa2", KeyType::Romanization, KeyMode::Poj),
            "poj:goa2"
        );
    }

    #[test]
    fn hanzi_input_skips_normalize() {
        assert_eq!(build("我", KeyType::Hanzi, KeyMode::Tl), "hanzi:我");
    }

    #[test]
    fn tps_mode_uses_tps_prefix_for_bopomofo_with_tone() {
        // ㆤˊ — Bopomofo `ㆤ` (U+3124) + modifier-letter tone-2 `ˊ` (U+02CA).
        // Matches `tps_num` emitted by the build pipeline for row `ê`.
        assert_eq!(
            build("\u{3124}\u{02CA}", KeyType::Romanization, KeyMode::Tps),
            "tps:\u{3124}\u{02CA}"
        );
    }

    #[test]
    fn tps_mode_strips_hyphens_and_spaces() {
        // User-visible separators `-` and ` ` are stripped so the key
        // matches the fused Bopomofo emitted by the pipeline.
        let result = build(
            "\u{3110}\u{3127}-\u{3124}",
            KeyType::Romanization,
            KeyMode::Tps,
        );
        assert_eq!(result, "tps:\u{3110}\u{3127}\u{3124}");

        let with_space = build(
            "\u{3110}\u{3127} \u{3124}",
            KeyType::Romanization,
            KeyMode::Tps,
        );
        assert_eq!(with_space, "tps:\u{3110}\u{3127}\u{3124}");
    }

    #[test]
    fn tps_mode_substitutes_standalone_tone8_dot() {
        // Keyboard layouts emit modifier-letter dot `˙` (U+02D9) for tone 8;
        // build pipeline emits combining dot `̇` (U+0307). Substitute so the
        // FST exact-lookup hits.
        let input = format!("\u{3110}\u{3127}\u{02D9}");
        assert_eq!(
            build(&input, KeyType::Romanization, KeyMode::Tps),
            "tps:\u{3110}\u{3127}\u{0307}"
        );
    }

    #[test]
    fn tps_mode_preserves_combining_dot() {
        // Input already in U+0307 form (e.g. internal callers passing the
        // pipeline form) round-trips unchanged.
        let input = "\u{3110}\u{3127}\u{0307}";
        assert_eq!(
            build(input, KeyType::Romanization, KeyMode::Tps),
            "tps:\u{3110}\u{3127}\u{0307}"
        );
    }
}
