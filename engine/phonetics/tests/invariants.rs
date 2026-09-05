//! INVARIANT_* tests required by the D9 boolean check (`SKILL.md:336`).
//! Mirrors `TaigiPhoneticsTests.swift` test_INVARIANT_* methods so the iOS
//! production fixtures travel intact into the Rust crate.

// D9 規定的 INVARIANT_* 測試,把 iOS 的 test_INVARIANT_* 方法原樣搬到 Rust。

use phonetics::api::{poj_display_to_tl_display, tl_display_to_poj_display};

const TL_POJ_FIXTURES: &[(&str, &str)] = &[
    ("t\u{00e2}i", "t\u{00e2}i"),
    ("ts\u{00e1}i", "ch\u{00e1}i"),
    ("T\u{00e2}i-g\u{00ed}", "T\u{00e2}i-g\u{00ed}"),
];

#[test]
fn invariant_tl_to_poj_roundtrip_is_lossless() {
    for (tl, _) in TL_POJ_FIXTURES {
        let poj = tl_display_to_poj_display(tl);
        let back = poj_display_to_tl_display(&poj);
        assert_eq!(back, *tl, "TL→POJ→TL drift on {tl}: got {back}");
    }
}

#[test]
fn invariant_poj_to_tl_roundtrip_is_lossless() {
    for (_, poj) in TL_POJ_FIXTURES {
        let tl = poj_display_to_tl_display(poj);
        let back = tl_display_to_poj_display(&tl);
        assert_eq!(back, *poj, "POJ→TL→POJ drift on {poj}: got {back}");
    }
}

#[test]
fn invariant_oo_combining_form_roundtrips() {
    let tl = "h\u{00f4}o";
    let poj = tl_display_to_poj_display(tl);
    assert!(
        poj.chars().any(|c| c == '\u{0358}'),
        "POJ form must carry U+0358, got {poj}"
    );
    assert_eq!(poj_display_to_tl_display(&poj), tl);
}

#[test]
fn invariant_nasal_marker_variants_collapse_on_parse() {
    let via_superscript = poj_display_to_tl_display("sa\u{207f}");
    let via_small_caps = poj_display_to_tl_display("sa\u{1d3a}");
    let via_literal_nn = poj_display_to_tl_display("sann");
    assert_eq!(via_superscript, via_small_caps);
    assert_eq!(via_superscript, via_literal_nn);
}

// =========================================================================
// INVARIANT_ROMAN_NASAL_OO_ALIAS_IS_SYLLABLE_LOCAL
// `docs/architecture/behavioral-invariants.md` §45
// =========================================================================

/// The alternate `o͘ⁿ` / `oonn` spelling of the nasal final may only be
/// rewritten inside ONE syllable. Across a seam the same letters are an `oo`
/// final meeting the next syllable's onset, and rewriting there destroys a real
/// dictionary key.
// 鼻化韻的 o͘ⁿ / oonn 別名拼法只能在「單一音節內」改寫;跨接縫的同樣字母是
//   oo 韻碰下一個音節的聲母,在那裡改寫會毀掉真正的字典鍵。
#[test]
fn invariant_roman_nasal_oo_alias_is_syllable_local() {
    // The helper respells ONE syllable, and that is the whole of its contract:
    // it is the EXPANDING direction, so it has no boundaries of its own to
    // check. Scope is owned by the callers, which hand it one syllable at a
    // time — `dictionary/build/create_fst.py` splits `*_num` on tone digits
    // first (pinned by `dictionary/tests/test_nasal_oo_alias.py`, which asserts
    // 滷卵 / 芋卵 / 菜脯卵 / 飛烏卵 are left alone), and `fst-builder`'s
    // `build-syllables` is already one syllable per token.
    //
    // Pinned here: every shape the build hands over respells, tone digit
    // surviving so the numeric-tone key family gets an alias too, and a
    // syllable with no nasal final is left alone.
    // helper 只改寫「一個音節」,而那就是它契約的全部 —— 它是展開方向,自己沒有
    //   邊界可檢查。scope 由呼叫端持有,逐音節餵給它:create_fst.py 先依聲調數字
    //   切 *_num(由 test_nasal_oo_alias.py 釘住,斷言 滷卵/芋卵/菜脯卵/飛烏卵 不被動),
    //   而 fst-builder 的 build-syllables 本來就是一行一個音節。
    for (canonical, alias) in [
        ("honn", "hoonn"),
        ("honnh", "hoonnh"),
        ("sionn", "sioonn"),
        ("onn", "oonn"),
        ("honn3", "hoonn3"),
        ("honnh4", "hoonnh4"),
    ] {
        assert_eq!(
            phonetics::nasal_oo_alias_spelling(canonical).as_deref(),
            Some(alias),
            "{canonical} should respell to {alias}",
        );
    }
    for canonical in ["hoo", "tai", "nng", "loo", "tsiah", ""] {
        assert_eq!(
            phonetics::nasal_oo_alias_spelling(canonical),
            None,
            "{canonical} has no nasal final",
        );
    }
}

/// The whole-buffer form of the fold is the shape that broke real words
/// (滷卵 `lo͘nng` → `loonng` → `lonng`), so `TL_ENCODING_RULES` — the list the
/// search shadow applies to a whole buffer — must never carry it. The
/// per-syllable list keeps it, because `canonical_tl_form` needs it to hold the
/// cross-mode identity together (Core Principle #7).
// 整段套用的折疊正是弄壞真實詞的那個形狀(滷卵 lo͘nng → lonng),
//   所以 shadow 整段套用的 TL_ENCODING_RULES 絕不可帶它;逐音節的表要留著,
//   canonical_tl_form 靠它守跨模式身分(Core Principle #7)。
#[test]
fn invariant_roman_nasal_oo_alias_never_folds_whole_buffer() {
    assert!(
        !phonetics::TL_ENCODING_RULES
            .iter()
            .any(|(find, _)| *find == "oonn"),
        "TL_ENCODING_RULES is applied whole-buffer and must not fold across a syllable seam",
    );
    assert!(
        phonetics::NORMALIZE_TO_TL_RULES
            .iter()
            .any(|(find, _)| *find == "oonn"),
        "the per-syllable list must keep the fold",
    );
}
