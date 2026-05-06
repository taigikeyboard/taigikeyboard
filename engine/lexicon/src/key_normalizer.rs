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

use phonetics::normalize_input;

/// Build the trie lookup key for an input + input-type/mode pair.
///
/// `input_type` decides the prefix family. `input_mode` decides POJ vs TL
/// for romanization paths. Returns the full prefix-qualified key (e.g.
/// `tl:gua2`).
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyType {
    Romanization,
    Hanzi,
}

/// Lexicon-internal mode selector. Mapped from `engine/protos::InputMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyMode {
    Tl,
    Poj,
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
