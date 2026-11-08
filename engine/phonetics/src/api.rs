//! High-level Phonetics API for direct in-process callers (the dev
//! `cli` crate, integration tests, and `phonetics::requests::handle`).
//! The cross-platform FFI envelope lives in `engine/dispatch` per
//! `docs/contributing/rust-ffi-safety.md` §2; this module never decodes a
//! top-level `taigi.engine.Request` or owns a panic boundary.

use crate::case_transform::{apply_nasal_marker_case, match_case};
use crate::poj::to_poj;
use crate::syllable::{
    is_stop_tone, normalize_to_tl, normalize_to_tl_keep_tl_finals, split_initial_final,
    strip_tone_mark,
};
use crate::tl::to_tl;
use crate::tps::is_zhuyin;
use protos::engine::AppConfig;
use thiserror::Error;
use unicode_normalization::UnicodeNormalization;

// Keyboard input mode, mirroring the `AppConfig.input_mode` string. The TPS layout composes with
// the TL tables ([`composing_mode`]); `composing::requests` upgrades to `Tps` once
// `contains_tps(raw)` fires, so the `Tps` variant mostly circulates inside the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Tl,
    Poj,
    Tps,
    English,
}

// Target writing system of a conversion: TL / POJ romanization, or TPS Bopomofo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum System {
    Tl,
    Poj,
    Tps,
}

// Public error type; the single unsupported-op variant usually means proto schema drift.
#[derive(Debug, Error)]
pub enum PhoneticsError {
    #[error(
        "unsupported op (e.g. PhoneticsRequest.method is None — likely proto schema mismatch)"
    )]
    UnsupportedOp,
}

/// Whether text is punctuation that attaches before an auto space (§23).
pub fn is_attaching_punctuation(text: &str) -> bool {
    crate::punctuation::is_attaching_punctuation(text)
}

/// Whether text is a joiner, which takes the place of an auto space (§23).
pub fn is_joiner_punctuation(text: &str) -> bool {
    crate::punctuation::is_joiner_punctuation(text)
}

pub(crate) fn capitalize_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Translate the proto `AppConfig.input_mode` string into the typed enum.
/// Unknown / empty / "tl" → `Tl`. Mirrors `phonetics::requests::parse_input_mode`.
pub fn parse_input_mode(mode: &str) -> InputMode {
    match mode {
        "poj" | "POJ" => InputMode::Poj,
        "tps" | "TPS" => InputMode::Tps,
        "english" | "English" | "EN" => InputMode::English,
        _ => InputMode::Tl,
    }
}

/// The mode a request composes and cases under: [`parse_input_mode`], except
/// that the TPS layout (`AppConfig::is_tps_layout`) reads as `Tl` — a
/// Bopomofo-free TPS buffer (`-`, a lone tone mark) takes the TL tables, as it
/// did while platforms sent TPS as `"tl"`. Bopomofo content still upgrades to
/// `Tps` at the composing seam (`contains_tps`).
pub fn composing_mode(config: &AppConfig) -> InputMode {
    if config.is_tps_layout() {
        InputMode::Tl
    } else {
        parse_input_mode(&config.input_mode)
    }
}

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

/// `true` if the text contains TPS (Taiwanese Phonetic Symbols / Zhuyin)
/// codepoints. Used by composing-derived display to skip POJ/TL tone-mark
/// conversion (TPS strings are already display-ready).
pub fn contains_tps(text: &str) -> bool {
    is_zhuyin(text)
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

/// Numeric-tone TL (`"hoo2"`, `"tai5-gi2"`) → TPS Bopomofo, one syllable per
/// `-`-separated token joined by a single space (the platforms'
/// `joined(separator: " ")` / `joinToString(" ")`). Empty tokens (`--`, a
/// leading or trailing `-`) are dropped so nothing double-spaces.
pub fn tl_numeric_to_tps(text: &str, or_maps_to_er: bool) -> String {
    text.split('-')
        .filter(|tok| !tok.is_empty())
        .map(|tok| {
            crate::tps::to_zhuyin(tok, false, or_maps_to_er)
                .trim_end()
                .to_string()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Display TL with tone marks (`"hóo"`, `"tâi-gí"`) → TPS Bopomofo: the tone
/// marks become digits, then [`tl_numeric_to_tps`]. What a TPS-layout cell
/// shows for a candidate with no Hanji, and what picking it writes.
pub fn tl_display_to_tps(text: &str, or_maps_to_er: bool) -> String {
    tl_numeric_to_tps(&to_tone_number(text), or_maps_to_er)
}

fn is_letter_like(c: char) -> bool {
    c.is_alphabetic() || c == '\u{0358}' || is_nasal_marker(c)
}

/// True when `c` is the POJ nasal marker in either case (`ⁿ` / `ᴺ`).
pub fn is_nasal_marker(c: char) -> bool {
    c == crate::case_transform::NASAL_LOWER || c == crate::case_transform::NASAL_UPPER
}

/// True when `c` could be part of a Taiwanese word, as opposed to a mark the
/// user happened to commit alongside one.
///
/// `is_alphabetic` covers everything this IME writes: Latin for TL / POJ —
/// including the decomposed forms, since `ji̍t` / `m̄` / `o͘` all keep a Latin
/// base under their combining marks — Hanji (`Lo`), and both Bopomofo blocks for
/// TPS (`Lo`). Digits, punctuation, symbols, emoji, and bare combining marks
/// are not.
///
/// The two exclusions are characters Unicode calls alphabetic that cannot be a
/// word by themselves: four of the seven TPS tone marks are modifier letters
/// (`Lm` — `ˆ ˇ ˊ ˋ`; the other three are `Sk` and never qualified), and so is
/// the POJ nasal. Attached to a syllable they are harmless — the syllable
/// already brought its own base letter — but alone they are a diacritic.
///
/// Distinct from [`is_letter_like`], which asks whether a character *carries*
/// case or a diacritic and so counts a bare nasal in; this asks whether a
/// character could stand in a word, and a bare nasal cannot.
pub fn is_word_material(c: char) -> bool {
    c.is_alphabetic() && !crate::tps::is_tps_tone_mark(c) && !is_nasal_marker(c)
}

fn is_combining(c: char) -> bool {
    matches!(c, '\u{0300}'..='\u{036f}')
}

// MARK: - Display-level helpers (iOS / Android `pojDisplayToTLDisplay` / `tlDisplayToPOJDisplay`).
//        Exposed so the iOS+Android fixture suite can exercise them.

pub fn poj_display_to_tl_display(text: &str) -> String {
    rewrite_display(text, System::Tl, false, DisplayCase::Keep)
}

pub fn tl_display_to_poj_display(text: &str) -> String {
    rewrite_display(text, System::Poj, false, DisplayCase::Keep)
}

/// §55 — `roman` with the separator at every typed boundary replaced by
/// the `-` run the user typed there. `runs` are `(typed offset, run
/// length)` pairs (`(4, 1)` for a `-`, `(7, 2)` for a `--`), `typed` the
/// lowercased typed body the offsets index into, `reading` the canonical
/// TL the walk aligns ([`crate::typed_syllable_walk`]). `roman` is
/// `reading` under a separator-preserving transform — recased, or a
/// custom entry's native POJ form — so it is tokenized by the same
/// separators as the reading's syllables and rewritten by index rather
/// than walked itself. A boundary that is not a syllable end of the
/// reading, or ends its last syllable, changes nothing; a reading that
/// runs past the typed body (a partial-prefix extension) is rewritten on
/// the syllables the typed body covers. `None` when nothing changed or
/// the reading does not align with the typed body (an abbreviation body,
/// a TPS / English buffer).
pub fn render_typed_separators(
    mode: InputMode,
    typed: &str,
    runs: &[(usize, usize)],
    reading: &str,
    roman: &str,
) -> Option<String> {
    let walk = crate::typed_syllable_walk(mode, typed, reading)?;
    // Token byte ranges of `roman` — the `split` + drop-empty rule of
    // `tl_syllable_tokens`, so the count matches the walk's syllables.
    // Every separator is one ASCII byte.
    let mut tokens: Vec<std::ops::Range<usize>> = Vec::with_capacity(walk.ends.len());
    let mut offset = 0;
    for piece in roman.split(crate::tps::TL_SYLLABLE_SEPARATORS) {
        if !piece.is_empty() {
            tokens.push(offset..offset + piece.len());
        }
        offset += piece.len() + 1;
    }
    // The walk stops where the typed text ends, so a partial-prefix row
    // (`tāi-gi̍k` under `taigi`) aligns only its leading syllables.
    debug_assert!(tokens.len() >= walk.ends.len(), "{roman:?} vs {reading:?}");
    if tokens.len() < walk.ends.len() {
        return None;
    }
    // The separator after token `k`, and the typed run that replaces it.
    let separator_after =
        |k: usize| tokens[k].end..tokens.get(k + 1).map_or(roman.len(), |t| t.start);
    let typed_run = |k: usize| {
        (k + 1 < tokens.len())
            .then(|| runs.iter().find(|&&(at, _)| Some(&at) == walk.ends.get(k)))
            .flatten()
            .map(|&(_, run)| run)
    };
    let changes = (0..tokens.len()).any(|k| {
        typed_run(k).is_some_and(|run| {
            let stored = &roman[separator_after(k)];
            stored.len() != run || stored.bytes().any(|b| b != b'-')
        })
    });
    if !changes {
        return None;
    }
    let mut out = String::with_capacity(roman.len());
    for k in 0..tokens.len() {
        out.push_str(&roman[tokens[k].clone()]);
        match typed_run(k) {
            Some(run) => out.extend(std::iter::repeat_n('-', run)),
            None => out.push_str(&roman[separator_after(k)]),
        }
    }
    Some(out)
}

/// Syllable Separator (`AppConfig.syllable_separator`, `behavioral-invariants.md`
/// §49) — the rendered form of a dictionary-supplied romanization with each
/// run of `-` rewritten by [`push_syllable_joiner_run`]: `joiner` `""` (None)
/// gives `tâi-uân` → `tâiuân`, `hōo--guá` → `hōo·guá`; `" "` (Space) gives
/// `tâi uân`, `hōo ·guá`. The joiner only ever stands between two syllables:
/// a run that opens the text or a word (`--ah` → `·ah`) or closes one adds
/// none. Spaces (word boundaries) are kept. Display only: identity keys
/// (`display_text`, `canonical_tl`) never pass through here, and user-typed
/// hyphens are rendered by `to_tone_marks` as typed.
pub fn syllable_joiner_display(roman: &str, joiner: &str) -> String {
    let mut out = String::with_capacity(roman.len());
    let mut chars = roman.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '-' {
            out.push(c);
            continue;
        }
        let mut run_len = 1;
        while chars.next_if_eq(&'-').is_some() {
            run_len += 1;
        }
        // A piece rendered on its own (a nailed `--guá`) opens here too;
        // `composing::api::push_boundary` writes its joiner.
        let opens_word = out.is_empty() || out.ends_with(' ');
        let closes_word = matches!(chars.peek(), None | Some(' '));
        let joiner = if opens_word || closes_word {
            ""
        } else {
            joiner
        };
        push_syllable_joiner_run(&mut out, run_len, joiner);
    }
    out
}

/// Appends one run of `run_len` hyphens between two syllables, rendered:
/// `joiner`, then one [`crate::tps::KHINSIANN_DOT`] per neutral-tone `--` (an
/// odd hyphen left over is the plain syllable boundary the joiner already
/// wrote). The dot stays glued to the syllable after it: `---` → ` ·`, never
/// `· `. `composing::api::push_boundary` calls it for a typed run between
/// nailed segments, where no run ever opens the text.
pub fn push_syllable_joiner_run(out: &mut String, run_len: usize, joiner: &str) {
    out.push_str(joiner);
    for _ in 0..run_len / 2 {
        out.push_str(crate::tps::KHINSIANN_DOT);
    }
}

/// The mode a stored custom-dictionary `roman` is read in when the user types
/// in `mode`: under TPS a TL / POJ roman (no Bopomofo) is read as TL, so its
/// walker key and its `(hanji, canonical TL)` identity agree with TL mode's;
/// otherwise `mode` itself. The one rule both derive from.
pub fn stored_roman_mode(roman: &str, mode: InputMode) -> InputMode {
    match mode {
        InputMode::Tps if !contains_tps(roman) => InputMode::Tl,
        mode => mode,
    }
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
/// **Case is not identity.** A `TL` / `POJ` syllable comes out lowercase: an
/// auto-capitalized `Kóng` and a typed `kóng` are one word (Core Principle
/// 6), and the dictionary's readings are all lowercase. Text that does not
/// parse as a syllable (`iPhone`) and `English` keep their case: that is the
/// word the user wrote.
pub fn canonical_tl_form(text: &str, mode: InputMode) -> String {
    match mode {
        // TL mode: still fold POJ-shaped spellings onto canonical TL for the
        // cross-mode identity (`góa`→`guá`, B-4 / R2 / R5), but keep TL special
        // finals `eng`[ɛŋ] / `ek` literal so a hanji-absent `teng` candidate's
        // committed `display_text` is `teng`, not `tíng` (2026-06-05 TL-literal).
        InputMode::Tl => rewrite_display(text, System::Tl, true, DisplayCase::Drop),
        // POJ mode: input genuinely IS POJ, so `eng`→`ing` is a correct POJ→TL
        // conversion — use the full fold.
        InputMode::Poj => rewrite_display(text, System::Tl, false, DisplayCase::Drop),
        // v3.5.9 D / C-3b — TPS roman is Bopomofo, not POJ/TL Latin shape;
        // poj_display_to_tl_display would no-op on Bopomofo anyway (no Latin
        // patterns to substitute). Only the ㆳ glyph alias folds onto ㆪ, so a
        // raw commit typed with either glyph keys one user_frequency.db row.
        InputMode::Tps => text.chars().map(crate::tps::fold_tps_glyph_alias).collect(),
        InputMode::English => text.to_string(),
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
/// **Tone-INSENSITIVE** — unlike [`roman_reading_eq`] (which is
/// separator-insensitive but tone-PRESERVING), this folds tones away so
/// `tāng` (重量) and `tàng` both key to `tang`; the caller is responsible
/// for keeping genuinely distinct readings (Core Principle #7) apart by
/// only collapsing when a single canonical target exists.
///
/// **Residual** — folds ENCODING glyphs, not spelling families. The POJ
/// nasal marker `ⁿ` and the `o͘` dot U+0358 are alternate encodings of
/// `nn` / `oo` and DO fold (via `taigi_unicode_base_form`), but a genuine
/// spelling difference does not: POJ `chiah` keys to `chiah`, TL `tsiah`
/// to `tsiah` (no `ch↔ts` / `oa↔ua`). A POJ-spelled raw next_tl therefore
/// will not collapse onto its TL canonical here; the write-side R2 fix
/// (continuous commit carries canonical TL) closes that gap. R1 only needs
/// the common same-family case.
pub fn toneless_reading_key(roman: &str) -> String {
    crate::derivation::derive_notone(roman)
}

/// Two romanizations are the **same reading** when they differ only in
/// syllable separators: ASCII space (walker synth multi-word join), `-`
/// (hyphen compound), or `--` (neutral-tone khinsiann, two `-` chars). Tone
/// diacritics live inside the syllable letters and are NOT stripped, so
/// the comparison is tone-preserving:
///
/// * `roman_reading_eq("hōo guá", "hōo--guá")` → `true` (予我: synth
///   space-join vs dict khinsiann form — the same reading).
/// * `roman_reading_eq("hōo guá", "hōo-guā")` → `false` (戶外: guā tone 7
///   ≠ guá tone 2 — a different word, Core Principle #6).
///
/// Callers: the composing slot-0 promote and learned-pair fold
/// (`composing::continuous`, `composing::requests`) and the nailed-join
/// compound oracle (`lexicon::compound_hanji_exists`).
pub fn roman_reading_eq(a: &str, b: &str) -> bool {
    fn is_kept(c: &char) -> bool {
        *c != ' ' && *c != '-'
    }
    // Allocation-free: compare the separator-filtered char streams
    // directly (`Iterator::eq`). This runs per candidate in the per-
    // keystroke slot-0 scan, so it must not heap-allocate a stripped
    // `String` for each comparison.
    a.chars().filter(is_kept).eq(b.chars().filter(is_kept))
}

// In-process search-key derivation for the engine's own user-data stores
// (`userdata`): the write-side bundle plus the read-side query key. No wire
// op: the read side answers `SearchCustomEntries` (the `DeriveCustomQueryKey`
// op went with its last caller on 2026-09-30).
pub use crate::custom_search::{
    custom_search_reading, derive_custom_query_key, derive_custom_search_keys,
    tps_toneless_edge_key, CustomSearchKey,
};

pub use crate::hanji::{is_hanji, is_roman_only_spelling, CJK_RANGES};

/// The syllables of a canonical TL string — `-` inside a word, ` ` between
/// the words of a multi-word entry (`iā sī`); an empty piece (the khinsiann
/// `--`) is not a syllable. The one tokenizer `tps` already keys records by.
pub fn tl_syllables(canonical_tl: &str) -> impl Iterator<Item = &str> {
    crate::tps::tl_syllable_tokens(canonical_tl)
}

/// `keep_tl_finals = true` swaps the spelling fold to
/// [`normalize_to_tl_keep_tl_finals`] (drops `eng→ing` / `ek→ik`) so a TL
/// special final survives — used only by `canonical_tl_form` for TL mode. The
/// public `poj_display_to_tl_display` / `tl_display_to_poj_display` (and POJ-mode
/// `canonical_tl_form`) pass `false` for the full POJ↔TL fold.
/// Whether a rewritten syllable keeps the capital its token started with.
#[derive(Clone, Copy, PartialEq, Eq)]
enum DisplayCase {
    /// Display conversions: `Góa` stays `Guá`.
    Keep,
    /// The identity fold: `Góa` keys `guá`. Only parsed syllables lose case;
    /// a token that is not Taigi (`iPhone`) is returned as typed.
    Drop,
}

fn rewrite_display(text: &str, target: System, keep_tl_finals: bool, case: DisplayCase) -> String {
    if text.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch == '-' || ch == ' ' {
            out.push_str(&rewrite_token(&current, target, keep_tl_finals, case));
            out.push(ch);
            current.clear();
        } else {
            current.push(ch);
        }
    }
    out.push_str(&rewrite_token(&current, target, keep_tl_finals, case));
    out
}

fn rewrite_token(token: &str, target: System, keep_tl_finals: bool, case: DisplayCase) -> String {
    if token.is_empty() {
        return String::new();
    }
    let (bare, tone) = strip_tone_mark(token);
    if bare.is_empty() {
        return token.to_string();
    }
    let lowered = bare.to_lowercase();
    let normalized = if keep_tl_finals {
        normalize_to_tl_keep_tl_finals(&lowered)
    } else {
        normalize_to_tl(&lowered)
    };
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
    if case == DisplayCase::Keep && first.is_uppercase() {
        capitalize_first(&assembled)
    } else {
        assembled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roman_reading_eq_matches_separator_variants_preserves_tone() {
        // 予我: walker space-join vs dict khinsiann — same reading.
        assert!(roman_reading_eq("hōo guá", "hōo--guá"));
        // Hyphen vs space — same reading.
        assert!(roman_reading_eq("tâi gí", "tâi-gí"));
        // 戶外: tone 7 `guā` ≠ tone 2 `guá` — different reading despite
        // sharing the separator-stripped consonant/vowel skeleton.
        assert!(!roman_reading_eq("hōo guá", "hōo-guā"));
        // Identical strings (also-是 `iā sī` synth == dict) trivially eq.
        assert!(roman_reading_eq("iā sī", "iā sī"));
        // Different syllables entirely — not a reading match.
        assert!(!roman_reading_eq("hōo guá", "tâi gí"));
        // The nailed-join oracle's shape: per-syllable picks concatenated
        // without a separator against the dictionary row.
        assert!(roman_reading_eq("ánniâ", "án-niâ"));
        assert!(!roman_reading_eq("ánniâ", "án-nia"));
    }

    #[test]
    fn composing_mode_reads_the_tps_layout_as_tl() {
        let mode = |input_mode: &str| {
            composing_mode(&AppConfig {
                input_mode: input_mode.to_string(),
                ..AppConfig::default()
            })
        };
        assert_eq!(mode("tps"), InputMode::Tl);
        assert_eq!(mode("TPS"), InputMode::Tl);
        assert_eq!(mode("tl"), InputMode::Tl);
        assert_eq!(mode("poj"), InputMode::Poj);
        assert_eq!(mode("english"), InputMode::English);
        assert_eq!(mode(""), InputMode::Tl);
        // `parse_input_mode` itself still knows TPS (the content upgrade's target).
        assert_eq!(parse_input_mode("tps"), InputMode::Tps);
    }

    #[test]
    fn syllable_joiner_display_none_drops_hyphens_and_dots_the_khinsiann_marker() {
        let none = |roman| syllable_joiner_display(roman, "");
        assert_eq!(none("tâi-uân"), "tâiuân");
        assert_eq!(none("tshit-niû-má-senn"), "tshitniûmásenn");
        assert_eq!(none("hōo--guá"), "hōo\u{00b7}guá");
        assert_eq!(none("--ah"), "\u{00b7}ah");
        assert_eq!(none("tsáu--tshut-khì"), "tsáu\u{00b7}tshutkhì");
        // Longer runs: one dot per `--`, an odd hyphen adds nothing.
        assert_eq!(none("sia̍p---khì"), "sia̍p\u{00b7}khì");
        assert_eq!(none("a----b"), "a\u{00b7}\u{00b7}b");
        // Word boundaries and hyphen-free input are left alone.
        assert_eq!(none("tâi gí"), "tâi gí");
        assert_eq!(none("keng-lâm su-īⁿ"), "kenglâm suīⁿ");
    }

    #[test]
    fn syllable_joiner_display_space_spaces_syllables_and_glues_the_dot_forward() {
        let space = |roman| syllable_joiner_display(roman, " ");
        assert_eq!(space("tâi-uân"), "tâi uân");
        assert_eq!(space("tshit-niû-má-senn"), "tshit niû má senn");
        assert_eq!(space("hōo--guá"), "hōo \u{00b7}guá");
        assert_eq!(space("tsáu--tshut-khì"), "tsáu \u{00b7}tshut khì");
        assert_eq!(space("sia̍p---khì"), "sia̍p \u{00b7}khì");
        assert_eq!(
            space("tshiànn--tsha̍t---kòo-kenn"),
            "tshiànn \u{00b7}tsha̍t \u{00b7}kòo kenn"
        );
        // A run opening the text or a word, or closing one, adds no space.
        assert_eq!(space("--ah"), "\u{00b7}ah");
        assert_eq!(space("khioh piān--ê"), "khioh piān \u{00b7}ê");
        assert_eq!(space("tâi --ah"), "tâi \u{00b7}ah");
        assert_eq!(space("tai-"), "tai");
        assert_eq!(space("keng-lâm su-īⁿ"), "keng lâm su īⁿ");
    }

    #[test]
    fn push_syllable_joiner_run_writes_the_joiner_then_one_dot_per_pair() {
        let run = |run_len, joiner| {
            let mut out = String::new();
            push_syllable_joiner_run(&mut out, run_len, joiner);
            out
        };
        assert_eq!(run(1, ""), "");
        assert_eq!(run(2, ""), "\u{00b7}");
        assert_eq!(run(1, " "), " ");
        assert_eq!(run(2, " "), " \u{00b7}");
        assert_eq!(run(3, " "), " \u{00b7}");
        assert_eq!(run(4, " "), " \u{00b7}\u{00b7}");
    }

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

    // trace: derive_notone lowercases, folds ⁿ→nn, NFD, drops combining
    // tone marks + ASCII digits + '-' + ' '. So all three renderings of
    // 台語 collapse to the bare spelling "taigi".
    #[test]
    fn toneless_reading_key_folds_tone_and_separator_variants() {
        assert_eq!(toneless_reading_key("tâi-gí"), "taigi");
        assert_eq!(toneless_reading_key("taigi"), "taigi");
        assert_eq!(toneless_reading_key("tai5gi2"), "taigi");
        // canonical (diacritic + hyphen) and continuous raw (fused) agree.
        assert_eq!(
            toneless_reading_key("tâi-gí"),
            toneless_reading_key("taigi")
        );
    }

    // trace: all-tone-1 multi-syllable canonical "khong-an" vs continuous
    // raw "khongan" — separator-only difference, both toneless — still
    // collapse to one key. (The caller keeps the separator-bearing form as
    // canonical.)
    #[test]
    fn toneless_reading_key_collapses_separator_only_difference() {
        assert_eq!(toneless_reading_key("khong-an"), "khongan");
        assert_eq!(
            toneless_reading_key("khong-an"),
            toneless_reading_key("khongan")
        );
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

    // trace: an ENCODING difference is not a spelling family — POJ writes the
    // same vowel/nasal as TL with a combining dot / modifier letter, so both
    // renderings are one reading and must key alike.
    #[test]
    fn toneless_reading_key_folds_poj_encoding_glyphs() {
        assert_eq!(
            toneless_reading_key("ó\u{0358}"),
            toneless_reading_key("oo")
        );
        assert_eq!(toneless_reading_key("kiaⁿ"), toneless_reading_key("kiann"));
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

#[cfg(test)]
mod render_typed_separators_tests {
    use super::render_typed_separators;
    use crate::InputMode::{self, Poj, Tl};

    fn render(
        mode: InputMode,
        typed: &str,
        runs: &[(usize, usize)],
        roman: &str,
    ) -> Option<String> {
        render_typed_separators(mode, typed, runs, roman, roman)
    }

    // §55 — the reported shape: 放重利 stores a space, the user typed `-`.
    // trace: face `pang3tang7lai7`; typed `pangtanglai` → syllable ends
    // [4, 8, 11]; runs at 4 and 8 → tokens `pàng` | `tāng` | `lāi`,
    // separators ` ` and `-` → `-` and `-`.
    #[test]
    fn typed_hyphen_replaces_a_records_space() {
        for (typed, runs) in [
            ("pangtanglai", &[(4, 1), (8, 1)][..]),
            // Only the boundary the user typed is rewritten.
            ("pangtanglai", &[(4, 1)][..]),
            // The digits sit in the typed body; the runs index past them.
            ("pang3tang7lai", &[(5, 1), (10, 1)][..]),
            // A partial-prefix row: the typed body ends inside `lāi`.
            ("pangtangl", &[(4, 1)][..]),
        ] {
            assert_eq!(
                render(Tl, typed, runs, "pàng tāng-lāi").as_deref(),
                Some("pàng-tāng-lāi"),
                "{typed} {runs:?}"
            );
        }
    }

    // The roman may be recased or POJ-rendered; the walk reads the
    // canonical TL and the tokens come from the roman.
    #[test]
    fn rewrites_the_roman_by_the_readings_syllables() {
        assert_eq!(
            render_typed_separators(
                Tl,
                "pangtanglai",
                &[(4, 1)],
                "pàng tāng-lāi",
                "PÀNG TĀNG-LĀI"
            )
            .as_deref(),
            Some("PÀNG-TĀNG-LĀI")
        );
        // POJ mode walks the POJ face (`góa` ← `guá`) of the TL reading.
        assert_eq!(
            render(Poj, "goasi", &[(3, 2)], "guá sī").as_deref(),
            Some("guá--sī")
        );
    }

    // The walker's space join between two edges takes the typed run; a
    // part that opens with its own `--` (custom `--ah`, joined ` --ah`)
    // has its whole separator run replaced, never doubled.
    #[test]
    fn walker_space_join_takes_the_typed_run_without_doubling() {
        assert_eq!(
            render(Tl, "guasi", &[(3, 2)], "guá sī").as_deref(),
            Some("guá--sī")
        );
        assert_eq!(
            render(Tl, "khiah", &[(3, 2)], "khì --ah").as_deref(),
            Some("khì--ah")
        );
        // The run is verbatim (`---` stays `---`, §51).
        assert_eq!(
            render(Tl, "guasi", &[(3, 3)], "guá sī").as_deref(),
            Some("guá---sī")
        );
    }

    // Nothing to rewrite → `None`: typed and stored already agree, the
    // boundary is the reading's last syllable end (a trailing `tai-`) or
    // no syllable end at all, the reading ends before the boundary, or
    // the typed body is not the reading's (an abbreviation).
    #[test]
    fn leaves_the_roman_alone_when_nothing_changes() {
        for (typed, runs, roman) in [
            ("taigi", &[(3, 1)][..], "tâi-gí"),
            ("hoogua", &[(3, 2)][..], "hōo--guá"),
            ("tai", &[(3, 1)][..], "tâi"),
            ("khiah", &[(2, 1)][..], "khiah"),
            ("pangtanglai", &[(8, 1)][..], "pàng"),
            ("ptl", &[(1, 1)][..], "pàng tāng-lāi"),
        ] {
            assert_eq!(
                render(Tl, typed, runs, roman),
                None,
                "{typed} {runs:?} {roman}"
            );
        }
    }
}
