//! v3.5.8 Phase 9 Item 2 — pin the §10.2 / §10.3 invariant, **Model B**:
//!
//!   For every Continuous-phase mutation that surfaces a preedit response,
//!   - `response.preedit.display_text == phase.composing_display(config)`
//!     (the whole composition: Σ nailed display + derived pending tail) —
//!     the universally-true Model B invariant a hard finalize / Enter
//!     commits, AND
//!   - `response.preedit.raw_input == <raw pending-tail bytes>` — i.e. the
//!     `Phase::Continuous.raw` field verbatim. This is deliberately **NOT**
//!     `phase.raw_input(config)`: `preedit.raw_input` has always carried the
//!     undecorated raw buffer, while the `raw_input` accessor applies the
//!     derived tone-mark chain (`tsua-li2` vs `tsua-lí`). That divergence is
//!     a long-standing engine property, not a Model B change.
//!
//! Pre-Model-B `display_text == raw_input` collapsed because there were no
//! nailed segments; that identity now holds ONLY when `nailed` is empty.
//! `Phase::composing_display` is the canonical whole-composition accessor.
//! If either invariant breaks, the Enter contract diverges silently.

use composing::{Engine, Intent, Phase};
use protos::engine::{AppConfig, ComposingResponse};

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

fn assert_preedit_model_b_invariants(engine: &Engine, response: &ComposingResponse, label: &str) {
    let preedit = response
        .preedit
        .as_ref()
        .unwrap_or_else(|| panic!("{label}: response has no preedit"));
    let phase = engine.snapshot_state().phase;
    // Model B §10.2 invariant. The true, universally-holding invariant is
    // that the rendered preedit display equals the whole-composition
    // surface accessor:
    //
    //   preedit.display_text == Phase::composing_display(config)
    //
    // (Σ nailed display + derived pending tail). This is the exact Model B
    // generalization of the pre-Model-B `display_text == raw_input` identity
    // — `composing_display` and the old `raw_input` accessor share the same
    // derived chain, and for the no-nailed case they coincide.
    //
    // The pending-tail check pins `preedit.raw_input` against the engine's
    // *raw* pending bytes (the still-editable tail). Note this is the raw
    // (undecorated) form, NOT `Phase::raw_input` which applies the derived
    // tone-mark chain — `preedit.raw_input` has always carried the raw
    // buffer, so comparing it to the derived accessor would spuriously fail
    // for any toned/hyphenated input (e.g. "tsua-li2" vs "tsua-lí"). This is
    // a long-standing engine property, not a Model B change. (Contract step
    // 4 ambiguity resolved: keep the universally-true display invariant; pin
    // raw_input to the raw pending tail rather than the derived accessor.)
    let expected_pending_raw = match &phase {
        Phase::Idle => String::new(),
        Phase::Composing { raw } | Phase::Continuous { raw, .. } => raw.clone(),
    };
    let expected_display = phase.composing_display(&config_tl());
    assert_eq!(
        preedit.raw_input, expected_pending_raw,
        "{label}: Preedit.raw_input != raw pending tail — §10.2 invariant broken"
    );
    assert_eq!(
        preedit.display_text, expected_display,
        "{label}: Preedit.display_text != Phase::composing_display — §10.2 invariant broken"
    );
}

#[test]
fn invariant_holds_after_enter_continuous() {
    let mut engine = Engine::new();
    engine.apply(
        Intent::Start {
            text: "tsua-li2".to_string(),
        },
        &config_tl(),
    );
    let response = engine.apply(Intent::EnterContinuous, &config_tl());
    assert_preedit_model_b_invariants(&engine, &response, "after EnterContinuous");
}

#[test]
fn invariant_holds_after_append_in_continuous() {
    let mut engine = Engine::new();
    engine.apply(
        Intent::Start {
            text: "tsua".to_string(),
        },
        &config_tl(),
    );
    engine.apply(Intent::EnterContinuous, &config_tl());
    let response = engine.apply(
        Intent::Append {
            ch: "2".to_string(),
        },
        &config_tl(),
    );
    assert_preedit_model_b_invariants(&engine, &response, "after Append");
}

#[test]
fn invariant_holds_after_delete_backward_in_continuous() {
    let mut engine = Engine::new();
    engine.apply(
        Intent::Start {
            text: "tsua-li2".to_string(),
        },
        &config_tl(),
    );
    engine.apply(Intent::EnterContinuous, &config_tl());
    let response = engine.apply(Intent::DeleteBackward, &config_tl());
    assert_preedit_model_b_invariants(&engine, &response, "after DeleteBackward");
}

#[test]
fn invariant_holds_after_mid_commit_leaves_pending_tail() {
    let mut engine = Engine::new();
    engine.apply(
        Intent::Start {
            text: "tsuali2".to_string(),
        },
        &config_tl(),
    );
    engine.apply(Intent::EnterContinuous, &config_tl());
    // Mid-commit: consume the first 4 bytes ("tsua" → 紙), pending tail = "li2".
    let response = engine.apply(
        Intent::CommitContinuous {
            display_text: "紙".to_string(),
            canonical_text: String::new(),
            association_tl: String::new(),
            consumed_bytes: 4,
            syllable_count: 1,
        },
        &config_tl(),
    );
    assert_preedit_model_b_invariants(&engine, &response, "after mid-commit");

    // Model B: display_text is the whole composition — nailed "紙" + derived
    // display of the pending tail "li2" → "紙 lí". raw_input is the still-raw
    // pending tail "li2" alone (NOT the original "tsuali2").
    let preedit = response.preedit.as_ref().unwrap();
    assert_eq!(preedit.raw_input, "li2", "pending tail raw must be li2");
    assert_eq!(
        preedit.display_text, "紙 lí",
        "whole composition = nailed 紙 + derived(li2)"
    );
}

#[test]
fn invariant_holds_after_replace_last_in_continuous() {
    let mut engine = Engine::new();
    engine.apply(
        Intent::Start {
            text: "tsua".to_string(),
        },
        &config_tl(),
    );
    engine.apply(Intent::EnterContinuous, &config_tl());
    let response = engine.apply(
        Intent::ReplaceLast {
            replacement: "1".to_string(),
        },
        &config_tl(),
    );
    assert_preedit_model_b_invariants(&engine, &response, "after ReplaceLast");
}

#[test]
fn idle_phase_raw_input_is_empty() {
    let engine = Engine::new();
    assert_eq!(engine.snapshot_state().phase, Phase::Idle);
    assert_eq!(engine.snapshot_state().phase.raw_input(&config_tl()), "");
}

// ---- Item-3 safety blockers (Codex post-impl review 2026-05-13) ----------

/// Append after mid-commit: Item 3's Enter must commit the *new* pending tail
/// (`raw` after the second keystroke), not anything cached from the pre-commit
/// raw or the segment that was already nailed. If `Phase::raw_input` ever drifts
/// to return stale or full-history text after `Append` lands on a Continuous
/// state whose pending tail just shrank from a mid-commit, Item 3 silently
/// commits the wrong thing.
#[test]
fn invariant_holds_for_append_after_mid_commit() {
    let mut engine = Engine::new();
    engine.apply(
        Intent::Start {
            text: "tsuali".to_string(),
        },
        &config_tl(),
    );
    engine.apply(Intent::EnterContinuous, &config_tl());
    // Mid-commit consumes "tsua" (4 bytes), pending tail = "li".
    engine.apply(
        Intent::CommitContinuous {
            display_text: "紙".to_string(),
            canonical_text: String::new(),
            association_tl: String::new(),
            consumed_bytes: 4,
            syllable_count: 1,
        },
        &config_tl(),
    );
    // Append "2" → new pending = "li2", raw_input must render to "lí".
    let response = engine.apply(
        Intent::Append {
            ch: "2".to_string(),
        },
        &config_tl(),
    );
    assert_preedit_model_b_invariants(&engine, &response, "append-after-mid-commit");
    let preedit = response.preedit.as_ref().unwrap();
    // Model B: whole composition = nailed "紙" + derived(new pending "li2").
    assert_eq!(
        preedit.raw_input, "li2",
        "pending tail after append must reflect new tail li2, not stale pre-commit raw"
    );
    assert_eq!(
        preedit.display_text, "紙 lí",
        "whole composition = nailed 紙 + derived(li2)"
    );
}

/// Multi-step mid-commit chain: pin the invariant across two nailed segments.
/// Item 3's Enter at any point in this chain must commit *only* the current
/// pending tail. If `Phase::raw_input` regresses to include already-nailed
/// `NailedSegment.raw_text`, Item 3 would commit duplicate text into the
/// document.
#[test]
fn invariant_holds_through_multi_step_mid_commit_chain() {
    let mut engine = Engine::new();
    engine.apply(
        Intent::Start {
            text: "tsualipoo".to_string(), // tsua-li-poo, 9 bytes
        },
        &config_tl(),
    );
    engine.apply(Intent::EnterContinuous, &config_tl());

    // Step 1: nail "tsua" → 紙, pending = "lipoo".
    let r1 = engine.apply(
        Intent::CommitContinuous {
            display_text: "紙".to_string(),
            canonical_text: String::new(),
            association_tl: String::new(),
            consumed_bytes: 4,
            syllable_count: 1,
        },
        &config_tl(),
    );
    assert_preedit_model_b_invariants(&engine, &r1, "after first mid-commit");
    // Model B: whole composition = nailed "紙" + derived(pending "lipoo").
    assert_eq!(r1.preedit.as_ref().unwrap().raw_input, "lipoo");
    assert_eq!(r1.preedit.as_ref().unwrap().display_text, "紙 lipoo");

    // Step 2: nail "li" → 你, pending = "poo".
    let r2 = engine.apply(
        Intent::CommitContinuous {
            display_text: "你".to_string(),
            canonical_text: String::new(),
            association_tl: String::new(),
            consumed_bytes: 2,
            syllable_count: 1,
        },
        &config_tl(),
    );
    assert_preedit_model_b_invariants(&engine, &r2, "after second mid-commit");
    // Model B: raw pending tail is "poo" alone (no prior segment raw); the
    // display is the whole composition = nailed "紙你" + derived("poo").
    assert_eq!(
        r2.preedit.as_ref().unwrap().raw_input,
        "poo",
        "pending tail after two mid-commits must be 'poo' alone, not include prior segments"
    );
    assert_eq!(
        r2.preedit.as_ref().unwrap().display_text,
        "紙 你 poo",
        "whole composition = nailed 紙你 + derived(poo)"
    );

    // Confirm engine state matches: two nailed segments + "poo" pending.
    let state = engine.snapshot_state();
    let Phase::Continuous { raw, nailed } = state.phase else {
        panic!("expected Continuous after chained mid-commit");
    };
    assert_eq!(raw, "poo");
    assert_eq!(nailed.len(), 2);
    assert_eq!(nailed[0].display_text, "紙");
    assert_eq!(nailed[1].display_text, "你");
}
