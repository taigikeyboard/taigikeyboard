//! INVARIANT_composing_* parity oracles.
//!
//! Sourced from the pre-Rust iOS and Android composing-state test suites
//! (both deleted with the migration). These pin
//! the behavior the platform state machines deliver pre-deletion (commit 9/10).

use composing::{Engine, Intent};
use protos::engine::{effect::Kind as EffectKind, ComposingResponse};

use crate::common::{commit_text, config_tl};

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
            EffectKind::ClearCandidates(_) => "clearCandidates",
            EffectKind::RefreshCandidates(_) => "refreshCandidates",
            EffectKind::ResetCandidateContext(_) => "resetCandidateContext",
            EffectKind::NextWordUpdateLastSelectedWord(_) => "nextWordUpdateLastSelectedWord",
            EffectKind::NextWordWordSelected(_) => "nextWordWordSelected",
            EffectKind::NextWordClearForNewComposing(_) => "nextWordClearForNewComposing",
        })
        .collect()
}

// ---- Initial state ----

#[test]
fn invariant_initial_state_is_idle() {
    let engine = Engine::new();
    let snap = engine.snapshot(&config_tl());
    assert!(!snap.is_composing);
    assert_eq!(snap.preedit.unwrap_or_default().raw_input, "");
}

// ---- Start / Append effect ordering ----

#[test]
fn invariant_start_emits_update_preedit_then_refresh_candidates() {
    let mut engine = Engine::new();
    let resp = engine.apply(Intent::Start { text: "a".into() }, &config_tl());
    assert_eq!(
        effect_kinds(&resp),
        vec!["updatePreedit", "refreshCandidates"]
    );
    assert!(resp.is_composing);
}

#[test]
fn invariant_append_when_idle_behaves_as_start() {
    let mut engine = Engine::new();
    let resp = engine.apply(Intent::Append { ch: "a".into() }, &config_tl());
    assert_eq!(
        effect_kinds(&resp),
        vec!["updatePreedit", "refreshCandidates"]
    );
    assert!(resp.is_composing);
    assert_eq!(raw_input(&resp), "a");
}

#[test]
fn invariant_append_when_composing_appends() {
    let mut engine = Engine::new();
    engine.apply(Intent::Start { text: "a".into() }, &config_tl());
    let resp = engine.apply(Intent::Append { ch: "b".into() }, &config_tl());
    assert_eq!(raw_input(&resp), "ab");
}

#[test]
fn invariant_append_hyphen_appends_literal_dash() {
    let mut engine = Engine::new();
    engine.apply(Intent::Start { text: "a".into() }, &config_tl());
    let resp = engine.apply(Intent::AppendHyphen, &config_tl());
    assert_eq!(raw_input(&resp), "a-");
}

// ---- ReplaceLast ----

#[test]
fn invariant_replace_last_swaps_the_last_char() {
    let mut engine = Engine::new();
    engine.apply(Intent::Start { text: "ab".into() }, &config_tl());
    let resp = engine.apply(
        Intent::ReplaceLast {
            replacement: "c".into(),
        },
        &config_tl(),
    );
    assert_eq!(raw_input(&resp), "ac");
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
fn invariant_delete_backward_to_empty_aborts_without_touching_the_document() {
    // The char only ever lived in the marked region: backspace to empty is
    // the abort trio, never a document delete.
    let mut engine = Engine::new();
    engine.apply(Intent::Start { text: "a".into() }, &config_tl());
    let resp = engine.apply(Intent::DeleteBackward, &config_tl());
    assert!(!resp.is_composing);
    assert_eq!(
        effect_kinds(&resp),
        vec![
            "clearPreeditWithoutCommit",
            "clearCandidates",
            "nextWordClearForNewComposing"
        ]
    );
}

#[test]
fn invariant_delete_backward_partial_keeps_composing() {
    let mut engine = Engine::new();
    engine.apply(Intent::Start { text: "ab".into() }, &config_tl());
    let resp = engine.apply(Intent::DeleteBackward, &config_tl());
    assert_eq!(raw_input(&resp), "a");
    assert!(resp.is_composing);
}

#[test]
fn invariant_delete_backward_idle_is_noop() {
    let mut engine = Engine::new();
    let resp = engine.apply(Intent::DeleteBackward, &config_tl());
    assert!(resp.effect.is_empty());
}

// ---- CommitRaw ----

#[test]
fn invariant_commit_raw_idle_is_noop() {
    let mut engine = Engine::new();
    let resp = engine.apply(Intent::CommitRaw, &config_tl());
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
    assert!(!resp.is_composing, "an idle insert stays idle");
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

// INVARIANT_COMPOSING_IDLE_TO_IDLE_IS_NOOP (behavioral-invariants.md §13)
#[test]
fn invariant_reset_idle_is_noop() {
    let mut engine = Engine::new();
    let resp = engine.apply(Intent::Reset, &config_tl());
    assert!(resp.effect.is_empty());
}

// ---- Snapshot ----

#[test]
fn invariant_snapshot_does_not_mutate() {
    let mut engine = Engine::new();
    engine.apply(Intent::Start { text: "abc".into() }, &config_tl());
    let snap_a = engine.snapshot(&config_tl());
    let snap_b = engine.snapshot(&config_tl());
    assert!(snap_a.effect.is_empty(), "a snapshot emits no effects");
    assert_eq!(snap_a.preedit.unwrap().raw_input, "abc");
    assert_eq!(snap_b.preedit.unwrap().raw_input, "abc");
}

// ---- Phase invariants ----

#[test]
fn invariant_idle_implies_empty_raw() {
    let engine = Engine::new();
    let snap = engine.snapshot(&config_tl());
    assert!(!snap.is_composing);
    assert_eq!(snap.preedit.unwrap().raw_input, "");
}

#[test]
fn invariant_start_snapshot_is_composing() {
    let mut engine = Engine::new();
    engine.apply(Intent::Start { text: "a".into() }, &config_tl());
    let snap = engine.snapshot(&config_tl());
    assert!(snap.is_composing);
}

#[test]
fn invariant_is_composing_matches_phase_after_every_response() {
    let mut engine = Engine::new();
    let r1 = engine.apply(Intent::Start { text: "a".into() }, &config_tl());
    assert!(r1.is_composing);
    let r2 = engine.apply(Intent::CommitRaw, &config_tl());
    assert!(!r2.is_composing);
    let r3 = engine.apply(Intent::Append { ch: "b".into() }, &config_tl());
    assert!(r3.is_composing);
    let r4 = engine.apply(Intent::Reset, &config_tl());
    assert!(!r4.is_composing);
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
    engine.apply(Intent::CommitRaw, &config_tl());
    let resp = engine.apply(Intent::Start { text: "b".into() }, &config_tl());
    assert_eq!(raw_input(&resp), "b");
}
