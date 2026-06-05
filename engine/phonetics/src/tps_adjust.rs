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

// Syllable-boundary chars: end the current syllable so a new dual-form
// initial after one of these stays as the initial form. Includes tone
// marks, entering-tone finals, syllabic nasals, precomposed nasal-coda
// compound finals, and nasalized vowel finals — all sourced from
// `tps::ZHUYIN_VOWELS`. (Space is handled separately by the inline
// `last == ' '` check at the call site.)
//
// Pure-vowel finals (ㄚ ㄧ ㄨ ㄛ ㄜ ㄞ ㄠ etc.) are INTENTIONALLY excluded:
// they are syllable-end-ambiguous and the entering-tone auto-correct
// (ㄍㄚ + ㄉ → ㄍㄚㆵ for `kat`) depends on Rule 2 treating them as non-boundary.
//
// Nasalized vowel + ㄏ also forms the legal nasalized checked final `-nnh`
// (annh, ennh, innh, iannh per §3.2.5; ainnh, aunnh per §3.2.6 dialectal),
// but local look-back cannot distinguish that from cross-syllable
// `<nasalized-vowel> + ㄏ-initial` like `ㄏㄨㆩㄏㄧ` (歡喜 = huann-hi).
// Continuous-input correctness wins — `-nnh` checked syllables are rare
// and the dictionary stores them in precomposed `ㆷ` form, so users reach
// them via dictionary lookup rather than auto-correct.
// 中文: 音節邊界字元;聲調符號、空白、入聲韻尾、自鳴鼻音、precomposed 鼻音韻尾與鼻化母音。純母音故意排除以保留入聲輸入。鼻化母音 + ㄏ 雖也是合法 `-nnh`,但局部無法分辨跨音節 (歡喜 `ㄏㄨㆩㄏㄧ`),取連續輸入正確性。
static SYLLABLE_BOUNDARY_CHARS: Lazy<HashSet<char>> = Lazy::new(|| {
    let mut set: HashSet<char> = TONE_MARK_CHARS.iter().copied().collect();
    // Checked-tone finals (entering-tone consonants — end syllable).
    for c in ['ㆴ', 'ㆵ', 'ㆻ', 'ㆷ'] {
        set.insert(c);
    }
    // Syllabic nasals (ㆬ ㄣ ㆭ = m/n/ng in ZHUYIN_VOWELS) + contextual
    // `ㄥ` (ㄫ after ㄧ — see adjust_initial_key).
    for c in ['ㆬ', 'ㄣ', 'ㆭ', 'ㄥ'] {
        set.insert(c);
    }
    // Precomposed nasal-coda compound finals (am/an/ang/om/ong per
    // `ZHUYIN_VOWELS` lines 51-64).
    for c in ['ㆰ', 'ㄢ', 'ㄤ', 'ㆱ', 'ㆲ'] {
        set.insert(c);
    }
    // Nasalized vowel finals (ainn/aunn/ann/enn/inn/onn/unn per
    // `ZHUYIN_VOWELS` lines 44-50). Vowel + inherent nasalization = complete
    // syllable; next dual-form initial starts a new syllable.
    for c in ['ㆪ', 'ㆥ', 'ㆧ', 'ㆫ', 'ㆩ', 'ㆮ', 'ㆯ'] {
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
    // Phonotactic gate (STOP codas only, ㄅㄉㄍㄏ → ㆴㆵㆻㆷ): a stop coda
    // can attach only to specific nuclei per the entering-tone finals
    // table (`taigi-phonetics-reference.md` §3.2.4 / `tables.rs::TL_FINALS`).
    // `au` admits NO stop coda (no aut/aup/auk — only `-h`, e.g. auh 搯),
    // likewise `ot`/`iaut`/… do not exist. The blanket "convert after any
    // pure vowel" rule therefore mis-builds `ㄍㄠ`+`ㄉ` → `ㄍㄠㆵ` (=kaut, a
    // non-syllable), stranding the NEXT syllable's initial — first tone has
    // no tone mark to delimit, so nothing stops it (the `第一調穩死` /
    // 交代 = kau-tài bug). When the current open syllable already cannot
    // take this stop as a coda, keep the INITIAL form so it starts the next
    // syllable; the existing lattice then segments `ㄍㄠㄉㄞ` → 交代.
    //
    // Gate only when the boundary-suffix is itself exactly ONE valid
    // syllable (a vowel-final open syllable, or a syllabic `m`/`ng`) AND
    // `<suffix><coda>` is NOT a valid syllable. A multi-syllable no-space
    // suffix (`ㄍㄠㄉㄚ`) is not one syllable → fall through to the legacy
    // convert (unchanged behaviour, no regression); an ambiguous-but-valid
    // coda (`ㄚ`+`ㄉ`=kat) converts as before (the alternate next-initial
    // reading is reached via the boundary space). Validity is decided by
    // `TL_FINALS` (+ the production dict): `uat`/`uak` are codas (convert)
    // but `uap` is NOT. The §3.2.4 "(rare)" note for `uap` is contradicted
    // by the authoritative MOE §4 table (which lists `uat`, not `uap`) and
    // is absent from every code table and the dict (only 鬱懊癖's `tl_abbrev`
    // "uap" exists — an acronym, not a final), so `ㄍㄨㄚ`+`ㄅ` correctly
    // keeps the initial.
    // Nasals (ㄇㄋㄫ) are deliberately untouched here — same over-broad
    // assumption applies but they entangle with the syllabic-nasal Rule 2b
    // and the ㄫ→ㄥ/ㆭ context branch; tracked as a separate follow-up.
    // 中文: 音韻 gate(僅塞音尾 ㄅㄉㄍㄏ):塞音尾只接特定韻(§3.2.4 / TL_FINALS),
    // 中文:   `au` 不接塞音尾 → `ㄍㄠ`+`ㄉ` 不可成 `kaut`,應保持 ㄉ 為下一字聲母(交代)。
    // 中文:   僅當 boundary-suffix 本身為單一合法開音節且 suffix+coda 非法時保持聲母;
    // 中文:   多音節 no-space 串非單一開音節 → 照舊轉(零回歸)。鼻音另案處理(Rule 2b 糾纏)。
    if let Some(coda) = stop_coda_form(first) {
        let pending = pending_open_syllable(raw_input);
        let pending_is_open = crate::is_valid_syllable(&crate::tps_to_tl(pending));
        let coda_valid = crate::is_valid_syllable(&crate::tps_to_tl(&format!("{pending}{coda}")));
        if pending_is_open && !coda_valid {
            return char_str.to_string();
        }
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

/// The entering-tone coda glyph for a dual-form STOP initial
/// (`ㄅ→ㆴ`, `ㄉ→ㆵ`, `ㄍ→ㆻ`, `ㄏ→ㆷ`), or `None` for non-stops. Nasals
/// (`ㄇ`/`ㄋ`/`ㄫ`) are intentionally excluded — the phonotactic stop-coda
/// gate in [`adjust_initial_key`] does not apply to them.
// 中文: 雙形塞音聲母對應的入聲尾字元;非塞音(鼻音)回 None,音韻 gate 不套用鼻音。
fn stop_coda_form(initial: char) -> Option<char> {
    match initial {
        'ㄅ' => Some('ㆴ'),
        'ㄉ' => Some('ㆵ'),
        'ㄍ' => Some('ㆻ'),
        'ㄏ' => Some('ㆷ'),
        _ => None,
    }
}

/// The trailing run of open (vowel-final) syllables currently being typed:
/// `raw_input` after the last syllable-boundary char (tone mark, stop /
/// nasal coda, nasalized-vowel final — all in [`SYLLABLE_BOUNDARY_CHARS`]
/// — or a literal space). Boundary chars terminate a syllable, so the
/// remainder is onset+vowel material with no internal boundary. The caller
/// checks whether it is exactly ONE valid open syllable before using it as
/// the coda-attachment target (a multi-syllable run like `ㄍㄠㄉㄚ` is not
/// one open syllable, so the gate falls through to the legacy convert).
// 中文: 目前正在輸入的開音節串 = 最後一個邊界字元(聲調符/入聲尾/鼻韻尾/空白)之後的子字串。
fn pending_open_syllable(raw_input: &str) -> &str {
    let mut start = 0;
    for (idx, ch) in raw_input.char_indices() {
        if ch == ' ' || SYLLABLE_BOUNDARY_CHARS.contains(&ch) {
            start = idx + ch.len_utf8();
        }
    }
    &raw_input[start..]
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

#[cfg(test)]
mod tests {
    use super::adjust_initial_key;

    // INVARIANT_TPS_STOPCODA_PHONOTACTIC_GATE — a dual-form STOP (ㄅㄉㄍㄏ)
    // after a pure vowel converts to its entering-tone coda ONLY when the
    // resulting final is phonotactically valid (§3.2.4 / TL_FINALS). When
    // it is not, the stop stays an INITIAL so it starts the next syllable
    // (no-space first-tone continuous input: ㄍㄠ + ㄉ → keep ㄉ → 交代).

    #[test]
    fn stop_after_vowel_with_valid_coda_converts() {
        // a/i/o/u nuclei that DO take the stop coda (kat, kap, kak, op, ok,
        // it, ut) — unchanged from the legacy blanket rule.
        for (incoming, raw, expected) in [
            ("ㄉ", "ㄍㄚ", "ㆵ"),   // kat 結/甲
            ("ㄅ", "ㄍㄚ", "ㆴ"),   // kap
            ("ㄍ", "ㄍㄚ", "ㆻ"),   // kak
            ("ㄅ", "ㄛ", "ㆴ"),     // op 欲
            ("ㄍ", "ㄛ", "ㆻ"),     // ok 惡
            ("ㄉ", "ㄒㄧ", "ㆵ"),   // sit
            ("ㄉ", "ㄍㄨ", "ㆵ"),   // kut
            ("ㄉ", "ㄍㄨㄚ", "ㆵ"), // kuat (uat IS a final — MOE §4 #69 + TL_FINALS)
            ("ㄍ", "ㄍㄨㄚ", "ㆻ"), // kuak (uak IS a final — TL_FINALS + dict)
        ] {
            assert_eq!(
                adjust_initial_key(incoming, raw),
                expected,
                "{raw}+{incoming} has a valid coda → must convert",
            );
        }
    }

    #[test]
    fn stop_after_vowel_with_impossible_coda_keeps_initial() {
        // `au` takes NO stop coda (no aut/aup/auk); `o`+t (ot) and `io`+t
        // (iot) / `iau`+t (iaut) likewise do not exist. The stop must stay
        // an initial — this is the 交代 (ㄍㄠ|ㄉㄞ) fix.
        for (incoming, raw) in [
            ("ㄉ", "ㄍㄠ"),   // kaut ✗ → 交代
            ("ㄅ", "ㄍㄠ"),   // kaup ✗
            ("ㄍ", "ㄍㄠ"),   // kauk ✗
            ("ㄉ", "ㄛ"),     // ot ✗
            ("ㄉ", "ㄍㄧㄠ"), // kiaut ✗ (multi-char nucleus iau)
            ("ㄉ", "ㄠ"),     // aut ✗ (zero onset)
            ("ㄅ", "ㄧㄛ"),   // iop ✗
            ("ㄅ", "ㄍㄨㄚ"), // kuap ✗ — uap is NOT a final (absent from MOE §4,
                              // TL_FINALS, and the dict); contrast kuat/kuak above
        ] {
            assert_eq!(
                adjust_initial_key(incoming, raw),
                incoming,
                "{raw}+{incoming} has an impossible coda → must keep initial",
            );
        }
    }

    #[test]
    fn h_coda_after_au_still_converts() {
        // `au`+h = auh (搯) IS valid, so ㄏ still converts after ㄠ — the
        // gate is coda-specific, not "keep all consonants after au".
        assert_eq!(adjust_initial_key("ㄏ", "ㄍㄠ"), "ㆷ");
        assert_eq!(adjust_initial_key("ㄏ", "ㄍㄧㄠ"), "ㆷ"); // kiauh valid
    }

    #[test]
    fn multi_syllable_no_space_suffix_falls_through_to_convert() {
        // `ㄍㄠㄉㄚ` (no boundary) is NOT one open syllable, so the gate does
        // not fire — the trailing `ㄉㄚ`(ta) legitimately takes -t (tat), and
        // the legacy convert produces it. (No regression for the chained
        // valid-coda case; the rare chained impossible-coda case stays
        // default-convert, same as before, disambiguated via the space.)
        assert_eq!(adjust_initial_key("ㄉ", "ㄍㄠㄉㄚ"), "ㆵ");
    }

    #[test]
    fn nasals_are_not_gated() {
        // Nasals (ㄇㄋㄫ) are intentionally outside the stop-coda gate
        // (Rule 2b syllabic-nasal + ㄫ→ㄥ/ㆭ context interplay). They keep
        // the legacy positional behaviour: ㄚ+ㄇ→ㆬ, ㄚ+ㄫ→ㆭ, ㄧ+ㄫ→ㄥ —
        // AND ㄠ+ㄇ→ㆬ stays as before (unchanged, even though aum is not a
        // valid final), so this fix introduces no nasal behaviour change.
        assert_eq!(adjust_initial_key("ㄇ", "ㄚ"), "ㆬ");
        assert_eq!(adjust_initial_key("ㄫ", "ㄚ"), "ㆭ");
        assert_eq!(adjust_initial_key("ㄫ", "ㄧ"), "ㄥ");
        assert_eq!(adjust_initial_key("ㄇ", "ㄍㄠ"), "ㆬ");
    }

    #[test]
    fn boundary_and_empty_buffer_keep_initial_as_before() {
        // Syllable-start positions (empty buffer, after a tone mark, after
        // an existing coda, after a space) keep the initial form unchanged —
        // the gate is downstream of these early returns.
        assert_eq!(adjust_initial_key("ㄉ", ""), "ㄉ");
        assert_eq!(adjust_initial_key("ㄉ", "ㄍㄚ\u{02cb}"), "ㄉ"); // after tone-2
        assert_eq!(adjust_initial_key("ㄉ", "ㄍㄚㆵ"), "ㄉ"); // after stop coda
        assert_eq!(adjust_initial_key("ㄉ", "ㄍㄠ "), "ㄉ"); // after space
    }
}
