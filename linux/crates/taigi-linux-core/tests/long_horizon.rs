//! Long-horizon in-process user-interaction harness for `TaigiKeyboard`.

mod long_horizon_support;

use long_horizon_support::acquire_process_env;
use long_horizon_support::corpus::{load_corpus, parse_example_sentences_js};

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
