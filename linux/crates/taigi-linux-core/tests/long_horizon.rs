//! Long-horizon in-process user-interaction harness for `TaigiKeyboard`.

mod long_horizon_support;

use long_horizon_support::corpus::{load_corpus, parse_example_sentences_js};
use long_horizon_support::host::HostSimulator;
use long_horizon_support::personas::{build_episode_settings, compile_sentence_steps, PersonaKind};
use long_horizon_support::rng::Rng;
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
