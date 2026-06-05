//! TL (Tâi-lô) assembly — ported from `taigi-converter/src/tl.js`.

// 中文: TL (台羅) 音節組裝,從 (聲母, 韻母, 聲調) 組出 TL 顯示形式並決定聲調符號標哪個母音。

use crate::tables::tl_tone_mark;
use unicode_normalization::UnicodeNormalization;

/// Assemble a TL syllable from `initial + final + tone`. Output is NFC.
// 中文: 從 (聲母, 韻母, 聲調) 組出 TL 音節,輸出為 NFC。
pub fn to_tl(initial: &str, final_str: &str, tone: &str) -> String {
    let mark = tl_tone_mark(tone);
    let marked = place_tl_tone_mark(final_str, mark);
    let mut combined = String::with_capacity(initial.len() + marked.len());
    combined.push_str(initial);
    combined.push_str(&marked);
    combined.nfc().collect()
}

/// Place the tone mark for `tone` directly on the literal `syllable`, with NO
/// spelling normalization. Used by the TL-literal composing display so the
/// typed letters survive — `goa2` → `goá` (not `guá`), `teng2` → `téng` (the TL
/// special final `eng` is not folded to `ing`). The mark lands on the priority
/// vowel per [`place_tl_tone_mark`]; leading consonants carry no vowel so they
/// are untouched, and syllabic nasals (`ng`/`m`) get the mark on `n`/`m`.
/// Output is NFC. Tone 1/4 (and toneless) have no mark → returns `syllable`.
// 中文: 直接在字面音節上放 TL 聲調符號,不做任何拼寫正規化(goa→goá、teng→téng)。
pub fn apply_tl_tone_literal(syllable: &str, tone: &str) -> String {
    let mark = tl_tone_mark(tone);
    place_tl_tone_mark(syllable, mark).nfc().collect()
}

/// Vowel priority for TL: `a > oo > ere > e > o > ui→i > iu→u > iri > i > u > ng > m`.
/// Mirrors `placeTlToneMark` in `tl.js`.
fn place_tl_tone_mark(final_str: &str, mark: &str) -> String {
    if mark.is_empty() {
        return final_str.to_string();
    }
    if final_str.contains('a') {
        return final_str.replacen('a', &format!("a{mark}"), 1);
    }
    if final_str.contains("oo") {
        return final_str.replacen("oo", &format!("o{mark}o"), 1);
    }
    if final_str.contains("ere") {
        return final_str.replacen("ere", &format!("ere{mark}"), 1);
    }
    if final_str.contains('e') {
        return final_str.replacen('e', &format!("e{mark}"), 1);
    }
    if final_str.contains('o') {
        return final_str.replacen('o', &format!("o{mark}"), 1);
    }
    if final_str.contains("ui") {
        return final_str.replacen('i', &format!("i{mark}"), 1);
    }
    if final_str.contains("iu") {
        return final_str.replacen('u', &format!("u{mark}"), 1);
    }
    if final_str.contains("iri") {
        return final_str.replacen("iri", &format!("iri{mark}"), 1);
    }
    if final_str.contains('i') {
        return final_str.replacen('i', &format!("i{mark}"), 1);
    }
    if final_str.contains('u') {
        return final_str.replacen('u', &format!("u{mark}"), 1);
    }
    if final_str.contains("ng") {
        return final_str.replacen("ng", &format!("n{mark}g"), 1);
    }
    if final_str.contains('m') {
        return final_str.replacen('m', &format!("m{mark}"), 1);
    }
    final_str.to_string()
}
