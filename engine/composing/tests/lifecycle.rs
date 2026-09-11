//! Lifecycle tests — generation-mismatch silent drop + idempotent reset.
//! Per plan §4 + §5b.2.

use composing::EngineHandle;
use protos::engine::composing_request::Method;
use protos::engine::{
    Append, ComposingRequest, EnterContinuous, FetchAtPos, QueryState, Reset, Start,
};

mod common;
use common::{config_tl, req};

fn req_start(text: &str) -> ComposingRequest {
    ComposingRequest {
        method: Some(Method::Start(Start {
            text: text.to_string(),
        })),
    }
}

#[test]
fn lifecycle_intent_reset_when_idle_emits_no_effects() {
    let handle = EngineHandle::new();
    let resp = handle
        .handle(
            &ComposingRequest {
                method: Some(Method::Reset(Reset {})),
            },
            &config_tl(),
            1,
        )
        .unwrap();
    assert!(resp.effect.is_empty());
}

#[test]
fn lifecycle_intent_reset_when_composing_emits_clear_and_reset_autocomplete() {
    let handle = EngineHandle::new();
    handle.handle(&req_start("a"), &config_tl(), 1).unwrap();
    let resp = handle
        .handle(
            &ComposingRequest {
                method: Some(Method::Reset(Reset {})),
            },
            &config_tl(),
            1,
        )
        .unwrap();
    // Two effects per plan §5b.2: ClearPreeditWithoutCommit + ResetAutocomplete.
    assert_eq!(resp.effect.len(), 2);
}

#[test]
fn lifecycle_generation_increment_silently_drops_state() {
    let handle = EngineHandle::new();
    handle.handle(&req_start("abc"), &config_tl(), 1).unwrap();
    // Same generation → state preserved.
    let resp_same = handle.handle(&req_start("xyz"), &config_tl(), 1).unwrap();
    // The Start intent always sets buffer to "xyz" — but also: state was carried.
    // Generation mismatch → silently drops state; the new request executes against
    // a fresh engine.
    let resp_after_bump = handle.handle(&req_start("fresh"), &config_tl(), 2).unwrap();
    assert_eq!(resp_after_bump.preedit.unwrap().raw_input, "fresh");
    // The drop itself emits NO effects; the Start request emits its normal effects.
    assert_eq!(resp_after_bump.effect.len(), 2);
    let _ = resp_same; // suppress unused
}

#[test]
fn lifecycle_generation_mismatch_resets_phase_to_idle() {
    let handle = EngineHandle::new();
    handle.handle(&req_start("abc"), &config_tl(), 1).unwrap();
    // Send a Reset intent with a NEW generation. The mismatch silently resets state to Idle FIRST,
    // then the Reset intent runs against an Idle engine — which is a noop (no effects).
    let resp = handle
        .handle(
            &ComposingRequest {
                method: Some(Method::Reset(Reset {})),
            },
            &config_tl(),
            2,
        )
        .unwrap();
    assert!(resp.effect.is_empty());
    assert!(!resp.is_composing);
}

#[test]
fn lifecycle_engine_reset_is_idempotent() {
    let handle = EngineHandle::new();
    handle.handle(&req_start("a"), &config_tl(), 1).unwrap();
    // Two consecutive Reset intents at the same generation.
    let r1 = handle
        .handle(
            &ComposingRequest {
                method: Some(Method::Reset(Reset {})),
            },
            &config_tl(),
            1,
        )
        .unwrap();
    let r2 = handle
        .handle(
            &ComposingRequest {
                method: Some(Method::Reset(Reset {})),
            },
            &config_tl(),
            1,
        )
        .unwrap();
    // First Reset emits effects (was composing). Second Reset is idle → noop.
    assert_eq!(r1.effect.len(), 2);
    assert!(r2.effect.is_empty());
}

// --- Read-only intents never reset on a generation mismatch ----------------

fn req_fetch_at_pos() -> ComposingRequest {
    req(Method::FetchAtPos(FetchAtPos::default()))
}

fn start_continuous(handle: &EngineHandle, text: &str) {
    handle.handle(&req_start(text), &config_tl(), 1).unwrap();
    handle
        .handle(
            &req(Method::EnterContinuous(EnterContinuous {})),
            &config_tl(),
            1,
        )
        .unwrap();
}

#[test]
fn lifecycle_fetch_at_pos_with_matching_generation_reads_continuous_state() {
    let handle = EngineHandle::new();
    start_continuous(&handle, "ta");
    // trace: Continuous { raw: "ta" } → handle clones the engine and answers
    // from the clone: composing, carrier present (no lexicon installed in this
    // process → empty candidate list), no effects.
    let resp = handle.handle(&req_fetch_at_pos(), &config_tl(), 1).unwrap();
    assert!(resp.is_composing);
    assert!(resp.continuous.is_some());
    assert!(resp.effect.is_empty());
    assert_eq!(resp.preedit.unwrap().raw_input, "ta");
}

#[test]
fn lifecycle_fetch_at_pos_with_mismatched_generation_leaves_state_intact() {
    let handle = EngineHandle::new();
    start_continuous(&handle, "ta");
    // A stale worker-thread fetch carrying an old generation answers Idle-shaped …
    let stale = handle.handle(&req_fetch_at_pos(), &config_tl(), 2).unwrap();
    assert!(!stale.is_composing);
    assert!(stale.continuous.is_none());
    assert!(stale.effect.is_empty());
    // … and neither resets the engine nor records generation 2: the current
    // context (generation 1) keeps composing on the same buffer.
    let resp = handle
        .handle(
            &req(Method::Append(Append { char: "i".into() })),
            &config_tl(),
            1,
        )
        .unwrap();
    assert!(resp.is_composing);
    assert_eq!(resp.preedit.unwrap().raw_input, "tai");
}

#[test]
fn lifecycle_query_state_with_mismatched_generation_leaves_state_intact() {
    let handle = EngineHandle::new();
    handle.handle(&req_start("ta"), &config_tl(), 1).unwrap();
    let stale = handle
        .handle(&req(Method::QueryState(QueryState {})), &config_tl(), 2)
        .unwrap();
    assert!(!stale.is_composing);
    let resp = handle
        .handle(&req(Method::QueryState(QueryState {})), &config_tl(), 1)
        .unwrap();
    assert_eq!(resp.preedit.unwrap().raw_input, "ta");
}

#[test]
fn lifecycle_mutating_intent_after_stale_fetch_still_resets_on_new_generation() {
    let handle = EngineHandle::new();
    handle.handle(&req_start("ta"), &config_tl(), 1).unwrap();
    handle.handle(&req_fetch_at_pos(), &config_tl(), 2).unwrap();
    // The first MUTATING request at generation 2 performs the silent reset, so
    // Start runs against an Idle engine exactly as it does without the fetch.
    let resp = handle.handle(&req_start("x"), &config_tl(), 2).unwrap();
    assert_eq!(resp.preedit.unwrap().raw_input, "x");
    assert_eq!(resp.effect.len(), 2);
}

/// Concurrent regression for the read-only path: a worker thread keeps
/// fetching at generation 1 while the main thread flips the engine between
/// generation 1 (buffer "a") and generation 2 (buffer "b"). A generation-1
/// fetch may answer Idle or "a", but must never leak generation 2's "b" —
/// that would mean the generation check and the engine clone were not
/// taken under the same lock.
#[test]
fn lifecycle_read_only_fetch_never_observes_a_newer_generation() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let handle = Arc::new(EngineHandle::new());
    let stop = Arc::new(AtomicBool::new(false));

    let reader = {
        let handle = Arc::clone(&handle);
        let stop = Arc::clone(&stop);
        std::thread::spawn(move || {
            let mut leaked = 0usize;
            while !stop.load(Ordering::Relaxed) {
                let resp = handle
                    .handle(&req(Method::QueryState(QueryState {})), &config_tl(), 1)
                    .unwrap();
                if resp.preedit.map(|p| p.raw_input) == Some("b".to_string()) {
                    leaked += 1;
                }
            }
            leaked
        })
    };

    for _ in 0..2_000 {
        handle.handle(&req_start("a"), &config_tl(), 1).unwrap();
        handle.handle(&req_start("b"), &config_tl(), 2).unwrap();
    }
    stop.store(true, Ordering::Relaxed);
    assert_eq!(
        reader.join().unwrap(),
        0,
        "generation-1 reader observed generation-2 state"
    );
}
