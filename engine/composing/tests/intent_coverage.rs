//! One test per intent — round-trip through `dispatch::handle` to exercise
//! the proto decode path alongside the transition table.

// 中文: 每個 Intent 一條測試,透過 dispatch::handle 同時驗證 proto 解碼與狀態轉移。

use composing::{dispatch, Engine};
use protos::engine::composing_request::Method;
use protos::engine::{
    AppConfig, Append, AppendHyphen, CommitDerived, CommitPreeditThenInsertExternal, CommitRaw,
    ComposingRequest, DeleteBackward, QueryState, ReplaceLast, Reset, SelectSuggestion,
    SetSelectedCandidateIndex, Start,
};

fn config_tl() -> AppConfig {
    AppConfig {
        tone_mode: String::new(),
        input_mode: "tl".to_string(),
        oo_doubletap_enabled: false,
        nn_doubletap_enabled: false,
        is_translate_swapped: false,
        is_association_recording_enabled: false,
        platform_id: 0,
    }
}

fn req(method: Method) -> ComposingRequest {
    ComposingRequest {
        method: Some(method),
    }
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
