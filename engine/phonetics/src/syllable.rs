//! Syllable parsing — ported from `taigi-converter/src/phonetics.js`.

// 中文: 音節解析,把字串拆成 (聲母, 韻母, 聲調),並提供 POJ→TL 拼寫正規化、聲調符號剝離等基礎工具。

use crate::tables::{COMBINING_TO_TONE_NUM, TL_FINALS, TL_INITIALS};
use unicode_normalization::UnicodeNormalization;

/// Strip the tone mark from `text`, returning `(bare NFC text, tone digit)`.
/// Recognises both NFD combining marks and trailing ASCII digits 1..=9.
/// `tone` is the empty string when no mark is present.
// 中文: 把聲調符號從字串裡剝出來,回傳 (去聲調 NFC 字串, 聲調數字);辨識 NFD 組合符號跟結尾 ASCII 數字 1..=9。
pub fn strip_tone_mark(text: &str) -> (String, String) {
    // Fast path: pure-ASCII input cannot carry combining marks. NFD/NFC are
    // no-ops on ASCII, so skip the allocation. This is the common case for
    // raw IME keystrokes (e.g. "ka2", "tshiu7", "hello").
    if text.is_ascii() {
        if let Some(last) = text.chars().last() {
            if let Some(d) = last.to_digit(10) {
                if (1..=9).contains(&d) {
                    return (text[..text.len() - 1].to_string(), d.to_string());
                }
            }
        }
        return (text.to_string(), String::new());
    }

    let decomposed: String = text.nfd().collect();
    if let Some((idx, ch)) = decomposed
        .char_indices()
        .find(|(_, c)| COMBINING_TO_TONE_NUM.contains_key(c))
    {
        let tone = COMBINING_TO_TONE_NUM[&ch];
        let mut bare = String::with_capacity(decomposed.len() - ch.len_utf8());
        bare.push_str(&decomposed[..idx]);
        bare.push_str(&decomposed[idx + ch.len_utf8()..]);
        return (bare.nfc().collect(), tone.to_string());
    }

    // No combining mark — check for trailing 1..=9 digit on the original input.
    if let Some(last) = text.chars().last() {
        if let Some(d) = last.to_digit(10) {
            if (1..=9).contains(&d) {
                let bare: String = text.chars().take(text.chars().count() - 1).collect();
                return (bare.nfc().collect(), d.to_string());
            }
        }
    }

    (text.nfc().collect(), String::new())
}

/// Lowercase, then map POJ-style spellings into TL spellings.
/// Order is meaningful: `oonn` collapses into `onn` only after `oo` substitutions
/// have already happened, mirroring the JS source.
// 中文: 小寫後把 POJ 寫法替換成 TL 寫法;替換順序有意義,跟 JS 來源一致。
pub fn normalize_to_tl(text: &str) -> String {
    text.replace("ch", "ts")
        .replace("ou", "oo")
        .replace("o\u{0358}", "oo")
        .replace(['\u{207f}', '\u{1d3a}'], "nn")
        .replace("oa", "ua")
        .replace("oe", "ue")
        .replace("eng", "ing")
        .replace("ek", "ik")
        .replace("oonn", "onn")
}

/// True when the final ends with a stop consonant (p, t, k, h), ignoring trailing
/// nasal `nn`. `kah4` → true; `kann2` → false.
// 中文: 判斷韻母是否以入聲子音 (p/t/k/h) 結尾;結尾的鼻化 `nn` 不計入。
pub(crate) fn is_stop_tone(final_str: &str) -> bool {
    let cleaned = final_str.to_lowercase().replace("nn", "");
    cleaned.ends_with('p')
        || cleaned.ends_with('t')
        || cleaned.ends_with('k')
        || cleaned.ends_with('h')
}

/// Split `text` into `(initial, final)` by iterating prefixes against the TL
/// initial / final tables. `text` must already be lowercase + TL-normalised.
// 中文: 把音節拆成 (聲母, 韻母);輸入必須先小寫化並正規化成 TL 拼寫。
pub(crate) fn split_initial_final(text: &str) -> Option<(String, String)> {
    for i in 0..=text.len() {
        if !text.is_char_boundary(i) {
            continue;
        }
        let initial = &text[..i];
        if TL_INITIALS.contains(initial) {
            let final_str = &text[i..];
            if TL_FINALS.contains(final_str) {
                return Some((initial.to_string(), final_str.to_string()));
            }
        }
    }
    None
}

/// Canonicalize one TL- or POJ-shaped syllable token into its TL form,
/// returning `(canonical_toneless, tone_digit)` on phonotactic success.
///
/// Pipeline: `strip_tone_mark` (extract tone, fold NFD → NFC bare),
/// `to_lowercase`, `normalize_to_tl` (POJ→TL spelling + `ⁿ` → `nn` + `o͘`
/// → `oo`), then `split_initial_final` for membership in the
/// `TL_INITIALS` × `TL_FINALS` table at `tables.rs:11-36`. The tone
/// string is whatever `strip_tone_mark` returned ("1".."9" or empty
/// when the caller supplied a toneless token).
///
/// Used by `engine/build-helpers/fst-builder` `build-syllables` to emit
/// canonical numeric + toneless keys for the v3.5.8 Phase 2 syllable
/// inventory FST. Mainstream IMEs (khiin-rs `engine/src/data/`) use a
/// PHF table for the same job; we lean on the existing TL initial/final
/// tables to avoid table duplication.
// 中文: 把單一音節 token 正規化為 TL 形式,回傳 (去聲調 canonical, 聲調數字)。
// 中文: 失敗 = phonotactic 不合法 (聲母或韻母不在 TL 表)。供 Phase 2 syllables.fst 建置使用。
pub fn canonicalize_syllable(token: &str) -> Option<(String, String)> {
    let (bare, tone) = strip_tone_mark(token);
    let canonical = normalize_to_tl(&bare.to_lowercase());
    split_initial_final(&canonical)?;
    Some((canonical, tone))
}

/// Phonotactic validity test for a single TL/POJ-shaped syllable token.
/// Equivalent to `canonicalize_syllable(token).is_some()`. Empty input,
/// initial-without-final (`tsh`), and unknown letters (`xyz`, `tj`) all
/// return false.
// 中文: 判斷音節 token 是否 phonotactically 合法 (POJ 形式會先正規化成 TL)。
pub fn is_valid_syllable(token: &str) -> bool {
    canonicalize_syllable(token).is_some()
}

/// Parse a syllable into `(initial, final, tone)`. Returns `None` when the
/// syllable cannot be split. Inferred tones: `4` for stop finals, `1` otherwise.
///
/// Currently only used by this module's unit tests — the runtime
/// `*_display_to_*_display` path uses `strip_tone_mark` + `split_initial_final`
/// directly. Kept as a primitive for future callers.
// 中文: 把音節解析成 (聲母, 韻母, 聲調);無聲調時依入聲韻母推 4、其他推 1。目前僅單元測試使用。
#[cfg(test)]
fn parse_syllable(text: &str) -> Option<(String, String, String)> {
    let (bare, tone) = strip_tone_mark(text);
    let normalized = normalize_to_tl(&bare.to_lowercase());
    let (initial, final_str) = split_initial_final(&normalized)?;
    let final_tone = if tone.is_empty() {
        if is_stop_tone(&final_str) { "4" } else { "1" }.to_string()
    } else {
        tone
    };
    Some((initial, final_str, final_tone))
}

#[cfg(test)]
mod tests {
    use super::*;

    // MARK: - is_stop_tone. SOURCE: phonetics.test.js + iOS + Android.

    #[test]
    fn is_stop_tone_cases() {
        assert!(is_stop_tone("ap"));
        assert!(is_stop_tone("at"));
        assert!(is_stop_tone("ak"));
        assert!(is_stop_tone("ah"));
        assert!(!is_stop_tone("a"));
        assert!(!is_stop_tone("an"));
        assert!(!is_stop_tone("ang"));
        assert!(is_stop_tone("annh"));
    }

    // MARK: - split_initial_final. SOURCE: TaigiPhoneticsTests.swift +
    // TaigiPhoneticsTest.kt — both add cases beyond JS.

    #[test]
    fn split_initial_final_valid() {
        let cases = [
            ("ka", "k", "a"),
            ("tshiu", "tsh", "iu"),
            ("a", "", "a"),
            ("ng", "", "ng"),
            ("m", "", "m"),
            ("phang", "ph", "ang"),
            ("iang", "", "iang"),
            ("oo", "", "oo"),
        ];
        for (input, init, fin) in cases {
            let result = split_initial_final(input);
            assert_eq!(
                result.as_ref().map(|(i, _)| i.as_str()),
                Some(init),
                "initial of {input}"
            );
            assert_eq!(
                result.as_ref().map(|(_, f)| f.as_str()),
                Some(fin),
                "final of {input}"
            );
        }
    }

    #[test]
    fn split_initial_final_invalid_returns_none() {
        assert!(split_initial_final("xyz").is_none());
    }

    // MARK: - parse_syllable. SOURCE: phonetics.test.js + iOS + Android.

    #[test]
    fn parse_syllable_simple() {
        let cases = [
            ("ka2", "k", "a", "2"),
            ("kang1", "k", "ang", "1"),
            ("a1", "", "a", "1"),
            ("k\u{00e1}", "k", "a", "2"),
            ("kah", "k", "ah", "4"),
            ("ka", "k", "a", "1"),
            ("pha3", "ph", "a", "3"),
            ("tshiu7", "tsh", "iu", "7"),
        ];
        for (input, init, fin, tone) in cases {
            let r = parse_syllable(input)
                .unwrap_or_else(|| panic!("parse_syllable({input}) returned None"));
            assert_eq!(r.0, init, "initial of {input}");
            assert_eq!(r.1, fin, "final of {input}");
            assert_eq!(r.2, tone, "tone of {input}");
        }
    }

    #[test]
    fn parse_syllable_poj_forms() {
        let cases = [
            ("chhi2", "tsh", "i", "2"),
            ("koa1", "k", "ua", "1"),
            ("koe1", "k", "ue", "1"),
            ("peng5", "p", "ing", "5"),
        ];
        for (input, init, fin, tone) in cases {
            let r = parse_syllable(input)
                .unwrap_or_else(|| panic!("parse_syllable({input}) returned None"));
            assert_eq!(r.0, init);
            assert_eq!(r.1, fin);
            assert_eq!(r.2, tone);
        }
    }

    #[test]
    fn parse_syllable_syllabic_consonants() {
        let r = parse_syllable("ng5").unwrap();
        assert_eq!(r, ("".into(), "ng".into(), "5".into()));
        let r = parse_syllable("m7").unwrap();
        assert_eq!(r, ("".into(), "m".into(), "7".into()));
    }

    #[test]
    fn parse_syllable_invalid_returns_none() {
        assert!(parse_syllable("xyz").is_none());
    }

    // MARK: - canonicalize_syllable / is_valid_syllable.
    // SOURCE: dictionary.csv tl_num samples — exercises the POJ→TL
    // normalization path because real CSV rows still carry POJ-shaped
    // fragments like `chiau2`, `choa7`, `eng1`, plus non-ASCII forms
    // `peⁿ5`, `so͘3`. Pinned by v3.5.8 Phase 2 (syllables.fst builder).

    #[test]
    fn canonicalize_syllable_poj_shaped_inputs() {
        let cases = [
            ("chiau2", "tsiau", "2"),
            ("chha1", "tsha", "1"),
            ("choa7", "tsua", "7"),
            ("eng1", "ing", "1"),
            ("pek4", "pik", "4"),
            ("koe1", "kue", "1"),
            ("peng5", "ping", "5"),
        ];
        for (input, expected_canonical, expected_tone) in cases {
            let (canonical, tone) = canonicalize_syllable(input)
                .unwrap_or_else(|| panic!("canonicalize_syllable({input}) returned None"));
            assert_eq!(canonical, expected_canonical, "canonical of {input}");
            assert_eq!(tone, expected_tone, "tone of {input}");
        }
    }

    #[test]
    fn canonicalize_syllable_non_ascii_inputs() {
        // `ⁿ` (U+207F) → `nn`, `o͘` (o + U+0358) → `oo` per normalize_to_tl.
        let cases = [
            ("peⁿ5", "penn", "5"),
            ("so͘3", "soo", "3"),
            ("tsiuⁿ7", "tsiunn", "7"),
            ("pho͘5", "phoo", "5"),
        ];
        for (input, expected_canonical, expected_tone) in cases {
            let (canonical, tone) = canonicalize_syllable(input)
                .unwrap_or_else(|| panic!("canonicalize_syllable({input}) returned None"));
            assert_eq!(canonical, expected_canonical, "canonical of {input}");
            assert_eq!(tone, expected_tone, "tone of {input}");
        }
    }

    #[test]
    fn canonicalize_syllable_pure_tl_inputs() {
        let cases = [
            ("tai5", "tai", "5"),
            ("bak4", "bak", "4"),
            ("khih4", "khih", "4"),
            ("m7", "m", "7"),
            ("ng5", "ng", "5"),
            ("oo7", "oo", "7"),
            ("uainn3", "uainn", "3"),
        ];
        for (input, expected_canonical, expected_tone) in cases {
            let (canonical, tone) = canonicalize_syllable(input)
                .unwrap_or_else(|| panic!("canonicalize_syllable({input}) returned None"));
            assert_eq!(canonical, expected_canonical, "canonical of {input}");
            assert_eq!(tone, expected_tone, "tone of {input}");
        }
    }

    #[test]
    fn canonicalize_syllable_toneless_inputs() {
        // No tone supplied — bare canonical returned with empty tone string.
        let cases = [
            ("tai", "tai"),
            ("bak", "bak"),
            ("m", "m"),
            ("ng", "ng"),
            ("oo", "oo"),
            ("choa", "tsua"),
        ];
        for (input, expected_canonical) in cases {
            let (canonical, tone) = canonicalize_syllable(input)
                .unwrap_or_else(|| panic!("canonicalize_syllable({input}) returned None"));
            assert_eq!(canonical, expected_canonical, "canonical of {input}");
            assert_eq!(tone, "", "expected empty tone for toneless {input}");
        }
    }

    #[test]
    fn canonicalize_syllable_invalid_returns_none() {
        // Initial-without-final, unknown letters, malformed dual-marked
        // (combining mark + trailing digit) all reject.
        let cases = ["", "tsh", "kh", "xyz", "tj", "qq", "bx", "tn̄g6", "123"];
        for input in cases {
            assert!(
                canonicalize_syllable(input).is_none(),
                "canonicalize_syllable({input:?}) should be None"
            );
        }
    }

    #[test]
    fn is_valid_syllable_matches_canonicalize() {
        for input in ["tai5", "choa7", "peⁿ5", "ng", ""] {
            assert_eq!(
                is_valid_syllable(input),
                canonicalize_syllable(input).is_some(),
                "is_valid_syllable / canonicalize_syllable disagree on {input:?}",
            );
        }
    }
}
