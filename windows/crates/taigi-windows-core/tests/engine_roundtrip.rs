//! End-to-end round trips against the REAL dictionary artefacts
//! (`windows/resources/Dictionaries`, the files the installer ships), so
//! the envelope, the install path and the composing/continuous ops are
//! proven on the host before a Windows machine ever runs them.
//!
//! The composing engine is a process singleton, so every test here takes the
//! same lock and its own generation: two tests interleaving `Append`s under
//! different generations would reset each other mid-composition.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use taigi_windows_core::dictionary_artifacts::DictionaryArtifacts;
use taigi_windows_core::engine::{
    self, CommitContinuousArgs, CustomEntry, Effect, FetchArgs, FrequencyRow,
};
use taigi_windows_core::settings::{EngineSettings, InputMode};

fn dictionaries_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../resources/Dictionaries")
}

/// Installs the lexicon once per test process and hands out the engine lock.
fn engine() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    static INSTALLED: OnceLock<engine::LexiconInstallStats> = OnceLock::new();
    let guard = LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    INSTALLED.get_or_init(|| {
        let artifacts =
            DictionaryArtifacts::locate(&dictionaries_dir()).expect("repo dictionaries present");
        let stats = engine::lexicon_install(&artifacts, 1).expect("lexicon installs");
        assert!(stats.dictionary_record_count > 100_000, "{stats:?}");
        stats
    });
    guard
}

/// Fresh generation per test, from 1000 like the macOS `TestFixtures`, so a
/// composition left behind by one test is dropped by the next.
fn fresh_generation() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1000);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

fn compose(text: &str, settings: &EngineSettings, generation: u64) -> engine::ComposingTransition {
    let mut last = None;
    for character in text.chars() {
        let step = engine::append(&character.to_string(), settings, generation)
            .expect("append round trip");
        last = Some(step);
    }
    engine::enter_continuous(settings, generation).expect("enter continuous");
    last.expect("non-empty text")
}

#[test]
fn append_renders_tone_digit_as_diacritic_and_updates_preedit() {
    let _engine = engine();
    let settings = EngineSettings::default();
    let generation = fresh_generation();
    let transition = compose("tai5", &settings, generation);
    assert!(transition.is_composing);
    assert_eq!(transition.raw_input, "tai5");
    assert_eq!(transition.display_text, "tâi");
    assert!(transition
        .effects
        .iter()
        .any(|effect| matches!(effect, Effect::UpdatePreedit(display) if display == "tâi")));
    engine::reset(generation);
}

#[test]
fn fetch_at_pos_returns_dictionary_candidates_and_commit_finalizes() {
    let _engine = engine();
    let settings = EngineSettings::default();
    let generation = fresh_generation();
    compose("taigi", &settings, generation);

    let sources = engine::enabled_sources_bitmask(&settings.dictionary_sources);
    assert_ne!(
        sources, 0,
        "toggles resolve through the engine, never a failed 0"
    );
    let fetch = engine::fetch_at_pos(
        &settings,
        generation,
        &FetchArgs {
            enabled_sources_bitmask: sources,
            ..FetchArgs::default()
        },
    )
    .expect("fetch round trip");
    let candidates = fetch
        .candidates
        .expect("continuous phase answers with a list");
    let taigi = candidates
        .iter()
        .find(|candidate| candidate.hanji.as_deref() == Some("台語"))
        .unwrap_or_else(|| {
            panic!(
                "台語 among {:?}",
                candidates
                    .iter()
                    .map(|c| &c.display_text)
                    .collect::<Vec<_>>()
            )
        });
    assert_eq!(taigi.canonical_tl, "tâi-gí");
    assert_eq!(taigi.consumed_span_end, 5, "whole `taigi` buffer");

    let commit = engine::commit_continuous(
        &CommitContinuousArgs {
            document_text: &taigi.display_text,
            canonical_text: &taigi.display_text,
            association_tl: &taigi.canonical_tl,
            consumed_bytes: taigi.consumed_span_end,
            syllable_count: taigi.syllable_count,
        },
        &settings,
        generation,
    )
    .expect("commit round trip");
    assert!(
        !commit.is_composing,
        "consuming the whole buffer is a final commit"
    );
    assert!(commit.effects.iter().any(
        |effect| matches!(effect, Effect::CommitTextReplacingPreedit(text) if text == "台語")
    ));
    assert!(commit.effects.iter().any(|effect| matches!(
        effect,
        Effect::NextWordWordSelected { text, roman, .. } if text == "台語" && roman == "tâi-gí"
    )));
}

#[test]
fn partial_commit_nails_a_segment_and_stays_composing() {
    let _engine = engine();
    let settings = EngineSettings::default();
    let generation = fresh_generation();
    compose("taigi", &settings, generation);
    let fetch = engine::fetch_at_pos(&settings, generation, &FetchArgs::default()).expect("fetch");
    let candidates = fetch.candidates.expect("continuous");
    let tai = candidates
        .iter()
        .find(|candidate| {
            candidate.hanji.as_deref() == Some("台") && candidate.consumed_span_end == 3
        })
        .expect("single-syllable 台 spanning `tai`");
    let commit = engine::commit_continuous(
        &CommitContinuousArgs {
            document_text: "台",
            canonical_text: "台",
            association_tl: &tai.canonical_tl,
            consumed_bytes: tai.consumed_span_end,
            syllable_count: tai.syllable_count,
        },
        &settings,
        generation,
    )
    .expect("commit");
    assert!(
        commit.is_composing,
        "Model B: a nailed segment stays in the composition"
    );
    assert!(commit.effects.iter().any(|effect| matches!(
        effect,
        Effect::NextWordUpdateLastSelectedWord { text, .. } if text == "台"
    )));
    assert!(
        !commit
            .effects
            .iter()
            .any(|effect| matches!(effect, Effect::CommitTextReplacingPreedit(_))),
        "nothing reaches the document until the final commit"
    );
    engine::reset(generation);
}

/// §34 leads the list under the shipped defaults: 顯示當咧拍的字 is ON out of
/// the box on all four platforms (USER 2026-09-03), so with TL/POJ text
/// composed the preedit literal is slot 0. What a commit of that slot writes,
/// and the OFF half of the switch, are `composing_manager.rs`'s
/// `enter_on_a_fresh_bar_commits_the_typed_literal_in_either_mode` /
/// `…_commits_the_dictionary_word_when_the_literal_row_is_off`.
#[test]
fn literal_roman_candidate_leads_the_list_under_the_shipped_defaults() {
    let _engine = engine();
    let generation = fresh_generation();
    let settings = EngineSettings::default();
    compose("tai", &settings, generation);
    let candidates = engine::fetch_at_pos(&settings, generation, &FetchArgs::default())
        .expect("fetch")
        .candidates
        .expect("continuous");
    assert_eq!(
        candidates[0].display_text, "tai",
        "§34: the preedit literal leads the list"
    );
    assert!(
        candidates[0].hanji.is_none(),
        "the literal carries one script — a commit writes the romanization"
    );
    engine::reset(generation);
}

#[test]
fn frequency_rows_re_rank_the_boosted_fetch() {
    let _engine = engine();
    let settings = EngineSettings::default();
    let generation = fresh_generation();
    compose("tai", &settings, generation);
    let neutral = engine::fetch_at_pos(&settings, generation, &FetchArgs::default())
        .expect("fetch")
        .candidates
        .expect("continuous");
    assert!(neutral.len() > 2, "{}", neutral.len());
    // Boost the LAST candidate hard; it must move up past where it was.
    let underdog = neutral.last().expect("non-empty").clone();
    assert_ne!(underdog.display_text, neutral[0].display_text);
    let rows = [FrequencyRow {
        word: underdog.display_text.clone(),
        tl: underdog.canonical_tl.clone(),
        count: 1_000,
        last_used_ms: 999_000,
    }];
    let boosted = engine::fetch_at_pos(
        &settings,
        generation,
        &FetchArgs {
            frequency_rows: &rows,
            now_ms: 1_000_000,
            ..FetchArgs::default()
        },
    )
    .expect("fetch")
    .candidates
    .expect("continuous");
    let boosted_index = boosted
        .iter()
        .position(|candidate| {
            candidate.display_text == underdog.display_text
                && candidate.canonical_tl == underdog.canonical_tl
        })
        .expect("underdog still listed");
    let neutral_index = neutral.len() - 1;
    assert!(
        boosted_index < neutral_index,
        "a learned count moves the candidate up: {neutral_index} -> {boosted_index}"
    );
    engine::reset(generation);
}

#[test]
fn custom_entries_ride_the_fetch() {
    let _engine = engine();
    let settings = EngineSettings::default();
    let generation = fresh_generation();
    compose("gautsa", &settings, generation);
    let custom = [CustomEntry {
        roman: "gâu-tsá".into(),
        hanzi: "𠢕早".into(),
    }];
    let rows = [FrequencyRow {
        word: "𠢕早".into(),
        tl: "gâu-tsá".into(),
        count: 5,
        last_used_ms: 1_000,
    }];
    let fetch = engine::fetch_at_pos(
        &settings,
        generation,
        &FetchArgs {
            frequency_rows: &rows,
            now_ms: 2_000,
            custom_entries: &custom,
            ..FetchArgs::default()
        },
    )
    .expect("fetch");
    let candidates = fetch.candidates.expect("continuous");
    assert!(
        candidates
            .iter()
            .any(|candidate| candidate.hanji.as_deref() == Some("𠢕早")),
        "custom entry surfaces: {:?}",
        candidates
            .iter()
            .map(|c| &c.display_text)
            .collect::<Vec<_>>()
    );
    engine::reset(generation);
}

#[test]
fn poj_mode_renders_poj_display_and_keeps_canonical_tl() {
    let _engine = engine();
    let generation = fresh_generation();
    let poj = EngineSettings {
        input_mode: InputMode::Poj,
        ..EngineSettings::default()
    };
    compose("chiah", &poj, generation);
    let candidates = engine::fetch_at_pos(&poj, generation, &FetchArgs::default())
        .expect("fetch")
        .candidates
        .expect("continuous");
    let eat = candidates
        .iter()
        .find(|candidate| candidate.hanji.as_deref() == Some("食"))
        .expect("食 for chiah");
    assert!(eat.roman.starts_with("chia"), "POJ display: {}", eat.roman);
    assert!(
        eat.canonical_tl.starts_with("tsia"),
        "canonical TL identity: {}",
        eat.canonical_tl
    );
    engine::reset(generation);
}

#[test]
fn commit_preedit_then_insert_external_is_one_effect() {
    let _engine = engine();
    let settings = EngineSettings::default();
    let generation = fresh_generation();
    compose("gua2", &settings, generation);
    let commit =
        engine::commit_preedit_then_insert_external(" ", &settings, generation).expect("commit");
    assert!(!commit.is_composing);
    let commits: Vec<_> = commit
        .effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::CommitTextReplacingPreedit(text) => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        commits,
        ["guá "],
        "preedit + external text in ONE document mutation"
    );
    assert!(
        commit
            .effects
            .contains(&Effect::NextWordClearForNewComposing),
        "the engine emits only the clear handshake here; the platform sends ResetFull itself"
    );
}

#[test]
fn nextword_learning_records_the_pair_within_the_window() {
    let _engine = engine();
    let settings = EngineSettings::default();
    let generation = fresh_generation();
    engine::nextword_reset_full(0, &settings, generation).expect("reset");
    let first =
        engine::nextword_word_selected("台", "tâi", 1_000, &settings, generation).expect("first");
    assert!(
        first.effects.is_empty(),
        "no predecessor yet: {:?}",
        first.effects
    );
    let second =
        engine::nextword_word_selected("語", "gí", 2_000, &settings, generation).expect("second");
    assert!(
        second.effects.iter().any(|effect| matches!(
            effect,
            engine::NextWordEffect::RecordAssociation(pair)
                if pair.previous == "台" && pair.previous_tl == "tâi" && pair.next == "語" && pair.next_tl == "gí"
        )),
        "{:?}",
        second.effects
    );
    let off = EngineSettings {
        is_association_recording_enabled: false,
        ..EngineSettings::default()
    };
    let third =
        engine::nextword_word_selected("文", "bûn", 3_000, &off, generation).expect("third");
    assert!(
        third.effects.is_empty(),
        "recording switch gates the effect: {:?}",
        third.effects
    );
}

#[test]
fn all_sources_off_resolves_to_the_non_zero_sentinel() {
    let _engine = engine();
    let mut sources = EngineSettings::default().dictionary_sources;
    sources.kautian = false;
    sources.taigitv = false;
    sources.itaigi = false;
    sources.sitbut = false;
    sources.taihoa = false;
    sources.taijit = false;
    sources.kungge = false;
    sources.stti = false;
    sources.khpoo = false;
    sources.variant = false;
    sources.khiin = false;
    sources.lkk = false;
    sources.dev = false;
    let filters = engine::dictionary_filters(&sources).expect("resolve");
    assert_eq!(
        filters.wire_mask(),
        engine::NO_SOURCES_ENABLED_BITMASK,
        "exactly the kautian-gate bit, no source bits: {:#b}",
        filters.wire_mask()
    );
    assert_eq!(engine::NO_SOURCES_ENABLED_BITMASK, 1 << 13);
    let defaults =
        engine::dictionary_filters(&EngineSettings::default().dictionary_sources).expect("resolve");
    assert!(defaults
        .enabled_sources
        .contains(&engine::DictionarySource::Kautian));
    assert!(!defaults
        .enabled_sources
        .contains(&engine::DictionarySource::Itaigi));
    assert!(
        defaults
            .enabled_sources
            .contains(&engine::DictionarySource::Custom),
        "always-on"
    );
}
