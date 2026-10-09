//! Choosing a word in a converted TPS preedit
//! (`docs/architecture/desktop-tps-hanji-conversion-roadmap.md` H4, H5, H7):
//! the list of the word before the caret, a pick that nails what precedes it
//! as shown and keeps composing, the commits as shown and as typed, the
//! picked mark that keeps unpicked words out of learning, and a walk that
//! ranks with the user's learned counts.
//!
//! The fixture of `tps_hanji_conversion.rs` (its strict-prefix controls
//! included), plus 絲 (si, a homophone of 詩 the user can prefer).

use std::cell::RefCell;

use composing::api::{CaretDirection, CommitScript};
use composing::{requests, ConversionFrequency, Engine, Intent, ListContext, Phase};
use lexicon::LearnedEntry;
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};
use protos::engine::effect::Kind;
use protos::engine::{AppConfig, CandidateMessage, CommitOutcome, ComposingResponse};
use ranking::FrequencyMap;
use test_support::engine_install_lock;

use crate::common::{
    commit_text, config, config_converting, converted_words, effect_kinds, frequency_map, selected,
    Fetch, Selected, NOW_MS,
};
use crate::tps_hanji_conversion::{
    caret_utf16, composing_engine, display, install_fixture_with, nailed, pick, raw_input, step,
    tps_key,
};

/// 絲 (si1, frequency 10) under 詩 (si1, 50): the same toned reading.
const EXTRA_ROWS: &[(&str, &str, u32)] = &[("絲", "si", 10)];

/// The list `fetch` answers; empty when idle.
fn list_with(engine: &mut Engine, fetch: Fetch, config: &AppConfig) -> Vec<CandidateMessage> {
    requests::apply(fetch.intent(), engine, config)
        .continuous
        .map(|continuous| continuous.candidates)
        .unwrap_or_default()
}

/// The list `FetchAtPos` answers.
fn list(engine: &mut Engine, config: &AppConfig) -> Vec<CandidateMessage> {
    list_with(engine, Fetch::default(), config)
}

fn spans_and_hanji(candidates: &[CandidateMessage]) -> Vec<(u32, Option<String>)> {
    candidates
        .iter()
        .map(|c| (c.consumed_span_end, c.hanji.clone()))
        .collect()
}

/// `(display, raw, picked)` of each nailed segment.
fn nailed_shown(engine: &Engine) -> Vec<(String, String, bool)> {
    nailed(engine)
        .into_iter()
        .map(|segment| (segment.display_text, segment.raw_text, segment.is_picked))
        .collect()
}

fn outcome(response: &ComposingResponse) -> Option<i32> {
    response.commit.as_ref().map(|commit| commit.outcome)
}

// trace: "ㄒㄧˋㄒㄧ " → 死 (0, 8) + 詩 (8, 15), caret 15. The word before the
// caret is 詩, so the list is "ㄒㄧ "'s, from 8: the Space pins tone 1, so 詩
// and 絲 (si1), 詩 first by frequency; spans in the tail's coordinates. The
// whole tail's list leads with the walker's phrase 死詩 (0, 15); the word's
// list never holds a phrase. After a step left the word before the caret is
// 死: its list starts at 0 — 死 alone (`ㄒㄧˋ` is tone 2), no 死詩.
#[test]
fn the_list_is_the_word_before_the_caret() {
    let _lock = engine_install_lock();
    install_fixture_with(EXTRA_ROWS);
    let config = config_converting("tps");
    let (mut engine, _) = composing_engine("ㄒㄧˋㄒㄧ ", &config);
    assert_eq!(engine.word_list_start(&config), Some(8));
    assert_eq!(
        spans_and_hanji(&list(&mut engine, &config)),
        vec![(15, Some("詩".to_string())), (15, Some("絲".to_string()))]
    );

    step(&mut engine, CaretDirection::Left, &config);
    assert_eq!(engine.word_list_start(&config), Some(0));
    assert_eq!(
        spans_and_hanji(&list(&mut engine, &config)),
        vec![(8, Some("死".to_string()))]
    );
    // At the start of the tail the list is the first word's.
    step(&mut engine, CaretDirection::Left, &config);
    assert_eq!(engine.word_list_start(&config), Some(0));
}

// trace: "ㄒㄧˋㄒㄧ" — 死 (0, 8), then the open reading "ㄒㄧ" (8, 14) the caret
// ends. The list is that reading's: every tone of ㄒㄧ, from 8.
#[test]
fn an_open_reading_before_the_caret_lists_its_own_words() {
    let _lock = engine_install_lock();
    install_fixture_with(EXTRA_ROWS);
    let config = config_converting("tps");
    let (mut engine, response) = composing_engine("ㄒㄧˋㄒㄧ", &config);
    assert_eq!(display(&response), "死ㄒㄧ");
    assert_eq!(engine.word_list_start(&config), Some(8));
    let listed = list(&mut engine, &config);
    let hanji: Vec<_> = listed.iter().filter_map(|c| c.hanji.as_deref()).collect();
    for word in ["是", "死", "詩", "絲"] {
        assert!(hanji.contains(&word), "{word} in {hanji:?}");
    }
}

// Without the switch the list is the whole tail's, the walker's phrase 死詩
// (0, 15) first, as before the conversion.
#[test]
fn without_the_conversion_the_list_is_the_whole_tails() {
    let _lock = engine_install_lock();
    install_fixture_with(EXTRA_ROWS);
    let config = config("tps");
    let (mut engine, _) = composing_engine("ㄒㄧˋㄒㄧ ", &config);
    assert_eq!(engine.word_list_start(&config), None);
    let whole = spans_and_hanji(&list(&mut engine, &config));
    assert_eq!(whole[0], (15, Some("死詩".to_string())));
}

// A pick that leaves only a tone mark pending still belongs to a TPS
// composition (cloud review 2026-10-05): the list stays the word's and its
// pick keeps composing. trace: "ㄒㄧˋˊ" — the second mark after the closed
// reading stops the lattice, nothing converts, the list starts at 0; picking
// 死 (0, 8) nails it and leaves "ˊ", a tail with no Bopomofo of its own.
#[test]
fn a_tail_of_tone_marks_after_a_pick_keeps_composing() {
    let _lock = engine_install_lock();
    install_fixture_with(EXTRA_ROWS);
    let config = config_converting("tps");
    let (mut engine, _) = composing_engine("ㄒㄧˋˊ", &config);
    assert_eq!(engine.word_list_start(&config), Some(0));
    engine.apply(pick("死", "sí", 8), &config);
    assert_eq!(raw_input(&engine.snapshot(&config)), "ˊ");
    assert_eq!(engine.word_list_start(&config), Some(0));
    let row = list(&mut engine, &config)
        .into_iter()
        .next()
        .expect("the tone mark's own row");
    let response = engine.apply(
        Intent::CommitContinuous {
            canonical_text: row.display_text.clone(),
            association_tl: row.canonical_tl.clone(),
            hanji: row.hanji.clone(),
            consumed_bytes: row.consumed_span_end as usize,
            syllable_count: row.syllable_count as u8,
            script: Some(CommitScript::Lead),
            roman: row.roman.clone(),
        },
        &config,
    );
    assert_eq!(outcome(&response), Some(CommitOutcome::Nailed as i32));
    assert!(response.is_composing);
}

// Un-nailing back to a segment nailed as shown names no selected word.
// trace: pick 絲 at the end → 死 (not picked) 絲, empty tail; Backspace
// un-nails 絲, leaving 死 last: the handshake is Clear, not
// UpdateLastSelectedWord.
#[test]
fn unnailing_back_to_an_unpicked_segment_names_no_word() {
    let _lock = engine_install_lock();
    install_fixture_with(EXTRA_ROWS);
    let config = config_converting("tps");
    let (mut engine, _) = composing_engine("ㄒㄧˋㄒㄧ ", &config);
    engine.apply(pick("絲", "si", 15), &config);
    let response = engine.apply(Intent::DeleteBackward, &config);
    assert_eq!(
        effect_kinds(&response.effect)[0],
        "NextWordClearForNewComposing"
    );
    assert_eq!(nailed(&engine).len(), 1);
}

// The pick of the last word reaches the end of the tail and keeps composing.
// trace: "ㄒㄧˋㄒㄧ " caret 15, list from 8; pick 絲 ending at 15 → 死 (0, 8)
// nailed as shown, not picked; 絲 nailed, picked; the tail is empty; the
// composition stays. The pick's usage is counted; the window closes.
#[test]
fn a_pick_nails_the_words_before_it_as_shown_and_keeps_composing() {
    let _lock = engine_install_lock();
    install_fixture_with(EXTRA_ROWS);
    let config = config_converting("tps");
    let (mut engine, _) = composing_engine("ㄒㄧˋㄒㄧ ", &config);
    let applied = engine.apply_learning(pick("絲", "si", 15), &config);
    let response = &applied.response;
    assert_eq!(outcome(response), Some(CommitOutcome::Nailed as i32));
    assert!(response.is_composing);
    assert_eq!(display(response), "死絲");
    assert_eq!(raw_input(response), "");
    assert_eq!(caret_utf16(response), 2);
    assert_eq!(
        effect_kinds(&response.effect),
        vec![
            "UpdatePreedit",
            "NextWordUpdateLastSelectedWord",
            "ClearCandidates"
        ]
    );
    assert_eq!(
        nailed_shown(&engine),
        vec![
            ("死".to_string(), "ㄒㄧˋ".to_string(), false),
            ("絲".to_string(), "ㄒㄧ ".to_string(), true),
        ]
    );
    let usage = applied.usage.expect("the pick is counted");
    assert_eq!(
        (usage.display_text.as_str(), usage.canonical_tl.as_str()),
        ("絲", "si")
    );
    // The word nailed as shown keeps its identity for context.
    assert_eq!(nailed(&engine)[0].association_tl, "sí");
    assert_eq!(nailed(&engine)[0].hanji.as_deref(), Some("死"));
}

// A pick of the first word: nothing before it; the rest is walked again and
// the caret goes to its end. trace: Left (caret 8, after 死) → list from 0 →
// pick 是 ending at 8 → tail "ㄒㄧ " → 詩 (0, 7), caret 7.
#[test]
fn a_pick_inside_the_tail_walks_the_rest_and_moves_the_caret_to_the_end() {
    let _lock = engine_install_lock();
    install_fixture_with(EXTRA_ROWS);
    let config = config_converting("tps");
    let (mut engine, _) = composing_engine("ㄒㄧˋㄒㄧ ", &config);
    step(&mut engine, CaretDirection::Left, &config);
    let response = engine.apply(pick("是", "sī", 8), &config);
    assert_eq!(display(&response), "是詩");
    assert_eq!(caret_utf16(&response), 2);
    assert_eq!(converted_words(&engine), vec![((0, 7), "詩".to_string())]);
    assert_eq!(
        nailed_shown(&engine),
        vec![("是".to_string(), "ㄒㄧˋ".to_string(), true)]
    );
}

// The list starts at the word before the caret; a pick that does not end
// after that start is not of this list and is ignored. trace: caret 15, list
// from 8, a pick ending at 8.
#[test]
fn a_pick_not_ending_after_the_list_start_is_ignored() {
    let _lock = engine_install_lock();
    install_fixture_with(EXTRA_ROWS);
    let config = config_converting("tps");
    let (mut engine, _) = composing_engine("ㄒㄧˋㄒㄧ ", &config);
    let before = engine.snapshot_state();
    let response = engine.apply(pick("是", "sī", 8), &config);
    assert_eq!(outcome(&response), Some(CommitOutcome::Ignored as i32));
    assert_eq!(engine.snapshot_state(), before);
}

// Glyphs between words are nailed as shown too, byte for byte. trace:
// "ㄒㄧˋㄒㄧ " → Left (8) → ㄒ typed there opens a reading the syllabifier
// cannot read ("ㄒㄒㄧ"), so the words on both sides stay: 死 (0, 8), the
// glyph ㄒ (8, 11), 詩 (11, 18). Right steps over 詩 to 18; the list starts at
// 11 and the glyphs before it cut the context. Picking 詩 nails 死 · ㄒ · 詩;
// their raw text is what was typed.
#[test]
fn glyphs_before_the_list_start_are_nailed_as_typed() {
    let _lock = engine_install_lock();
    install_fixture_with(EXTRA_ROWS);
    let config = config_converting("tps");
    let (mut engine, _) = composing_engine("ㄒㄧˋㄒㄧ ", &config);
    step(&mut engine, CaretDirection::Left, &config);
    let response = tps_key(&mut engine, "ㄒ", &config);
    assert_eq!(display(&response), "死ㄒ詩");
    step(&mut engine, CaretDirection::Right, &config);
    assert_eq!(engine.word_list_start(&config), Some(11));
    let snapshot = engine.pending_snapshot(&config);
    assert_eq!(snapshot.listed_raw, "ㄒㄧ ");
    assert_eq!(snapshot.context, ListContext::Cut);

    let response = engine.apply(pick("詩", "si", 18), &config);
    assert_eq!(display(&response), "死ㄒ詩");
    assert_eq!(
        nailed_shown(&engine),
        vec![
            ("死".to_string(), "ㄒㄧˋ".to_string(), false),
            ("ㄒ".to_string(), "ㄒ".to_string(), false),
            ("詩".to_string(), "ㄒㄧ ".to_string(), true),
        ]
    );
    let typed: String = nailed(&engine)
        .iter()
        .map(|s| s.raw_text.as_str())
        .collect();
    assert_eq!(typed, "ㄒㄧˋㄒㄒㄧ ");
    let glyph = &nailed(&engine)[1];
    assert_eq!(
        (glyph.hanji.as_deref(), glyph.association_tl.as_str()),
        (None, "")
    );
}

// The list's context word is the converted word ending at its start.
// trace: "ㄒㄧˋㄒㄧ " caret 15 → list from 8 follows 死 (sí); after a step left
// the list starts the composition and the committed context applies.
#[test]
fn the_list_follows_the_converted_word_before_it() {
    let _lock = engine_install_lock();
    install_fixture_with(EXTRA_ROWS);
    let config = config_converting("tps");
    let (mut engine, _) = composing_engine("ㄒㄧˋㄒㄧ ", &config);
    let snapshot = engine.pending_snapshot(&config);
    assert_eq!(
        snapshot.context,
        ListContext::Word("死".to_string(), "sí".to_string())
    );
    step(&mut engine, CaretDirection::Left, &config);
    let snapshot = engine.pending_snapshot(&config);
    assert_eq!(snapshot.context, ListContext::Committed);
}

// Commit as shown writes the preedit; with a word the user did not pick it
// teaches nothing. trace: pick 絲 at the end → 死 (not picked) 絲 → "死絲".
#[test]
fn commit_as_shown_writes_the_preedit_and_teaches_nothing_unpicked() {
    let _lock = engine_install_lock();
    install_fixture_with(EXTRA_ROWS);
    let config = config_converting("tps");
    let (mut engine, _) = composing_engine("ㄒㄧˋㄒㄧ ", &config);
    engine.apply(pick("絲", "si", 15), &config);
    let applied = engine.apply_learning(Intent::CommitAsShown, &config);
    assert_eq!(commit_text(&applied.response).as_deref(), Some("死絲"));
    assert_eq!(
        effect_kinds(&applied.response.effect),
        vec![
            "CommitTextReplacingPreedit",
            "ClearCandidates",
            "ResetCandidateContext",
            "NextWordClearForNewComposing"
        ]
    );
    assert!(!applied.response.is_composing);
    assert_eq!((applied.learned, applied.usage), (None, None));
}

// A pending tail is the walker's, never picked: the same. trace: "ㄒㄧˋㄒㄧ"
// shows 死ㄒㄧ and writes it.
#[test]
fn commit_as_shown_with_a_pending_tail_writes_its_glyphs() {
    let _lock = engine_install_lock();
    install_fixture_with(EXTRA_ROWS);
    let config = config_converting("tps");
    let (mut engine, _) = composing_engine("ㄒㄧˋㄒㄧ", &config);
    let applied = engine.apply_learning(Intent::CommitAsShown, &config);
    assert_eq!(commit_text(&applied.response).as_deref(), Some("死ㄒㄧ"));
    assert_eq!(
        effect_kinds(&applied.response.effect).last(),
        Some(&"NextWordClearForNewComposing")
    );
    assert_eq!(applied.learned, None);
}

// Every word picked: commit as shown teaches as a final pick does. trace:
// Left → pick 是 (8) → tail "ㄒㄧ " → pick 詩 (7) → 是詩, both picked, nothing
// pending. Learned phrase 是詩 / `sī si` (two words, no compound in the
// fixture); next word learns 詩 after 是.
#[test]
fn commit_as_shown_of_picked_words_learns_the_phrase_and_the_sequence() {
    let _lock = engine_install_lock();
    install_fixture_with(EXTRA_ROWS);
    let config = config_converting("tps");
    let (mut engine, _) = composing_engine("ㄒㄧˋㄒㄧ ", &config);
    step(&mut engine, CaretDirection::Left, &config);
    engine.apply(pick("是", "sī", 8), &config);
    engine.apply(pick("詩", "si", 7), &config);
    let applied = engine.apply_learning(Intent::CommitAsShown, &config);
    assert_eq!(commit_text(&applied.response).as_deref(), Some("是詩"));
    assert_eq!(
        applied.learned,
        Some(LearnedEntry {
            hanji: "是詩".to_string(),
            canonical_tl: "sī si".to_string(),
        })
    );
    let selected = applied
        .response
        .effect
        .iter()
        .find_map(|effect| match &effect.kind {
            Some(Kind::NextWordWordSelected(selected)) => Some(selected.clone()),
            _ => None,
        })
        .expect("NextWordWordSelected");
    assert_eq!(
        (selected.text.as_str(), selected.roman.as_str()),
        ("詩", "si")
    );
    assert_eq!(
        selected
            .preceding
            .iter()
            .map(|word| word.text.as_str())
            .collect::<Vec<_>>(),
        vec!["是"]
    );
}

// Commit as typed writes the glyphs of the whole composition, the picks
// included, separators dropped; it teaches nothing.
#[test]
fn commit_as_typed_writes_the_glyphs_of_the_whole_composition() {
    let _lock = engine_install_lock();
    install_fixture_with(EXTRA_ROWS);
    let config = config_converting("tps");
    let (mut engine, _) = composing_engine("ㄒㄧˋㄒㄧ ", &config);
    engine.apply(pick("絲", "si", 15), &config);
    tps_key(&mut engine, "ㄒ", &config);
    let applied = engine.apply_learning(Intent::CommitAsTyped, &config);
    assert_eq!(
        commit_text(&applied.response).as_deref(),
        Some("ㄒㄧˋㄒㄧㄒ")
    );
    assert_eq!(
        effect_kinds(&applied.response.effect).last(),
        Some(&"NextWordClearForNewComposing")
    );
    assert_eq!(applied.learned, None);
    assert_eq!(engine.snapshot_state().phase, Phase::Idle);
}

// A key that commits the composition before itself (punctuation, Shift+Space)
// writes it as shown, then the key, in one write; it teaches nothing, as
// without the conversion. trace: pick 絲 at the end → 死 (not picked) 絲, then
// `ㄒㄧˋ` shows 死絲死 → "死絲死？".
#[test]
fn commit_then_insert_writes_the_composition_as_shown() {
    let _lock = engine_install_lock();
    install_fixture_with(EXTRA_ROWS);
    let config = config_converting("tps");
    let (mut engine, _) = composing_engine("ㄒㄧˋㄒㄧ ", &config);
    engine.apply(pick("絲", "si", 15), &config);
    for key in ["ㄒ", "ㄧ", "ˋ"] {
        tps_key(&mut engine, key, &config);
    }
    let response = engine.apply(
        Intent::CommitPreeditThenInsertExternal {
            text: "？".to_string(),
        },
        &config,
    );
    assert_eq!(commit_text(&response).as_deref(), Some("死絲死？"));
    assert_eq!(
        effect_kinds(&response.effect).last(),
        Some(&"NextWordClearForNewComposing")
    );
    assert_eq!(engine.snapshot_state().phase, Phase::Idle);
}

// A word nailed as shown is never one end of a next-word pair: the final
// legacy-shaped terminal of an all-nailed composition with an unpicked
// segment answers Clear. trace: pick 絲 at the end → 死 (not picked) 絲;
// CommitRaw (Enter, D7) writes the nailed text and teaches nothing.
#[test]
fn commit_raw_after_an_unpicked_segment_teaches_nothing() {
    let _lock = engine_install_lock();
    install_fixture_with(EXTRA_ROWS);
    let config = config_converting("tps");
    let (mut engine, _) = composing_engine("ㄒㄧˋㄒㄧ ", &config);
    engine.apply(pick("絲", "si", 15), &config);
    let response = engine.apply(Intent::CommitRaw, &config);
    assert_eq!(commit_text(&response).as_deref(), Some("死絲"));
    assert_eq!(
        effect_kinds(&response.effect).last(),
        Some(&"NextWordClearForNewComposing")
    );
}

/// A store of learned counts: the rows it holds, and every word a walk asked
/// it for.
struct Counts {
    rows: Vec<Selected>,
    asked: RefCell<Vec<String>>,
}

impl Counts {
    fn new(rows: Vec<Selected>) -> Self {
        Self {
            rows,
            asked: RefCell::new(Vec::new()),
        }
    }
}

impl ConversionFrequency for Counts {
    fn rows_for_words(&self, words: &[String]) -> FrequencyMap {
        self.asked.borrow_mut().extend(words.iter().cloned());
        frequency_map(
            self.rows
                .iter()
                .filter(|row| words.contains(&row.hanji))
                .cloned(),
        )
    }

    fn now_ms(&self) -> i64 {
        NOW_MS
    }
}

// The walk ranks with the user's counts for every word an edge could take.
// trace: "ㄒㄧ " (Space pins tone 1) → 詩 (50) over 絲 (10) neutral; a count for
// 絲 / `si` leads the pick (user weight first) → 絲. The words asked for are
// the homophones of the one edge. A count under another reading (絲 / `sī`)
// is another word and changes nothing.
#[test]
fn the_walk_ranks_with_the_users_counts() {
    let _lock = engine_install_lock();
    install_fixture_with(EXTRA_ROWS);
    let config = config_converting("tps");
    let walk = |rows| {
        let counts = Counts::new(rows);
        let mut engine = Engine::new();
        let applied = engine.apply_ranked(
            Intent::Start {
                text: "ㄒㄧ ".into(),
            },
            &config,
            Some(&counts),
        );
        (
            display(&applied.response).to_string(),
            counts.asked.into_inner(),
        )
    };
    let (neutral, asked) = walk(Vec::new());
    assert_eq!(neutral, "詩");
    assert_eq!(asked, vec!["詩".to_string(), "絲".to_string()]);
    assert_eq!(walk(vec![selected("絲", "si", 5, 1_000)]).0, "絲");
    assert_eq!(walk(vec![selected("絲", "sī", 5, 1_000)]).0, "詩");
}

// A glyph of an open reading walks nothing, so it asks for no counts.
#[test]
fn an_open_reading_asks_for_no_counts() {
    let _lock = engine_install_lock();
    install_fixture_with(EXTRA_ROWS);
    let config = config_converting("tps");
    let counts = Counts::new(Vec::new());
    let mut engine = Engine::new();
    engine.apply_ranked(
        Intent::Start {
            text: "ㄒㄧ".into(),
        },
        &config,
        Some(&counts),
    );
    assert!(counts.asked.borrow().is_empty());
}

/// One step of a random session under the conversion.
#[derive(Clone, Debug)]
enum Step {
    Key(&'static str),
    Caret(CaretDirection),
    DeleteBackward,
    /// Pick the `n`-th row of the list of the word before the caret.
    Pick(usize),
    CommitAsShown,
    CommitAsTyped,
}

fn arb_step() -> impl Strategy<Value = Step> {
    prop_oneof![
        6 => prop::sample::select(vec!["ㄒ", "ㄧ", "ˋ", " ", "ㄍ", "ㄣ"]).prop_map(Step::Key),
        2 => prop::sample::select(vec![
            CaretDirection::Left,
            CaretDirection::Right,
            CaretDirection::Start,
            CaretDirection::End,
        ])
            .prop_map(Step::Caret),
        1 => Just(Step::DeleteBackward),
        2 => (0usize..3).prop_map(Step::Pick),
        1 => Just(Step::CommitAsShown),
        1 => Just(Step::CommitAsTyped),
    ]
}

/// What every response under the conversion keeps: the caret on a char
/// boundary and never inside a word, the written preedit equal to the one
/// reported, the nailed segments contiguous over what was typed, and a
/// commit as shown writing what the preedit showed.
fn check_session(steps: Vec<Step>, config: &AppConfig) -> Result<(), TestCaseError> {
    let mut engine = Engine::new();
    for step in steps {
        let shown_before = engine
            .snapshot(config)
            .preedit
            .unwrap_or_default()
            .display_text;
        let response = match step {
            Step::Key(key) => tps_key(&mut engine, key, config),
            Step::Caret(direction) => self::step(&mut engine, direction, config),
            Step::DeleteBackward => engine.apply(Intent::DeleteBackward, config),
            Step::Pick(index) => {
                let listed = list(&mut engine, config);
                let Some(row) = listed.get(index) else {
                    continue;
                };
                engine.apply(
                    Intent::CommitContinuous {
                        canonical_text: row.display_text.clone(),
                        association_tl: row.canonical_tl.clone(),
                        hanji: row.hanji.clone(),
                        consumed_bytes: row.consumed_span_end as usize,
                        syllable_count: row.syllable_count as u8,
                        script: Some(CommitScript::Lead),
                        roman: row.roman.clone(),
                    },
                    config,
                )
            }
            Step::CommitAsShown => {
                let response = engine.apply(Intent::CommitAsShown, config);
                if !shown_before.is_empty() {
                    prop_assert_eq!(commit_text(&response), Some(shown_before));
                }
                response
            }
            Step::CommitAsTyped => engine.apply(Intent::CommitAsTyped, config),
        };
        let preedit = response.preedit.clone().unwrap_or_default();
        for effect in &response.effect {
            if let Some(Kind::UpdatePreedit(update)) = &effect.kind {
                prop_assert_eq!(&update.display, &preedit.display_text);
                prop_assert_eq!(update.caret_utf16, preedit.caret_utf16);
            }
        }
        let Phase::Continuous {
            raw,
            caret,
            nailed,
            conversion,
        } = engine.snapshot_state().phase
        else {
            continue;
        };
        prop_assert!(!raw.is_empty() || !nailed.is_empty());
        prop_assert!(raw.is_char_boundary(caret) && caret <= raw.len());
        for word in conversion
            .iter()
            .flat_map(|conversion| &conversion.segments)
        {
            prop_assert!(!(word.raw_span.0 < caret && caret < word.raw_span.1));
        }
        let mut position = 0;
        for segment in &nailed {
            prop_assert_eq!(
                segment.raw_span,
                (position, position + segment.raw_text.len())
            );
            position = segment.raw_span.1;
        }
    }
    Ok(())
}

#[test]
fn random_sessions_keep_the_conversion_invariants() {
    let _lock = engine_install_lock();
    install_fixture_with(EXTRA_ROWS);
    let config = config_converting("tps");
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 256,
        ..ProptestConfig::default()
    });
    runner
        .run(&prop::collection::vec(arb_step(), 1..=16), |steps| {
            check_session(steps, &config)
        })
        .unwrap();
}

// Glyphs nailed as shown carry no word identity, so a list after them has no
// context word, and not the committed one either. trace: the glyph session
// of `glyphs_before_the_list_start_are_nailed_as_typed` ends 死 · ㄒ · 詩
// nailed; Left from the empty tail re-opens 詩: nailed 死 · ㄒ, tail "ㄒㄧ "
// with the caret at 0, so the list starts the tail and follows the glyph ㄒ.
#[test]
fn a_list_after_nailed_glyphs_has_no_context_word() {
    let _lock = engine_install_lock();
    install_fixture_with(EXTRA_ROWS);
    let config = config_converting("tps");
    let (mut engine, _) = composing_engine("ㄒㄧˋㄒㄧ ", &config);
    step(&mut engine, CaretDirection::Left, &config);
    tps_key(&mut engine, "ㄒ", &config);
    step(&mut engine, CaretDirection::Right, &config);
    engine.apply(pick("詩", "si", 18), &config);
    step(&mut engine, CaretDirection::Left, &config);
    assert_eq!(nailed(&engine).len(), 2);
    assert_eq!(engine.word_list_start(&config), Some(0));
    let snapshot = engine.pending_snapshot(&config);
    assert_eq!(snapshot.context, ListContext::Cut);
}

// The word's list puts longer words first, whatever the counts (H4).
// trace: "ㄍㄧㄣ ㄚˋㆢㄧㆵ˙ㄒㄧˋ" → 今仔日 (0, 26) + 死 (26, 34); Left puts the
// caret after 今仔日, whose list starts at 0 over the whole tail: 今仔日 (0, 26)
// and 今 (0, 10), both short of the tail. A count for 今 would lead the
// whole tail's ranking (user weight first); the word's list keeps 今仔日 first.
#[test]
fn the_words_list_puts_longer_words_first() {
    let _lock = engine_install_lock();
    install_fixture_with(EXTRA_ROWS);
    let config = config_converting("tps");
    let (mut engine, response) = composing_engine("ㄍㄧㄣ ㄚˋㆢㄧㆵ˙ㄒㄧˋ", &config);
    assert_eq!(display(&response), "今仔日死");
    step(&mut engine, CaretDirection::Left, &config);
    assert_eq!(engine.word_list_start(&config), Some(0));
    let fetch = || Fetch {
        now_ms: NOW_MS,
        frequency: vec![selected("今", "kin", 20, 1_000)],
        ..Fetch::default()
    };
    let word = spans_and_hanji(&list_with(&mut engine, fetch(), &config));
    // The same tail without the switch: the whole tail's ranking.
    let unconverted = crate::common::config("tps");
    let (mut plain, _) = composing_engine("ㄍㄧㄣ ㄚˋㆢㄧㆵ˙ㄒㄧˋ", &unconverted);
    let whole = spans_and_hanji(&list_with(&mut plain, fetch(), &unconverted));
    let position = |rows: &[(u32, Option<String>)], hanji: &str| {
        rows.iter()
            .position(|(_, h)| h.as_deref() == Some(hanji))
            .unwrap_or_else(|| panic!("{hanji} in {rows:?}"))
    };
    assert!(
        position(&whole, "今") < position(&whole, "今仔日"),
        "{whole:?}"
    );
    assert_eq!(word[0], (26, Some("今仔日".to_string())));
    assert!(
        position(&word, "今仔日") < position(&word, "今"),
        "{word:?}"
    );
}
