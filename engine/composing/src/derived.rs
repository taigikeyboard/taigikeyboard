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

// 中文: 計算組字緩衝區的顯示形式;TPS 回傳去掉分隔記號的字面,POJ/TL 走 phonetics 正規化流程。
// 中文: 本檔案是 §10.2 rawInput 契約的 rendering primitive。
// 中文: 使用者輸入的 hyphen 維持作為轉換邊界 (tone-mark chain 以 - 切段),引擎不驗證音節合法性。

use protos::engine::AppConfig;

pub(crate) fn derived_display(raw: &str, config: &AppConfig) -> String {
    if raw.is_empty() {
        return String::new();
    }
    if phonetics::api::contains_tps(raw) {
        return strip_tps_separator_markers(raw);
    }
    phonetics::api::normalize_tone(raw, config)
}

/// §41 — drop the TPS separator markers from `raw`.
///
/// The keyboard's ASCII space in a TPS buffer is the tone-1 /
/// syllable-boundary MARKER, not text the user typed: it is appended so the
/// next dual-form consonant stays an onset (`phonetics::tps_adjust`), the
/// shadow records it as a lattice barrier
/// (`shadow::build_separator_shadow`), and §41 reads that barrier to pin the
/// unmarked tone. It therefore stays in the raw buffer and must be absent
/// from everything the raw buffer FEEDS the outside world: the pre-edit, the
/// text a commit writes, and the romanization handed to NextWord. Every TPS
/// glyph, tone mark included, passes through untouched; only `U+0020` goes,
/// and only for a buffer that actually carries TPS (a TL / POJ space is a
/// literal word boundary and survives).
// 中文: §41 — 去掉 TPS 分隔記號。鍵盤在 TPS buffer 裡的 ASCII 空白是第一調/音節邊界「記號」,
// 中文:   不是使用者打的字:附加它讓下一個雙形聲母維持聲母形、shadow 記成 lattice barrier、
// 中文:   §41 據以釘無調號調。所以它留在 raw,但必須不出現在 raw 對外供給的東西裡 ——
// 中文:   preedit、commit 寫進文件的字、交給 NextWord 的羅馬字。TPS glyph(含調號)原樣通過,
// 中文:   只丟 U+0020,且只對確實含 TPS 的 buffer(TL/POJ 空白是字面詞界,保留)。
pub(crate) fn strip_tps_separator_markers(raw: &str) -> String {
    if phonetics::api::contains_tps(raw) {
        return raw.replace(' ', "");
    }
    raw.to_string()
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
    fn raw_input_composing_tl_is_literal_no_poj_spelling_fold() {
        // TL composing display is literal (2026-06-05): the tone mark lands
        // on the typed letters with NO spelling fold. POJ-style `goa2` keeps
        // `goa` (mark on `a` per TL rule → `goá`), NOT canonicalized to `guá`.
        // `ai3`→`ài`, `li2`→`lí`; hyphens preserved.
        let phase = Phase::Composing {
            raw: "goa2-ai3-li2".to_string(),
        };
        assert_eq!(phase.raw_input(&config_tl()), "go\u{e1}-\u{e0}i-l\u{ed}");
    }

    #[test]
    fn raw_input_composing_tl_preserves_special_final_eng() {
        // Regression for the `téng` bug: the TL special nasal final `eng`
        // [ɛŋ] must NOT be folded to `ing` [iŋ]. `teng2` → `téng`, NOT `tíng`.
        let phase = Phase::Composing {
            raw: "teng2".to_string(),
        };
        assert_eq!(phase.raw_input(&config_tl()), "t\u{e9}ng");
    }

    #[test]
    fn raw_input_composing_poj_is_literal_no_spelling_fold() {
        // POJ composing display is ALSO literal (2026-06-05): the tone mark
        // lands on the typed letters, NO spelling conversion. A user typing TL
        // spelling in POJ mode keeps it: `ting2` → `tíng` (NOT `téng`). POJ-
        // spelled input is likewise verbatim with POJ tone placement:
        // `goa2` → `góa` (mark on `o`), `teng2` → `téng`.
        assert_eq!(
            Phase::Composing {
                raw: "ting2".to_string(),
            }
            .raw_input(&config_poj()),
            "t\u{ed}ng"
        );
        assert_eq!(
            Phase::Composing {
                raw: "goa2".to_string(),
            }
            .raw_input(&config_poj()),
            "g\u{f3}a"
        );
        assert_eq!(
            Phase::Composing {
                raw: "teng2".to_string(),
            }
            .raw_input(&config_poj()),
            "t\u{e9}ng"
        );
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
                association_tl: "tsuá".to_string(),
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
                association_tl: "tsuá".to_string(),
                raw_span: (0, 4),
                syllable_count: 1,
            }],
        };
        assert_eq!(phase.raw_input(&config_tl()), "");
    }

    #[test]
    fn raw_input_tps_glyphs_pass_through_verbatim() {
        let phase = Phase::Composing {
            raw: "ㄍㄨㄚˋ".to_string(),
        };
        assert_eq!(phase.raw_input(&config_tl()), "ㄍㄨㄚˋ");
    }

    // §41 — the separator marker is hidden at the display seam. Trailing
    // (`ㄒㄧ` + space, the reported case), interior (`ㄍㄠ`␣`ㄉㄞ`), and
    // repeated separators all render as the bare glyph run.
    // 中文: §41 — 分隔記號在顯示接縫被隱藏;尾端(回報案例)、中間、連續多個皆然。
    #[test]
    fn raw_input_tps_hides_the_trailing_separator_marker() {
        let phase = Phase::Composing {
            raw: "ㄒㄧ ".to_string(),
        };
        assert_eq!(phase.raw_input(&config_tl()), "ㄒㄧ");
    }

    #[test]
    fn raw_input_tps_hides_an_interior_separator_marker() {
        let phase = Phase::Composing {
            raw: "ㄍㄠ ㄉㄞ".to_string(),
        };
        assert_eq!(phase.raw_input(&config_tl()), "ㄍㄠㄉㄞ");
    }

    #[test]
    fn raw_input_tps_hides_repeated_separator_markers() {
        let phase = Phase::Composing {
            raw: "ㄍㄠ  ㄉㄞ ".to_string(),
        };
        assert_eq!(phase.raw_input(&config_tl()), "ㄍㄠㄉㄞ");
    }

    #[test]
    fn raw_input_tps_keeps_tone_marks_while_hiding_the_separator() {
        let phase = Phase::Composing {
            raw: "ㄉㄞˊ ㆣㄧˋ".to_string(),
        };
        assert_eq!(phase.raw_input(&config_tl()), "ㄉㄞˊㆣㄧˋ");
    }

    // A TL/POJ space is a literal word boundary and must survive — the
    // hide rule is TPS-only, keyed on the buffer actually carrying TPS.
    // 中文: TL/POJ 的空白是字面詞界,必須留著 — 隱藏規則只對「確實含 TPS」的 buffer 生效。
    #[test]
    fn raw_input_tl_keeps_a_literal_space() {
        let phase = Phase::Composing {
            raw: "tai uan".to_string(),
        };
        assert_eq!(phase.raw_input(&config_tl()), "tai uan");
    }
}
