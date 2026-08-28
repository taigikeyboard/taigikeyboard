//! POJ (Pe̍h-ōe-jī) assembly — ported from `taigi-converter/src/poj.js`.

// 中文: POJ (白話字) 音節組裝,從 TL 的 (聲母, 韻母, 聲調) 三元組組出 POJ 顯示形式 (含聲調符號)。

use crate::tables::{poj_tone_mark, POJ_FINAL_SUBSTITUTIONS, POJ_INITIAL_FROM_TL};
use once_cell::sync::Lazy;
use regex::Regex;
use unicode_normalization::UnicodeNormalization;

static TWO_VOWELS: Lazy<Regex> = Lazy::new(|| Regex::new("[aeiou]{2}").unwrap());
static SINGLE_VOWEL: Lazy<Regex> = Lazy::new(|| Regex::new("[aeiou]").unwrap());

/// Assemble a POJ syllable from a TL `initial + final + tone`. Output is NFC.
// 中文: 從 TL 的 (聲母, 韻母, 聲調) 組出 POJ 音節,輸出為 NFC。
pub fn to_poj(initial: &str, final_str: &str, tone: &str) -> String {
    let poj_initial = POJ_INITIAL_FROM_TL.get(initial).copied().unwrap_or(initial);
    let poj_final = tl_final_to_poj(final_str);
    let mark = poj_tone_mark(tone);
    let marked = place_poj_tone_mark(&poj_final, mark);
    let mut combined = String::with_capacity(poj_initial.len() + marked.len());
    combined.push_str(poj_initial);
    combined.push_str(&marked);
    combined.nfc().collect()
}

/// Place the tone mark for `tone` directly on the literal `syllable`, with NO
/// spelling normalization. Used by the POJ-literal composing display so the
/// typed letters survive — `ting2` → `tíng` (not `téng`), `goa2` → `góa` (POJ
/// mark on `o`). The mark lands per [`place_poj_tone_mark`] (handles `o͘`, vowel
/// pairs, syllabic `ng`/`m`); leading consonants are untouched. Output is NFC.
/// Tone 1/4 (and toneless) have no mark → returns `syllable`.
// 中文: 直接在字面音節上放 POJ 聲調符號,不做任何拼寫正規化(ting→tíng、goa→góa)。
pub fn apply_poj_tone_literal(syllable: &str, tone: &str) -> String {
    let mark = poj_tone_mark(tone);
    place_poj_tone_mark(syllable, mark).nfc().collect()
}

fn tl_final_to_poj(final_str: &str) -> String {
    let mut result = final_str.to_string();
    for (tl_part, poj_part) in POJ_FINAL_SUBSTITUTIONS {
        result = result.replace(tl_part, poj_part);
    }
    result
}

fn place_poj_tone_mark(final_str: &str, mark: &str) -> String {
    if mark.is_empty() {
        return final_str.to_string();
    }

    // o͘ (o + U+0358) takes the mark between o and combining dot.
    if let Some(pos) = final_str.find("o\u{0358}") {
        let mut out = String::with_capacity(final_str.len() + mark.len());
        out.push_str(&final_str[..pos]);
        out.push('o');
        out.push_str(mark);
        out.push('\u{0358}');
        out.push_str(&final_str[pos + "o\u{0358}".len()..]);
        return out;
    }

    // iau / oai → mark on `a`.
    if final_str.contains("iau") || final_str.contains("oai") {
        return final_str.replacen('a', &format!("a{mark}"), 1);
    }

    // Dialectal `ere` / `iri` take the mark on the trailing vowel (`erê`,
    // `irî`), matching TL and the MOE manual rule for `ere`. They hold no
    // adjacent ASCII vowel pair, so without these the single-vowel fallback
    // below would mark the leading vowel.
    if final_str.contains("ere") {
        return final_str.replacen("ere", &format!("ere{mark}"), 1);
    }
    if final_str.contains("iri") {
        return final_str.replacen("iri", &format!("iri{mark}"), 1);
    }

    // Two adjacent ASCII vowels. Mirrors poj.js placePojToneMark vowel-pair
    // logic; multi-branch chain collapsed for clippy::if_same_then_else.
    if let Some(m) = TWO_VOWELS.find(final_str) {
        let bytes = final_str.as_bytes();
        let start = m.start();
        let first = bytes[start] as char;
        let second = bytes[start + 1] as char;
        let nasal_without_h_prefix = (final_str.ends_with('\u{207f}')
            || final_str.ends_with('\u{1d3a}'))
            && !final_str.ends_with("h\u{207f}")
            && !final_str.ends_with("h\u{1d3a}");
        // `oa` / `oe` are the only pairs whose mark moves off the leading
        // vowel, and only when a consonant coda closes the syllable (oa̍h
        // 活, choân 全, koe̍h); open (góa 我, ōe 話) and nasalized
        // (pòaⁿ 半) forms keep it on `o`. Every other pair keeps its own
        // nucleus — `au` is `a̍u` even before a coda (la̍uh 落), which the
        // unguarded coda lookahead used to break.
        let is_oa_oe_pair = first == 'o' && (second == 'a' || second == 'e');
        let target = if first == 'i' {
            second
        } else if is_oa_oe_pair && !nasal_without_h_prefix {
            // Suffix lookahead must decode the next Unicode scalar, not cast
            // a single byte. `final_str` here can carry multi-byte ⁿ (U+207F,
            // 3 bytes) or ᴺ (U+1D3A, 3 bytes) at `after`; `bytes[after] as
            // char` would yield a UTF-8 lead byte (e.g. 0xe2 → 'â') and the
            // suffix-set test would silently miss real nasal markers,
            // mis-placing the tone on the first vowel for finals like
            // `oa\u{207f}h` (POJ form of TL `uannh`). Found by Codex review
            // on PR #183 (discussion r3143646949).
            let after = start + 2;
            let suffix_char = final_str.get(after..).and_then(|s| s.chars().next());
            match suffix_char {
                Some(c) if "nmgptkh\u{207f}\u{1d3a}".contains(c) => second,
                _ => first,
            }
        } else {
            first
        };
        return final_str.replacen(target, &format!("{target}{mark}"), 1);
    }

    if let Some(m) = SINGLE_VOWEL.find(final_str) {
        let vowel = m.as_str();
        return final_str.replacen(vowel, &format!("{vowel}{mark}"), 1);
    }

    if final_str.contains("ng") {
        return final_str.replacen('n', &format!("n{mark}"), 1);
    }
    if final_str.contains('m') {
        return final_str.replacen('m', &format!("m{mark}"), 1);
    }
    final_str.to_string()
}
