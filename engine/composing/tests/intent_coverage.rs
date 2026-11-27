//! The proto decode of every composing method, §21 in-buffer leading hyphens
//! through `requests::handle`, and the desktop Telex key path.

use composing::CommitScript;
use composing::{requests, Engine, Intent};
use protos::engine::composing_request::Method;
use protos::engine::{
    Append, AppendHyphen, CommitContinuous, CommitPrediction, CommitPreeditThenInsertExternal,
    CommitRaw, CommitScript as WireCommitScript, ComposingRequest, DeleteBackward, Reset, Start,
};

use crate::common;
use crate::common::{commit_text, config_tl, req};

// §21 INVARIANT_LEADING_HYPHENS_COMPOSE_IN_BUFFER — a leading hyphen typed from
// Idle enters the composing buffer just like any other composing keystroke.
#[test]
fn intent_start_leading_hyphen_begins_composing() {
    let mut engine = Engine::new();
    let resp = requests::handle(
        &req(Method::Start(Start { text: "-".into() })),
        &mut engine,
        &config_tl(),
    )
    .unwrap();
    assert!(resp.is_composing, "leading `-` enters composing");
    assert_eq!(commit_text(&resp), None);
    let preedit = resp.preedit.unwrap();
    assert_eq!(preedit.raw_input, "-");
    assert_eq!(preedit.display_text, "-");
}

#[test]
fn intent_append_leading_hyphen_in_idle_begins_composing() {
    let mut engine = Engine::new();
    let resp = requests::handle(
        &req(Method::Append(Append { char: "-".into() })),
        &mut engine,
        &config_tl(),
    )
    .unwrap();
    assert!(resp.is_composing);
    assert_eq!(commit_text(&resp), None);
    assert_eq!(resp.preedit.unwrap().raw_input, "-");
}

// Multi-char Start (engine API / test path): the entire `--ah` stays in the
// composing buffer with no premature literal commit.
#[test]
fn intent_start_leading_hyphens_then_syllable_composes_together() {
    let mut engine = Engine::new();
    let resp = requests::handle(
        &req(Method::Start(Start {
            text: "--ah".into(),
        })),
        &mut engine,
        &config_tl(),
    )
    .unwrap();
    assert!(resp.is_composing);
    assert_eq!(commit_text(&resp), None);
    let preedit = resp.preedit.unwrap();
    assert_eq!(preedit.raw_input, "--ah");
    assert_eq!(preedit.display_text, "--ah");
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
        (
            Method::CommitPrediction(CommitPrediction {
                script: WireCommitScript::Roman as i32,
                roman: "tsu".into(),
                hanji: "珠".into(),
                canonical_tl: "tsu".into(),
                canonical_text: "珠".into(),
            }),
            Intent::CommitPrediction {
                script: Some(CommitScript::Roman),
                roman: "tsu".into(),
                hanji: "珠".into(),
                canonical_tl: "tsu".into(),
                canonical_text: "珠".into(),
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
        &ComposingRequest {
            method: None,
            next_word_context: None,
        },
        &mut engine,
        &config_tl(),
    );
    assert!(result.is_err());
}

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
fn intent_telex_c_f_and_h_are_not_telex_keys() {
    // trace: `resolve` names none of them; a `TelexKey` of any is a no-op,
    // idle or composing. The desktop sends all three as `Append`.
    for key in ["c", "C", "f", "F", "h", "H"] {
        let mut engine = Engine::new();
        let idle = telex(&mut engine, key, &config_tl());
        assert!(!idle.is_composing, "{key}");
        assert!(idle.effect.is_empty(), "{key}");
        append(&mut engine, "ts", &config_tl());
        let composing = telex(&mut engine, key, &config_tl());
        assert!(composing.effect.is_empty(), "{key}");
        assert_eq!(composing.preedit.unwrap().raw_input, "ts", "{key}");
    }
}

// INVARIANT_C_TYPES_AFFRICATE (behavioral-invariants.md §62)
#[test]
fn intent_append_c_types_the_affricate_when_the_setting_is_on() {
    // trace: `affricate::expand` — `c` → ts (TL) / ch (POJ), from Idle and
    // mid-tail; `h` writes itself, so `c` `h` spells the aspirated initial
    // and a third `h` is typed as it is (no slip rule).
    for (mode, initial, aspirated, spelled_out) in [
        ("tl", "ts", "tshia", "tshhia"),
        ("poj", "ch", "chhia", "chhhia"),
    ] {
        let config = protos::engine::AppConfig {
            c_types_affricate: true,
            ..common::config(mode)
        };
        let mut engine = Engine::new();
        append(&mut engine, "c", &config);
        let resp = engine.snapshot(&config);
        assert!(resp.is_composing, "{mode}");
        assert_eq!(resp.preedit.unwrap().raw_input, initial, "{mode}");

        let mut engine = Engine::new();
        append(&mut engine, "chia", &config);
        let resp = engine.snapshot(&config);
        assert_eq!(resp.preedit.unwrap().raw_input, aspirated, "{mode}");

        let mut engine = Engine::new();
        append(&mut engine, "chhia", &config);
        let resp = engine.snapshot(&config);
        assert_eq!(resp.preedit.unwrap().raw_input, spelled_out, "{mode}");
    }
}

#[test]
fn intent_append_c_is_a_plain_letter_when_the_setting_is_off() {
    for mode in ["tl", "poj"] {
        let config = common::config(mode);
        let mut engine = Engine::new();
        append(&mut engine, "chia", &config);
        let resp = engine.snapshot(&config);
        assert_eq!(resp.preedit.unwrap().raw_input, "chia", "{mode}");
    }
}

#[test]
fn intent_telex_z_writes_tone_six_and_is_noop_while_idle() {
    let mut engine = Engine::new();
    let resp = telex(&mut engine, "z", &config_tl());
    assert!(!resp.is_composing);
    assert!(resp.effect.is_empty());
    append(&mut engine, "te", &config_tl());
    let preedit = telex(&mut engine, "z", &config_tl()).preedit.unwrap();
    assert_eq!(preedit.raw_input, "te6");
    assert_eq!(preedit.display_text, "tě");
}

#[test]
fn intent_telex_tone_key_while_idle_is_noop() {
    let mut engine = Engine::new();
    let resp = telex(&mut engine, "v", &config_tl());
    assert!(!resp.is_composing);
    assert!(resp.effect.is_empty());
}

fn config_f_hyphen(mode: &str) -> protos::engine::AppConfig {
    protos::engine::AppConfig {
        f_types_hyphen: true,
        ..common::config(mode)
    }
}

// INVARIANT_F_TYPES_HYPHEN (behavioral-invariants.md §62)
#[test]
fn intent_append_f_types_a_hyphen_when_the_setting_is_on() {
    // trace: `f_hyphen::expand` — `f` → `-` mid-tail, either case.
    for key in ["f", "F"] {
        let config = config_f_hyphen("tl");
        let mut engine = Engine::new();
        append(&mut engine, "tai5", &config);
        append(&mut engine, key, &config);
        let resp = engine.snapshot(&config);
        assert_eq!(resp.preedit.unwrap().raw_input, "tai5-", "{key}");
    }
}

#[test]
fn intent_append_f_is_a_plain_letter_when_the_setting_is_off() {
    for mode in ["tl", "poj"] {
        let config = common::config(mode);
        let mut engine = Engine::new();
        append(&mut engine, "taif", &config);
        let resp = engine.snapshot(&config);
        assert_eq!(resp.preedit.unwrap().raw_input, "taif", "{mode}");
    }
}

// §21 INVARIANT_LEADING_HYPHENS_COMPOSE_IN_BUFFER + §62 INVARIANT_F_TYPES_HYPHEN —
// with `f_types_hyphen` on, an idle `f` writes `-` into the composing buffer,
// just as typing `-` does.
#[test]
fn intent_append_idle_f_enters_composing_with_a_hyphen() {
    for key in ["f", "F"] {
        let config = config_f_hyphen("tl");
        let mut engine = Engine::new();
        let resp = requests::handle(
            &req(Method::Append(Append { char: key.into() })),
            &mut engine,
            &config,
        )
        .unwrap();
        assert!(resp.is_composing, "{key}");
        assert_eq!(commit_text(&resp), None, "{key}");
        let preedit = resp.preedit.unwrap();
        assert_eq!(preedit.raw_input, "-", "{key}");
        assert_eq!(preedit.display_text, "-", "{key}");
    }
}

#[test]
fn intent_append_idle_ff_then_syllable_composes_khinsiann_in_buffer() {
    // trace: each idle/leading `f` appends `-` inside the composing buffer, so
    // `f f a h` produces `--ah` in-buffer with zero premature document commits.
    for mode in ["tl", "poj"] {
        let config = config_f_hyphen(mode);
        let mut engine = Engine::new();
        for expected in ["-", "--"] {
            let resp = requests::handle(
                &req(Method::Append(Append { char: "f".into() })),
                &mut engine,
                &config,
            )
            .unwrap();
            assert!(resp.is_composing, "{mode}");
            assert_eq!(commit_text(&resp), None, "{mode}");
            assert_eq!(resp.preedit.unwrap().raw_input, expected, "{mode}");
        }
        append(&mut engine, "a", &config);
        append(&mut engine, "h", &config);
        let resp = engine.snapshot(&config);
        assert!(resp.is_composing, "{mode}");
        let preedit = resp.preedit.unwrap();
        assert_eq!(preedit.raw_input, "--ah", "{mode}");
        assert_eq!(preedit.display_text, "--ah", "{mode}");
    }
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
    assert_eq!(preedit.display_text, "珠tsâi");
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
fn intent_telex_poj_tone_then_uppercase_f_hyphen() {
    let poj = config_f_hyphen("poj");
    let mut engine = Engine::new();
    append(&mut engine, "Pa", &poj);
    let resp = telex(&mut engine, "Y", &poj);
    assert_eq!(resp.preedit.as_ref().unwrap().display_text, "Pà");
    append(&mut engine, "F", &poj);
    assert_eq!(engine.snapshot(&poj).preedit.unwrap().raw_input, "Pa3-");
}
