//! Compute the display form for a raw composition buffer.
//!
//! TPS inputs are already display-ready (return as-is). POJ/TL inputs go
//! through the full `phonetics::api::normalize_tone` chain (POJ doubletap
//! preprocessing → tone-mark application → nasal-marker case adjustment) per
//! plan §3.2a. The platform `RustEngineBridge.normalizeTone` call sites are
//! replaced by this in-process call.
//!
//! This is the rendering primitive behind [`crate::api::Phase::raw_input`] —
//! see `docs/engine/continuous-input-ranking.md` §10.2 / §10.3 clarification β
//! for the `rawInput` contract. User-typed hyphens are preserved as conversion
//! boundaries (the tone-mark chain splits on `-`); the engine does NOT validate
//! whether each chunk is a real syllable, and does not insert hyphens on its
//! own. Engine-side syllabifier-driven auto-hyphenation is out of scope for
//! v3.5.8 Item 2 (see §10.2 amendment 2026-05-13).

// 中文: 計算組字緩衝區的顯示形式;TPS 直接回傳,POJ/TL 走 phonetics 正規化流程。
// 中文: 本檔案是 §10.2 rawInput 契約的 rendering primitive。
// 中文: 使用者輸入的 hyphen 維持作為轉換邊界 (tone-mark chain 以 - 切段),引擎不驗證音節合法性。

use protos::engine::AppConfig;

pub(crate) fn derived_display(raw: &str, config: &AppConfig) -> String {
    if raw.is_empty() {
        return String::new();
    }
    if phonetics::api::contains_tps(raw) {
        return raw.to_string();
    }
    phonetics::api::normalize_tone(raw, config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{NailedSegment, Phase};

    fn config_tl() -> AppConfig {
        AppConfig {
            tone_mode: String::new(),
            input_mode: "tl".to_string(),
            oo_doubletap_enabled: false,
            nn_doubletap_enabled: false,
            is_translate_swapped: false,
            is_association_recording_enabled: false,
            platform_id: 0,
            output_both_scripts: false,
        }
    }

    fn config_poj() -> AppConfig {
        AppConfig {
            input_mode: "poj".to_string(),
            ..config_tl()
        }
    }

    #[test]
    fn raw_input_idle_is_empty() {
        assert_eq!(Phase::Idle.raw_input(&config_tl()), "");
    }

    #[test]
    fn raw_input_composing_empty_is_empty() {
        let phase = Phase::Composing { raw: String::new() };
        assert_eq!(phase.raw_input(&config_tl()), "");
    }

    #[test]
    fn raw_input_composing_tl_applies_tone_marks_over_user_hyphens() {
        // TL canonicalizes the POJ-style "goa" to "gua" via normalize_to_tl
        // before applying tone marks; hyphens are user-typed and preserved.
        let phase = Phase::Composing {
            raw: "goa2-ai3-li2".to_string(),
        };
        assert_eq!(phase.raw_input(&config_tl()), "guá-ài-lí");
    }

    #[test]
    fn raw_input_composing_tl_preserves_unhyphenated_input_verbatim() {
        // §10.2 amendment 2026-05-13 — engine does NOT auto-insert syllable
        // boundaries; if the user typed no hyphens, derived display has no
        // boundary to convert and returns the raw single chunk.
        let phase = Phase::Composing {
            raw: "goa2ai3li2".to_string(),
        };
        assert_eq!(phase.raw_input(&config_tl()), "goa2ai3li2");
    }

    #[test]
    fn raw_input_composing_poj_uses_poj_diacritics() {
        let phase = Phase::Composing {
            raw: "goa2".to_string(),
        };
        assert_eq!(phase.raw_input(&config_poj()), "góa");
    }

    #[test]
    fn raw_input_continuous_returns_pending_tail_only() {
        // Mid-commit state: "tsua" already nailed, "li2" still pending.
        let phase = Phase::Continuous {
            raw: "li2".to_string(),
            nailed: vec![NailedSegment {
                display_text: "紙".to_string(),
                canonical_text: "紙".to_string(),
                raw_text: "tsua".to_string(),
                raw_span: (0, 4),
                syllable_count: 1,
            }],
        };
        assert_eq!(phase.raw_input(&config_tl()), "lí");
    }

    #[test]
    fn raw_input_continuous_empty_pending_is_empty() {
        let phase = Phase::Continuous {
            raw: String::new(),
            nailed: vec![NailedSegment {
                display_text: "紙".to_string(),
                canonical_text: "紙".to_string(),
                raw_text: "tsua".to_string(),
                raw_span: (0, 4),
                syllable_count: 1,
            }],
        };
        assert_eq!(phase.raw_input(&config_tl()), "");
    }

    #[test]
    fn raw_input_tps_passes_through_verbatim() {
        let phase = Phase::Composing {
            raw: "ㄍㄨㄚˋ".to_string(),
        };
        assert_eq!(phase.raw_input(&config_tl()), "ㄍㄨㄚˋ");
    }
}
