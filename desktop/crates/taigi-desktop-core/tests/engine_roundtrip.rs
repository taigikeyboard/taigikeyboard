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
use taigi_desktop_core::engine::{self, CommitContinuousArgs, Effect, RequestStamp};
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
fn fresh_stamp() -> RequestStamp {
    static NEXT: AtomicU64 = AtomicU64::new(1000);
    RequestStamp {
        generation: NEXT.fetch_add(1, Ordering::Relaxed),
        now_ms: 0,
    }
}

fn compose(
    text: &str,
    settings: &EngineSettings,
    stamp: RequestStamp,
) -> engine::ComposingTransition {
    let mut last = None;
    for character in text.chars() {
        let step =
            engine::append(&character.to_string(), settings, stamp).expect("append round trip");
        last = Some(step);
    }
    last.expect("non-empty text")
}

#[test]
fn append_renders_tone_digit_as_diacritic_and_updates_preedit() {
    let _engine = engine();
    let settings = EngineSettings::default();
    let stamp = fresh_stamp();
    let transition = compose("tai5", &settings, stamp);
    assert!(transition.is_composing);
    assert_eq!(transition.raw_input, "tai5");
    assert_eq!(transition.display_text, "tâi");
    // The caret rides the effect: at the end, which is 3 UTF-16 units here.
    assert!(transition.effects.iter().any(|effect| matches!(
        effect,
        Effect::UpdatePreedit { text, caret_utf16: 3 } if text == "tâi"
    )));
    engine::reset(stamp);
}

#[test]
fn telex_key_writes_the_tone() {
    // trace: engine/composing/src/telex.rs — `v` is tone 2, so `te` + `v`
    // stores `te2` and renders `té`; a second tone letter replaces it
    // (`y` = tone 3 → `tè`, `z` = tone 6 → `tě`).
    let _engine = engine();
    let settings = EngineSettings::default();
    let stamp = fresh_stamp();
    compose("te", &settings, stamp);
    let toned = engine::telex_key("v", &settings, stamp).expect("telex round trip");
    assert_eq!(toned.raw_input, "te2");
    assert_eq!(toned.display_text, "té");
    let retoned = engine::telex_key("y", &settings, stamp).expect("telex round trip");
    assert_eq!(retoned.raw_input, "te3");
    assert_eq!(retoned.display_text, "tè");
    let sixth = engine::telex_key("z", &settings, stamp).expect("telex round trip");
    assert_eq!(sixth.raw_input, "te6");
    assert_eq!(sixth.display_text, "tě");
    engine::reset(stamp);
}

#[test]
fn append_c_spells_the_mode_affricate_only_when_the_setting_is_on() {
    // trace: `app_config` sends `is_c_affricate_enabled` as
    // `AppConfig.c_types_affricate`; engine `affricate::expand` writes `ts`
    // under TL and `ch` under POJ, and leaves `c` alone when off.
    let _engine = engine();
    for (mode, is_on, raw) in [
        (InputMode::Tl, true, "ts"),
        (InputMode::Poj, true, "ch"),
        (InputMode::Tl, false, "c"),
        (InputMode::Poj, false, "c"),
    ] {
        let settings = EngineSettings {
            input_mode: mode,
            is_c_affricate_enabled: is_on,
            ..EngineSettings::default()
        };
        let stamp = fresh_stamp();
        let started = engine::append("c", &settings, stamp).expect("append round trip");
        assert!(started.is_composing, "{mode:?} {is_on}");
        assert_eq!(started.raw_input, raw, "{mode:?} {is_on}");
        engine::reset(stamp);
    }
}

#[test]
fn append_f_types_the_hyphen_only_when_the_setting_is_on() {
    // trace: `app_config` sends `is_f_hyphen_enabled` as
    // `AppConfig.f_types_hyphen`; engine `f_hyphen::expand` writes `-` after
    // `tai5` and leaves `f` alone when off.
    let _engine = engine();
    for (is_on, raw) in [(true, "tai5-"), (false, "tai5f")] {
        let settings = EngineSettings {
            is_f_hyphen_enabled: is_on,
            ..EngineSettings::default()
        };
        let stamp = fresh_stamp();
        engine::append("tai5", &settings, stamp).expect("append round trip");
        let after = engine::append("f", &settings, stamp).expect("append round trip");
        assert_eq!(after.raw_input, raw, "{is_on}");
        engine::reset(stamp);
    }
}

fn tps_settings() -> EngineSettings {
    EngineSettings {
        input_mode: InputMode::Tps,
        is_hanji_conversion_enabled: true,
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
    let stamp = fresh_stamp();
    for key in ["ㄍ", "ㄚ", "ㄉ"] {
        engine::tps_key(key, &settings, stamp).expect("tps round trip");
    }
    let separated = engine::tps_key(" ", &settings, stamp).expect("tps round trip");
    assert_eq!(separated.raw_input, "ㄍㄚㆵ ");
    assert_eq!(separated.display_text, "結");
    assert!(!separated.effects.is_empty(), "the separator is taken");
    let refused = engine::tps_key(" ", &settings, stamp).expect("tps round trip");
    assert!(
        refused.effects.is_empty(),
        "a closed syllable refuses Space"
    );
    assert_eq!(refused.raw_input, "ㄍㄚㆵ ");
    engine::reset(stamp);
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
        .chain(",;/.-".chars())
        .map(|key| (key.to_string(), KeyModifiers::NONE))
        .chain(
            "!#EDRY*)OPUJ(L^>?_"
                .chars()
                .map(|key| (key.to_string(), KeyModifiers::SHIFT)),
        );
    for (typed, modifiers) in keys {
        let glyph = tps_glyph_for_event(&KeyEventSnapshot::text(&typed, modifiers))
            .unwrap_or_else(|| panic!("{typed:?} is a layout key"));
        let stamp = fresh_stamp();
        let taken = engine::tps_key(glyph, &settings, stamp).expect("tps round trip");
        assert!(!taken.effects.is_empty(), "{typed:?} → {glyph:?} taken");
        assert!(
            taken.raw_input.contains(glyph),
            "{typed:?} → {glyph:?} in {:?}",
            taken.raw_input
        );
        engine::reset(stamp);
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
    let stamp = fresh_stamp();
    compose("taigi", &settings, stamp);

    let fetch = engine::fetch_at_pos(&settings, stamp).expect("fetch round trip");
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
        stamp,
    )
    .expect("commit round trip");
    // trace: hanji-first default → LEAD resolves the Hanji; no romanization
    // written, so no auto space (engine `commit_text`).
    assert_eq!(commit.commit.outcome(), CommitOutcome::Finalized);
    assert_eq!(commit.commit.document_text, "台語");
    assert!(!commit.commit.earns_auto_space);
    let taught = commit.commit.taught_next_word;
    let commit = commit.transition;
    assert!(
        !commit.is_composing,
        "consuming the whole buffer is a final commit"
    );
    assert!(commit.effects.iter().any(
        |effect| matches!(effect, Effect::CommitTextReplacingPreedit(text) if text == "台語")
    ));
    // The engine applied the final-commit handshake in the same call: the
    // commit taught next word, and the handshake's place holds its decision.
    assert!(taught);
    assert!(commit.effects.contains(&Effect::NextWordDecided));
}

#[test]
fn partial_commit_nails_a_segment_and_stays_composing() {
    let _engine = engine();
    let settings = EngineSettings::default();
    let stamp = fresh_stamp();
    compose("taigi", &settings, stamp);
    let fetch = engine::fetch_at_pos(&settings, stamp).expect("fetch");
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
        stamp,
    )
    .expect("commit");
    assert_eq!(commit.commit.outcome(), CommitOutcome::Nailed);
    let commit = commit.transition;
    assert!(
        commit.is_composing,
        "Model B: a nailed segment stays in the composition"
    );
    // The nail handshake, applied by the engine.
    assert!(commit.effects.contains(&Effect::NextWordDecided));
    assert!(
        !commit
            .effects
            .iter()
            .any(|effect| matches!(effect, Effect::CommitTextReplacingPreedit(_))),
        "nothing reaches the document until the final commit"
    );
    engine::reset(stamp);
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
    let stamp = fresh_stamp();
    let settings = EngineSettings::default();
    compose("tai", &settings, stamp);
    let candidates = engine::fetch_at_pos(&settings, stamp)
        .expect("fetch")
        .candidates
        .expect("continuous");
    assert!(
        candidates[0].hanji.is_some(),
        "§34 off: nothing forces the one-script literal to the front: {:?}",
        candidates[0]
    );
    engine::reset(stamp);
}

#[test]
fn permissive_tones_are_standard_on_the_desktop() {
    let _engine = engine();
    let stamp = fresh_stamp();
    let settings = EngineSettings {
        is_literal_roman_candidate_enabled: true,
        ..EngineSettings::default()
    };
    let mut last = None;
    for character in "tai5gi2".chars() {
        last = engine::append(&character.to_string(), &settings, stamp);
    }
    assert_eq!(last.unwrap().display_text, "tâigí");
    let snapshot = engine::fetch_at_pos(&settings, stamp).unwrap();
    assert_eq!(snapshot.transition.display_text, "tâigí");
    assert_eq!(snapshot.candidates.unwrap()[0].roman, "tâigí");
    engine::reset(stamp);
}

#[test]
fn telex_keys_mark_unseparated_syllables() {
    // trace: the reported Telex keys `taidgiv` — `d` = tone 5, `v` = tone 2
    // (engine/composing/src/telex.rs) — store `tai5gi2`, which renders
    // `tâigí` (each tone digit closes its segment).
    let _engine = engine();
    let settings = EngineSettings::default();
    let stamp = fresh_stamp();
    compose("tai", &settings, stamp);
    engine::telex_key("d", &settings, stamp).expect("telex round trip");
    compose("gi", &settings, stamp);
    let toned = engine::telex_key("v", &settings, stamp).expect("telex round trip");
    assert_eq!(toned.raw_input, "tai5gi2");
    assert_eq!(toned.display_text, "tâigí");
    engine::reset(stamp);
}

#[test]
fn poj_mode_renders_poj_display_and_keeps_canonical_tl() {
    let _engine = engine();
    let stamp = fresh_stamp();
    let poj = EngineSettings {
        input_mode: InputMode::Poj,
        ..EngineSettings::default()
    };
    compose("chiah", &poj, stamp);
    let candidates = engine::fetch_at_pos(&poj, stamp)
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
    engine::reset(stamp);
}

#[test]
fn confirm_composition_then_insert_is_one_effect() {
    let _engine = engine();
    let settings = EngineSettings::default();
    let stamp = fresh_stamp();
    compose("gua2", &settings, stamp);
    let committed =
        engine::confirm_composition_then_insert(None, " ", false, false, &settings, stamp)
            .expect("commit");
    let taught = committed.commit.taught_next_word;
    let commit = committed.transition;
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
    // The engine applied only the clear handshake here (nothing picked); the
    // platform sends ResetAll itself.
    assert!(commit.effects.contains(&Effect::NextWordDecided));
    assert!(!taught, "an unpicked composition teaches next word nothing");
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
    let stamp = fresh_stamp();
    compose("taigi", &settings, stamp);
    let candidates = engine::fetch_at_pos(&settings, stamp)
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
    engine::reset(stamp);
}

/// The badges a search row wears are the row's sources the user has on:
/// the default toggles badge the MOE dictionary, never iTaigi (off by
/// default) — until iTaigi is switched on.
#[test]
fn default_toggles_resolve_their_badge_sources() {
    use engine::dictionary_search::lookup;
    use engine::DictionarySource;

    // trace: `DictionarySourceToggles::DEFAULT` has kautian on, itaigi off
    // (engine_settings.rs); the shipped dictionary holds MOE and iTaigi rows
    // for tsiah (食 / tsia̍h is MOE; the positive control below pins iTaigi).
    let _engine = engine();
    let badges = |toggles| -> Vec<DictionarySource> {
        lookup("tsiah", InputMode::Tl, &toggles, false, 20)
            .into_iter()
            .flat_map(|result| result.sources)
            .collect()
    };
    let defaults = EngineSettings::default().dictionary_sources;
    let default_badges = badges(defaults.clone());
    assert!(default_badges.contains(&DictionarySource::Kautian));
    assert!(!default_badges.contains(&DictionarySource::Itaigi));
    let mut with_itaigi = defaults;
    with_itaigi.itaigi = true;
    assert!(
        badges(with_itaigi).contains(&DictionarySource::Itaigi),
        "positive control: an iTaigi row exists for tsiah"
    );
}
