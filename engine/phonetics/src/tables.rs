//! Shared phonetic tables (TL initials/finals, POJ substitutions, tone
//! diacritics). TPS / Zhuyin tables live alongside the converter that
//! consumes them in `tps.rs`. Ported 1:1 from `taigi-converter/src/tables.js`.

use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};

pub(crate) static TL_INITIALS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        "p", "ph", "m", "b", "t", "th", "n", "l", "k", "kh", "ng", "g", "ts", "tsh", "s", "j", "h",
        "",
    ]
    .into_iter()
    .collect()
});

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
pub(crate) const TL_TONE9_COMBINING: &str = "\u{030b}";

/// Combining-mark scalar -> tone number. Includes both POJ breve (U+0306) and
/// TL double acute (U+030B) for tone 9.
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
pub(crate) static POJ_INITIAL_FROM_TL: Lazy<HashMap<&'static str, &'static str>> =
    Lazy::new(|| [("ts", "ch"), ("tsh", "chh")].into_iter().collect());

/// TL final -> POJ final substitutions. Order matters: `nn` before `oo` so
/// `oonn` does not collapse into `oo + nn`.
pub(crate) const POJ_FINAL_SUBSTITUTIONS: &[(&str, &str)] = &[
    ("nn", "\u{207f}"),
    ("oo", "o\u{0358}"),
    ("ua", "oa"),
    ("ue", "oe"),
    ("ing", "eng"),
    ("ik", "ek"),
];

/// Resolve TL combining mark for a tone digit. Tone 9 is TL-specific (double acute).
pub(crate) fn tl_tone_mark(tone: &str) -> &'static str {
    if tone == "9" {
        TL_TONE9_COMBINING
    } else {
        TONE_NUM_TO_COMBINING.get(tone).copied().unwrap_or("")
    }
}

/// Resolve POJ combining mark for a tone digit. Tone 9 is the breve already in
/// the table.
pub(crate) fn poj_tone_mark(tone: &str) -> &'static str {
    TONE_NUM_TO_COMBINING.get(tone).copied().unwrap_or("")
}
