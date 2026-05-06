//! Trie key construction.
//!
//! Mirrors iOS `Lexicon/Trie/InputNormalizer` + `DictionaryRepository`'s
//! prefix logic and Android `LexiconService::buildSearchKey` (the latter
//! drops in this slice per audit D-1 — see plan §10.2).
//!
//! Three prefix families:
//! - `tl:` — TL romanization (numeric tone or no-tone or abbrev)
//! - `poj:` — POJ romanization (parallel to TL)
//! - `hanzi:` — hanji prefix
//!
//! Romanization paths run through `phonetics::normalize_input` so input
//! ↔ stored key match regardless of diacritic vs numeric tone form.

// 中文: 建立 FST 查詢用的前綴 key — 三大族群:tl: / poj: / hanzi:。
// 中文: 羅馬字輸入會先經 phonetics::normalize_input 規範化,確保聲調符號 / 數字寫法都能對齊到儲存形式。

use phonetics::normalize_input;

/// Build the trie lookup key for an input + input-type/mode pair.
///
/// `input_type` decides the prefix family. `input_mode` decides POJ vs TL
/// for romanization paths. Returns the full prefix-qualified key (e.g.
/// `tl:gua2`).
// 中文: 依輸入類型與模式組出 FST 查詢用的完整前綴 key (如 tl:gua2 / poj:goa2 / hanzi:好)。
pub fn build(input: &str, input_type: KeyType, mode: KeyMode) -> String {
    match input_type {
        KeyType::Hanzi => format!("hanzi:{input}"),
        KeyType::Romanization => {
            let normalized = normalize_input(input);
            match mode {
                KeyMode::Tl => format!("tl:{normalized}"),
                KeyMode::Poj => format!("poj:{normalized}"),
                KeyMode::Tps => format!("tl:{normalized}"),
            }
        }
    }
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
    // 中文: TPS bopomofo 模式 (查詢時併入 tl: 族群)。
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
    fn tps_mode_keys_into_tl_family() {
        let result = build("hf", KeyType::Romanization, KeyMode::Tps);
        assert!(
            result.starts_with("tl:"),
            "tps falls through tl: family: {result}"
        );
    }
}
