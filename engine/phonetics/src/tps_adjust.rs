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
    // Phonotactic gate (all dual-form finals — STOPS ㄅㄉㄍㄏ → ㆴㆵㆻㆷ AND
    // NASALS ㄇㄋㄫ → ㆬㄣㆭ/ㄥ): a dual-form final can attach only to specific
    // nuclei per the entering-tone finals (§3.2.4) + nasal-coda finals
    // (§3.2.3) tables (`taigi-phonetics-reference.md` / `tables.rs::TL_FINALS`).
    // `au` admits NO stop coda (no aut/aup/auk — only `-h`, e.g. auh 搯) and
    // NO nasal coda (no aum/aun/aung); `u` admits no `-m` (no `um`). The
    // blanket "convert after any pure vowel" rule therefore mis-builds
    // `ㄍㄠ`+`ㄉ` → `ㄍㄠㆵ` (kaut) and `ㄍㄨ`+`ㄇ` → `ㄍㄨㆬ` (kum) — both
    // non-syllables — stranding the NEXT syllable's initial; first tone has
    // no tone mark to delimit, so nothing stops it (the `第一調穩死` bug:
    // 交代 kau-tài, 龜毛 ku-môo). When the current open syllable cannot take
    // this final, keep the INITIAL form so it begins the next syllable; the
    // existing lattice then segments `ㄍㄠㄉㄞ` → 交代, `ㄍㄨㄇㆦ` → 龜毛.
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
    // Rule 2b (syllabic-nasal, `syllabic_nasal_replacement`) interaction:
    // when the gate now keeps `ㄇ`/`ㄫ` an initial after a vowel and a TONE
    // MARK follows, Rule 2b retroactively folds it to ㆬ/ㆭ. For `ㄇ` this
    // reconstructs byte-identically the same invalid `kum` the old blanket
    // fold produced; for `ㄋ`/`ㄫ` the glyph differs but the syllable is
    // still invalid TL — no working-word regression, since those
    // `<vowel><nasal><tone>` sequences map to no dict word either way. The
    // improvement is the `<vowel><nasal><vowel>` next-syllable case (龜毛).
    // (`dual_final_form` is the single glyph source for both the gate test
    // and the conversion below — see its doc comment.)
    // 中文: 音韻 gate(雙形尾 — 塞音 ㄅㄉㄍㄏ + 鼻音 ㄇㄋㄫ):雙形尾只接特定韻
    // 中文:   (入聲韻 §3.2.4 + 鼻音韻 §3.2.3 / TL_FINALS);`au` 不接塞音/鼻音尾、`u` 不接 -m
    // 中文:   → `ㄍㄠ`+`ㄉ`=kaut、`ㄍㄨ`+`ㄇ`=kum 皆非音節,保持聲母為下一字起始(交代/龜毛)。
    // 中文:   僅當 boundary-suffix 為單一合法開音節且 suffix+final 非法時保持聲母;多音節串照舊轉(零回歸)。
    // 中文:   Rule 2b 互動:gate 保留的 ㄇ/ㄫ 後接聲調會被折回 ㆬ/ㆭ;ㄇ 與舊行為 byte-identical,
    // 中文:   ㄋ/ㄫ glyph 不同但仍 invalid TL(無 working-word 回歸)。改善 = <母音><鼻音><母音>(龜毛)。
    let Some(coda) = dual_final_form(first, last) else {
        return char_str.to_string();
    };
    let pending = pending_open_syllable(raw_input);
    let pending_is_open = crate::is_valid_syllable(&crate::tps_to_tl(pending));
    let coda_valid = crate::is_valid_syllable(&crate::tps_to_tl(&format!("{pending}{coda}")));
    if pending_is_open && !coda_valid {
        return char_str.to_string();
    }
    coda.to_string()
}

/// The final/coda glyph for a dual-form initial — STOPS (`ㄅ→ㆴ`, `ㄉ→ㆵ`,
/// `ㄍ→ㆻ`, `ㄏ→ㆷ`) and NASALS (`ㄇ→ㆬ`, `ㄋ→ㄣ`, `ㄫ→ㄥ/ㆭ`) — or `None`
/// for any other char. `ㄫ` is context-dependent: after `ㄧ` it is `ㄥ`
/// (the `-ing` rhyme), otherwise `ㆭ`; `last` is therefore a parameter so
/// the phonotactic gate in [`adjust_initial_key`] tests validity against
/// the EXACT glyph that will be emitted — one glyph source for both the
/// gate test and the conversion, no gate/conversion drift.
// 中文: 雙形聲母(塞音+鼻音)對應的韻尾/終形字元;ㄫ 依前字決定 ㄥ(ing)/ㆭ 故帶 last;
// 中文:   gate 與轉換共用同一份 glyph(避免漂移),非雙形回 None。
fn dual_final_form(initial: char, last: char) -> Option<char> {
    match initial {
        'ㄅ' => Some('ㆴ'),
        'ㄉ' => Some('ㆵ'),
        'ㄍ' => Some('ㆻ'),
        'ㄏ' => Some('ㆷ'),
        'ㄇ' => Some('ㆬ'),
        'ㄋ' => Some('ㄣ'),
        'ㄫ' => Some(if last == 'ㄧ' { 'ㄥ' } else { 'ㆭ' }),
        _ => None,
    }
}

/// Inverse of [`dual_final_form`]: the syllable-ONSET glyph for a TPS
/// final/coda glyph (`ㆴ→ㄅ`, `ㆵ→ㄉ`, `ㆻ→ㄍ`, `ㆷ→ㄏ`, `ㆬ→ㄇ`, `ㄣ→ㄋ`,
/// `ㆭ→ㄫ`, `ㄥ→ㄫ`), or `None` for any non-coda char. Both `ㆭ` and the
/// `-ing`-context `ㄥ` map back to the single onset `ㄫ`.
///
/// Used by the continuous-input de-fold reading (`composing::shadow`):
/// when the per-keystroke auto-correct folded a dual-form consonant into
/// a coda (`ㄍㆤ`+`ㄏ`→`ㄍㆤㆷ`), a coda glyph that is actually the NEXT
/// syllable's onset (雞胸 ke-hing = `ㄍㆤ|ㄏㄧㄥ`) is structurally hidden —
/// a coda glyph cannot start a syllable in the `tps:` inventory. De-folding
/// it back to the onset glyph lets the segmenter surface the alternate
/// reading. Every pair is a 3-byte Bopomofo↔3-byte Bopomofo glyph, so the
/// substitution is byte-length preserving (offset maps stay valid).
// 中文: dual_final_form 的反向 — TPS 韻尾/終形 glyph → 對應聲母 glyph(ㆭ 與 ㄥ 皆回 ㄫ);
// 中文:   供連續輸入 de-fold 讀法用:被 auto-correct 摺成韻尾、實為下字聲母者(雞胸 ㄍㆤ|ㄏㄧㄥ)
// 中文:   反摺回聲母讓切分器列出替代讀法。每對皆 3-byte↔3-byte,byte 長度不變(offset map 不動)。
pub fn defold_coda_to_initial(coda: char) -> Option<char> {
    match coda {
        'ㆴ' => Some('ㄅ'),
        'ㆵ' => Some('ㄉ'),
        'ㆻ' => Some('ㄍ'),
        'ㆷ' => Some('ㄏ'),
        'ㆬ' => Some('ㄇ'),
        'ㄣ' => Some('ㄋ'),
        'ㆭ' => Some('ㄫ'),
        'ㄥ' => Some('ㄫ'),
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

/// Convert the TPS digit-popup `9` to the tone-9 mark `ˆ` (U+02C6) when a
/// toneable syllable is pending. Tone-9 is the ONLY Taiwanese tone with no
/// dedicated TPS mark key — neither layout exposes `ˆ`, so its sole input
/// affordance is the digit-`9` popup (Android) / numeric pad (iOS). Without
/// this normalization the literal `9` dangles as a non-syllable char and the
/// tone-9 dictionary words (昨昏 `ㄗㄤˆ`, 才 `ㄘㄞˆ`, the 日語借詞 set) stay
/// unreachable.
///
/// Fires only when `raw_input` ends in a TPS syllable body that can carry a
/// non-entering tone — a Bopomofo nucleus / nasal-coda / nasalized-vowel
/// final. Kept as a literal `9` when:
///   - `raw_input` is empty (standalone digit → committed verbatim upstream),
///   - the last char is already a tone mark (no double-toning),
///   - the last char is an entering-tone stop coda `ㆴㆵㆻㆷ` (tone 4/8 only),
///   - the last char is a space / non-Bopomofo (literal-digit context).
///
/// Mirrors the non-entering tone marks (ˋˊˇ˫˪) the user types via dedicated
/// keys; tone-9 alone must be reconstructed from the digit. Runs first in
/// [`adjust`] so the produced `ˆ` feeds [`syllabic_nasal_replacement`]
/// identically to a directly-typed mark (Rule 2b parity).
// 中文: 把 TPS 數字鍵 9 在有可標調音節時轉成第九調符號 ˆ(U+02C6)。第九調是唯一沒有
// 中文:   專屬 TPS 調號鍵的聲調(兩平台佈局皆無 ˆ),只能靠數字 9 popup 輸入;不轉換則
// 中文:   字面 9 落單,第九調詞(昨昏 ㄗㄤˆ / 才 ㄘㄞˆ / 日語借詞)永遠查不到。
// 中文:   僅在 raw_input 結尾為可承載非入聲調的注音音節主體(韻核/鼻韻尾/鼻化母音)時觸發;
// 中文:   空輸入、已帶調、入聲塞音尾 ㆴㆵㆻㆷ、空白/非注音 → 保留字面 9。
fn adjust_tone_nine_digit(char_str: &str, raw_input: &str) -> String {
    if char_str != "9" {
        return char_str.to_string();
    }
    let Some(last) = raw_input.chars().last() else {
        return char_str.to_string();
    };
    // Entering-tone stop codas are part of a tone-4/8 syllable body and cannot
    // take tone-9. Every other Bopomofo body char (nucleus / nasal coda /
    // nasalized-vowel final) is a valid tone-9 attachment point.
    if crate::tps::is_tps_char(last) && !matches!(last, 'ㆴ' | 'ㆵ' | 'ㆻ' | 'ㆷ') {
        return "\u{02c6}".to_string();
    }
    char_str.to_string()
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
    let mut adjusted = adjust_tone_nine_digit(incoming, raw_input);
    adjusted = adjust_initial_key(&adjusted, raw_input);
    adjusted = adjust_nasalized_vowel_key(&adjusted, raw_input);

    let last_char = raw_input.chars().last();
    let replace_last = syllabic_nasal_replacement(&adjusted, last_char)
        .or_else(|| palatalization_replacement(&adjusted, last_char));

    (adjusted, replace_last)
}

#[cfg(test)]
mod tests {
    use super::{adjust_initial_key, adjust_tone_nine_digit};

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
    fn nasals_with_valid_coda_convert() {
        // Nasals (ㄇㄋㄫ) now go through the SAME phonotactic gate as stops.
        // A valid nasal-coda final still converts (unchanged): am/an/ang,
        // ing (ㄧ+ㄫ→ㄥ), un. (knowledge §3.2.3 / TL_FINALS).
        assert_eq!(adjust_initial_key("ㄇ", "ㄚ"), "ㆬ"); // am
        assert_eq!(adjust_initial_key("ㄋ", "ㄚ"), "ㄣ"); // an
        assert_eq!(adjust_initial_key("ㄫ", "ㄚ"), "ㆭ"); // ang
        assert_eq!(adjust_initial_key("ㄫ", "ㄧ"), "ㄥ"); // ing (ㄫ→ㄥ after ㄧ)
        assert_eq!(adjust_initial_key("ㄋ", "ㄍㄨ"), "ㄣ"); // kun valid
        assert_eq!(adjust_initial_key("ㄋ", "ㄒㄧ"), "ㄣ"); // sin valid
    }

    #[test]
    fn nasal_after_vowel_with_impossible_coda_keeps_initial() {
        // The 龜毛 fix: a nasal whose coda would be a non-syllable stays an
        // INITIAL so it begins the next syllable. `um`/`aum`/`aun`/`aung`/
        // `uing` are NOT in TL_FINALS.
        assert_eq!(adjust_initial_key("ㄇ", "ㄍㄨ"), "ㄇ"); // kum ✗ → 龜毛 ku-môo
        assert_eq!(adjust_initial_key("ㄇ", "ㄍㄠ"), "ㄇ"); // kaum ✗
        assert_eq!(adjust_initial_key("ㄋ", "ㄍㄠ"), "ㄋ"); // kaun ✗
        assert_eq!(adjust_initial_key("ㄫ", "ㄍㄠ"), "ㄫ"); // kaung ✗ (ㆭ glyph, last≠ㄧ)
        assert_eq!(adjust_initial_key("ㄫ", "ㄍㄨㄧ"), "ㄫ"); // kuing ✗ (ㄥ glyph, last=ㄧ)
    }

    #[test]
    fn nasal_rule2b_interaction_is_benign() {
        // Gate keeps `ㄇ` after `ㄍㄨ` (kum invalid). If a tone mark then
        // arrives, Rule 2b (`adjust`) folds the kept `ㄇ` to ㆬ →
        // byte-identical to the pre-fix blanket fold (`ㄍㄨㆬ`); still the
        // same invalid syllable, no working-word regression.
        assert_eq!(adjust_initial_key("ㄇ", "ㄍㄨ"), "ㄇ"); // kept by gate
        let (adjusted, replace_last) = super::adjust("\u{02cb}", "ㄍㄨㄇ"); // tone 2 after kept ㄇ
        assert_eq!(adjusted, "\u{02cb}");
        assert_eq!(replace_last.as_deref(), Some("ㆬ")); // Rule 2b reconstructs the coda
    }

    #[test]
    fn defold_is_inverse_of_dual_final_form() {
        use super::{defold_coda_to_initial, dual_final_form};
        // Every dual-form initial → its coda → back to the initial.
        // `last` only matters for ㄫ (ㄧ→ㄥ else ㆭ); both ㄥ and ㆭ defold to ㄫ.
        for (initial, last) in [
            ('ㄅ', 'ㄚ'),
            ('ㄉ', 'ㄚ'),
            ('ㄍ', 'ㄚ'),
            ('ㄏ', 'ㄚ'),
            ('ㄇ', 'ㄚ'),
            ('ㄋ', 'ㄚ'),
            ('ㄫ', 'ㄚ'), // → ㆭ → ㄫ
            ('ㄫ', 'ㄧ'), // → ㄥ → ㄫ
        ] {
            let coda = dual_final_form(initial, last).expect("dual-form initial has a coda");
            assert_eq!(
                defold_coda_to_initial(coda),
                Some(initial),
                "{initial}+{last} → {coda} must defold back to {initial}",
            );
        }
        // Non-coda chars return None.
        assert_eq!(defold_coda_to_initial('ㄚ'), None);
        assert_eq!(defold_coda_to_initial('ㄍ'), None); // onset, not a coda
    }

    // INVARIANT_TPS_TONE9_DIGIT_TO_MARK — the digit `9` is the only input
    // affordance for tone-9 (no `ˆ` key in either layout); convert it to the
    // `ˆ` mark when a toneable syllable body is pending so tone-9 dictionary
    // words (昨昏 ㄗㄤˆ, 才 ㄘㄞˆ) become reachable.

    #[test]
    fn tone_nine_digit_becomes_mark_after_syllable_body() {
        // Nucleus / nasal-coda / nasalized-vowel finals all carry tone-9.
        for raw in [
            "ㄗㄤ",   // tsang → 昨昏
            "ㄘㄞ",   // tshai → 才
            "ㄗㄨ",   // tsu → 喌
            "ㄒㄧㄢ", // sian (an nasal-coda final) → 日語借詞 せんせい
            "ㄍㄚ",   // bare onset+vowel
            "ㆦ",     // zero-onset vowel
        ] {
            assert_eq!(
                adjust_tone_nine_digit("9", raw),
                "\u{02c6}",
                "{raw}+9 should become the ˆ tone-9 mark",
            );
        }
    }

    #[test]
    fn tone_nine_digit_stays_literal_when_not_toneable() {
        // No pending syllable, already toned, entering-tone stop coda, space,
        // or non-Bopomofo context → keep the literal `9`.
        for raw in [
            "",               // standalone digit (committed verbatim upstream)
            "ㄗㄤ\u{02c6}",   // already tone-9 — no double-toning
            "ㄍㄚ\u{02cb}",   // already tone-2
            "ㄍㄚㆵ",         // kat — entering-tone stop coda (tone 4/8 only)
            "ㄗㄤ ",          // after a space boundary
            "abc",            // non-Bopomofo
        ] {
            assert_eq!(
                adjust_tone_nine_digit("9", raw),
                "9",
                "{raw:?}+9 should stay the literal digit",
            );
        }
    }

    #[test]
    fn tone_nine_only_digit_nine_converts() {
        // Scope = tone-9 only (USER 2026-06-14). Other digits pass through
        // unchanged even after a toneable body.
        for digit in ["1", "2", "3", "4", "5", "6", "7", "8", "0"] {
            assert_eq!(
                adjust_tone_nine_digit(digit, "ㄗㄤ"),
                digit,
                "digit {digit} must not be rewritten",
            );
        }
    }

    #[test]
    fn tone_nine_digit_through_adjust_entry() {
        // End-to-end through the collapsed entry point: produces the mark and
        // no retroactive last-char replacement for a plain nucleus body.
        let (adjusted, replace_last) = super::adjust("9", "ㄗㄤ");
        assert_eq!(adjusted, "\u{02c6}");
        assert_eq!(replace_last, None);
        // Standalone digit stays literal.
        assert_eq!(super::adjust("9", ""), ("9".to_string(), None));
    }

    #[test]
    fn tone_nine_digit_folds_kept_nasal_via_rule_2b() {
        // Rule 2b parity: a `9` that becomes `ˆ` after a gate-kept ㄇ folds it
        // to ㆬ exactly like a directly-typed tone mark (mirrors
        // `nasal_rule2b_interaction_is_benign`).
        assert_eq!(adjust_initial_key("ㄇ", "ㄍㄨ"), "ㄇ"); // kum invalid → kept
        let (adjusted, replace_last) = super::adjust("9", "ㄍㄨㄇ");
        assert_eq!(adjusted, "\u{02c6}");
        assert_eq!(replace_last.as_deref(), Some("ㆬ"));
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
