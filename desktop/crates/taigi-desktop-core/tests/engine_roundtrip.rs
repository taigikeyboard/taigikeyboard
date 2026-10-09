//! End-to-end round trips against the REAL dictionary artefacts
//! (the repo-root `assets/dictionaries`, the files the installer ships), so
//! the envelope, the install path and the composing/continuous ops are
//! proven on the host before a Windows machine ever runs them.
//!
//! The composing engine is a process singleton, so every test here takes the
//! same lock and its own generation: two tests interleaving `Append`s under
//! different generations would reset each other mid-composition.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use protos::engine::CommitOutcome;
use taigi_desktop_core::dictionary_artifacts::DictionaryArtifacts;
use taigi_desktop_core::engine::{self, CommitContinuousArgs, Effect};
use taigi_desktop_core::settings::{EngineSettings, InputMode};

fn dictionaries_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../assets/dictionaries")
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
    // The caret rides the effect: at the end, which is 3 UTF-16 units here.
    assert!(transition.effects.iter().any(|effect| matches!(
        effect,
        Effect::UpdatePreedit { text, caret_utf16: 3 } if text == "tâi"
    )));
    engine::reset(generation);
}

#[test]
fn telex_key_writes_the_tone_and_z_spells_the_mode_affricate() {
    // trace: engine/composing/src/telex.rs — `v` is tone 2, so `te` + `v`
    // stores `te2` and renders `té`; a second tone letter replaces it
    // (`y` = tone 3 → `tè`); `z` idle starts a composition as `ts` under TL
    // and `ch` under POJ. Round-tripped with the app config, since `z`
    // resolves by `input_mode`.
    let _engine = engine();
    let settings = EngineSettings::default();
    let generation = fresh_generation();
    compose("te", &settings, generation);
    let toned = engine::telex_key("v", &settings, generation).expect("telex round trip");
    assert_eq!(toned.raw_input, "te2");
    assert_eq!(toned.display_text, "té");
    let retoned = engine::telex_key("y", &settings, generation).expect("telex round trip");
    assert_eq!(retoned.raw_input, "te3");
    assert_eq!(retoned.display_text, "tè");
    engine::reset(generation);

    let generation = fresh_generation();
    let started = engine::telex_key("z", &settings, generation).expect("telex round trip");
    assert!(started.is_composing, "an idle z starts a composition");
    assert_eq!(started.raw_input, "ts");
    engine::reset(generation);

    let generation = fresh_generation();
    let poj = EngineSettings {
        input_mode: InputMode::Poj,
        ..EngineSettings::default()
    };
    let started = engine::telex_key("z", &poj, generation).expect("telex round trip");
    assert_eq!(started.raw_input, "ch");
    engine::reset(generation);
}

fn tps_settings() -> EngineSettings {
    EngineSettings {
        input_mode: InputMode::Tps,
        ..EngineSettings::default()
    }
}

#[test]
fn tps_key_composes_glyphs_and_space_is_taken_once() {
    // trace: engine `TpsKey` — ㄍ ㄚ ㄉ: the adjuster folds ㄉ after ㄚ to ㆵ
    // (`kat` is a valid final); Space after the stop coda is the separator,
    // hidden from the preedit, and closes the reading, which TPS settings ask
    // to see converted (`app_config` → Hanji conversion; read by running:
    // 結); a second Space is refused with no effects.
    let _engine = engine();
    let settings = tps_settings();
    let generation = fresh_generation();
    for key in ["ㄍ", "ㄚ", "ㄉ"] {
        engine::tps_key(key, &settings, generation).expect("tps round trip");
    }
    let separated = engine::tps_key(" ", &settings, generation).expect("tps round trip");
    assert_eq!(separated.raw_input, "ㄍㄚㆵ ");
    assert_eq!(separated.display_text, "結");
    assert!(!separated.effects.is_empty(), "the separator is taken");
    let refused = engine::tps_key(" ", &settings, generation).expect("tps round trip");
    assert!(
        refused.effects.is_empty(),
        "a closed syllable refuses Space"
    );
    assert_eq!(refused.raw_input, "ㄍㄚㆵ ");
    engine::reset(generation);
}

#[test]
fn every_layout_glyph_begins_a_composition_the_engine_takes() {
    // D2's hand check, automated against engine-table drift: each glyph the
    // layout types, sent alone from idle, is taken and lands in the buffer.
    // The hyphen alone is §21's document literal, not a composition.
    use taigi_desktop_core::keys::{tps_glyph_for_event, KeyEventSnapshot, KeyModifiers};
    let _engine = engine();
    let settings = tps_settings();
    let keys = ('a'..='z')
        .chain('0'..='9')
        .chain(",;/.=".chars())
        .map(|key| (key.to_string(), KeyModifiers::NONE))
        .chain(
            "!#EDRY*)OMAPUJ(L:^"
                .chars()
                .map(|key| (key.to_string(), KeyModifiers::SHIFT)),
        );
    for (typed, modifiers) in keys {
        let glyph = tps_glyph_for_event(&KeyEventSnapshot::text(&typed, modifiers))
            .unwrap_or_else(|| panic!("{typed:?} is a layout key"));
        let generation = fresh_generation();
        let taken = engine::tps_key(glyph, &settings, generation).expect("tps round trip");
        assert!(!taken.effects.is_empty(), "{typed:?} → {glyph:?} taken");
        assert!(
            taken.raw_input.contains(glyph),
            "{typed:?} → {glyph:?} in {:?}",
            taken.raw_input
        );
        engine::reset(generation);
    }
}

#[test]
fn tl_display_to_tps_spells_a_reading_in_tps() {
    // trace: phonetics `tl_display_to_tps` — `ka` → ㄍㄚ; `kò` (tone 3) →
    // ㄍㄛ˪; `or` follows the flag (ㄜ when it maps to `er`).
    assert_eq!(
        engine::tl_display_to_tps("ka", true).as_deref(),
        Some("ㄍㄚ")
    );
    assert_eq!(
        engine::tl_display_to_tps("kò", true).as_deref(),
        Some("ㄍㄛ˪")
    );
    // trace: tps.rs `to_zhuyin` — vowel `or` is ㄜ, or ㄛ when the flag is off.
    assert_eq!(engine::tl_display_to_tps("or", true).as_deref(), Some("ㄜ"));
    assert_eq!(
        engine::tl_display_to_tps("or", false).as_deref(),
        Some("ㄛ")
    );
}

#[test]
fn fetch_at_pos_returns_dictionary_candidates_and_commit_finalizes() {
    let _engine = engine();
    let settings = EngineSettings::default();
    let generation = fresh_generation();
    compose("taigi", &settings, generation);

    let fetch = engine::fetch_at_pos(&settings, generation, 0).expect("fetch round trip");
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
            script: engine::CommitScript::Lead,
            roman: &taigi.roman,
            canonical_text: &taigi.display_text,
            association_tl: &taigi.canonical_tl,
            hanji: taigi.hanji.as_deref(),
            consumed_bytes: taigi.consumed_span_end,
            syllable_count: taigi.syllable_count,
        },
        &settings,
        generation,
    )
    .expect("commit round trip");
    // trace: hanji-first default → LEAD resolves the Hanji; no romanization
    // written, so no auto space (engine `commit_text`).
    assert_eq!(commit.commit.outcome(), CommitOutcome::Finalized);
    assert_eq!(commit.commit.document_text, "台語");
    assert!(!commit.commit.earns_auto_space);
    let commit = commit.transition;
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
    let fetch = engine::fetch_at_pos(&settings, generation, 0).expect("fetch");
    let candidates = fetch.candidates.expect("continuous");
    let tai = candidates
        .iter()
        .find(|candidate| {
            candidate.hanji.as_deref() == Some("台") && candidate.consumed_span_end == 3
        })
        .expect("single-syllable 台 spanning `tai`");
    let commit = engine::commit_continuous(
        &CommitContinuousArgs {
            script: engine::CommitScript::Lead,
            roman: &tai.roman,
            canonical_text: "台",
            association_tl: &tai.canonical_tl,
            hanji: tai.hanji.as_deref(),
            consumed_bytes: tai.consumed_span_end,
            syllable_count: tai.syllable_count,
        },
        &settings,
        generation,
    )
    .expect("commit");
    assert_eq!(commit.commit.outcome(), CommitOutcome::Nailed);
    let commit = commit.transition;
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

/// §34 under the shipped defaults: Show Typed Text First is OFF out of the box on
/// every platform (USER 2026-10-02), so with TL/POJ text composed slot 0 is a
/// dictionary candidate, not the preedit literal. What a commit writes either
/// way is `composing_manager.rs`'s
/// `enter_on_a_fresh_bar_commits_the_typed_literal_in_either_mode` /
/// `…_commits_the_dictionary_word_when_the_literal_row_is_off`.
#[test]
fn literal_roman_candidate_is_absent_under_the_shipped_defaults() {
    let _engine = engine();
    let generation = fresh_generation();
    let settings = EngineSettings::default();
    compose("tai", &settings, generation);
    let candidates = engine::fetch_at_pos(&settings, generation, 0)
        .expect("fetch")
        .candidates
        .expect("continuous");
    assert!(
        candidates[0].hanji.is_some(),
        "§34 off: nothing forces the one-script literal to the front: {:?}",
        candidates[0]
    );
    engine::reset(generation);
}

#[test]
fn permissive_tones_are_standard_on_the_desktop() {
    let _engine = engine();
    let generation = fresh_generation();
    let settings = EngineSettings {
        is_literal_roman_candidate_enabled: true,
        ..EngineSettings::default()
    };
    let mut last = None;
    for character in "tai5gi2".chars() {
        last = engine::append(&character.to_string(), &settings, generation);
    }
    assert_eq!(last.unwrap().display_text, "tâigí");
    let snapshot = engine::fetch_at_pos(&settings, generation, 0).unwrap();
    assert_eq!(snapshot.transition.display_text, "tâigí");
    assert_eq!(snapshot.candidates.unwrap()[0].roman, "tâigí");
    engine::reset(generation);
}

#[test]
fn telex_keys_mark_unseparated_syllables() {
    // trace: the reported Telex keys `taidgiv` — `d` = tone 5, `v` = tone 2
    // (engine/composing/src/telex.rs) — store `tai5gi2`, which renders
    // `tâigí` (each tone digit closes its segment).
    let _engine = engine();
    let settings = EngineSettings::default();
    let generation = fresh_generation();
    compose("tai", &settings, generation);
    engine::telex_key("d", &settings, generation).expect("telex round trip");
    compose("gi", &settings, generation);
    let toned = engine::telex_key("v", &settings, generation).expect("telex round trip");
    assert_eq!(toned.raw_input, "tai5gi2");
    assert_eq!(toned.display_text, "tâigí");
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
    let candidates = engine::fetch_at_pos(&poj, generation, 0)
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
        "the engine emits only the clear handshake here; the platform sends ResetAll itself"
    );
}

// INVARIANT_DICTIONARIES_ALL_OFF_OFFERS_NO_DICTIONARY_CANDIDATES (behavioral-invariants.md §57)
#[test]
fn all_sources_off_fetches_no_dictionary_candidates() {
    let _engine = engine();
    let mut settings = EngineSettings::default();
    let sources = &mut settings.dictionary_sources;
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
    let generation = fresh_generation();
    compose("taigi", &settings, generation);
    let candidates = engine::fetch_at_pos(&settings, generation, 0)
        .expect("fetch round trip")
        .candidates
        .expect("continuous phase answers with a list");
    let hanji: Vec<_> = candidates
        .iter()
        .filter_map(|c| c.hanji.as_deref())
        .collect();
    assert!(hanji.is_empty(), "no dictionary hanji: {hanji:?}");
    assert!(
        !candidates.is_empty(),
        "the typed-text literal is not a dictionary row"
    );
    engine::reset(generation);
}

#[test]
fn default_toggles_resolve_their_badge_sources() {
    let _engine = engine();
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
