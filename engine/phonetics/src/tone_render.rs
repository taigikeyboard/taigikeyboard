//! Tone rendering: POJ double-tap preprocessing, typed tone digits → marks
//! (`normalize_tone`, `to_tone_marks`, `tone_digits_to_marks`) and back to numbers.

use crate::api::{composing_mode, is_letter_like, InputMode};
use crate::case_transform::{apply_nasal_marker_case, match_case};
use crate::syllable::{is_stop_tone, normalize_to_tl, split_initial_final, strip_tone_mark};
use crate::tps::is_zhuyin;
use protos::engine::AppConfig;
use unicode_normalization::UnicodeNormalization;

/// POJ doubletap preprocessing: `nn`→nasal marker + `oo`→`o\u{0358}`, gated
/// by `AppConfig.{oo,nn}_doubletap_enabled`. No-op for non-POJ modes.
///
/// **The `nn` fold must run FIRST.** It only fires when the character
/// immediately before `nn` is an ASCII vowel ([`convert_nasal_double_n`]), and
/// the `oo` fold inserts a combining mark (`U+0358`) exactly there — so with
/// `oo` first, `hoonn` became `ho͘nn` and the `nn` was stranded, while `honn`
/// (no `oo` to fold) converted fine. The two affordances are independent
/// per-key rewrites and both should fire: `hoonn` → `hooⁿ` → `ho͘ⁿ`.
///
/// The reverse ordering is safe in a way the old one was not: the `nn` fold
/// only ever CONTRACTS `nn` to `ⁿ`, so it cannot manufacture an `oo` for the
/// second pass to misread. It also matches the canonical converter, whose
/// `POJ_FINAL_SUBS` has ordered `nn`→`ⁿ` ahead of `oo`→`o͘` all along
/// (`taigi-converter/src/tables.js`, mirrored at `tables.rs`
/// `POJ_FINAL_SUBSTITUTIONS`).
pub(crate) fn preprocess_for_normalize_tone(
    input: &str,
    mode: InputMode,
    config: &AppConfig,
) -> String {
    if !matches!(mode, InputMode::Poj) {
        return input.to_string();
    }
    let mut s = input.to_string();
    if config.nn_doubletap_enabled {
        s = convert_nasal_double_n(&s);
    }
    if config.oo_doubletap_enabled {
        s = fold_double_o(&s);
    }
    s
}

/// Double-tapped `o` → `o\u{0358}`, case-insensitive on both taps, with the
/// FIRST tap's case deciding the letter (`Oo` and `OO` both give `O\u{0358}`,
/// `oo` and `oO` both give `o\u{0358}`).
///
/// A single left-to-right scan, pairing non-greedily — the same shape as
/// [`convert_nasal_double_n`]. It replaces three sequential `String::replace`
/// passes (`oo`, `Oo`, `OO`) that had two defects between them:
///
/// - `oO` matched no pass at all, so a shift between the two taps silently
///   produced nothing;
/// - the passes ate each other's output. `OOoo` folded to `O\u{0358}o\u{0358}`
///   in pass 1, whose `Oo` seam pass 2 then matched, and whose `OO` seam pass 3
///   matched again — three combining dots on one letter (`O\u{0358}\u{0358}\u{0358}`).
///
/// Scanning once cannot re-read what it just wrote, so both are structural
/// rather than patched case by case.
fn fold_double_o(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut result = String::with_capacity(input.len());
    let mut i = 0;
    while i < chars.len() {
        if i + 1 < chars.len() && matches!(chars[i], 'o' | 'O') && matches!(chars[i + 1], 'o' | 'O')
        {
            result.push(chars[i]);
            result.push('\u{0358}');
            i += 2;
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }
    result
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
/// permissive tone-mark application → nasal-marker case adjustment. `composing::derived`
/// calls this directly. Plan §3.2a.
///
/// The last step is [`apply_nasal_marker_case`], which adjusts the nasal
/// marker after preserving the user's typed case during tone placement.
pub fn normalize_tone(input: &str, config: &AppConfig) -> String {
    let mode = composing_mode(config);
    let preprocessed = preprocess_for_normalize_tone(input, mode, config);
    let tone_marked = if renders_roman_tones(config, mode) {
        crate::permissive_tone::apply(&preprocessed, mode)
    } else {
        to_tone_marks(&preprocessed, mode)
    };
    apply_nasal_marker_case(&tone_marked, config.force_lowercase_nasal_marker).into_owned()
}

/// Stable text component of a literal candidate's learning identity.
///
/// Keep the pre-permissive normalization contract for stored keys: in
/// particular, internal tone digits and explicit tones 1/4 remain present.
/// Rendering can hide those boundaries (`a1i1` and `ai1` both show `ai`),
/// but frequency and association learning must still distinguish them and
/// reuse existing records. This is an identity transform, not display text.
pub fn literal_learning_text(input: &str, config: &AppConfig) -> String {
    let mode = composing_mode(config);
    let preprocessed = preprocess_for_normalize_tone(input, mode, config);
    let tone_marked = to_tone_marks(&preprocessed, mode);
    apply_nasal_marker_case(&tone_marked, config.force_lowercase_nasal_marker).into_owned()
}

fn renders_roman_tones(config: &AppConfig, mode: InputMode) -> bool {
    !config.is_tps_layout() && matches!(mode, InputMode::Tl | InputMode::Poj)
}

/// Raw byte offsets of tone digits consumed by permissive normalization.
/// Caret projection uses the renderer's decisions rather than matching a
/// hidden tone digit to an identical digit that remains visible (`a2b2` → `áb2`).
pub fn consumed_tone_digit_offsets(input: &str, config: &AppConfig) -> Vec<usize> {
    let mode = composing_mode(config);
    if !renders_roman_tones(config, mode) {
        return Vec::new();
    }
    let preprocessed = preprocess_for_normalize_tone(input, mode, config);
    // POJ preprocessing folds letters only: digit order is preserved, while
    // their byte offsets can change. Project each decision back to raw input.
    input
        .char_indices()
        .filter(|&(_, ch)| matches!(ch, '1'..='9'))
        .zip(crate::permissive_tone::consumed_digits(&preprocessed, mode))
        .filter_map(|((offset, _), consumed)| consumed.then_some(offset))
        .collect()
}

/// Convert hyphen-separated syllables to tone marks. Tone digits 1 and 4 are
/// kept as-is — matches the keyboard convention in iOS `convertSyllable` and
/// Android `convertSyllable`.
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

/// A custom-dictionary roman as it is stored: each hyphen- or space-separated
/// token that is ONE syllable closed by a tone digit takes the tone mark the
/// digit names — `tsui2-ong5-tshut` → `tsuí-ông-tshut`, `tshut4` → `tshut`
/// (tones 1 / 4 carry no mark). The letters, their case and the nasal / `o͘`
/// encodings stay as written; only the digit becomes a mark, placed by the
/// token's own spelling (POJ placement when the token is POJ-spelled, TL
/// otherwise — an ambiguous `kui2` is TL `kuí`). Everything else is kept
/// verbatim: a token already marked, a fused run (`tsui2ong5tshut` — splitting
/// it would invent syllable boundaries the user did not write), a non-syllable,
/// and a TPS roman. Output is NFC.
pub fn tone_digits_to_marks(roman: &str) -> String {
    let roman: String = roman.nfc().collect();
    if is_zhuyin(&roman) || !roman.bytes().any(|byte| matches!(byte, b'1'..=b'9')) {
        return roman;
    }
    let mut out = String::with_capacity(roman.len());
    let mut token = String::new();
    for ch in roman.chars() {
        if ch == '-' || ch == ' ' {
            out.push_str(&token_with_tone_mark(&token));
            out.push(ch);
            token.clear();
        } else {
            token.push(ch);
        }
    }
    out.push_str(&token_with_tone_mark(&token));
    out
}

fn token_with_tone_mark(token: &str) -> String {
    let Some(base) = token.strip_suffix(|ch: char| matches!(ch, '1'..='9')) else {
        return token.to_string();
    };
    let is_one_unmarked_syllable = !base.chars().any(|ch| ch.is_ascii_digit())
        && !base.nfd().any(crate::normalization::is_combining_tone_mark)
        && crate::syllable::is_valid_syllable(base);
    if !is_one_unmarked_syllable {
        return token.to_string();
    }
    let mode = if is_poj_spelled(base) {
        InputMode::Poj
    } else {
        InputMode::Tl
    };
    crate::permissive_tone::apply(token, mode)
}

/// Spelling only POJ writes (`docs/phonetics/taigi-phonetics-reference.md`
/// § TL vs POJ): the `ch` initials, the `oa` / `oe` vowel pairs (TL `ua` /
/// `ue`), the `ek` final (TL `ik`), and the `o͘` / `ⁿ` encodings. `eng` is not
/// evidence — TL writes it as a special final.
fn is_poj_spelled(syllable: &str) -> bool {
    let lowered = syllable.to_lowercase();
    lowered.starts_with("ch")
        || lowered.contains("oa")
        || lowered.contains("oe")
        || lowered.ends_with("ek")
        || lowered.contains('\u{0358}')
        || lowered.contains('\u{207F}')
        || lowered.contains('\u{1D3A}')
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
    if matches!(mode, InputMode::English | InputMode::Tps) {
        return syllable.to_string();
    }
    if tone_digit == 1 || tone_digit == 4 {
        return syllable.to_string();
    }
    let tone = tone_digit.to_string();
    let lowered = base.to_lowercase();
    // Validate the token is ONE syllable via the canonical tables — normalize
    // POJ→TL only to CHECK validity (recognizes `goa`/`chiah`/`ere`/`iri`, and
    // rejects unhyphenated multi-syllable blobs `goa2ai3li` + garbage `xyz`).
    // The engine does NOT auto-syllabify (§10.2), so an invalid token is left
    // verbatim. Crucially the tone mark is placed on the LITERAL spelling below,
    // so a recognized syllable keeps its typed letters (no conversion in the
    // output).
    if split_initial_final(&normalize_to_tl(&lowered)).is_none() {
        return syllable.to_string();
    }

    // LITERAL composing display (both TL and POJ): place the tone mark directly
    // on the typed letters — NO spelling conversion. The user sees exactly what
    // they typed, only the tone digit becomes a tone mark. `teng2`→`téng`,
    // `goa2`→`goá` (TL) / `góa` (POJ), `ting2`→`tíng`. The mode only selects the
    // tone-mark glyph + the vowel-priority placement convention; it never
    // rewrites the spelling family (`ch`↔`ts`, `oa`↔`ua`, `eng`↔`ing` all
    // stay). The cross-mode `user_frequency.db` / NextWord identity is computed
    // separately via `canonical_tl_form` (still canonical), so this literal
    // display does not affect mode-independent learning.
    let placed = if matches!(mode, InputMode::Tl) {
        crate::tl::apply_tl_tone_literal(&lowered, &tone)
    } else {
        // POJ (English / TPS already returned above).
        crate::poj::apply_poj_tone_literal(&lowered, &tone)
    };
    // Letter by letter, so Caps Lock `SIANN5` → `SIÂⁿ` (nasal marker re-cased
    // to `ᴺ` by `adjust_nasal_marker_case` afterwards), not `Siâⁿ`.
    match_case(&placed, &base, mode)
}

/// Convert tone-marked text to numeric-tone form. Mirrors `toToneNumber` in
/// `converter.js`, including the NFD / per-syllable boundary scan.
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

fn is_combining(c: char) -> bool {
    matches!(c, '\u{0300}'..='\u{036f}')
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The premise `composing::derived::display_caret_utf16` aligns raw and
    /// display on: the chain may insert combining marks, drop a character
    /// (tone digit, second tap of a POJ `oo` / `nn`) and change case, but it
    /// never writes a base letter the input does not have — so the display's
    /// base letters stay a subsequence of the input's. Pinned here, next to
    /// the code that would break it (an auto-inserted glide, a spelling
    /// rewrite), rather than three crates downstream.
    #[test]
    fn normalize_tone_never_adds_a_base_letter() {
        fn base_letters(text: &str) -> Vec<char> {
            text.chars()
                .filter(|c| unicode_normalization::char::canonical_combining_class(*c) == 0)
                .map(|c| c.nfkd().next().unwrap_or(c))
                .flat_map(char::to_lowercase)
                .collect()
        }
        fn is_subsequence(needle: &[char], haystack: &[char]) -> bool {
            let mut rest = haystack.iter();
            needle.iter().all(|c| rest.any(|h| h == c))
        }
        let poj_doubletap = AppConfig {
            input_mode: "poj".to_string(),
            oo_doubletap_enabled: true,
            nn_doubletap_enabled: true,
            ..AppConfig::default()
        };
        let configs = [
            AppConfig {
                input_mode: "tl".to_string(),
                ..AppConfig::default()
            },
            AppConfig {
                input_mode: "poj".to_string(),
                ..AppConfig::default()
            },
            poj_doubletap,
        ];
        let inputs = [
            "tai5",
            "ng5",
            "ka2i",
            "Tai5-gI2",
            "hoon",
            "hooon",
            "tinn",
            "tinnn",
            "koo2",
            "chhiunn5",
            "goa2",
            "ere2",
            "iri2",
            "m7",
            "lauh8",
            "xyz2",
            "OOoo",
            "ho\u{358}o",
        ];
        for config in &configs {
            for input in inputs {
                let output = normalize_tone(input, config);
                assert!(
                    is_subsequence(&base_letters(&output), &base_letters(input)),
                    "{input:?} → {output:?} under {} added a base letter",
                    config.input_mode
                );
            }
        }
    }

    // trace: TL literal — `teng2` → `apply_tl_tone_literal("teng","2")` places
    // acute on `e` (eng has no a/oo/ere before e) → "téng". `goa2` → place on
    // `a` → "goá" (letters kept, NOT folded to `guá`). No spelling conversion.
    #[test]
    fn to_tone_marks_tl_is_literal() {
        assert_eq!(to_tone_marks("teng2", InputMode::Tl), "t\u{e9}ng");
        assert_eq!(to_tone_marks("goa2", InputMode::Tl), "go\u{e1}");
    }

    // trace: POJ literal — `apply_poj_tone_literal` places the POJ mark on the
    // typed letters, no spelling conversion. `ting2` → "tíng" (NOT folded to
    // `téng`); `teng2` → "téng"; `goa2` → "góa" (POJ mark on `o`); `chiah8` →
    // "chia̍h" (ch kept, mark on `a` of the `ia` pair).
    #[test]
    fn to_tone_marks_poj_is_literal() {
        assert_eq!(to_tone_marks("ting2", InputMode::Poj), "t\u{ed}ng");
        assert_eq!(to_tone_marks("teng2", InputMode::Poj), "t\u{e9}ng");
        assert_eq!(to_tone_marks("goa2", InputMode::Poj), "g\u{f3}a");
        assert_eq!(to_tone_marks("chiah8", InputMode::Poj), "chia\u{30d}h");
        // `au` keeps its `a` nucleus before a coda (落 la̍uh), unlike the
        // `oa` / `oe` closed-syllable exception.
        assert_eq!(to_tone_marks("lauh8", InputMode::Poj), "la\u{30d}uh");
    }

    // trace: the single-syllable validator is `split_initial_final ∘
    // normalize_to_tl`, which recognizes a special final like `ere` (a TL final
    // with a consonant `r` between two vowels) so the literal mark still lands:
    // `ere2` → place on the `ere` cluster → "eré". Garbage (`xyz2`) and
    // unhyphenated multi-syllable (`goaai3` / internal digits) fail the split
    // and stay verbatim (engine does not auto-syllabify, §10.2) — the tone digit
    // is never silently dropped.
    #[test]
    fn to_tone_marks_special_final_and_invalid_tokens() {
        // `ere` is a valid special TL final → literal mark on the second `e`.
        assert_eq!(to_tone_marks("ere2", InputMode::Tl), "er\u{e9}");
        // Garbage with no valid syllable stays verbatim (digit kept, not lost).
        assert_eq!(to_tone_marks("xyz2", InputMode::Tl), "xyz2");
        // Unhyphenated multi-syllable stays verbatim (no auto-syllabify).
        assert_eq!(to_tone_marks("goa2ai3li2", InputMode::Tl), "goa2ai3li2");
    }

    #[test]
    fn tone_digits_to_marks_marks_each_syllable_closed_by_a_digit() {
        // trace: each token = one valid syllable + one digit → the TL rules
        // (`place_tl_tone_mark`) unless the token is POJ-spelled; tones 1/4
        // carry no mark, so their digit just goes.
        let cases = [
            ("tsui2-ong5-tshut", "tsuí-ông-tshut"),
            ("tsui1-tshut4", "tsui-tshut"),
            ("ia7 si7", "iā sī"),
            ("tai5-gi2", "tâi-gí"),
            ("teng2", "téng"),
            // `ui` is spelled the same in TL and POJ: TL placement.
            ("kui2", "kuí"),
            // POJ-spelled → POJ placement: `ch` initial, open `oa`, `oa` + coda.
            ("chui2", "chúi"),
            ("goa2", "góa"),
            ("hoat8", "hoa\u{30d}t"),
            ("sin1-o\u{358}2", "sin-ó\u{358}"),
            ("si\u{207f}2", "sí\u{207f}"),
            ("Chhoa7", "Chhōa"),
            // Tone 9 is the one mark the systems spell apart: TL double
            // acute, POJ breve (`pek` is POJ-spelled, TL `pik`).
            ("tsa9", "tsa\u{30b}"),
            ("pek9", "p\u{115}k"),
            // TL letters stay TL: `oo`, `nn`, the dialectal `iri`, syllabic
            // nasals.
            ("oo2", "óo"),
            ("ann2", "ánn"),
            ("iri5", "irî"),
            ("ng7", "n\u{304}g"),
            ("m7", "m\u{304}"),
            ("hm7", "hm\u{304}"),
            // The letters' case stays.
            ("Tai5-gi2", "Tâi-gí"),
            ("TAI5", "T\u{c2}I"),
        ];
        for (roman, stored) in cases {
            assert_eq!(tone_digits_to_marks(roman), stored, "{roman}");
            assert_eq!(tone_digits_to_marks(stored), stored, "{stored} is stable");
        }
    }

    #[test]
    fn tone_digits_to_marks_keeps_what_is_not_one_digit_closed_syllable() {
        for roman in [
            // Fused run: no boundary the user wrote.
            "tsui2ong5tshut",
            // Already marked: the digit is not a second tone.
            "tsuí2",
            // Not a syllable / a number.
            "xyz2",
            "covid19",
            "win10",
            "mp3",
            // No digit, TPS.
            "tâi-gí",
            "ㄊㄞˊ",
        ] {
            assert_eq!(tone_digits_to_marks(roman), roman, "{roman}");
        }
        // NFC whatever else happens.
        assert_eq!(tone_digits_to_marks("li\u{301}"), "l\u{ed}");
    }
}
