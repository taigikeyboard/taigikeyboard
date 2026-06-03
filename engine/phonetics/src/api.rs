//! High-level Phonetics API for direct in-process callers (the dev
//! `cli` crate, integration tests, and `phonetics::dispatch::handle`).
//! The cross-platform FFI envelope lives in `engine/dispatch` per
//! `.claude/rules/rust-best-practices.md §3a`; this module never decodes a
//! top-level `taigi.engine.Request` or owns a panic boundary.

// 中文: Phonetics 高階 Rust API,給 cli/測試/dispatch 直接呼叫;不負責解碼最外層 Request 或 panic 邊界。

use crate::case_transform::adjust_nasal_marker_case;
use crate::poj::to_poj;
use crate::syllable::{is_stop_tone, normalize_to_tl, split_initial_final, strip_tone_mark};
use crate::tl::to_tl;
use crate::tps::is_zhuyin;
use protos::engine::AppConfig;
use thiserror::Error;
use unicode_normalization::UnicodeNormalization;

// 中文: 鍵盤輸入模式,對應 `AppConfig.input_mode` 字串。
// 中文: v3.5.9 D / C-3b — Tps 變體加入,作為連續輸入 first-class mode-axis。
// 中文:   平台 AppConfig.input_mode 字串仍送 "tl"/"poj" 並用 is_translate_swapped 旗標
// 中文:   表示 TPS;composing::dispatch 端以 contains_tps(raw) 偵測 TPS 字元後升級為
// 中文:   InputMode::Tps,所以本 enum 的 Tps 變體主要在 engine 內部 mode-axis 流通。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Tl,
    Poj,
    Tps,
    English,
}

// 中文: 表音系統 enum,用於選擇要轉換成的目標羅馬字 (TL/POJ) 或注音 (TPS)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum System {
    Tl,
    Poj,
    Tps,
}

// 中文: Phonetics 對外錯誤型別。目前只有「不支援的 op」,通常代表 proto schema 跟平台不一致。
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
// 中文: 把 `AppConfig.input_mode` 字串轉成型別化 enum;未知/空字串/"tl" 一律當作 TL。
pub fn parse_input_mode(mode: &str) -> InputMode {
    match mode {
        "poj" | "POJ" => InputMode::Poj,
        "tps" | "TPS" => InputMode::Tps,
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
// 中文: 聲調正規化主流程:判斷模式 → POJ 雙擊預處理 → 套用聲調符號 → 鼻化符號大小寫對齊。
pub fn normalize_tone(input: &str, config: &AppConfig) -> String {
    let mode = parse_input_mode(&config.input_mode);
    let preprocessed = preprocess_for_normalize_tone(input, mode, config);
    let tone_marked = to_tone_marks(&preprocessed, mode);
    adjust_nasal_marker_case(&tone_marked)
}

/// `true` if the text contains TPS (Taiwanese Phonetic Symbols / Zhuyin)
/// codepoints. Used by composing-derived display to skip POJ/TL tone-mark
/// conversion (TPS strings are already display-ready).
// 中文: 判斷字串是否含有 TPS (台羅注音/Zhuyin) 字元;有的話組字區的 derived 顯示就直接跳過聲調轉換。
pub fn contains_tps(text: &str) -> bool {
    is_zhuyin(text)
}

/// Convert hyphen-separated syllables to tone marks. Tone digits 1 and 4 are
/// kept as-is — matches the keyboard convention in iOS `convertSyllable` and
/// Android `convertSyllable`.
// 中文: 把 hyphen 分隔的數字聲調音節串轉成聲調符號形式;聲調 1、4 留著當數字 (跟兩平台鍵盤一致)。
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
    // v3.5.9 D / C-3b — English and TPS both bypass TL/POJ syllable assembly.
    // 中文: TPS 走自家 zhuyin path,不經 to_tl / to_poj 組裝;與 English 同走 identity 提前返回。
    if matches!(mode, InputMode::English | InputMode::Tps) {
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
        InputMode::English | InputMode::Tps => return syllable.to_string(),
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
// 中文: 反向轉換,把聲調符號形式換成聲調數字形式 (NFD 拆解後逐音節掃描)。
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

// 中文: 顯示層 POJ → TL 轉換 (給跨平台 fixture 測試使用)。
pub fn poj_display_to_tl_display(text: &str) -> String {
    rewrite_display(text, System::Tl)
}

// 中文: 顯示層 TL → POJ 轉換 (給跨平台 fixture 測試使用)。
pub fn tl_display_to_poj_display(text: &str) -> String {
    rewrite_display(text, System::Poj)
}

/// v3.5.9 B-4 — mode-aware canonicalizer for the `user_frequency.db`
/// commit key (`RawCandidate.display_text`). Folds Taigi-script
/// romanization onto canonical TL display form so a romanization-only
/// candidate (hanji-absent) keys identically across modes; downstream
/// presentation reverses via `recase_tl_as_poj_display`. Composes
/// [`poj_display_to_tl_display`] for **both** `Tl` and `Poj` modes
/// (PR #310 r3278520895 fix — a POJ-form custom entry typed in POJ
/// mode and later accessed in TL mode still needs the fold or
/// `user_frequency.db` keys split). `English` mode is intentionally
/// identity — English custom entries like `hello` must not be
/// reinterpreted as Taigi.
///
/// Idempotence on TL input: for already-TL display form the rewrite
/// chain (`strip_tone_mark` → `normalize_to_tl` → `split_initial_final`
/// → `to_tl`) returns the same TL string. Pass-through on non-Taigi:
/// `split_initial_final` fails the phonotactic split and the rewriter
/// returns the original token. Both cases mean `Tl` mode for ordinary
/// TL custom entries is observably identity at the byte level.
///
/// **Bounded non-idempotence — CapsLock input** (Codex pre-impl
/// 2026-05-21 SHOULD): `rewrite_token` lowercases for the parse and
/// only title-cases its assembled output via `capitalize_first(_)` per
/// hyphen-split sub-token, so a fully uppercase Taigi-shaped token
/// like `TÂI-GÍ` (POJ) or `TSÁI-GÍ` (TL) round-trips as `Tâi-Gí` /
/// `Tsái-Gí` instead of preserving CapsLock. After PR #310 r3278520895
/// this asymmetry applies to **both** `Tl` and `Poj` modes (was POJ
/// only — the cross-mode parity fold forced the symmetry). Production
/// touch is bounded — the case asymmetry only affects a hanji-absent
/// custom dict entry whose stored roman is all-caps Taigi-shaped, an
/// extremely rare user pattern. `English` mode preserves CapsLock
/// identically via the identity branch. Pinned by
/// `engine/phonetics/tests/canonical_tl_form.rs::capslock_taigi_is_known_non_idempotent`.
// 中文: B-4 — display_text (user_frequency.db commit key) 的 mode-aware canonicalizer;
// 中文:   Tl/Poj 兩 mode 都走 poj_display_to_tl_display(idempotent on TL form,fold POJ form);
// 中文:   English mode identity(`hello` 不該被當 Taigi 重解)。CapsLock 邊角現在對 Tl/Poj
// 中文:   兩 mode 都非冪等(全大寫純羅馬 hanji-absent custom entry,production 觸發面極小)。
pub fn canonical_tl_form(text: &str, mode: InputMode) -> String {
    match mode {
        InputMode::Tl | InputMode::Poj => poj_display_to_tl_display(text),
        // v3.5.9 D / C-3b — TPS roman is Bopomofo, not POJ/TL Latin shape;
        // poj_display_to_tl_display would no-op on Bopomofo anyway (no Latin
        // patterns to substitute), but routing through identity makes the
        // contract explicit and avoids an unnecessary string scan per
        // user_frequency.db commit key derivation.
        // 中文: D / C-3b — TPS 漢羅 user-history key 改走 identity;TPS 字形為 Bopomofo,
        // 中文:   poj_display_to_tl_display 對 Bopomofo 為 no-op,改 identity 顯式
        // 中文:   表達契約且省一輪掃描。
        InputMode::Tps | InputMode::English => text.to_string(),
    }
}

/// Separator- and tone-insensitive reading key for collapsing romanization
/// variants of the **same syllable sequence** — `taigi`, `tai5gi2`, and
/// `tâi-gí` all map to `taigi`. Lowercases, folds the nasal marker
/// (ⁿ U+207F / ᴺ U+1D3A → `nn`), then drops every tone diacritic, ASCII
/// digit, hyphen, and space.
///
/// Reuses [`derivation::derive_notone`] (the custom-dictionary `notone`
/// derivation) verbatim so the two never drift — both want exactly
/// "strip everything that is not a base spelling letter". Used by
/// `nextword::filter` to recognize that a continuous-input raw next_tl
/// (`taigi`) and a normal-commit canonical next_tl (`tâi-gí`) are the
/// same reading and must not surface as two predictions of one word.
///
/// **Tone-INSENSITIVE** — unlike `composing::roman_reading_eq` (which is
/// separator-insensitive but tone-PRESERVING), this folds tones away so
/// `tāng` (重量) and `tàng` both key to `tang`; the caller is responsible
/// for keeping genuinely distinct readings (Core Principle #7) apart by
/// only collapsing when a single canonical target exists.
///
/// **Residual** — does NOT fold spelling families: POJ `chiah` keys to
/// `chiah`, TL `tsiah` to `tsiah` (no `ch↔ts` / `oa↔ua`). A POJ-spelled
/// raw next_tl therefore will not collapse onto its TL canonical here;
/// the write-side R2 fix (continuous commit carries canonical TL) closes
/// that gap. R1 only needs the common same-family case.
// 中文: 分隔符 + 聲調皆不敏感的「讀音鍵」,把同一音節序列的不同羅馬字寫法收斂到同一鍵
// 中文:   (taigi / tai5gi2 / tâi-gí → 全部 taigi)。復用 derive_notone(自訂詞 notone 衍生)
// 中文:   避免兩處邏輯漂移。供 nextword::filter 辨識連續輸入 raw next_tl 與一般 commit
// 中文:   canonical next_tl 為同讀音,避免同一詞出現兩筆預測。聲調不敏感(tāng/tàng 同鍵 tang),
// 中文:   故呼叫端須在唯一 canonical 目標存在時才折疊以守 #7。殘留:不折拼寫家族(ch↔ts),
// 中文:   POJ-spelled raw 不會在此收斂到 TL canonical,留待 R2 寫層修正。
pub fn toneless_reading_key(roman: &str) -> String {
    crate::derivation::derive_notone(roman)
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

#[cfg(test)]
mod tests {
    use super::*;

    // trace: derive_notone lowercases, folds ⁿ→nn, NFD, drops combining
    // tone marks + ASCII digits + '-' + ' '. So all three renderings of
    // 台語 collapse to the bare spelling "taigi".
    #[test]
    fn toneless_reading_key_folds_tone_and_separator_variants() {
        assert_eq!(toneless_reading_key("tâi-gí"), "taigi");
        assert_eq!(toneless_reading_key("taigi"), "taigi");
        assert_eq!(toneless_reading_key("tai5gi2"), "taigi");
        // canonical (diacritic + hyphen) and continuous raw (fused) agree.
        assert_eq!(toneless_reading_key("tâi-gí"), toneless_reading_key("taigi"));
    }

    // trace: all-tone-1 multi-syllable canonical "khong-an" vs continuous
    // raw "khongan" — separator-only difference, both toneless — still
    // collapse to one key. (The caller keeps the separator-bearing form as
    // canonical.)
    #[test]
    fn toneless_reading_key_collapses_separator_only_difference() {
        assert_eq!(toneless_reading_key("khong-an"), "khongan");
        assert_eq!(toneless_reading_key("khong-an"), toneless_reading_key("khongan"));
    }

    // trace: 重/tāng (重量) and 重/tàng are genuinely distinct readings
    // (Core Principle #7) but tone-INSENSITIVE folding maps both to "tang".
    // The key intentionally collides; the nextword caller must NOT collapse
    // such a bare single-syllable group (no separator → no canonical target).
    #[test]
    fn toneless_reading_key_is_tone_insensitive_distinct_readings_collide() {
        assert_eq!(toneless_reading_key("tāng"), "tang");
        assert_eq!(toneless_reading_key("tàng"), "tang");
    }

    // trace: spelling families are NOT folded (no ch↔ts). Documented R1
    // residual — POJ-spelled raw will not collapse onto TL canonical here.
    #[test]
    fn toneless_reading_key_does_not_fold_spelling_families() {
        assert_ne!(toneless_reading_key("chiah"), toneless_reading_key("tsiah"));
    }
}
