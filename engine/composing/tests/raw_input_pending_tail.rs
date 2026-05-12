//! v3.5.8 Phase 9 Item 2 — pin the §10.2 / §10.3 clarification β invariant:
//!
//!   For every Continuous-phase mutation that surfaces a preedit response,
//!   `response.preedit.display_text == engine.snapshot_state().phase.raw_input(config)`.
//!
//! `Phase::raw_input` is the canonical accessor Item 3 (Enter-raw commit)
//! will use to commit exactly what the user sees inline. If this invariant
//! ever breaks, Item 3's Enter contract diverges silently. Pinning it here
//! makes that regression a test failure instead of dogfood-only signal.

// 中文: 釘住 §10.2 不變式 — Preedit.display_text 必須恆等於 Phase::raw_input。
// 中文: Item 3 Enter commit 將用此 helper 作為唯一字串來源,所以這條不變式不能默默破。

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
    }
}

fn assert_preedit_matches_raw_input(engine: &Engine, response: &ComposingResponse, label: &str) {
    let preedit = response
        .preedit
        .as_ref()
        .unwrap_or_else(|| panic!("{label}: response has no preedit"));
    let expected = engine.snapshot_state().phase.raw_input(&config_tl());
    assert_eq!(
        preedit.display_text, expected,
        "{label}: Preedit.display_text != Phase::raw_input — §10.2 invariant broken"
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
    assert_preedit_matches_raw_input(&engine, &response, "after EnterContinuous");
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
    assert_preedit_matches_raw_input(&engine, &response, "after Append");
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
    assert_preedit_matches_raw_input(&engine, &response, "after DeleteBackward");
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
            consumed_bytes: 4,
            syllable_count: 1,
        },
        &config_tl(),
    );
    assert_preedit_matches_raw_input(&engine, &response, "after mid-commit");

    // Pending tail must be derived display of "li2" alone, NOT of the
    // original "tsuali2" — pins clarification β.
    let preedit = response.preedit.as_ref().unwrap();
    assert_eq!(preedit.display_text, "lí", "pending tail must be li2 → lí");
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
    assert_preedit_matches_raw_input(&engine, &response, "after ReplaceLast");
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
    assert_preedit_matches_raw_input(&engine, &response, "append-after-mid-commit");
    let preedit = response.preedit.as_ref().unwrap();
    assert_eq!(
        preedit.display_text, "lí",
        "pending tail after append must reflect new tail li2, not stale pre-commit raw"
    );
}

/// Multi-step mid-commit chain: pin the invariant across two nailed segments.
/// Item 3's Enter at any point in this chain must commit *only* the current
/// pending tail. If `Phase::raw_input` regresses to include already-nailed
/// `CommittedSegment.raw_text`, Item 3 would commit duplicate text into the
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
            consumed_bytes: 4,
            syllable_count: 1,
        },
        &config_tl(),
    );
    assert_preedit_matches_raw_input(&engine, &r1, "after first mid-commit");
    assert_eq!(r1.preedit.as_ref().unwrap().display_text, "lipoo");

    // Step 2: nail "li" → 你, pending = "poo".
    let r2 = engine.apply(
        Intent::CommitContinuous {
            display_text: "你".to_string(),
            consumed_bytes: 2,
            syllable_count: 1,
        },
        &config_tl(),
    );
    assert_preedit_matches_raw_input(&engine, &r2, "after second mid-commit");
    assert_eq!(
        r2.preedit.as_ref().unwrap().display_text,
        "poo",
        "pending tail after two mid-commits must be 'poo' alone, not include prior segments"
    );

    // Confirm engine state matches: two committed segments + "poo" pending.
    let state = engine.snapshot_state();
    let Phase::Continuous { raw, committed } = state.phase else {
        panic!("expected Continuous after chained mid-commit");
    };
    assert_eq!(raw, "poo");
    assert_eq!(committed.len(), 2);
    assert_eq!(committed[0].display_text, "紙");
    assert_eq!(committed[1].display_text, "你");
}
