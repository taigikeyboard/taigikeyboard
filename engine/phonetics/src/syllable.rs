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

/// Ordered POJ→TL substitution rules consumed by [`normalize_to_tl`]. v3.5.9 A1
/// D2 export — the offset-aware mirror `composing::shadow::apply_normalize_to_tl_with_offsets`
/// iterates this same list, so the two implementations cannot drift. Order is
/// meaningful: `oonn` collapses into `onn` only after `oo` substitutions have
/// already happened, mirroring the JS source. Scoped to `normalize_to_tl` only
/// — `is_stop_tone`'s own `.replace("nn", "")` is a separate helper and is
/// intentionally NOT folded in.
// 中文: D2 — POJ→TL 取代規則的單一順序表;composing::shadow 的 offset-aware 版本同步
// 中文:   消費此 list,兩端不會漂移。順序有意義(oonn 必須在 oo 之後);is_stop_tone
// 中文:   的 .replace("nn","") 是另一個 helper,刻意不併入。
pub const NORMALIZE_TO_TL_RULES: &[(&str, &str)] = &[
    ("ch", "ts"),
    ("ou", "oo"),
    ("o\u{0358}", "oo"),
    // Char-class `['\u{207f}','\u{1d3a}']` split into two single-pattern
    // entries. Equivalent because the two source chars are disjoint
    // single scalars and the replacement `"nn"` contains neither, so
    // sequential replace-all calls cannot re-introduce a match.
    ("\u{207f}", "nn"),
    ("\u{1d3a}", "nn"),
    ("oa", "ua"),
    ("oe", "ue"),
    ("eng", "ing"),
    ("ek", "ik"),
    ("oonn", "onn"),
];

/// Apply the [`NORMALIZE_TO_TL_RULES`] chain in order, returning the POJ→TL
/// normalized string. The byte-identical proof for the split-nn form lives in
/// the const's doc-comment above.
// 中文: 依 NORMALIZE_TO_TL_RULES 順序套用代換鏈;與舊版鏈式 .replace 行為位元相同。
pub fn normalize_to_tl(text: &str) -> String {
    NORMALIZE_TO_TL_RULES
        .iter()
        .fold(text.to_string(), |acc, (find, repl)| {
            acc.replace(find, repl)
        })
}

/// POJ normalization rules — fold the non-ASCII POJ glyphs (`o͘`, `ⁿ`,
/// `ᴺ`) to their ASCII spellings (`oo`, `nn`), plus the legacy `ou` →
/// `oo` alias (Codex pre-impl B-1 SHOULD, 2026-05-20: an unaudited
/// dirty single-syllable source row like `sou2` must not leak as a
/// literal `poj:sou` key while validating phonotactically through TL's
/// `soo` fold). Crucially we **do not** run the POJ→TL spelling chain
/// (`ch→ts`, `oa→ua`, `oe→ue`, `eng→ing`, `ek→ik`).
///
/// **Per-syllable use only.** The `ou → oo` alias is safe under
/// per-syllable / single-token application (which is how
/// [`canonicalize_poj_syllable`] and `derive_poj_notone_for_match`
/// consume this list — `ou` can only appear inside one POJ syllable,
/// and that one syllable is the dirty-row case we want to canonicalize).
/// For whole-buffer use (`composing::shadow::canonicalize_poj_shadow`,
/// in the downstream `composing` crate) the alias would mis-fire across
/// syllable boundaries — the runtime shadow uses
/// [`NORMALIZE_TO_POJ_GLYPH_RULES`] (the glyph-only subset) instead.
/// v3.5.9 B-2 PR #309 Codex P1 (`r3276402303`) caught that regression
/// on hyphenless user typing `toui` (intended POJ `tó-uī`).
// 中文: 逐音節 (per-syllable) 專用 — fold POJ 非-ASCII 字形 (o͘/ⁿ/ᴺ) 與 ou 別名為 ASCII (oo/nn),
// 中文:   不跑 POJ→TL 拼寫鏈 (ch→ts 等)。`ou→oo` 在單音節下安全 (dirty `sou2` 等);
// 中文:   全 buffer 套用會跨音節邊界誤觸發 → shadow pipeline 改用 NORMALIZE_TO_POJ_GLYPH_RULES。
pub const NORMALIZE_TO_POJ_RULES: &[(&str, &str)] = &[
    ("ou", "oo"),
    ("o\u{0358}", "oo"),
    ("\u{207f}", "nn"),
    ("\u{1d3a}", "nn"),
];

/// Glyph-only POJ normalization rules — the encoding subset of
/// [`NORMALIZE_TO_POJ_RULES`] without the legacy `ou → oo` alias.
/// Safe to apply whole-buffer because every rule is a non-ASCII →
/// ASCII codepoint substitution that cannot fire across token
/// boundaries by construction (the LHS is a single non-ASCII codepoint
/// or a base+combining pair, both of which sit inside one syllable).
///
/// Consumed by `composing::shadow::canonicalize_poj_shadow` (downstream
/// `composing` crate — plain reference, not an intra-doc link) so the
/// shadow stays boundary-preserving for hyphenless multi-syllable POJ
/// input (`toui` stays `toui`, the lattice then finds `poj:to` +
/// `poj:ui`). Per-syllable callers should keep using
/// [`NORMALIZE_TO_POJ_RULES`] — they get the dirty-row `ou` alias
/// protection where it is structurally safe.
// 中文: NORMALIZE_TO_POJ_RULES 的 glyph-only 子集,移除 `ou→oo` alias。
// 中文:   全 buffer 套用安全 (每條規則 LHS 都在單一音節內);供 shadow pipeline 使用,
// 中文:   不跨音節邊界誤觸發。逐音節呼叫端仍用完整 NORMALIZE_TO_POJ_RULES 保 dirty-row 防護。
pub const NORMALIZE_TO_POJ_GLYPH_RULES: &[(&str, &str)] =
    &[("o\u{0358}", "oo"), ("\u{207f}", "nn"), ("\u{1d3a}", "nn")];

/// Apply [`NORMALIZE_TO_POJ_RULES`] in order. The non-ASCII rules
/// (`o͘`/`ⁿ`/`ᴺ` → ASCII pairs) are no-ops on already-ASCII input. The
/// legacy `ou → oo` alias does fire on ASCII input — `kou` → `koo`,
/// `sou` → `soo` — so already-ASCII POJ is **not** strictly idempotent.
/// POJ-shaped spelling is otherwise preserved: `ch`, `oa`, `oe`,
/// `eng`, `ek` chain rules belong to `NORMALIZE_TO_TL_RULES`, not this
/// list.
// 中文: 依 NORMALIZE_TO_POJ_RULES 順序套用;非-ASCII 規則對 ASCII POJ 是 no-op,
// 中文:   但 `ou→oo` 在 ASCII 上會觸發 — 因此非嚴格 idempotent,只保證 POJ 拼寫不退到 TL。
pub fn normalize_to_poj(text: &str) -> String {
    NORMALIZE_TO_POJ_RULES
        .iter()
        .fold(text.to_string(), |acc, (find, repl)| {
            acc.replace(find, repl)
        })
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

/// Canonicalize one POJ-shaped syllable token into its **POJ ASCII** form
/// (no POJ→TL spelling fold), returning `(canonical_toneless, tone_digit)`
/// on phonotactic success.
///
/// Pipeline: `strip_tone_mark` (extract tone, fold NFD → NFC bare),
/// `to_lowercase`, [`normalize_to_poj`] (encoding-only `o͘`→`oo` /
/// `ⁿ`→`nn` / `ᴺ`→`nn`). Phonotactic validity is checked by routing
/// through [`normalize_to_tl`] + `split_initial_final` against the TL
/// initials × finals table — POJ source rows that pass TL validation
/// after fold are accepted, but the emitted key is the **POJ ASCII**
/// shape (e.g. `chit`, `goa`, `toa`, `che`), distinct from the TL forms
/// (`tsit`, `gua`, `tua`, `tse`).
///
/// Used by `engine/build-helpers/fst-builder` `build-syllables` to emit
/// the `poj:` family of the v3.5.9 B-1 tagged-single-FST syllable
/// inventory (`syllables.fst`). See
/// `docs/reports/2026-05-20-v359-b-plan.md` §B-1.
// 中文: 把單一 POJ 音節 token 正規化為 POJ ASCII 形式 (而非 TL),回傳 (去聲調 canonical, 聲調數字)。
// 中文:   Phonotactic 驗證仍走 TL 表 (避免複製 initials/finals 表),但 emit 的 key 保留 POJ ASCII。
// 中文:   供 v3.5.9 B-1 syllables.fst `poj:` 家族使用。
pub fn canonicalize_poj_syllable(token: &str) -> Option<(String, String)> {
    let (bare, tone) = strip_tone_mark(token);
    let lowered = bare.to_lowercase();
    // Phonotactic gate via TL tables (shared with TL path) — POJ rows
    // that fail TL phonotactics after fold are dictionary anomalies.
    split_initial_final(&normalize_to_tl(&lowered))?;
    Some((normalize_to_poj(&lowered), tone))
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

    // MARK: - canonicalize_poj_syllable / normalize_to_poj. SOURCE:
    // dictionary.csv poj_num samples — POJ rows like `chit8`, `goa2`,
    // `toa7`, `che1` must stay POJ-shaped in the inventory (not folded
    // to TL `tsit`, `gua`, `tua`, `tse`). Non-ASCII POJ `peⁿ5` / `so͘3`
    // collapse to ASCII `penn` / `soo` per encoding-only rules.

    #[test]
    fn normalize_to_poj_encoding_only() {
        // Encoding fixes apply, but POJ→TL spelling rules do NOT.
        assert_eq!(normalize_to_poj("so\u{0358}"), "soo");
        assert_eq!(normalize_to_poj("pe\u{207f}"), "penn");
        assert_eq!(normalize_to_poj("a\u{1d3a}"), "ann");
        // POJ-shaped spellings preserved (would fold to TL via NORMALIZE_TO_TL_RULES).
        assert_eq!(normalize_to_poj("chiah"), "chiah");
        assert_eq!(normalize_to_poj("goa"), "goa");
        assert_eq!(normalize_to_poj("koe"), "koe");
        assert_eq!(normalize_to_poj("peng"), "peng");
        assert_eq!(normalize_to_poj("pek"), "pek");
    }

    #[test]
    fn normalize_to_poj_legacy_ou_alias_fires_on_ascii() {
        // Codex pre-impl B-1 SHOULD (2026-05-20): `ou → oo` is the legacy
        // alias shared with NORMALIZE_TO_TL_RULES — a forward guard so a
        // future dirty `sou*` / `kou*` source row cannot leak as a
        // literal `poj:sou` while validating phonotactically via TL's
        // `soo` fold. Current `dictionary.csv` has zero `poj_num` rows
        // containing `ou`, so this is no-op against today's data; the
        // pin captures the contract so a later asset audit catches
        // accidental drift.
        assert_eq!(normalize_to_poj("sou"), "soo");
        assert_eq!(normalize_to_poj("kou"), "koo");
        // The phonotactic gate then accepts these as valid via TL's
        // `soo` / `koo` finals → emit POJ ASCII forms.
        let (poj, tone) = canonicalize_poj_syllable("sou2").expect("sou2 must canonicalize");
        assert_eq!(poj, "soo");
        assert_eq!(tone, "2");
    }

    #[test]
    fn canonicalize_poj_syllable_preserves_poj_shape() {
        let cases = [
            ("chit8", "chit", "8"),
            ("goa2", "goa", "2"),
            ("toa7", "toa", "7"),
            ("che1", "che", "1"),
            ("koe1", "koe", "1"),
            ("peng5", "peng", "5"),
            ("pek4", "pek", "4"),
            ("chiau2", "chiau", "2"),
        ];
        for (input, expected_canonical, expected_tone) in cases {
            let (canonical, tone) = canonicalize_poj_syllable(input)
                .unwrap_or_else(|| panic!("canonicalize_poj_syllable({input}) returned None"));
            assert_eq!(canonical, expected_canonical, "canonical of {input}");
            assert_eq!(tone, expected_tone, "tone of {input}");
        }
    }

    #[test]
    fn canonicalize_poj_syllable_non_ascii_inputs() {
        // Encoding-only rules collapse `o͘` / `ⁿ` to ASCII; POJ spelling
        // is preserved otherwise.
        let cases = [
            ("peⁿ5", "penn", "5"),
            ("so͘3", "soo", "3"),
            ("tsiuⁿ7", "tsiunn", "7"),
            ("pho͘5", "phoo", "5"),
        ];
        for (input, expected_canonical, expected_tone) in cases {
            let (canonical, tone) = canonicalize_poj_syllable(input)
                .unwrap_or_else(|| panic!("canonicalize_poj_syllable({input}) returned None"));
            assert_eq!(canonical, expected_canonical, "canonical of {input}");
            assert_eq!(tone, expected_tone, "tone of {input}");
        }
    }

    #[test]
    fn canonicalize_poj_syllable_pure_tl_inputs_passthrough() {
        // Pure-TL spellings (`tai`, `bak`, `tsh*`) round-trip unchanged
        // — POJ inventory accepts them when the source row's poj_num
        // mirrors tl_num (≈ half of dictionary.csv rows).
        let cases = [
            ("tai5", "tai", "5"),
            ("bak4", "bak", "4"),
            ("tshiu7", "tshiu", "7"),
        ];
        for (input, expected_canonical, expected_tone) in cases {
            let (canonical, tone) = canonicalize_poj_syllable(input)
                .unwrap_or_else(|| panic!("canonicalize_poj_syllable({input}) returned None"));
            assert_eq!(canonical, expected_canonical, "canonical of {input}");
            assert_eq!(tone, expected_tone, "tone of {input}");
        }
    }

    #[test]
    fn canonicalize_poj_syllable_invalid_returns_none() {
        // Same phonotactic gate as TL (shared TL initials × finals
        // table) — initial-without-final, unknown letters, malformed
        // dual-marked all reject.
        let cases = ["", "tsh", "kh", "xyz", "tj", "qq", "bx", "tn̄g6", "123"];
        for input in cases {
            assert!(
                canonicalize_poj_syllable(input).is_none(),
                "canonicalize_poj_syllable({input:?}) should be None"
            );
        }
    }

    #[test]
    fn canonicalize_poj_syllable_differs_from_tl_on_divergent_rows() {
        // Pin the contract: POJ-shaped inputs whose TL form differs MUST
        // keep their POJ shape under canonicalize_poj_syllable, while
        // canonicalize_syllable folds to TL.
        let divergent_pairs = [
            ("chit8", "chit", "tsit"),
            ("goa2", "goa", "gua"),
            ("toa7", "toa", "tua"),
            ("che1", "che", "tse"),
            ("koe1", "koe", "kue"),
            ("peng5", "peng", "ping"),
            ("pek4", "pek", "pik"),
        ];
        for (input, expected_poj, expected_tl) in divergent_pairs {
            let (poj_form, _) = canonicalize_poj_syllable(input).unwrap();
            let (tl_form, _) = canonicalize_syllable(input).unwrap();
            assert_eq!(poj_form, expected_poj, "POJ canonical of {input}");
            assert_eq!(tl_form, expected_tl, "TL canonical of {input}");
            assert_ne!(poj_form, tl_form, "POJ/TL must differ for {input}");
        }
    }
}
