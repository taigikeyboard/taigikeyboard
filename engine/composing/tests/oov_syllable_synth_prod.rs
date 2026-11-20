//! Production-lexicon regression for #1030: a lone syllable whose only
//! dictionary home is a longer word (`huh` ← `goo-lú-huh`) is not offered
//! as a bare roman candidate unless Show Typed Text First asks for it.

use crate::common::{config, fetch_cells, production_lexicon_ready, Cell, Fetch};

fn cells(raw: &str, mode: &str, literal_disabled: bool) -> Vec<Cell> {
    fetch_cells(
        &config(mode),
        raw,
        Fetch {
            literal_roman_candidate_disabled: literal_disabled,
            ..Default::default()
        },
    )
}

#[test]
fn lone_oov_syllable_is_not_a_bare_candidate_with_literal_off() {
    if !production_lexicon_ready() {
        eprintln!("production artifacts absent — run `make dict`; skipping.");
        return;
    }
    // trace: `huh` / `guh` / `aunn` are syllables of `goo-lú-huh`,
    // `kha-tá-lok-guh`, `ut-áunn` only → the walker spans the buffer with one
    // OOV edge → all-OOV carve-out, one greedy segment → no slot-0 synth.
    for (mode, raw) in [("tl", "huh"), ("tl", "guh"), ("tl", "aunn"), ("poj", "huh")] {
        let got = cells(raw, mode, true);
        assert!(
            !got.iter().any(|c| c.0.is_none() && c.1 == raw),
            "{mode} {raw}: bare `{raw}` must not be offered; got {got:?}"
        );
        assert!(
            got.first().is_some_and(|c| c.0.is_some()),
            "{mode} {raw}: slot 0 should carry hanji; got {got:?}"
        );
    }
}

#[test]
fn literal_on_still_leads_with_the_typed_text() {
    if !production_lexicon_ready() {
        return;
    }
    let got = cells("huh", "tl", false);
    assert_eq!(
        got.first().map(|c| (c.0.as_deref(), c.1.as_str())),
        Some((None, "huh")),
        "huh with literal on: §34 literal at slot 0; got {got:?}"
    );
}

#[test]
fn multi_syllable_oov_synth_and_dictionary_hits_are_kept() {
    if !production_lexicon_ready() {
        return;
    }
    // `huhguh`: two OOV syllables, no dictionary hit anywhere → the carve-out
    // keeps its two-segment synth, which adds the word break the literal lacks.
    let got = cells("huhguh", "tl", true);
    assert_eq!(
        got.first().map(|c| (c.0.as_deref(), c.1.as_str())),
        Some((None, "huh guh")),
        "huhguh: spaced synth at slot 0; got {got:?}"
    );
    // TPS has no §34 literal: its one-syllable synth (the typed Bopomofo) stays.
    let got = cells("ㄏㄨㆷ", "tps", true);
    assert!(
        got.iter().any(|c| c.0.is_none()),
        "ㄏㄨㆷ: TPS synth kept; got {got:?}"
    );
    // `kuh` has the one-syllable word 据 `ku̍h`: the walker never reaches the
    // carve-out and slot 0 is unchanged.
    let got = cells("kuh", "tl", true);
    assert_eq!(
        got.first().and_then(|c| c.0.as_deref()),
        Some("据"),
        "kuh: 据 at slot 0; got {got:?}"
    );
}
