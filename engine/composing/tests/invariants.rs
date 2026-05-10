//! INVARIANT_composing_* parity oracles.
//!
//! Sourced from iOS `ios/TaigiKeyboardTests/ComposingStateTests.swift` and
//! Android `android/.../ime/text/composing/ComposingStateTest.kt`. These pin
//! the behavior the platform state machines deliver pre-deletion (commit 9/10).

// 中文: 對齊 iOS / Android 平台原本狀態機行為的不變式測試 (parity oracle)。

use composing::{Engine, Intent};
use protos::engine::{effect::Kind as EffectKind, AppConfig, ComposingResponse};

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

fn raw_input(resp: &ComposingResponse) -> &str {
    resp.preedit
        .as_ref()
        .map(|p| p.raw_input.as_str())
        .unwrap_or("")
}

fn effect_kinds(resp: &ComposingResponse) -> Vec<&'static str> {
    resp.effect
        .iter()
        .filter_map(|e| e.kind.as_ref())
        .map(|k| match k {
            EffectKind::UpdatePreedit(_) => "updatePreedit",
            EffectKind::ClearPreeditWithoutCommit(_) => "clearPreeditWithoutCommit",
            EffectKind::CommitTextReplacingPreedit(_) => "commitTextReplacingPreedit",
            EffectKind::DeleteBackwardFromDocument(_) => "deleteBackwardFromDocument",
            EffectKind::ResetAutocomplete(_) => "resetAutocomplete",
            EffectKind::PerformAutocomplete(_) => "performAutocomplete",
            EffectKind::ResetAutocompleteContext(_) => "resetAutocompleteContext",
            EffectKind::NextWordUpdateLastSelectedWord(_) => "nextWordUpdateLastSelectedWord",
            EffectKind::NextWordWordSelected(_) => "nextWordWordSelected",
            EffectKind::NextWordClearForNewComposing(_) => "nextWordClearForNewComposing",
        })
        .collect()
}

fn commit_text(resp: &ComposingResponse) -> Option<String> {
    resp.effect.iter().find_map(|e| match e.kind.as_ref() {
        Some(EffectKind::CommitTextReplacingPreedit(c)) => Some(c.text.clone()),
        _ => None,
    })
}

// ---- Initial state ----

#[test]
fn invariant_initial_state_is_idle_with_index_minus_one() {
    let engine = Engine::new();
    let snap = engine.snapshot(&config_tl());
    assert!(!snap.is_composing);
    assert_eq!(snap.selected_candidate_index, -1);
    assert_eq!(snap.preedit.unwrap_or_default().raw_input, "");
}

// ---- Start / Append effect ordering ----

#[test]
fn invariant_start_emits_update_preedit_then_perform_autocomplete() {
    let mut engine = Engine::new();
    let resp = engine.apply(Intent::Start { text: "a".into() }, &config_tl());
    assert_eq!(
        effect_kinds(&resp),
        vec!["updatePreedit", "performAutocomplete"]
    );
    assert_eq!(resp.selected_candidate_index, 0);
    assert!(resp.is_composing);
}

#[test]
fn invariant_append_when_idle_behaves_as_start() {
    let mut engine = Engine::new();
    let resp = engine.apply(Intent::Append { ch: "a".into() }, &config_tl());
    assert_eq!(
        effect_kinds(&resp),
        vec!["updatePreedit", "performAutocomplete"]
    );
    assert_eq!(resp.selected_candidate_index, 0);
}

#[test]
fn invariant_append_when_composing_appends_and_resets_index() {
    let mut engine = Engine::new();
    engine.apply(Intent::Start { text: "a".into() }, &config_tl());
    engine.apply(Intent::SetSelectedCandidateIndex { index: 5 }, &config_tl());
    let resp = engine.apply(Intent::Append { ch: "b".into() }, &config_tl());
    assert_eq!(raw_input(&resp), "ab");
    assert_eq!(resp.selected_candidate_index, 0);
}

#[test]
fn invariant_append_hyphen_appends_literal_dash() {
    let mut engine = Engine::new();
    engine.apply(Intent::Start { text: "a".into() }, &config_tl());
    let resp = engine.apply(Intent::AppendHyphen, &config_tl());
    assert_eq!(raw_input(&resp), "a-");
}

// ---- ReplaceLast preserves selected_candidate_index ----

#[test]
fn invariant_replace_last_preserves_selected_index() {
    let mut engine = Engine::new();
    engine.apply(Intent::Start { text: "ab".into() }, &config_tl());
    engine.apply(Intent::SetSelectedCandidateIndex { index: 3 }, &config_tl());
    let resp = engine.apply(
        Intent::ReplaceLast {
            replacement: "c".into(),
        },
        &config_tl(),
    );
    assert_eq!(raw_input(&resp), "ac");
    assert_eq!(resp.selected_candidate_index, 3);
}

#[test]
fn invariant_replace_last_idle_is_noop() {
    let mut engine = Engine::new();
    let resp = engine.apply(
        Intent::ReplaceLast {
            replacement: "c".into(),
        },
        &config_tl(),
    );
    assert!(!resp.is_composing);
    assert!(resp.effect.is_empty());
}

#[test]
fn invariant_replace_last_empty_buffer_is_noop() {
    let mut engine = Engine::new();
    engine.apply(Intent::Start { text: "".into() }, &config_tl());
    engine.apply(Intent::Reset, &config_tl());
    let resp = engine.apply(
        Intent::ReplaceLast {
            replacement: "c".into(),
        },
        &config_tl(),
    );
    assert!(resp.effect.is_empty());
}

// ---- DeleteBackward ----

#[test]
fn invariant_delete_backward_to_empty_emits_clear_reset_delete_doc() {
    let mut engine = Engine::new();
    engine.apply(Intent::Start { text: "a".into() }, &config_tl());
    let resp = engine.apply(Intent::DeleteBackward, &config_tl());
    assert!(!resp.is_composing);
    assert_eq!(
        effect_kinds(&resp),
        vec![
            "clearPreeditWithoutCommit",
            "resetAutocomplete",
            "deleteBackwardFromDocument"
        ]
    );
}

#[test]
fn invariant_delete_backward_partial_resets_index_to_zero() {
    let mut engine = Engine::new();
    engine.apply(Intent::Start { text: "ab".into() }, &config_tl());
    engine.apply(Intent::SetSelectedCandidateIndex { index: 7 }, &config_tl());
    let resp = engine.apply(Intent::DeleteBackward, &config_tl());
    assert_eq!(raw_input(&resp), "a");
    assert_eq!(resp.selected_candidate_index, 0);
}

#[test]
fn invariant_delete_backward_idle_is_noop() {
    let mut engine = Engine::new();
    let resp = engine.apply(Intent::DeleteBackward, &config_tl());
    assert!(resp.effect.is_empty());
}

// ---- CommitDerived ----

#[test]
fn invariant_commit_derived_emits_commit_then_reset_pair() {
    let mut engine = Engine::new();
    engine.apply(Intent::Start { text: "a".into() }, &config_tl());
    let resp = engine.apply(Intent::CommitDerived, &config_tl());
    assert_eq!(
        effect_kinds(&resp),
        vec![
            "commitTextReplacingPreedit",
            "resetAutocomplete",
            "resetAutocompleteContext"
        ]
    );
    assert!(!resp.is_composing);
    assert_eq!(resp.selected_candidate_index, -1);
}

#[test]
fn invariant_commit_derived_idle_is_noop() {
    let mut engine = Engine::new();
    let resp = engine.apply(Intent::CommitDerived, &config_tl());
    assert!(resp.effect.is_empty());
}

// ---- CommitRaw ----

#[test]
fn invariant_commit_raw_commits_literal_buffer() {
    let mut engine = Engine::new();
    engine.apply(
        Intent::Start {
            text: "gua2".into(),
        },
        &config_tl(),
    );
    let resp = engine.apply(Intent::CommitRaw, &config_tl());
    assert_eq!(commit_text(&resp).as_deref(), Some("gua2"));
    assert!(!resp.is_composing);
}

#[test]
fn invariant_commit_raw_idle_is_noop() {
    let mut engine = Engine::new();
    let resp = engine.apply(Intent::CommitRaw, &config_tl());
    assert!(resp.effect.is_empty());
}

// ---- SelectSuggestion ----

#[test]
fn invariant_select_suggestion_commits_supplied_text() {
    let mut engine = Engine::new();
    engine.apply(Intent::Start { text: "a".into() }, &config_tl());
    let resp = engine.apply(Intent::SelectSuggestion { text: "好".into() }, &config_tl());
    assert_eq!(commit_text(&resp).as_deref(), Some("好"));
    assert!(!resp.is_composing);
}

#[test]
fn invariant_select_suggestion_idle_is_noop() {
    let mut engine = Engine::new();
    let resp = engine.apply(Intent::SelectSuggestion { text: "好".into() }, &config_tl());
    assert!(resp.effect.is_empty());
}

// ---- CommitPreeditThenInsertExternal ----

#[test]
fn invariant_commit_preedit_then_insert_in_composing_concatenates() {
    let mut engine = Engine::new();
    engine.apply(Intent::Start { text: "ka".into() }, &config_tl());
    let resp = engine.apply(
        Intent::CommitPreeditThenInsertExternal {
            text: "🎉".into()
        },
        &config_tl(),
    );
    let committed = commit_text(&resp).expect("commit text present");
    assert!(committed.ends_with("🎉"));
    assert!(!resp.is_composing);
}

#[test]
fn invariant_commit_preedit_then_insert_in_idle_inserts_only_external() {
    let mut engine = Engine::new();
    let resp = engine.apply(
        Intent::CommitPreeditThenInsertExternal {
            text: "🎉".into()
        },
        &config_tl(),
    );
    assert_eq!(commit_text(&resp).as_deref(), Some("🎉"));
}

#[test]
fn invariant_commit_preedit_then_insert_empty_external_is_noop() {
    let mut engine = Engine::new();
    engine.apply(Intent::Start { text: "a".into() }, &config_tl());
    let resp = engine.apply(
        Intent::CommitPreeditThenInsertExternal { text: "".into() },
        &config_tl(),
    );
    assert!(resp.effect.is_empty());
}

// ---- Reset ----

#[test]
fn invariant_reset_idle_is_noop() {
    let mut engine = Engine::new();
    let resp = engine.apply(Intent::Reset, &config_tl());
    assert!(resp.effect.is_empty());
}

#[test]
fn invariant_reset_composing_emits_clear_and_reset_autocomplete_only() {
    let mut engine = Engine::new();
    engine.apply(Intent::Start { text: "a".into() }, &config_tl());
    let resp = engine.apply(Intent::Reset, &config_tl());
    assert_eq!(
        effect_kinds(&resp),
        vec!["clearPreeditWithoutCommit", "resetAutocomplete"]
    );
    assert!(!resp.is_composing);
    assert_eq!(resp.selected_candidate_index, -1);
}

// ---- SetSelectedCandidateIndex ----

#[test]
fn invariant_set_selected_candidate_index_emits_no_effects() {
    let mut engine = Engine::new();
    engine.apply(Intent::Start { text: "a".into() }, &config_tl());
    let resp = engine.apply(Intent::SetSelectedCandidateIndex { index: 4 }, &config_tl());
    assert!(resp.effect.is_empty());
    assert_eq!(resp.selected_candidate_index, 4);
}

// ---- QueryState ----

#[test]
fn invariant_query_state_does_not_mutate() {
    let mut engine = Engine::new();
    engine.apply(Intent::Start { text: "abc".into() }, &config_tl());
    let snap_a = engine.snapshot(&config_tl());
    let snap_b = engine.snapshot(&config_tl());
    assert_eq!(snap_a.preedit.unwrap().raw_input, "abc");
    assert_eq!(snap_b.preedit.unwrap().raw_input, "abc");
}

// ---- Phase invariants ----

#[test]
fn invariant_idle_implies_empty_raw_and_minus_one_index() {
    let engine = Engine::new();
    let snap = engine.snapshot(&config_tl());
    assert!(!snap.is_composing);
    assert_eq!(snap.selected_candidate_index, -1);
    assert_eq!(snap.preedit.unwrap().raw_input, "");
}

#[test]
fn invariant_composing_implies_non_negative_index() {
    let mut engine = Engine::new();
    engine.apply(Intent::Start { text: "a".into() }, &config_tl());
    let snap = engine.snapshot(&config_tl());
    assert!(snap.is_composing);
    assert!(snap.selected_candidate_index >= 0);
}

#[test]
fn invariant_is_composing_matches_phase_after_every_response() {
    let mut engine = Engine::new();
    let r1 = engine.apply(Intent::Start { text: "a".into() }, &config_tl());
    assert!(r1.is_composing);
    let r2 = engine.apply(Intent::CommitDerived, &config_tl());
    assert!(!r2.is_composing);
    let r3 = engine.apply(Intent::Append { ch: "b".into() }, &config_tl());
    assert!(r3.is_composing);
    let r4 = engine.apply(Intent::Reset, &config_tl());
    assert!(!r4.is_composing);
}

#[test]
fn invariant_external_index_resets_on_append_and_delete_backward() {
    let mut engine = Engine::new();
    engine.apply(Intent::Start { text: "ab".into() }, &config_tl());
    engine.apply(Intent::SetSelectedCandidateIndex { index: 9 }, &config_tl());
    let r_app = engine.apply(Intent::Append { ch: "c".into() }, &config_tl());
    assert_eq!(r_app.selected_candidate_index, 0);
    engine.apply(Intent::SetSelectedCandidateIndex { index: 9 }, &config_tl());
    let r_del = engine.apply(Intent::DeleteBackward, &config_tl());
    assert_eq!(r_del.selected_candidate_index, 0);
}

#[test]
fn invariant_replace_last_does_not_reset_external_index() {
    let mut engine = Engine::new();
    engine.apply(Intent::Start { text: "ab".into() }, &config_tl());
    engine.apply(Intent::SetSelectedCandidateIndex { index: 9 }, &config_tl());
    let resp = engine.apply(
        Intent::ReplaceLast {
            replacement: "z".into(),
        },
        &config_tl(),
    );
    assert_eq!(resp.selected_candidate_index, 9);
}

// ---- Multi-step sequences ----

#[test]
fn invariant_start_append_append_buffer_grows() {
    let mut engine = Engine::new();
    engine.apply(Intent::Start { text: "a".into() }, &config_tl());
    engine.apply(Intent::Append { ch: "b".into() }, &config_tl());
    let resp = engine.apply(Intent::Append { ch: "c".into() }, &config_tl());
    assert_eq!(raw_input(&resp), "abc");
}

#[test]
fn invariant_commit_then_compose_again_works() {
    let mut engine = Engine::new();
    engine.apply(Intent::Start { text: "a".into() }, &config_tl());
    engine.apply(Intent::CommitDerived, &config_tl());
    let resp = engine.apply(Intent::Start { text: "b".into() }, &config_tl());
    assert_eq!(raw_input(&resp), "b");
    assert_eq!(resp.selected_candidate_index, 0);
}
