//! The developer explain view's sentence cut is the slot-0 walker synth's —
//! the same rule, the same gates (#1030 one-syllable suppression, the
//! trailing-hyphen gate, a word list's one-word filter). When the list
//! carries no synth the view shows no cut; when it does, the view's words
//! are that synth's.

use crate::common::{config, continuous_engine, production_lexicon_ready};
use composing::api::UserRows;
use protos::engine::AppConfig;

/// What the view shows for one buffer.
#[derive(Debug)]
struct Explained {
    cut_by: &'static str,
    /// Word romans of the cut, in order.
    words: Vec<String>,
    /// `(hanji, roman)` of the first candidate row.
    first: Option<(Option<String>, String)>,
}

/// The view for `raw` under `config`, Show Typed Text First off (the §34
/// literal would otherwise lead every TL / POJ list).
fn explained_with(config: &AppConfig, raw: &str) -> Explained {
    let engine = continuous_engine(config, raw);
    let view = composing::explain::mechanism(
        &engine,
        0,
        &UserRows::default(),
        &ranking::ContextRanks::default(),
        true,
        config,
    )
    .expect("a composition is open");
    Explained {
        cut_by: view.cut_by,
        words: view.words.iter().map(|word| word.roman.clone()).collect(),
        first: view
            .candidates
            .first()
            .map(|row| (row.hanji.clone(), row.roman.clone())),
    }
}

fn explained(raw: &str, mode: &str) -> Explained {
    explained_with(&config(mode), raw)
}

/// The list leads with the synth the view shows: its words joined by the
/// slot-0 word space.
fn assert_cut_leads(raw: &str, got: &Explained, cut_by: &str, words: &[&str]) {
    assert_eq!(
        (got.cut_by, got.words.as_slice()),
        (
            cut_by,
            words
                .iter()
                .map(|w| (*w).to_owned())
                .collect::<Vec<_>>()
                .as_slice()
        ),
        "{raw}: {got:?}"
    );
    assert_eq!(
        got.first.as_ref().map(|(_, roman)| roman.as_str()),
        Some(words.join(" ").as_str()),
        "{raw}: the list leads with the cut the view shows; {got:?}"
    );
}

#[test]
fn explain_cut_is_the_slot0_synth_cut() {
    if !production_lexicon_ready() {
        eprintln!("production artifacts absent — run `make dict`; skipping.");
        return;
    }
    // trace: `huh` is a syllable only inside `goo-lú-huh` → one OOV edge,
    // all-OOV carve-out, one greedy segment → #1030 suppresses the synth in
    // TL / POJ; the list leads with 芳苑鄉 / 風雨學 (prefix rows) and never
    // offers the `huh` reading. Before R3 the view showed `syllables ["huh"]`.
    for mode in ["tl", "poj"] {
        let got = explained("huh", mode);
        assert_eq!(
            (got.cut_by, got.words.as_slice()),
            ("none", &[] as &[String]),
            "{mode} huh: no synth → no cut; {got:?}"
        );
        assert_ne!(
            got.first
                .as_ref()
                .map(|(hanji, roman)| (hanji.as_deref(), roman.as_str())),
            Some((None, "huh")),
            "{mode} huh: {got:?}"
        );
    }
}

#[test]
fn trailing_hyphen_shows_no_cut() {
    if !production_lexicon_ready() {
        return;
    }
    // trace: the pending `-` keeps the shadow short of raw → `synth_consumed_span`
    // suppresses the synth; the list leads with the span-local 大 `tāi`.
    // Before R3 the view showed `walker ["tāi"]` / `walker ["tāi", "ba̍k"]`.
    for raw in ["tai-", "tai-bak-"] {
        let got = explained(raw, "tl");
        assert_eq!(
            (got.cut_by, got.words.as_slice()),
            ("none", &[] as &[String]),
            "{raw}: {got:?}"
        );
        assert_eq!(
            got.first,
            Some((Some("大".to_owned()), "tāi".to_owned())),
            "{raw}: {got:?}"
        );
    }
}

#[test]
fn listed_synths_keep_their_cut() {
    if !production_lexicon_ready() {
        return;
    }
    // Fixture rule: `hu`, the production syllable that is a strict prefix of
    // `huh`, is a dictionary word (赴 `hù`) → a one-edge walker cut.
    assert_cut_leads("hu", &explained("hu", "tl"), "walker", &["hù"]);
    // trace: two OOV syllables → the carve-out keeps its two-segment synth.
    assert_cut_leads(
        "huhguh",
        &explained("huhguh", "tl"),
        "syllables",
        &["huh", "guh"],
    );
    // trace: TPS has no §34 literal, so #1030 leaves its one-syllable synth.
    let tps = phonetics::api::tl_numeric_to_tps("huh4", false);
    assert_cut_leads(&tps, &explained(&tps, "tps"), "syllables", &[tps.as_str()]);
    assert_cut_leads(
        "taiuantai",
        &explained("taiuantai", "tl"),
        "walker",
        &["tâi-uân", "tāi"],
    );
}
