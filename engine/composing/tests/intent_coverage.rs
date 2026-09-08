//! One test per intent — round-trip through `dispatch::handle` to exercise
//! the proto decode path alongside the transition table.

use composing::{dispatch, Engine};
use protos::engine::composing_request::Method;
use protos::engine::{
    Append, AppendHyphen, CommitDerived, CommitPreeditThenInsertExternal, CommitRaw,
    ComposingRequest, DeleteBackward, QueryState, ReplaceLast, Reset, SelectSuggestion,
    SetSelectedCandidateIndex, Start,
};

mod common;
use common::{config_tl, req};

fn commit_text(resp: &protos::engine::ComposingResponse) -> Option<String> {
    resp.effect.iter().find_map(|e| match e.kind.as_ref()? {
        protos::engine::effect::Kind::CommitTextReplacingPreedit(c) => Some(c.text.clone()),
        _ => None,
    })
}

// §21 INVARIANT_KHINSIANN_LEADING_MARKER_LITERAL — a leading `--` typed from
// Idle is a document literal, NOT composing input (MOE-style). Production sends
// one char at a time, so the first `-` arrives as Start{"-"}.
#[test]
fn intent_start_leading_hyphen_inserts_literal_stays_idle() {
    let mut engine = Engine::new();
    let resp = dispatch::handle(
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
    let resp = dispatch::handle(
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
    let resp = dispatch::handle(
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

#[test]
fn intent_start() {
    let mut engine = Engine::new();
    let resp = dispatch::handle(
        &req(Method::Start(Start { text: "a".into() })),
        &mut engine,
        &config_tl(),
    )
    .expect("start dispatches");
    assert!(resp.is_composing);
    assert_eq!(resp.preedit.as_ref().unwrap().raw_input, "a");
    assert_eq!(resp.selected_candidate_index, 0);
}

#[test]
fn intent_append_idle_acts_as_start() {
    let mut engine = Engine::new();
    let resp = dispatch::handle(
        &req(Method::Append(Append { char: "k".into() })),
        &mut engine,
        &config_tl(),
    )
    .unwrap();
    assert_eq!(resp.preedit.unwrap().raw_input, "k");
    assert!(resp.is_composing);
}

#[test]
fn intent_append_hyphen_appends_dash() {
    let mut engine = Engine::new();
    dispatch::handle(
        &req(Method::Start(Start { text: "a".into() })),
        &mut engine,
        &config_tl(),
    )
    .unwrap();
    let resp = dispatch::handle(
        &req(Method::AppendHyphen(AppendHyphen {})),
        &mut engine,
        &config_tl(),
    )
    .unwrap();
    assert_eq!(resp.preedit.unwrap().raw_input, "a-");
}

#[test]
fn intent_replace_last_swaps_final_char() {
    let mut engine = Engine::new();
    dispatch::handle(
        &req(Method::Start(Start { text: "ab".into() })),
        &mut engine,
        &config_tl(),
    )
    .unwrap();
    let resp = dispatch::handle(
        &req(Method::ReplaceLast(ReplaceLast {
            replacement: "c".into(),
        })),
        &mut engine,
        &config_tl(),
    )
    .unwrap();
    assert_eq!(resp.preedit.unwrap().raw_input, "ac");
}

#[test]
fn intent_delete_backward_shortens_buffer() {
    let mut engine = Engine::new();
    dispatch::handle(
        &req(Method::Start(Start { text: "abc".into() })),
        &mut engine,
        &config_tl(),
    )
    .unwrap();
    let resp = dispatch::handle(
        &req(Method::DeleteBackward(DeleteBackward {})),
        &mut engine,
        &config_tl(),
    )
    .unwrap();
    assert_eq!(resp.preedit.unwrap().raw_input, "ab");
}

#[test]
fn intent_commit_derived_returns_to_idle() {
    let mut engine = Engine::new();
    dispatch::handle(
        &req(Method::Start(Start { text: "a".into() })),
        &mut engine,
        &config_tl(),
    )
    .unwrap();
    let resp = dispatch::handle(
        &req(Method::CommitDerived(CommitDerived {})),
        &mut engine,
        &config_tl(),
    )
    .unwrap();
    assert!(!resp.is_composing);
    assert_eq!(resp.selected_candidate_index, -1);
}

#[test]
fn intent_commit_raw_uses_literal_input() {
    let mut engine = Engine::new();
    dispatch::handle(
        &req(Method::Start(Start {
            text: "guá".into()
        })),
        &mut engine,
        &config_tl(),
    )
    .unwrap();
    let resp = dispatch::handle(
        &req(Method::CommitRaw(CommitRaw {})),
        &mut engine,
        &config_tl(),
    )
    .unwrap();
    assert!(!resp.is_composing);
}

#[test]
fn intent_select_suggestion_commits_supplied_text() {
    let mut engine = Engine::new();
    dispatch::handle(
        &req(Method::Start(Start { text: "a".into() })),
        &mut engine,
        &config_tl(),
    )
    .unwrap();
    let resp = dispatch::handle(
        &req(Method::SelectSuggestion(SelectSuggestion {
            text: "好".into(),
        })),
        &mut engine,
        &config_tl(),
    )
    .unwrap();
    assert!(!resp.is_composing);
}

#[test]
fn intent_commit_preedit_then_insert_external_in_composing() {
    let mut engine = Engine::new();
    dispatch::handle(
        &req(Method::Start(Start { text: "a".into() })),
        &mut engine,
        &config_tl(),
    )
    .unwrap();
    let resp = dispatch::handle(
        &req(Method::CommitPreeditThenInsertExternal(
            CommitPreeditThenInsertExternal {
                text: "🎉".into()
            },
        )),
        &mut engine,
        &config_tl(),
    )
    .unwrap();
    assert!(!resp.is_composing);
}

#[test]
fn intent_commit_preedit_then_insert_external_in_idle() {
    let mut engine = Engine::new();
    let resp = dispatch::handle(
        &req(Method::CommitPreeditThenInsertExternal(
            CommitPreeditThenInsertExternal {
                text: "🎉".into()
            },
        )),
        &mut engine,
        &config_tl(),
    )
    .unwrap();
    assert!(!resp.is_composing);
}

#[test]
fn intent_reset_returns_to_idle() {
    let mut engine = Engine::new();
    dispatch::handle(
        &req(Method::Start(Start { text: "a".into() })),
        &mut engine,
        &config_tl(),
    )
    .unwrap();
    let resp = dispatch::handle(&req(Method::Reset(Reset {})), &mut engine, &config_tl()).unwrap();
    assert!(!resp.is_composing);
    assert_eq!(resp.selected_candidate_index, -1);
}

#[test]
fn intent_set_selected_candidate_index_mutates_only_index() {
    let mut engine = Engine::new();
    dispatch::handle(
        &req(Method::Start(Start { text: "a".into() })),
        &mut engine,
        &config_tl(),
    )
    .unwrap();
    let resp = dispatch::handle(
        &req(Method::SetSelectedCandidateIndex(
            SetSelectedCandidateIndex { index: 3 },
        )),
        &mut engine,
        &config_tl(),
    )
    .unwrap();
    assert_eq!(resp.selected_candidate_index, 3);
    assert!(resp.effect.is_empty());
}

#[test]
fn intent_query_state_emits_no_effects() {
    let mut engine = Engine::new();
    dispatch::handle(
        &req(Method::Start(Start { text: "abc".into() })),
        &mut engine,
        &config_tl(),
    )
    .unwrap();
    let resp = dispatch::handle(
        &req(Method::QueryState(QueryState {})),
        &mut engine,
        &config_tl(),
    )
    .unwrap();
    assert!(resp.is_composing);
    assert_eq!(resp.preedit.unwrap().raw_input, "abc");
    assert!(resp.effect.is_empty());
}

#[test]
fn intent_missing_method_returns_error() {
    let mut engine = Engine::new();
    let result = dispatch::handle(
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
    dispatch::handle(
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
        dispatch::handle(
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
    assert_eq!(resp.selected_candidate_index, 0);
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
    let resp = dispatch::handle(
        &req(Method::QueryState(QueryState {})),
        &mut engine,
        &config_tl(),
    )
    .unwrap();
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
    dispatch::handle(
        &req(Method::EnterContinuous(protos::engine::EnterContinuous {})),
        &mut engine,
        &config_tl(),
    )
    .unwrap();
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
            display_text: display_text.to_string(),
            canonical_text: String::new(),
            association_tl: String::new(),
            consumed_bytes,
            syllable_count: 1,
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
    dispatch::handle(
        &req(Method::EnterContinuous(protos::engine::EnterContinuous {})),
        &mut engine,
        &config_tl(),
    )
    .unwrap();
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
fn intent_telex_edit_resets_selection_noop_keeps_it() {
    let mut engine = Engine::new();
    append(&mut engine, "te", &config_tl());
    let select = |engine: &mut Engine| {
        dispatch::handle(
            &req(Method::SetSelectedCandidateIndex(
                SetSelectedCandidateIndex { index: 2 },
            )),
            engine,
            &config_tl(),
        )
        .unwrap()
    };
    select(&mut engine);
    let resp = telex(&mut engine, "v", &config_tl());
    assert_eq!(resp.selected_candidate_index, 0);
    select(&mut engine);
    let resp = telex(&mut engine, "v", &config_tl());
    assert_eq!(
        resp.selected_candidate_index, 2,
        "no-op keeps the selection"
    );
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
