//! Long-horizon user-interaction simulator and metamorphic / corpus-guided
//! oracle suite over `taigi_desktop_core::session` + real dictionaries + real
//! SQLite `userdata` stores.
//!
//! Quick smoke suite (runs in normal `cargo test`):
//!   `cargo test -p taigi-linux-core --test long_horizon`
//!
//! 30-minute marathon run (release-optimized):
//!   `TAIGI_MARATHON_SECS=1800 cargo test --release -p taigi-linux-core --test long_horizon long_horizon_marathon -- --ignored --nocapture`

mod long_horizon_support;

use long_horizon_support::corpus::{load_corpus, parse_example_sentences_js};
use long_horizon_support::host::{HostSimulator, SimStep, SpecialKey};
use long_horizon_support::oracles::{run_metamorphic_suite, ReachabilityStats};
use long_horizon_support::personas::{build_episode_settings, compile_sentence_steps, PersonaKind};
use long_horizon_support::rng::Rng;
use long_horizon_support::shrinker::shrink_episode_failure;
use long_horizon_support::{acquire_process_env, reset_all_persistent_state};

#[test]
fn corpus_loader_and_js_trailing_comma_parser_work() {
    let env = acquire_process_env();
    let _ = (env.user_data_dir, env.dictionaries_dir);

    let sample_js = r#"
    const exampleSentences = [
      [
        {
          "hanji": "食飽未？今仔日天氣真好。",
          "tailo": "Tsia̍h-pá--buē? Kin-á-ji̍t thinn-khì tsin hó.",
          "dic_url": "synthetic://0",
        },
      ],
    ];
    "#;
    let parsed = parse_example_sentences_js(sample_js).expect("parses JS with trailing commas");
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].hanji, "食飽未？今仔日天氣真好。");

    let corpus = load_corpus(Some(24));
    assert!(!corpus.sentences.is_empty());
    let first = &corpus.sentences[0];
    assert!(!first.segments.is_empty());
}

#[test]
fn all_six_personas_preserve_host_and_session_invariants() {
    let env = acquire_process_env();
    reset_all_persistent_state();
    let corpus = load_corpus(Some(30));
    let mut rng = Rng::new(0x5441_4947_2026_0007);

    for persona in PersonaKind::ALL {
        reset_all_persistent_state();
        let settings = build_episode_settings(persona, &mut rng);
        let mut sim = HostSimulator::new(
            env.user_data_dir,
            env.dictionaries_dir,
            &settings,
            rng.chance(3, 4),
        );
        for i in 0..6 {
            let sentence = &corpus.sentences[i % corpus.sentences.len()];
            let steps = compile_sentence_steps(persona, sentence, &settings, &mut rng);
            for step in &steps {
                if let Err(v) = sim.step(step) {
                    panic!(
                        "persona {} tripped {} on sentence #{}: {} (step={:?})",
                        persona.name(),
                        v.code,
                        sentence.index,
                        v.message,
                        v.step
                    );
                }
            }
        }
    }
    reset_all_persistent_state();
}

#[test]
fn metamorphic_confluences_and_shrinker_work() {
    let env = acquire_process_env();
    reset_all_persistent_state();
    let corpus = load_corpus(Some(16));
    let mut stats = ReachabilityStats::default();

    for sentence in corpus.sentences.iter().take(6) {
        if let Err(fail) = run_metamorphic_suite(
            env.user_data_dir,
            env.dictionaries_dir,
            sentence,
            &mut stats,
        ) {
            panic!("metamorphic failure: {fail:?}");
        }
    }
    assert!(stats.metamorphic_checks >= 4);

    // Verify two-stage shrinker reduces noisy batches + steps down to the exact 2-step fault.
    let settings = taigi_desktop_core::settings::SettingsDocument::default();
    let prior = vec![
        vec![
            SimStep::KeyChar {
                ch: 'a',
                shift: false,
            },
            SimStep::Special(SpecialKey::Enter),
        ],
        vec![
            SimStep::KeyChar {
                ch: 'i',
                shift: false,
            },
            SimStep::Special(SpecialKey::Enter),
        ],
        vec![
            SimStep::KeyChar {
                ch: 'u',
                shift: false,
            },
            SimStep::Special(SpecialKey::Enter),
        ],
    ];
    let failing = vec![
        SimStep::KeyChar {
            ch: 'k',
            shift: false,
        },
        SimStep::KeyChar {
            ch: 'x',
            shift: false,
        },
        SimStep::KeyChar {
            ch: 'a',
            shift: false,
        },
        SimStep::Special(SpecialKey::Backspace),
        SimStep::KeyChar {
            ch: '!',
            shift: true,
        },
        SimStep::Special(SpecialKey::Enter),
    ];
    let shrunk = shrink_episode_failure(
        "synthetic-fault",
        "unit test",
        &settings,
        &prior,
        &failing,
        |_p, s| {
            let has_x = s.contains(&SimStep::KeyChar {
                ch: 'x',
                shift: false,
            });
            let has_bang = s.contains(&SimStep::KeyChar {
                ch: '!',
                shift: true,
            });
            has_x && has_bang
        },
    );
    assert_eq!(shrunk.shrunk_prior_sentences, 0);
    assert_eq!(shrunk.shrunk_steps, 2);
}
