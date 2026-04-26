//! INVARIANT_* tests required by the D9 boolean check (`SKILL.md:336`).
//! Mirrors `TaigiPhoneticsTests.swift` test_INVARIANT_* methods so the iOS
//! production fixtures travel intact into the Rust crate.

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
