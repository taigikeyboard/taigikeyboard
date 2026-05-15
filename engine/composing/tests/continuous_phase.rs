//! Phase 4 — `Phase::Continuous` integration tests.
//!
//! Phase 4 keeps the new Intents (`EnterContinuous` / `CommitContinuous` /
//! `ResetContinuous`) Rust-only — the proto `oneof method` carrier lands in
//! Phase 6. Tests therefore exercise the engine through the in-process
//! `Engine::apply` API and inspect `Phase::Continuous` internals via
//! `Engine::snapshot_state` (the `ComposingResponse.preedit` proto field
//! reflects pending only; committed lives in the document and in
//! `EngineState`).
//!
//! Each test pins the **exact effect order**, not just membership — the
//! `transition.rs` doc-comment promises proto-ordered effect consumption,
//! so order regressions must surface here.

// 中文: Phase 4 連續輸入 (Phase::Continuous) 整合測試。
// 中文: 新 Intent 還沒 proto carrier (Phase 6 才加),所以直接用 Engine::apply。
// 中文: Effect 順序鎖死位置而不是只測 membership。

use composing::{CommittedSegment, Engine, Intent, Phase};
use protos::engine::effect::Kind;
use protos::engine::{AppConfig, Effect};

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

fn engine_in_continuous(raw: &str) -> Engine {
    let mut e = Engine::new();
    e.apply(
        Intent::Start {
            text: raw.to_string(),
        },
        &config_tl(),
    );
    e.apply(Intent::EnterContinuous, &config_tl());
    e
}

fn assert_kinds<'a, K>(effects: &'a [Effect], expected: K)
where
    K: IntoIterator<Item = &'a str>,
{
    let actual: Vec<&'static str> = effects
        .iter()
        .map(|e| match e.kind.as_ref().expect("effect kind") {
            Kind::UpdatePreedit(_) => "UpdatePreedit",
            Kind::ClearPreeditWithoutCommit(_) => "ClearPreeditWithoutCommit",
            Kind::CommitTextReplacingPreedit(_) => "CommitTextReplacingPreedit",
            Kind::DeleteBackwardFromDocument(_) => "DeleteBackwardFromDocument",
            Kind::ResetAutocomplete(_) => "ResetAutocomplete",
            Kind::PerformAutocomplete(_) => "PerformAutocomplete",
            Kind::ResetAutocompleteContext(_) => "ResetAutocompleteContext",
            Kind::NextWordUpdateLastSelectedWord(_) => "NextWordUpdateLastSelectedWord",
            Kind::NextWordWordSelected(_) => "NextWordWordSelected",
            Kind::NextWordClearForNewComposing(_) => "NextWordClearForNewComposing",
        })
        .collect();
    let expected: Vec<&str> = expected.into_iter().collect();
    assert_eq!(actual, expected, "effect kinds (ordered)");
}

// ---- Enter ---------------------------------------------------------

#[test]
fn enter_continuous_from_composing_keeps_raw_no_effects() {
    let mut e = Engine::new();
    e.apply(
        Intent::Start {
            text: "tsua".to_string(),
        },
        &config_tl(),
    );
    let resp = e.apply(Intent::EnterContinuous, &config_tl());
    assert_kinds(&resp.effect, std::iter::empty());
    let state = e.snapshot_state();
    match state.phase {
        Phase::Continuous { raw, committed } => {
            assert_eq!(raw, "tsua");
            assert!(committed.is_empty());
        }
        other => panic!("expected Continuous, got {other:?}"),
    }
    assert!(resp.is_composing);
}

#[test]
fn enter_continuous_from_idle_is_noop() {
    let mut e = Engine::new();
    let resp = e.apply(Intent::EnterContinuous, &config_tl());
    assert!(resp.effect.is_empty());
    assert_eq!(e.snapshot_state().phase, Phase::Idle);
}

#[test]
fn enter_continuous_from_continuous_is_noop() {
    let mut e = engine_in_continuous("tsua");
    let resp = e.apply(Intent::EnterContinuous, &config_tl());
    assert!(resp.effect.is_empty());
    let state = e.snapshot_state();
    match state.phase {
        Phase::Continuous { raw, .. } => assert_eq!(raw, "tsua"),
        other => panic!("expected Continuous, got {other:?}"),
    }
}

// ---- Mid-commit ----------------------------------------------------

#[test]
fn mid_commit_pushes_segment_and_emits_ordered_effects() {
    let mut e = engine_in_continuous("tsua");
    let resp = e.apply(
        Intent::CommitContinuous {
            display_text: "珠".to_string(),
            canonical_text: String::new(),
            consumed_bytes: 3,
            syllable_count: 1,
        },
        &config_tl(),
    );

    assert_kinds(
        &resp.effect,
        [
            "CommitTextReplacingPreedit",
            "UpdatePreedit",
            "NextWordUpdateLastSelectedWord",
            "PerformAutocomplete",
        ],
    );

    let state = e.snapshot_state();
    let Phase::Continuous { raw, committed } = state.phase else {
        panic!("still in Continuous");
    };
    assert_eq!(raw, "a");
    assert_eq!(committed.len(), 1);
    assert_eq!(committed[0].display_text, "珠");
    assert_eq!(committed[0].raw_text, "tsu");
    assert_eq!(committed[0].raw_span, (0, 3));
    assert_eq!(committed[0].syllable_count, 1);

    // Effect payloads carry the just-committed segment.
    let Kind::CommitTextReplacingPreedit(commit) = resp.effect[0].kind.as_ref().unwrap() else {
        unreachable!();
    };
    assert_eq!(commit.text, "珠");
    let Kind::NextWordUpdateLastSelectedWord(nw) = resp.effect[2].kind.as_ref().unwrap() else {
        unreachable!();
    };
    assert_eq!(nw.text, "珠");
    assert_eq!(nw.roman, "tsu");
}

#[test]
fn mid_commit_chains_raw_span_from_previous_segment() {
    let mut e = engine_in_continuous("tsuagua");
    e.apply(
        Intent::CommitContinuous {
            display_text: "珠".to_string(),
            canonical_text: String::new(),
            consumed_bytes: 3,
            syllable_count: 1,
        },
        &config_tl(),
    );
    e.apply(
        Intent::CommitContinuous {
            display_text: "仔".to_string(),
            canonical_text: String::new(),
            consumed_bytes: 1,
            syllable_count: 1,
        },
        &config_tl(),
    );
    let state = e.snapshot_state();
    let Phase::Continuous { committed, .. } = state.phase else {
        panic!();
    };
    assert_eq!(committed[0].raw_span, (0, 3));
    assert_eq!(committed[1].raw_span, (3, 4));
    assert_eq!(committed[1].raw_text, "a");
}

// ---- Final commit --------------------------------------------------

#[test]
fn final_commit_exits_to_idle_emits_word_selected() {
    let mut e = engine_in_continuous("tsu");
    let resp = e.apply(
        Intent::CommitContinuous {
            display_text: "珠".to_string(),
            canonical_text: String::new(),
            consumed_bytes: 3,
            syllable_count: 1,
        },
        &config_tl(),
    );
    assert_kinds(
        &resp.effect,
        [
            "CommitTextReplacingPreedit",
            "ResetAutocomplete",
            "ResetAutocompleteContext",
            "NextWordWordSelected",
        ],
    );
    assert!(!resp.is_composing);
    assert_eq!(resp.selected_candidate_index, -1);
    assert_eq!(e.snapshot_state().phase, Phase::Idle);

    let Kind::NextWordWordSelected(nw) = resp.effect[3].kind.as_ref().unwrap() else {
        unreachable!();
    };
    assert_eq!(nw.text, "珠");
    assert_eq!(nw.roman, "tsu");
    assert!(nw.trigger_prediction);
}

// ---- CommitContinuous validation ----------------------------------

#[test]
fn commit_continuous_out_of_range_is_noop() {
    let mut e = engine_in_continuous("tsua");
    let resp = e.apply(
        Intent::CommitContinuous {
            display_text: "X".to_string(),
            canonical_text: String::new(),
            consumed_bytes: 99,
            syllable_count: 1,
        },
        &config_tl(),
    );
    assert!(resp.effect.is_empty());
    let Phase::Continuous { raw, committed } = e.snapshot_state().phase else {
        panic!();
    };
    assert_eq!(raw, "tsua");
    assert!(committed.is_empty());
}

#[test]
fn commit_continuous_at_non_char_boundary_is_noop() {
    // Multi-byte UTF-8 in pending: "ㄎㄚ" is 6 bytes (each Bopomofo char = 3 bytes).
    let mut e = Engine::new();
    e.apply(
        Intent::Start {
            text: "ㄎㄚ".to_string(),
        },
        &config_tl(),
    );
    e.apply(Intent::EnterContinuous, &config_tl());
    let resp = e.apply(
        Intent::CommitContinuous {
            display_text: "X".to_string(),
            canonical_text: String::new(),
            consumed_bytes: 1, // mid-codepoint
            syllable_count: 1,
        },
        &config_tl(),
    );
    assert!(resp.effect.is_empty());
}

#[test]
fn commit_continuous_zero_bytes_is_noop() {
    let mut e = engine_in_continuous("tsua");
    let resp = e.apply(
        Intent::CommitContinuous {
            display_text: "X".to_string(),
            canonical_text: String::new(),
            consumed_bytes: 0,
            syllable_count: 1,
        },
        &config_tl(),
    );
    assert!(resp.effect.is_empty());
}

#[test]
fn commit_continuous_empty_display_is_noop() {
    let mut e = engine_in_continuous("tsua");
    let resp = e.apply(
        Intent::CommitContinuous {
            display_text: String::new(),
            canonical_text: String::new(),
            consumed_bytes: 3,
            syllable_count: 1,
        },
        &config_tl(),
    );
    assert!(resp.effect.is_empty());
}

// ---- ResetContinuous ----------------------------------------------

#[test]
fn reset_continuous_exits_emits_clear_and_nextword_signal() {
    let mut e = engine_in_continuous("tsua");
    e.apply(
        Intent::CommitContinuous {
            display_text: "珠".to_string(),
            canonical_text: String::new(),
            consumed_bytes: 3,
            syllable_count: 1,
        },
        &config_tl(),
    );
    let resp = e.apply(Intent::ResetContinuous, &config_tl());
    assert_kinds(
        &resp.effect,
        [
            "ClearPreeditWithoutCommit",
            "ResetAutocomplete",
            "NextWordClearForNewComposing",
        ],
    );
    assert_eq!(e.snapshot_state().phase, Phase::Idle);
}

#[test]
fn reset_continuous_outside_continuous_is_noop() {
    let mut e = Engine::new();
    let resp = e.apply(Intent::ResetContinuous, &config_tl());
    assert!(resp.effect.is_empty());
}

// ---- Reset (existing Intent) under Continuous ---------------------

#[test]
fn reset_under_continuous_emits_nextword_clear_in_addition_to_composing_pair() {
    let mut e = engine_in_continuous("tsua");
    let resp = e.apply(Intent::Reset, &config_tl());
    assert_kinds(
        &resp.effect,
        [
            "ClearPreeditWithoutCommit",
            "ResetAutocomplete",
            "NextWordClearForNewComposing",
        ],
    );
    assert_eq!(e.snapshot_state().phase, Phase::Idle);
}

// ---- Append / AppendHyphen / ReplaceLast under Continuous ---------

#[test]
fn append_under_continuous_extends_pending_only() {
    let mut e = engine_in_continuous("tsu");
    e.apply(
        Intent::CommitContinuous {
            display_text: "珠".to_string(),
            canonical_text: String::new(),
            consumed_bytes: 3,
            syllable_count: 1,
        },
        &config_tl(),
    );
    // After mid-commit, raw="" — but the seed used "tsu" so pending is empty
    // here. Re-prime with a non-final mid-commit using a longer raw.
    let mut e = engine_in_continuous("tsuagua");
    e.apply(
        Intent::CommitContinuous {
            display_text: "珠".to_string(),
            canonical_text: String::new(),
            consumed_bytes: 3,
            syllable_count: 1,
        },
        &config_tl(),
    );
    // pending = "agua" now. Append "h".
    let resp = e.apply(
        Intent::Append {
            ch: "h".to_string(),
        },
        &config_tl(),
    );
    assert_kinds(&resp.effect, ["UpdatePreedit", "PerformAutocomplete"]);
    let Phase::Continuous { raw, committed } = e.snapshot_state().phase else {
        panic!();
    };
    assert_eq!(raw, "aguah");
    assert_eq!(committed.len(), 1);
    assert_eq!(committed[0].display_text, "珠");
}

#[test]
fn append_hyphen_under_continuous_appends_dash() {
    let mut e = engine_in_continuous("tsua");
    let resp = e.apply(Intent::AppendHyphen, &config_tl());
    assert_kinds(&resp.effect, ["UpdatePreedit", "PerformAutocomplete"]);
    let Phase::Continuous { raw, .. } = e.snapshot_state().phase else {
        panic!();
    };
    assert_eq!(raw, "tsua-");
}

#[test]
fn replace_last_under_continuous_modifies_pending_only() {
    let mut e = engine_in_continuous("tsuagua");
    e.apply(
        Intent::CommitContinuous {
            display_text: "珠".to_string(),
            canonical_text: String::new(),
            consumed_bytes: 3,
            syllable_count: 1,
        },
        &config_tl(),
    );
    // pending = "agua". Replace last char.
    let resp = e.apply(
        Intent::ReplaceLast {
            replacement: "i".to_string(),
        },
        &config_tl(),
    );
    assert_kinds(&resp.effect, ["UpdatePreedit", "PerformAutocomplete"]);
    let Phase::Continuous { raw, committed } = e.snapshot_state().phase else {
        panic!();
    };
    assert_eq!(raw, "agui");
    assert_eq!(committed.len(), 1);
}

// ---- DeleteBackward (folded backspace) under Continuous -----------

#[test]
fn delete_backward_under_continuous_drops_pending_char() {
    let mut e = engine_in_continuous("tsua");
    let resp = e.apply(Intent::DeleteBackward, &config_tl());
    assert_kinds(&resp.effect, ["UpdatePreedit", "PerformAutocomplete"]);
    let Phase::Continuous { raw, .. } = e.snapshot_state().phase else {
        panic!();
    };
    assert_eq!(raw, "tsu");
}

#[test]
fn delete_backward_under_continuous_pops_committed_when_pending_empty() {
    let mut e = engine_in_continuous("tsua");
    e.apply(
        Intent::CommitContinuous {
            display_text: "珠".to_string(),
            canonical_text: String::new(),
            consumed_bytes: 3,
            syllable_count: 1,
        },
        &config_tl(),
    );
    // pending = "a". Delete pending char first.
    e.apply(Intent::DeleteBackward, &config_tl());
    let Phase::Continuous { raw, committed } = e.snapshot_state().phase else {
        panic!();
    };
    assert_eq!(raw, "");
    assert_eq!(committed.len(), 1);

    // Next backspace pops committed segment, restoring "tsu" as pending.
    // Emit one DeleteBackwardFromDocument per char of popped display
    // ("珠" = 1 char), then a NextWordClearForNewComposing (no committed
    // remains so nextword's last_selected_word must clear — Codex
    // post-impl finding #1), then UpdatePreedit + PerformAutocomplete.
    let resp = e.apply(Intent::DeleteBackward, &config_tl());
    assert_kinds(
        &resp.effect,
        [
            "DeleteBackwardFromDocument",
            "NextWordClearForNewComposing",
            "UpdatePreedit",
            "PerformAutocomplete",
        ],
    );
    let Phase::Continuous { raw, committed } = e.snapshot_state().phase else {
        panic!();
    };
    assert_eq!(raw, "tsu");
    assert!(committed.is_empty());
}

#[test]
fn delete_backward_pop_with_remaining_committed_emits_nextword_update() {
    // Two committed segments → pop the latter → nextword's last_selected_word
    // should now correct to the segment behind, not clear.
    let mut e = engine_in_continuous("tsuagua");
    e.apply(
        Intent::CommitContinuous {
            display_text: "珠".to_string(),
            canonical_text: String::new(),
            consumed_bytes: 3,
            syllable_count: 1,
        },
        &config_tl(),
    );
    e.apply(
        Intent::CommitContinuous {
            display_text: "仔".to_string(),
            canonical_text: String::new(),
            consumed_bytes: 1,
            syllable_count: 1,
        },
        &config_tl(),
    );
    // pending = "gua", committed = ["珠", "仔"].
    // Drain pending so next backspace pops "仔".
    e.apply(Intent::DeleteBackward, &config_tl()); // pending = "gu"
    e.apply(Intent::DeleteBackward, &config_tl()); // pending = "g"
    e.apply(Intent::DeleteBackward, &config_tl()); // pending = ""

    let resp = e.apply(Intent::DeleteBackward, &config_tl());
    assert_kinds(
        &resp.effect,
        [
            "DeleteBackwardFromDocument",
            "NextWordUpdateLastSelectedWord",
            "UpdatePreedit",
            "PerformAutocomplete",
        ],
    );
    // The NextWordUpdateLastSelectedWord payload should reflect "珠" / "tsu",
    // the segment that's still in the document.
    let Kind::NextWordUpdateLastSelectedWord(nw) = resp.effect[1].kind.as_ref().unwrap() else {
        unreachable!();
    };
    assert_eq!(nw.text, "珠");
    assert_eq!(nw.roman, "tsu");

    let Phase::Continuous { raw, committed } = e.snapshot_state().phase else {
        panic!();
    };
    assert_eq!(raw, "a");
    assert_eq!(committed.len(), 1);
    assert_eq!(committed[0].display_text, "珠");
}

#[test]
fn delete_backward_pops_multi_char_display_emits_n_deletes() {
    let mut e = engine_in_continuous("tsuagua");
    // Mid-commit a 2-syllable segment (display "珠仔" = 2 chars, raw "tsua" = 4 bytes).
    e.apply(
        Intent::CommitContinuous {
            display_text: "珠仔".to_string(),
            canonical_text: String::new(),
            consumed_bytes: 4,
            syllable_count: 2,
        },
        &config_tl(),
    );
    // Drain pending so next backspace pops committed.
    e.apply(Intent::DeleteBackward, &config_tl()); // pending = "gu"
    e.apply(Intent::DeleteBackward, &config_tl()); // pending = "g"
    e.apply(Intent::DeleteBackward, &config_tl()); // pending = ""

    let resp = e.apply(Intent::DeleteBackward, &config_tl());
    // 2 char display → 2 DeleteBackwardFromDocument effects, then
    // NextWordClearForNewComposing (committed list now empty), then
    // UpdatePreedit + PerformAutocomplete.
    assert_kinds(
        &resp.effect,
        [
            "DeleteBackwardFromDocument",
            "DeleteBackwardFromDocument",
            "NextWordClearForNewComposing",
            "UpdatePreedit",
            "PerformAutocomplete",
        ],
    );
    let Phase::Continuous { raw, committed } = e.snapshot_state().phase else {
        panic!();
    };
    assert_eq!(raw, "tsua");
    assert!(committed.is_empty());
}

#[test]
fn delete_backward_under_continuous_with_empty_state_exits_to_idle() {
    // Engineered edge case: enter continuous on a single-char raw, then
    // delete that char with no committed segments. Engine exits to Idle and
    // forwards a document-side backspace.
    let mut e = engine_in_continuous("a");
    let resp = e.apply(Intent::DeleteBackward, &config_tl());
    assert_kinds(
        &resp.effect,
        [
            "ClearPreeditWithoutCommit",
            "ResetAutocomplete",
            "NextWordClearForNewComposing",
            "DeleteBackwardFromDocument",
        ],
    );
    assert_eq!(e.snapshot_state().phase, Phase::Idle);
}

// ---- SetSelectedCandidateIndex / QueryState under Continuous -----

#[test]
fn set_selected_candidate_index_under_continuous_keeps_phase() {
    let mut e = engine_in_continuous("tsua");
    let resp = e.apply(Intent::SetSelectedCandidateIndex { index: 3 }, &config_tl());
    assert!(resp.effect.is_empty());
    assert_eq!(resp.selected_candidate_index, 3);
    let Phase::Continuous { raw, .. } = e.snapshot_state().phase else {
        panic!();
    };
    assert_eq!(raw, "tsua");
}

#[test]
fn query_state_under_continuous_returns_pending_only_preedit() {
    let mut e = engine_in_continuous("tsuagua");
    e.apply(
        Intent::CommitContinuous {
            display_text: "珠".to_string(),
            canonical_text: String::new(),
            consumed_bytes: 3,
            syllable_count: 1,
        },
        &config_tl(),
    );
    let resp = e.apply(Intent::QueryState, &config_tl());
    assert!(resp.effect.is_empty());
    let preedit = resp.preedit.expect("preedit");
    assert_eq!(preedit.raw_input, "agua"); // pending only, NOT committed.raw + pending
    assert!(resp.is_composing);
}

// ---- Continuous-phase routing of legacy text-bearing Intents -----
//
// Per Codex post-impl finding #3, the three Intents that carry user text
// (`Start`, `SelectSuggestion`, `CommitPreeditThenInsertExternal`) under
// Continuous must NOT silently drop the text. They drop continuous state
// first, then route the text through a sane equivalent: `Start` becomes
// "abort continuous + begin fresh Composing"; `SelectSuggestion` becomes
// "abort continuous + commit text directly"; `CommitPreeditThenInsertExternal`
// becomes "abort continuous + commit (pending derived ++ external) atomically".
// `CommitDerived` stays snapshot noop — Continuous already auto-commits via
// mid-commit. `CommitRaw` was the same noop until v3.5.8 Phase 9 Item 3 made
// it commit the pending-tail derived display + fire NextWord (see
// `commit_raw_under_continuous_*` tests below and
// `docs/engine/continuous-input-ranking.md` §10.3).

#[test]
fn start_under_continuous_aborts_then_begins_fresh_composing() {
    let mut e = engine_in_continuous("tsua");
    let resp = e.apply(
        Intent::Start {
            text: "abc".to_string(),
        },
        &config_tl(),
    );
    assert_kinds(
        &resp.effect,
        [
            "ClearPreeditWithoutCommit",
            "ResetAutocomplete",
            "NextWordClearForNewComposing",
            "UpdatePreedit",
            "PerformAutocomplete",
        ],
    );
    let Phase::Composing { raw } = e.snapshot_state().phase else {
        panic!("Start under Continuous must land in Composing");
    };
    assert_eq!(raw, "abc");
}

#[test]
fn commit_derived_under_continuous_is_noop() {
    let mut e = engine_in_continuous("tsua");
    let resp = e.apply(Intent::CommitDerived, &config_tl());
    assert!(resp.effect.is_empty());
    assert!(matches!(e.snapshot_state().phase, Phase::Continuous { .. }));
}

// Phase 9 Item 3 — Enter-raw commit in Continuous. The four tests below pin
// the new contract: CommitRaw under Continuous now mirrors commit_continuous's
// final-commit shape (4 effects, NextWordWordSelected fires) and commits the
// pending-tail's derived display rather than literal keystrokes. Mid-commit
// state shrinks `Phase::Continuous.raw` to the pending tail, so Enter only
// commits that tail; nailed segments stay in the document untouched.
// 中文: Phase 9 Item 3 — Continuous 下的 CommitRaw 不再 noop,改提交 derived_display(pending) +
// 中文: 4 個 effect 對齊 commit_continuous final-commit;mid-commit 後只提交 pending 尾。

#[test]
fn commit_raw_under_continuous_commits_derived_display_and_fires_nextword() {
    // Pending = "li2" → derived display = "lí". Enter commits "lí" (display)
    // with roman = "li2" (raw) and trigger_prediction = true.
    let mut e = engine_in_continuous("li2");
    let resp = e.apply(Intent::CommitRaw, &config_tl());
    assert_kinds(
        &resp.effect,
        [
            "CommitTextReplacingPreedit",
            "ResetAutocomplete",
            "ResetAutocompleteContext",
            "NextWordWordSelected",
        ],
    );
    let Kind::CommitTextReplacingPreedit(commit) = resp.effect[0].kind.as_ref().unwrap() else {
        unreachable!();
    };
    assert_eq!(commit.text, "lí");
    let Kind::NextWordWordSelected(nw) = resp.effect[3].kind.as_ref().unwrap() else {
        unreachable!();
    };
    assert_eq!(nw.text, "lí");
    assert_eq!(nw.roman, "li2");
    assert!(nw.trigger_prediction);
    assert_eq!(e.snapshot_state().phase, Phase::Idle);
}

#[test]
fn commit_raw_under_continuous_after_mid_commit_only_commits_pending_tail() {
    // Setup: enter Continuous on "tsuali2", mid-commit "紙" consuming bytes 0..4
    // ("tsua"), leaving pending = "li2". Enter then commits "lí" — the pending
    // tail's derived display — NOT the original keystrokes "tsuali2".
    let mut e = engine_in_continuous("tsuali2");
    e.apply(
        Intent::CommitContinuous {
            display_text: "紙".to_string(),
            canonical_text: String::new(),
            consumed_bytes: 4,
            syllable_count: 1,
        },
        &config_tl(),
    );
    let Phase::Continuous { raw, committed } = e.snapshot_state().phase else {
        panic!("expected Continuous after mid-commit");
    };
    assert_eq!(raw, "li2");
    assert_eq!(committed.len(), 1);

    let resp = e.apply(Intent::CommitRaw, &config_tl());
    assert_kinds(
        &resp.effect,
        [
            "CommitTextReplacingPreedit",
            "ResetAutocomplete",
            "ResetAutocompleteContext",
            "NextWordWordSelected",
        ],
    );
    let Kind::CommitTextReplacingPreedit(commit) = resp.effect[0].kind.as_ref().unwrap() else {
        unreachable!();
    };
    // Only the pending tail "li2" → "lí"; "紙" was already in the document
    // from the earlier mid-commit and is not re-emitted here.
    // 中文: 只提交 pending 尾「lí」;先前 mid-commit 的「紙」已在文件中、不重發。
    assert_eq!(commit.text, "lí");
    let Kind::NextWordWordSelected(nw) = resp.effect[3].kind.as_ref().unwrap() else {
        unreachable!();
    };
    assert_eq!(nw.roman, "li2");
    assert_eq!(e.snapshot_state().phase, Phase::Idle);
}

#[test]
fn commit_raw_under_continuous_with_translate_swapped_unchanged() {
    // F6.A regression: is_translate_swapped is a candidate-display swap that
    // does not influence the inline pre-edit / Enter-raw contract. Enter
    // commits the same derived display whether swap is on or off.
    // 中文: F6.A 回歸測試 — is_translate_swapped 只影響候選顯示,不影響 Enter-raw commit 字串。
    let mut config = config_tl();
    config.is_translate_swapped = true;
    let mut e = Engine::new();
    e.apply(
        Intent::Start {
            text: "li2".to_string(),
        },
        &config,
    );
    e.apply(Intent::EnterContinuous, &config);
    let resp = e.apply(Intent::CommitRaw, &config);
    let Kind::CommitTextReplacingPreedit(commit) = resp.effect[0].kind.as_ref().unwrap() else {
        unreachable!();
    };
    assert_eq!(commit.text, "lí");
}

#[test]
fn commit_raw_under_continuous_tps_passes_through_verbatim() {
    // TPS pre-edit is rendered as-is (derived_display short-circuits TPS to
    // the raw bopomofo string). Enter commits the same string.
    // 中文: TPS 直接以原樣作為 derived display;Enter 提交一致字串。
    let mut e = Engine::new();
    e.apply(
        Intent::Start {
            text: "ㄍㄨㄚˋ".to_string(),
        },
        &config_tl(),
    );
    e.apply(Intent::EnterContinuous, &config_tl());
    let resp = e.apply(Intent::CommitRaw, &config_tl());
    let Kind::CommitTextReplacingPreedit(commit) = resp.effect[0].kind.as_ref().unwrap() else {
        unreachable!();
    };
    assert_eq!(commit.text, "ㄍㄨㄚˋ");
    let Kind::NextWordWordSelected(nw) = resp.effect[3].kind.as_ref().unwrap() else {
        unreachable!();
    };
    assert_eq!(nw.roman, "ㄍㄨㄚˋ");
    assert_eq!(e.snapshot_state().phase, Phase::Idle);
}

#[test]
fn select_suggestion_under_continuous_commits_text_and_exits() {
    let mut e = engine_in_continuous("tsua");
    let resp = e.apply(
        Intent::SelectSuggestion {
            text: "紙".to_string(),
        },
        &config_tl(),
    );
    assert_kinds(
        &resp.effect,
        [
            "CommitTextReplacingPreedit",
            "ResetAutocomplete",
            "ResetAutocompleteContext",
            "NextWordClearForNewComposing",
        ],
    );
    let Kind::CommitTextReplacingPreedit(commit) = resp.effect[0].kind.as_ref().unwrap() else {
        unreachable!();
    };
    assert_eq!(commit.text, "紙");
    assert_eq!(e.snapshot_state().phase, Phase::Idle);
}

#[test]
fn select_suggestion_under_continuous_with_empty_text_just_resets() {
    let mut e = engine_in_continuous("tsua");
    let resp = e.apply(
        Intent::SelectSuggestion {
            text: String::new(),
        },
        &config_tl(),
    );
    assert_kinds(
        &resp.effect,
        [
            "ClearPreeditWithoutCommit",
            "ResetAutocomplete",
            "NextWordClearForNewComposing",
        ],
    );
    assert_eq!(e.snapshot_state().phase, Phase::Idle);
}

#[test]
fn commit_preedit_then_insert_external_under_continuous_combines_pending_and_external() {
    let mut e = engine_in_continuous("tsua");
    let resp = e.apply(
        Intent::CommitPreeditThenInsertExternal {
            text: "!".to_string(),
        },
        &config_tl(),
    );
    assert_kinds(
        &resp.effect,
        [
            "CommitTextReplacingPreedit",
            "ResetAutocomplete",
            "ResetAutocompleteContext",
            "NextWordClearForNewComposing",
        ],
    );
    let Kind::CommitTextReplacingPreedit(commit) = resp.effect[0].kind.as_ref().unwrap() else {
        unreachable!();
    };
    // Pending "tsua" derived (no tone digit) + external "!" → "tsua!"
    assert_eq!(commit.text, "tsua!");
    assert_eq!(e.snapshot_state().phase, Phase::Idle);
}

#[test]
fn commit_preedit_then_insert_external_under_continuous_with_empty_external_is_noop() {
    let mut e = engine_in_continuous("tsua");
    let resp = e.apply(
        Intent::CommitPreeditThenInsertExternal {
            text: String::new(),
        },
        &config_tl(),
    );
    assert!(resp.effect.is_empty());
    assert!(matches!(e.snapshot_state().phase, Phase::Continuous { .. }));
}

// ---- Empty-Continuous invariant guards (Codex finding #2) --------

#[test]
fn enter_continuous_from_empty_composing_is_noop() {
    let mut e = Engine::new();
    e.apply(
        Intent::Start {
            text: String::new(),
        },
        &config_tl(),
    );
    // Composing { raw: "" } is degenerate but reachable. EnterContinuous must
    // not transition to Continuous { raw: "", committed: [] }.
    let resp = e.apply(Intent::EnterContinuous, &config_tl());
    assert!(resp.effect.is_empty());
    assert!(matches!(
        e.snapshot_state().phase,
        Phase::Composing { .. } | Phase::Idle
    ));
}

#[test]
fn replace_last_to_empty_pending_no_committed_exits_to_idle() {
    // Engineered: Continuous with single-char pending and no committed.
    // ReplaceLast with empty replacement collapses to Idle, fires nextword clear.
    let mut e = engine_in_continuous("a");
    let resp = e.apply(
        Intent::ReplaceLast {
            replacement: String::new(),
        },
        &config_tl(),
    );
    assert_kinds(
        &resp.effect,
        [
            "ClearPreeditWithoutCommit",
            "ResetAutocomplete",
            "NextWordClearForNewComposing",
        ],
    );
    assert_eq!(e.snapshot_state().phase, Phase::Idle);
}

// ---- CommittedSegment public surface ------------------------------

#[test]
fn committed_segment_public_fields_round_trip() {
    let seg = CommittedSegment {
        display_text: "珠仔".to_string(),
        canonical_text: "珠仔".to_string(),
        raw_text: "tsua".to_string(),
        raw_span: (0, 4),
        syllable_count: 2,
    };
    assert_eq!(seg.display_text, "珠仔");
    assert_eq!(seg.canonical_text, "珠仔");
    assert_eq!(seg.raw_text, "tsua");
    assert_eq!(seg.raw_span, (0, 4));
    assert_eq!(seg.syllable_count, 2);
}

// ---- v3.5.8 Phase 9 Bug 1 (Option A) — swap-aware document commit ----
// `display_text` = swap/TPS/both-scripts DOCUMENT string; `canonical_text`
// = canonical key for NextWord. Empty canonical → falls back to display
// (legacy callers). These pin: document write + CommittedSegment.display_text
// + backspace delete length use `display_text`; NextWord uses `canonical`.

#[test]
fn bug1_mid_commit_documents_display_but_nextword_uses_canonical() {
    let mut e = engine_in_continuous("tsua");
    let resp = e.apply(
        Intent::CommitContinuous {
            display_text: "tāi-uân".to_string(), // swapped roman → document
            canonical_text: "臺灣".to_string(),  // canonical → NextWord/freq
            consumed_bytes: 3,
            syllable_count: 1,
        },
        &config_tl(),
    );
    let Kind::CommitTextReplacingPreedit(commit) = resp.effect[0].kind.as_ref().unwrap() else {
        unreachable!();
    };
    assert_eq!(commit.text, "tāi-uân");
    let Kind::NextWordUpdateLastSelectedWord(nw) = resp.effect[2].kind.as_ref().unwrap() else {
        unreachable!();
    };
    assert_eq!(nw.text, "臺灣");
    assert_eq!(nw.roman, "tsu");

    let Phase::Continuous { committed, .. } = e.snapshot_state().phase else {
        panic!("still Continuous");
    };
    assert_eq!(committed[0].display_text, "tāi-uân");
    assert_eq!(committed[0].canonical_text, "臺灣");
}

#[test]
fn bug1_final_commit_documents_display_but_word_selected_uses_canonical() {
    let mut e = engine_in_continuous("tsu");
    let resp = e.apply(
        Intent::CommitContinuous {
            display_text: "tāi-uân".to_string(),
            canonical_text: "臺灣".to_string(),
            consumed_bytes: 3,
            syllable_count: 1,
        },
        &config_tl(),
    );
    let Kind::CommitTextReplacingPreedit(commit) = resp.effect[0].kind.as_ref().unwrap() else {
        unreachable!();
    };
    assert_eq!(commit.text, "tāi-uân");
    let Kind::NextWordWordSelected(nw) = resp.effect[3].kind.as_ref().unwrap() else {
        unreachable!();
    };
    assert_eq!(nw.text, "臺灣");
    assert!(nw.trigger_prediction);
    assert_eq!(e.snapshot_state().phase, Phase::Idle);
}

#[test]
fn bug1_backspace_pop_correction_uses_canonical_and_display_delete_len() {
    // seg0 multi-char swapped display so the delete count proves it uses
    // display_text length, while the NextWord correction proves canonical.
    let mut e = engine_in_continuous("tsuagua");
    e.apply(
        Intent::CommitContinuous {
            display_text: "tāi-uân".to_string(), // 7 chars
            canonical_text: "臺灣".to_string(),
            consumed_bytes: 3,
            syllable_count: 1,
        },
        &config_tl(),
    );
    e.apply(
        Intent::CommitContinuous {
            display_text: "gí".to_string(), // 2 chars
            canonical_text: "語".to_string(),
            consumed_bytes: 1,
            syllable_count: 1,
        },
        &config_tl(),
    );
    // Drain pending "gua" so the next backspace pops seg1 ("gí").
    e.apply(Intent::DeleteBackward, &config_tl());
    e.apply(Intent::DeleteBackward, &config_tl());
    e.apply(Intent::DeleteBackward, &config_tl());

    let resp = e.apply(Intent::DeleteBackward, &config_tl());
    // seg1.display_text "gí" = 2 chars → 2 DeleteBackwardFromDocument.
    assert_kinds(
        &resp.effect,
        [
            "DeleteBackwardFromDocument",
            "DeleteBackwardFromDocument",
            "NextWordUpdateLastSelectedWord",
            "UpdatePreedit",
            "PerformAutocomplete",
        ],
    );
    // Correction targets seg0 — canonical, not the swapped document string.
    let Kind::NextWordUpdateLastSelectedWord(nw) = resp.effect[2].kind.as_ref().unwrap() else {
        unreachable!();
    };
    assert_eq!(nw.text, "臺灣");
    assert_eq!(nw.roman, "tsu");
}

#[test]
fn bug1_empty_canonical_falls_back_to_display_text() {
    // Backward-compat: legacy callers send no canonical_text → engine uses
    // display_text for both document AND NextWord (pre-Bug-1 behavior).
    let mut e = engine_in_continuous("tsu");
    let resp = e.apply(
        Intent::CommitContinuous {
            display_text: "珠".to_string(),
            canonical_text: String::new(),
            consumed_bytes: 3,
            syllable_count: 1,
        },
        &config_tl(),
    );
    let Kind::NextWordWordSelected(nw) = resp.effect[3].kind.as_ref().unwrap() else {
        unreachable!();
    };
    assert_eq!(nw.text, "珠");
}
