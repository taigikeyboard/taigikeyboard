//! TPS / Zhuyin conversion — ported from `taigi-converter/src/zhuyin.js`.
//! The Zhuyin lookup tables and TPS punctuation map live in this module
//! because TPS is the only domain that owns them: this file consumes
//! all six tables and `tps_adjust` consumes the two tone tables. Keeping
//! them next to their primary consumer keeps `tables.rs` focused on
//! cross-module shared data (TL / tone diacritics).

use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Lookup tables — ported 1:1 from `taigi-converter/src/zhuyin.js`. Order is
// significant: longer keys appear first so `tsh` matches before `t`.
// ---------------------------------------------------------------------------

pub(crate) const ZHUYIN_INITIALS: &[(&str, &str)] = &[
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

pub(crate) const ZHUYIN_VOWELS: &[(&str, &str)] = &[
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
    // `or` defaults to ㄛ at runtime; this ㄜ entry is the toggle-ON form
    // selected by `to_zhuyin(_, _, or_maps_to_er = true)`.
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

pub(crate) const ZHUYIN_TONES: &[(&str, &str)] = &[
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

pub(crate) const ZHUYIN_TONES_ENCODE_SAFE: &[(&str, &str)] = &[
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

const PUNCTUATION_CHARS: &[&str] = &[
    "\u{ff0e}", "\u{300c}", "\u{300d}", "\u{ff0c}", "\u{3002}", "\u{ff1f}", "--", ",", ".", "?",
    "\"",
];

const PUNCTUATION_PAIRS: &[(&str, &str)] = &[
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

static ZHUYIN_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new("[\u{3100}-\u{312f}\u{31a0}-\u{31bf}]").unwrap());

// Reverse-direction tables (TPS → TL). Built once at first use; `from_zhuyin`
// previously rebuilt these on every call (~150 String allocations per syllable).
static REV_INITIALS: Lazy<Vec<(&'static str, &'static str)>> = Lazy::new(|| {
    let mut rev: Vec<(&'static str, &'static str)> = ZHUYIN_INITIALS
        .iter()
        .map(|(tl, tps)| (*tps, *tl))
        .collect();
    rev.extend([
        ("\u{3110}", "ts"),
        ("\u{3111}", "tsh"),
        ("\u{3112}", "s"),
        ("\u{31a2}", "j"),
    ]);
    rev.sort_by_key(|entry| std::cmp::Reverse(entry.0.len()));
    rev
});

static REV_VOWELS: Lazy<Vec<(&'static str, &'static str)>> = Lazy::new(|| {
    let mut rev: Vec<(&'static str, &'static str)> =
        ZHUYIN_VOWELS.iter().map(|(tl, tps)| (*tps, *tl)).collect();
    rev.push(("\u{3125}", "ng"));
    rev.sort_by_key(|entry| std::cmp::Reverse(entry.0.len()));
    rev
});

static REV_TONES: Lazy<Vec<(&'static str, &'static str)>> = Lazy::new(|| {
    let mut map: HashMap<&'static str, &'static str> = HashMap::new();
    for table in [ZHUYIN_TONES, ZHUYIN_TONES_ENCODE_SAFE] {
        for (tl, tps) in table {
            map.entry(*tps).or_insert(*tl);
        }
    }
    let mut entries: Vec<(&'static str, &'static str)> = map.into_iter().collect();
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.0.len()));
    entries
});

pub(crate) fn is_zhuyin(text: &str) -> bool {
    ZHUYIN_RE.is_match(text)
}

/// Convert a single TL token (with tone digit) to TPS.
///
/// - `encode_safe = true` substitutes `\u{02d9}` for `\u{0307}` so TPS
///   round-trips through systems that strip combining marks.
/// - `or_maps_to_er = false` (default) renders the vowel `or` as ㄛ
///   (`\u{311b}`); `true` renders it as ㄜ (`\u{311c}`), matching the iOS
///   `orMapsToER` toggle. Override is per-token (only applied when the
///   matched vowel slot is exactly `"or"`); other vowels are unaffected.
pub(crate) fn to_zhuyin(text: &str, encode_safe: bool, or_maps_to_er: bool) -> String {
    let mut remaining: String = text.to_lowercase();
    let mut pre_punct = String::new();
    let mut consonant = String::new();
    let mut vowel = String::new();
    let mut tone = String::new();

    loop {
        let mut matched = false;
        for punct in PUNCTUATION_CHARS {
            if remaining.starts_with(punct) {
                pre_punct.push_str(punct);
                remaining = remaining[punct.len()..].to_string();
                matched = true;
                break;
            }
        }
        if !matched {
            break;
        }
    }

    for (tl, tps) in ZHUYIN_INITIALS {
        if remaining.starts_with(tl) {
            consonant.push_str(tps);
            remaining = remaining[tl.len()..].to_string();
            break;
        }
    }

    loop {
        let mut matched = false;
        for (tl, tps) in ZHUYIN_VOWELS {
            if remaining.starts_with(tl) {
                // Per-token override: when toggle is OFF, the vowel `or`
                // renders as ㄛ (matches iOS default). With toggle ON, fall
                // through to the table value (ㄜ).
                let effective_tps: &str = if *tl == "or" && !or_maps_to_er {
                    "\u{311b}"
                } else {
                    tps
                };
                vowel.push_str(effective_tps);
                remaining = remaining[tl.len()..].to_string();
                matched = true;
                break;
            }
        }
        if !matched {
            break;
        }
    }

    let tone_table: &[(&str, &str)] = if encode_safe {
        ZHUYIN_TONES_ENCODE_SAFE
    } else {
        ZHUYIN_TONES
    };
    for (tl, tps) in tone_table {
        if remaining.starts_with(tl) {
            tone.push_str(tps);
            remaining = remaining[tl.len()..].to_string();
            break;
        }
    }

    // Idiosyncratic TPS adjustments — see zhuyin.js:173-185.
    if vowel.is_empty() && consonant == "\u{3107}" {
        vowel = "\u{31ac}".to_string();
        consonant.clear();
    }
    if vowel.is_empty() && consonant == "\u{312b}" {
        vowel = "\u{31ad}".to_string();
        consonant.clear();
    }
    if vowel == "\u{3125}" && consonant.is_empty() {
        vowel = "\u{31ad}".to_string();
    }
    let cv = format!("{consonant}{vowel}");
    if cv.ends_with('\u{31ad}') && cv.chars().rev().nth(1) == Some('\u{3127}') {
        vowel = vowel.replace('\u{31ad}', "\u{3125}");
    }
    if consonant.ends_with('\u{3127}') && vowel == "\u{3123}\u{3123}" {
        consonant.pop();
        vowel = "\u{31aa}".to_string();
    }
    if vowel.contains('\u{311b}') && !tone.is_empty() {
        if let Some(first) = tone.chars().next() {
            if "\u{31b4}\u{31b5}\u{31bb}".contains(first) {
                vowel = vowel.replace('\u{311b}', "\u{31a6}");
            }
        }
    }

    let mut result = format!("{pre_punct}{consonant}{vowel}{tone}{remaining}");

    loop {
        let mut matched = false;
        for (tps_punct, tl_punct) in PUNCTUATION_PAIRS {
            if result.contains(tl_punct) {
                result = result.replace(tl_punct, tps_punct);
                matched = true;
                break;
            }
        }
        if !matched {
            break;
        }
    }

    result.replace("--", "\u{00b7}")
}

/// Convert a TPS string to a TL tone-numbered string. Mirrors `fromZhuyin` in
/// `zhuyin.js`. Word segmentation is **not** performed here — that is the
/// segmenter's job, which belongs with the Lexicon slice.
pub(crate) fn from_zhuyin(text: &str) -> String {
    let rev_punct = [
        ("\u{3002}", "."),
        ("\u{300c}", "\""),
        ("\u{300d}", "\""),
        ("\u{ff0c}", ","),
        ("\u{ff1f}", "?"),
        ("\u{ff0e}", "\u{00b7}"),
    ];
    let mut input = text.to_string();
    for (tps, ascii) in rev_punct {
        input = input.replace(tps, ascii);
    }

    let mut parts: Vec<Part> = Vec::new();
    let mut remaining = input;
    let mut initial = String::new();
    let mut vowel = String::new();

    while !remaining.is_empty() {
        let mut matched = false;

        for (tps, tl) in REV_TONES.iter() {
            if remaining.starts_with(*tps) {
                remaining = remaining[tps.len()..].to_string();
                if !initial.is_empty() || !vowel.is_empty() {
                    let mut syl = format!("{initial}{vowel}");
                    if initial == "m" && vowel == "m" {
                        syl = "m".to_string();
                    }
                    if syl.contains("oo") && is_pt_or_k_stop(tl) {
                        syl = syl.replacen("oo", "o", 1);
                    }
                    parts.push(Part::Syllable(format!("{syl}{tl}")));
                }
                initial.clear();
                vowel.clear();
                matched = true;
                break;
            }
        }
        if matched {
            continue;
        }

        if initial.is_empty() && vowel.is_empty() {
            for (tps, tl) in REV_INITIALS.iter() {
                if remaining.starts_with(*tps) {
                    initial = (*tl).to_string();
                    remaining = remaining[tps.len()..].to_string();
                    matched = true;
                    break;
                }
            }
            if matched {
                continue;
            }
        }

        for (tps, tl) in REV_VOWELS.iter() {
            if remaining.starts_with(*tps) {
                vowel.push_str(tl);
                remaining = remaining[tps.len()..].to_string();
                matched = true;
                break;
            }
        }
        if matched {
            continue;
        }

        if !initial.is_empty() || !vowel.is_empty() {
            let mut syl = format!("{initial}{vowel}");
            if initial == "m" && vowel == "m" {
                syl = "m".to_string();
            }
            parts.push(Part::Syllable(syl));
            initial.clear();
            vowel.clear();
        }
        if let Some(c) = remaining.chars().next() {
            parts.push(Part::Other(c));
            remaining = remaining[c.len_utf8()..].to_string();
        }
    }

    if !initial.is_empty() || !vowel.is_empty() {
        let mut syl = format!("{initial}{vowel}");
        if initial == "m" && vowel == "m" {
            syl = "m".to_string();
        }
        parts.push(Part::Syllable(syl));
    }

    let mut out = String::new();
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            if let (Part::Syllable(_), Part::Syllable(_)) = (&parts[i - 1], part) {
                out.push('-');
            }
        }
        match part {
            Part::Syllable(s) => out.push_str(s),
            Part::Other(c) => out.push(*c),
        }
    }
    out
}

enum Part {
    Syllable(String),
    Other(char),
}

fn is_pt_or_k_stop(tl: &str) -> bool {
    matches!(tl, "p4" | "t4" | "k4" | "p8" | "t8" | "k8")
}
