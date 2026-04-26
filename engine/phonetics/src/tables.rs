//! Phonetic tables. Ported 1:1 from `taigi-converter/src/tables.js`.

use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};

pub static TL_INITIALS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        "p", "ph", "m", "b", "t", "th", "n", "l", "k", "kh", "ng", "g", "ts", "tsh", "s", "j", "h",
        "",
    ]
    .into_iter()
    .collect()
});

pub static TL_FINALS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
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
pub static TONE_NUM_TO_COMBINING: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
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
pub const TL_TONE9_COMBINING: &str = "\u{030b}";

/// Combining-mark scalar -> tone number. Includes both POJ breve (U+0306) and
/// TL double acute (U+030B) for tone 9.
pub static COMBINING_TO_TONE_NUM: Lazy<HashMap<char, &'static str>> = Lazy::new(|| {
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
pub static POJ_INITIAL_FROM_TL: Lazy<HashMap<&'static str, &'static str>> =
    Lazy::new(|| [("ts", "ch"), ("tsh", "chh")].into_iter().collect());

/// TL final -> POJ final substitutions. Order matters: `nn` before `oo` so
/// `oonn` does not collapse into `oo + nn`.
pub const POJ_FINAL_SUBSTITUTIONS: &[(&str, &str)] = &[
    ("nn", "\u{207f}"),
    ("oo", "o\u{0358}"),
    ("ua", "oa"),
    ("ue", "oe"),
    ("ing", "eng"),
    ("ik", "ek"),
];

/// Resolve TL combining mark for a tone digit. Tone 9 is TL-specific (double acute).
pub fn tl_tone_mark(tone: &str) -> &'static str {
    if tone == "9" {
        TL_TONE9_COMBINING
    } else {
        TONE_NUM_TO_COMBINING.get(tone).copied().unwrap_or("")
    }
}

/// Resolve POJ combining mark for a tone digit. Tone 9 is the breve already in
/// the table.
pub fn poj_tone_mark(tone: &str) -> &'static str {
    TONE_NUM_TO_COMBINING.get(tone).copied().unwrap_or("")
}

// MARK: - TPS (Zhuyin) tables — ported from zhuyin.js. Order matters: longer
// keys appear first so `tsh` matches before `t`.

pub const ZHUYIN_INITIALS: &[(&str, &str)] = &[
    ("tshi", "\u{3111}\u{3127}"),
    ("tsi", "\u{3110}\u{3127}"),
    ("tsh", "\u{3118}"),
    ("ph", "\u{3106}"),
    ("th", "\u{310a}"),
    ("ts", "\u{3117}"),
    ("si", "\u{3112}\u{3127}"),
    ("ji", "\u{31a2}\u{3127}"),
    ("kh", "\u{310e}"),
    ("ng", "\u{312b}"),
    ("p", "\u{3105}"),
    ("m", "\u{3107}"),
    ("b", "\u{31a0}"),
    ("t", "\u{3109}"),
    ("n", "\u{310b}"),
    ("l", "\u{310c}"),
    ("s", "\u{3119}"),
    ("j", "\u{31a1}"),
    ("k", "\u{310d}"),
    ("g", "\u{31a3}"),
    ("h", "\u{310f}"),
];

pub const ZHUYIN_VOWELS: &[(&str, &str)] = &[
    ("ainn", "\u{31ae}"),
    ("aunn", "\u{31af}"),
    ("ann", "\u{31a9}"),
    ("enn", "\u{31a5}"),
    ("inn", "\u{31aa}"),
    ("onn", "\u{31a7}"),
    ("unn", "\u{31ab}"),
    ("ang", "\u{3124}"),
    ("ong", "\u{31b2}"),
    ("oo", "\u{31a6}"),
    ("ee", "\u{311d}"),
    ("er", "\u{311c}"),
    ("or", "\u{311c}"),
    ("ir", "\u{31a8}"),
    ("ai", "\u{311e}"),
    ("au", "\u{3120}"),
    ("am", "\u{31b0}"),
    ("an", "\u{3122}"),
    ("om", "\u{31b1}"),
    ("ng", "\u{31ad}"),
    ("a", "\u{311a}"),
    ("e", "\u{31a4}"),
    ("i", "\u{3127}"),
    ("o", "\u{311b}"),
    ("u", "\u{3128}"),
    ("m", "\u{31ac}"),
    ("n", "\u{3123}"),
];

pub const ZHUYIN_TONES: &[(&str, &str)] = &[
    ("1", " "),
    ("2", "\u{02cb}"),
    ("3", "\u{02ea}"),
    ("p4", "\u{31b4}"),
    ("t4", "\u{31b5}"),
    ("k4", "\u{31bb}"),
    ("h4", "\u{31b7}"),
    ("5", "\u{02ca}"),
    ("6", "\u{02c7}"),
    ("7", "\u{02eb}"),
    ("p8", "\u{31b4}\u{0307}"),
    ("t8", "\u{31b5}\u{0307}"),
    ("k8", "\u{31bb}\u{0307}"),
    ("h8", "\u{31b7}\u{0307}"),
    ("8", "\u{0307}"),
    ("9", "\u{02c6}"),
];

pub const ZHUYIN_TONES_ENCODE_SAFE: &[(&str, &str)] = &[
    ("1", " "),
    ("2", "\u{02cb}"),
    ("3", "\u{02ea}"),
    ("p4", "\u{31b4}"),
    ("t4", "\u{31b5}"),
    ("k4", "\u{31bb}"),
    ("h4", "\u{31b7}"),
    ("5", "\u{02ca}"),
    ("6", "\u{02c7}"),
    ("7", "\u{02eb}"),
    ("p8", "\u{31b4}\u{02d9}"),
    ("t8", "\u{31b5}\u{02d9}"),
    ("k8", "\u{31bb}\u{02d9}"),
    ("h8", "\u{31b7}\u{02d9}"),
    ("8", "\u{02d9}"),
    ("9", "\u{02c6}"),
];

pub const PUNCTUATION_CHARS: &[&str] = &[
    "\u{ff0e}", "\u{300c}", "\u{300d}", "\u{ff0c}", "\u{3002}", "\u{ff1f}", "--", ",", ".", "?",
    "\"",
];

pub const PUNCTUATION_PAIRS: &[(&str, &str)] = &[
    ("\u{3002}", ". "),
    ("\u{3002}", "."),
    ("\u{300c}", "\""),
    ("\u{300d}", "\""),
    ("\u{ff0c}", ", "),
    ("\u{ff0c}", ","),
    ("\u{ff1f}", "? "),
    ("\u{ff1f}", "?"),
    ("\u{ff0e}", "\u{00b7} "),
    ("\u{ff0e}", "\u{00b7}"),
];
