//! Shared phonetic tables (TL initials/finals, POJ substitutions, tone
//! diacritics). TPS / Zhuyin tables live alongside the converter that
//! consumes them in `tps.rs`. Ported 1:1 from `taigi-converter/src/tables.js`.

// 共用表音資料表 (TL 聲母/韻母、POJ 替換、聲調符號)。TPS/注音表放在 tps.rs。

use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};

// TL 聲母合法集合,用來判斷音節起頭是否合法。
pub(crate) static TL_INITIALS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        "p", "ph", "m", "b", "t", "th", "n", "l", "k", "kh", "ng", "g", "ts", "tsh", "s", "j", "h",
        "",
    ]
    .into_iter()
    .collect()
});

// TL 韻母合法集合,涵蓋鼻化、入聲、雙韻、ee/er/ir 等地區變體。
pub(crate) static TL_FINALS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        "a", "ah", "ap", "at", "ak", "ann", "annh", "am", "an", "ang", "e", "eh", "enn", "ennh",
        "i", "ih", "ip", "it", "ik", "inn", "innh", "im", "in", "ing", "o", "oh", "oo", "ooh",
        "op", "ok", "om", "ong", "onn", "onnh", "u", "uh", "ut", "un", "ai", "aih", "ainn",
        "ainnh", "au", "auh", "aunn", "aunnh", "ia", "iah", "iap", "iat", "iak", "iam", "ian",
        "iang", "iann", "iannh", "io", "ioh", "iok", "iong", "ionn", "iu", "iuh", "iut", "iunn",
        "iunnh", "ua", "uah", "uat", "uak", "uan", "uann", "uannh", "ue", "ueh", "uenn", "uennh",
        "ui", "uih", "uinn", "uinnh", "iau", "iauh", "iaunn", "iaunnh", "uai", "uaih", "uainn",
        "uainnh", "m", "mh", "ng", "ngh", "ioo", "iooh", "iai", "iaih", "er", "erh", "erk", "erm",
        "ere", "ereh", "eng", "ir", "irh", "irp", "irt", "irk", "irm", "irn", "irng", "iri",
        "irinn", "ie", "or", "orh", "ior", "iorh", "uang", "oi", "oih", "ee", "eeh",
    ]
    .into_iter()
    .collect()
});

/// Tone number -> NFD combining mark. Tones 1 and 4 carry no mark.
/// Tone 9 here is the POJ form (breve U+0306); TL overrides via `tl_tone_mark`.
// 聲調數字 → NFD 組合符號;1、4 聲不帶符號;此處 9 聲為 POJ 形式 (U+0306),TL 用 `tl_tone_mark` 覆蓋。
pub(crate) static TONE_NUM_TO_COMBINING: Lazy<HashMap<&'static str, &'static str>> =
    Lazy::new(|| {
        [
            ("1", ""),
            ("2", "\u{0301}"),
            ("3", "\u{0300}"),
            ("4", ""),
            ("5", "\u{0302}"),
            ("6", "\u{030c}"),
            ("7", "\u{0304}"),
            ("8", "\u{030d}"),
            ("9", "\u{0306}"),
        ]
        .into_iter()
        .collect()
    });

/// TL tone 9 uses double acute accent (U+030B).
// TL 第 9 聲使用雙重銳音符 (U+030B)。
pub(crate) const TL_TONE9_COMBINING: &str = "\u{030b}";

/// Combining-mark scalar -> tone number. Includes both POJ breve (U+0306) and
/// TL double acute (U+030B) for tone 9.
// 反向表 — 組合符號 → 聲調數字;9 聲同時收 POJ (U+0306) 與 TL (U+030B) 兩種寫法。
pub(crate) static COMBINING_TO_TONE_NUM: Lazy<HashMap<char, &'static str>> = Lazy::new(|| {
    [
        ('\u{0301}', "2"),
        ('\u{0300}', "3"),
        ('\u{0302}', "5"),
        ('\u{030c}', "6"),
        ('\u{0304}', "7"),
        ('\u{030d}', "8"),
        ('\u{0306}', "9"),
        ('\u{030b}', "9"),
    ]
    .into_iter()
    .collect()
});

/// TL initial -> POJ initial.
// TL 聲母 → POJ 聲母對應 (僅 ts/tsh 兩條規則)。
pub(crate) static POJ_INITIAL_FROM_TL: Lazy<HashMap<&'static str, &'static str>> =
    Lazy::new(|| [("ts", "ch"), ("tsh", "chh")].into_iter().collect());

/// TL final -> POJ final substitutions. Order matters: `nn` before `oo` so
/// `oonn` does not collapse into `oo + nn`.
// TL 韻母 → POJ 韻母替換規則,順序有意義 (`nn` 必須在 `oo` 前)。
pub(crate) const POJ_FINAL_SUBSTITUTIONS: &[(&str, &str)] = &[
    ("nn", "\u{207f}"),
    ("oo", "o\u{0358}"),
    ("ua", "oa"),
    ("ue", "oe"),
    ("ing", "eng"),
    ("ik", "ek"),
];

/// Resolve TL combining mark for a tone digit. Tone 9 is TL-specific (double acute).
// 由聲調數字查 TL 用的組合符號 (9 聲走 TL 專屬的雙重銳音符)。
pub(crate) fn tl_tone_mark(tone: &str) -> &'static str {
    if tone == "9" {
        TL_TONE9_COMBINING
    } else {
        TONE_NUM_TO_COMBINING.get(tone).copied().unwrap_or("")
    }
}

/// Resolve POJ combining mark for a tone digit. Tone 9 is the breve already in
/// the table.
// 由聲調數字查 POJ 用的組合符號;9 聲為短音符 (已收在主表內)。
pub(crate) fn poj_tone_mark(tone: &str) -> &'static str {
    TONE_NUM_TO_COMBINING.get(tone).copied().unwrap_or("")
}
