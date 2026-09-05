//! Lifecycle tests — generation-mismatch silent drop + idempotent reset.
//! Per plan §4 + §5b.2.

// 生命週期測試:generation 不一致的靜默重設與冪等 reset 行為。

use composing::EngineHandle;
use protos::engine::composing_request::Method;
use protos::engine::{AppConfig, ComposingRequest, Reset, Start};

fn config_tl() -> AppConfig {
    AppConfig {
        tone_mode: String::new(),
        input_mode: "tl".to_string(),
        oo_doubletap_enabled: false,
        nn_doubletap_enabled: false,
        is_translate_swapped: false,
        is_association_recording_enabled: false,
        platform_id: 0,
        output_both_scripts: false,
        candidate_display_mode: 0,
    }
}

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
