//! Production-lexicon regression for hyphen-split toned groups (#867):
//! `ti-a2` is partial tone (§17 case 3), not the one toned syllable `tia2`.

use crate::common::{fetch_hanji, production_lexicon_ready, Fetch};

fn hanji(raw: &str, mode: &str) -> Vec<String> {
    fetch_hanji(raw, mode, Fetch::default())
}

fn has(got: &[String], word: &str) -> bool {
    got.iter().any(|h| h == word)
}

#[test]
fn hyphen_closes_an_untoned_syllable_before_a_vowel() {
    if !production_lexicon_ready() {
        eprintln!("production artifacts absent — run `make dict`; skipping.");
        return;
    }
    // trace: `ti-a2` → shadow `tia2`, barrier 2 inside the `tia` group →
    // not fully toned → key `tl:tia` + TypedTones{typed "tia2", boundary 2}
    // → 豬仔 `ti1a2` passes (syllable ends at 2, tail tone 2).
    let cases = [
        ("tl", "ti-a2", "豬仔"),
        ("tl", "to-a2", "刀仔"),
        ("tl", "poo3-lu-a2", "布攄仔"),
        ("tl", "a-i5", "阿姨"),
        ("tl", "tshiu2-ki-a2", "手機仔"),
        ("poj", "ti-a2", "豬仔"),
        ("poj", "a-i5", "阿姨"),
    ];
    for (mode, raw, word) in cases {
        let got = hanji(raw, mode);
        assert!(has(&got, word), "{mode} {raw}: {word} missing; got {got:?}");
    }
}

#[test]
fn typed_tones_and_boundaries_still_filter() {
    if !production_lexicon_ready() {
        return;
    }
    // The pin still honours every typed tone on the toneless key: 豬仔 is
    // `ti1a2`, so a typed `5` on `a` or `7` on `ti` drops it.
    for raw in ["ti-a5", "ti7-a2"] {
        let got = hanji(raw, "tl");
        assert!(!has(&got, "豬仔"), "{raw}: 豬仔 must drop; got {got:?}");
    }
    // `a-i5` needs a syllable end at the `-`: the one-syllable 噯 `âi`
    // drops. Without the hyphen `ai5` is one toned syllable (verbatim
    // `tl:ai5`): 噯 stays and 阿姨 `a1i5` does not come back.
    let got = hanji("a-i5", "tl");
    assert!(!has(&got, "噯"), "a-i5: 噯 must drop; got {got:?}");
    let got = hanji("ai5", "tl");
    assert!(
        has(&got, "噯") && !has(&got, "阿姨"),
        "ai5: want 噯, not 阿姨; got {got:?}"
    );
    // Likewise `tia2` keeps its verbatim key and does not reach 豬仔.
    let got = hanji("tia2", "tl");
    assert!(
        !has(&got, "豬仔"),
        "tia2: 豬仔 must not surface; got {got:?}"
    );
}

#[test]
fn controls_keep_their_words() {
    if !production_lexicon_ready() {
        return;
    }
    // `tai5-gi2`: every syllable toned, the `-` right after a digit — the
    // verbatim `tl:tai5gi2` key, unchanged. `kin-a2`: `kina` is no single
    // syllable, so it already took the toneless + pin path.
    for (raw, word) in [("tai5-gi2", "台語"), ("kin-a2", "巾仔")] {
        let got = hanji(raw, "tl");
        assert!(has(&got, word), "{raw}: {word} missing; got {got:?}");
    }
}
