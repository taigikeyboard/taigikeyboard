//! The proto decode of every composing method, the §21 leading-hyphen literal
//! through `requests::handle`, and the desktop Telex key path.

use composing::CommitScript;
use composing::{requests, Engine, Intent};
use protos::engine::composing_request::Method;
use protos::engine::{
    Append, AppendHyphen, CommitContinuous, CommitPreeditThenInsertExternal, CommitRaw,
    CommitScript as WireCommitScript, ComposingRequest, DeleteBackward, ReplaceLast, Reset, Start,
};

use crate::common;
use crate::common::{commit_text, config_tl, req};

// §21 INVARIANT_KHINSIANN_LEADING_MARKER_LITERAL — a leading `--` typed from
// Idle is a document literal, NOT composing input (MOE-style). Production sends
// one char at a time, so the first `-` arrives as Start{"-"}.
#[test]
fn intent_start_leading_hyphen_inserts_literal_stays_idle() {
    let mut engine = Engine::new();
    let resp = requests::handle(
        &req(Method::Start(Start { text: "-".into() })),
        &mut engine,
        &config_tl(),
    )
    .unwrap();
    assert!(!resp.is_composing, "leading `-` must not enter composing");
    assert_eq!(commit_text(&resp), Some("-".to_string()));
    // No preedit underline for the literal hyphen.
    assert_eq!(resp.preedit.unwrap_or_default().display_text, "");
}

#[test]
fn intent_append_leading_hyphen_in_idle_inserts_literal() {
    let mut engine = Engine::new();
    let resp = requests::handle(
        &req(Method::Append(Append { char: "-".into() })),
        &mut engine,
        &config_tl(),
    )
    .unwrap();
    assert!(!resp.is_composing);
    assert_eq!(commit_text(&resp), Some("-".to_string()));
}

// Multi-char Start (engine API / test path): split the leading hyphen run,
// insert it literally, then compose the syllable remainder.
#[test]
fn intent_start_leading_hyphens_then_syllable_splits() {
    let mut engine = Engine::new();
    let resp = requests::handle(
        &req(Method::Start(Start {
            text: "--ah".into(),
        })),
        &mut engine,
        &config_tl(),
    )
    .unwrap();
    assert!(resp.is_composing, "the syllable remainder composes");
    assert_eq!(commit_text(&resp), Some("--".to_string()));
    assert_eq!(resp.preedit.unwrap().raw_input, "ah");
}

// Every proto method decodes to its intent. The transition each intent
// performs is pinned once, on `Engine::apply`, in `invariants.rs`.
#[test]
fn every_method_decodes_to_its_intent() {
    let cases = [
        (
            Method::Start(Start { text: "a".into() }),
            Intent::Start { text: "a".into() },
        ),
        (
            Method::Append(Append { char: "k".into() }),
            Intent::Append { ch: "k".into() },
        ),
        (Method::AppendHyphen(AppendHyphen {}), Intent::AppendHyphen),
        (
            Method::ReplaceLast(ReplaceLast {
                replacement: "c".into(),
            }),
            Intent::ReplaceLast {
                replacement: "c".into(),
            },
        ),
        (
            Method::DeleteBackward(DeleteBackward {}),
            Intent::DeleteBackward,
        ),
        (Method::CommitRaw(CommitRaw {}), Intent::CommitRaw),
        (
            Method::CommitPreeditThenInsertExternal(CommitPreeditThenInsertExternal {
                text: "🎉".into(),
            }),
            Intent::CommitPreeditThenInsertExternal {
                text: "🎉".into()
            },
        ),
        (Method::Reset(Reset {}), Intent::Reset),
        (
            Method::CommitContinuous(CommitContinuous {
                consumed_bytes: 3,
                syllable_count: 1,
                canonical_text: "珠".into(),
                association_tl: "tsu".into(),
                hanji: Some("珠".into()),
                script: WireCommitScript::Hanji as i32,
                roman: "tsu".into(),
            }),
            Intent::CommitContinuous {
                canonical_text: "珠".into(),
                association_tl: "tsu".into(),
                hanji: Some("珠".into()),
                consumed_bytes: 3,
                syllable_count: 1,
                script: Some(CommitScript::Hanji),
                roman: "tsu".into(),
            },
        ),
    ];
    for (method, expected) in cases {
        let decoded = requests::decode_intent(&req(method.clone())).expect("method present");
        assert_eq!(decoded, expected, "{method:?}");
    }
}

#[test]
fn intent_missing_method_returns_error() {
    let mut engine = Engine::new();
    let result = requests::handle(
        &ComposingRequest { method: None },
        &mut engine,
        &config_tl(),
    );
    assert!(result.is_err());
}

// ---- TelexKey (desktop Telex scheme) ---------------------------------------

fn telex(
    engine: &mut Engine,
    key: &str,
    config: &protos::engine::AppConfig,
) -> protos::engine::ComposingResponse {
    requests::handle(
        &req(Method::TelexKey(protos::engine::TelexKey {
            key: key.into(),
        })),
        engine,
        config,
    )
    .unwrap()
}

fn append(engine: &mut Engine, text: &str, config: &protos::engine::AppConfig) {
    for ch in text.chars() {
        requests::handle(
            &req(Method::Append(Append {
                char: ch.to_string(),
            })),
            engine,
            config,
        )
        .unwrap();
    }
}

#[test]
fn intent_telex_tone_key_writes_the_digit_and_renders_the_mark() {
    // trace: "te" + v → raw "te2" → normalize_tone → "té"
    let mut engine = Engine::new();
    append(&mut engine, "te", &config_tl());
    let resp = telex(&mut engine, "v", &config_tl());
    let preedit = resp.preedit.unwrap();
    assert_eq!(preedit.raw_input, "te2");
    assert_eq!(preedit.display_text, "té");
}

#[test]
fn intent_telex_second_tone_key_replaces_same_key_is_noop() {
    let mut engine = Engine::new();
    append(&mut engine, "te", &config_tl());
    telex(&mut engine, "v", &config_tl());
    let resp = telex(&mut engine, "y", &config_tl());
    assert_eq!(resp.preedit.unwrap().raw_input, "te3");
    let resp = telex(&mut engine, "y", &config_tl());
    assert!(
        resp.effect.is_empty(),
        "same tone twice must not emit effects"
    );
    assert_eq!(resp.preedit.unwrap().raw_input, "te3");
}

#[test]
fn intent_telex_z_from_idle_enters_composing_with_the_affricate() {
    let mut engine = Engine::new();
    let resp = telex(&mut engine, "z", &config_tl());
    assert!(resp.is_composing);
    assert_eq!(resp.preedit.unwrap().raw_input, "ts");
    append(&mut engine, "hi", &config_tl());
    let resp = engine.snapshot(&config_tl());
    assert_eq!(resp.preedit.unwrap().raw_input, "tshi");
}

#[test]
fn intent_telex_z_under_poj_spells_ch() {
    let mut engine = Engine::new();
    let resp = telex(&mut engine, "z", &common::config("poj"));
    assert_eq!(resp.preedit.unwrap().raw_input, "ch");
}

#[test]
fn intent_telex_tone_key_while_idle_is_noop() {
    let mut engine = Engine::new();
    let resp = telex(&mut engine, "v", &config_tl());
    assert!(!resp.is_composing);
    assert!(resp.effect.is_empty());
}

#[test]
fn intent_telex_f_appends_a_hyphen() {
    let mut engine = Engine::new();
    append(&mut engine, "tai", &config_tl());
    telex(&mut engine, "d", &config_tl());
    let resp = telex(&mut engine, "f", &config_tl());
    assert_eq!(resp.preedit.unwrap().raw_input, "tai5-");
}

#[test]
fn intent_telex_under_continuous_edits_only_the_pending_tail() {
    let mut engine = Engine::new();
    append(&mut engine, "tai", &config_tl());
    let resp = telex(&mut engine, "d", &config_tl());
    let preedit = resp.preedit.unwrap();
    assert_eq!(preedit.raw_input, "tai5");
    assert_eq!(preedit.display_text, "tâi");
    assert!(matches!(
        engine.snapshot_state().phase,
        composing::Phase::Continuous { ref raw, .. } if raw == "tai5"
    ));
}

fn nail(engine: &mut Engine, display_text: &str, consumed_bytes: usize) {
    engine.apply(
        composing::Intent::CommitContinuous {
            canonical_text: display_text.to_string(),
            association_tl: String::new(),
            hanji: None,
            consumed_bytes,
            syllable_count: 1,
            script: Some(CommitScript::Roman),
            roman: display_text.to_string(),
        },
        &config_tl(),
    );
}

fn nailed_texts(engine: &Engine) -> Vec<String> {
    match engine.snapshot_state().phase {
        composing::Phase::Continuous { nailed, .. } => {
            nailed.iter().map(|s| s.display_text.clone()).collect()
        }
        _ => panic!("expected Continuous"),
    }
}

#[test]
fn intent_telex_under_continuous_keeps_nailed_segments() {
    // Model B: nailing the whole buffer finalizes, so a nailed prefix always
    // has a non-empty pending tail beside it — `tsu` nailed out of `tsuts`.
    let mut engine = Engine::new();
    append(&mut engine, "tsuts", &config_tl());
    nail(&mut engine, "珠", 3);
    append(&mut engine, "ai", &config_tl());
    let resp = telex(&mut engine, "d", &config_tl());
    let preedit = resp.preedit.unwrap();
    assert_eq!(preedit.raw_input, "tsai5");
    assert_eq!(preedit.display_text, "珠 tsâi");
    assert_eq!(nailed_texts(&engine), vec!["珠"]);
    // A no-op on the tail leaves the nailed prefix alone too.
    let resp = telex(&mut engine, "d", &config_tl());
    assert!(resp.effect.is_empty());
    assert_eq!(nailed_texts(&engine), vec!["珠"]);
}

#[test]
fn intent_telex_tone_after_trailing_hyphen_is_noop() {
    let mut engine = Engine::new();
    append(&mut engine, "tai-", &config_tl());
    let resp = telex(&mut engine, "v", &config_tl());
    assert!(resp.effect.is_empty());
    assert_eq!(resp.preedit.unwrap().raw_input, "tai-");
}

#[test]
fn intent_telex_uppercase_f_and_poj_tone_render() {
    let poj = common::config("poj");
    let mut engine = Engine::new();
    append(&mut engine, "Pa", &poj);
    let resp = telex(&mut engine, "Y", &poj);
    assert_eq!(resp.preedit.as_ref().unwrap().display_text, "Pà");
    let resp = telex(&mut engine, "F", &poj);
    assert_eq!(resp.preedit.unwrap().raw_input, "Pa3-");
}
