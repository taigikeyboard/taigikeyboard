//! Phase 4 — `Phase::Continuous` integration tests.
//!
//! Phase 4 keeps the new Intents (`EnterContinuous` / `CommitContinuous` /
//! `ResetContinuous`) Rust-only — the proto `oneof method` carrier lands in
//! Phase 6. Tests therefore exercise the engine through the in-process
//! `Engine::apply` API and inspect `Phase::Continuous` internals via
//! `Engine::snapshot_state`. **Model B**: nailed segments are NOT in the
//! document; `ComposingResponse.preedit.display_text` carries the whole
//! composition (Σ nailed display + derived pending tail) while
//! `preedit.raw_input` stays the pending tail only.
//!
//! Each test pins the **exact effect order**, not just membership — the
//! `transition.rs` doc-comment promises proto-ordered effect consumption,
//! so order regressions must surface here.

use composing::{Engine, Intent, NailedSegment, Phase};
use protos::engine::effect::Kind;
use protos::engine::Effect;

mod common;
use common::{config_tl, effect_kinds};

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
    let expected: Vec<&str> = expected.into_iter().collect();
    assert_eq!(effect_kinds(effects), expected, "effect kinds (ordered)");
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
        Phase::Continuous { raw, nailed, .. } => {
            assert_eq!(raw, "tsua");
            assert!(nailed.is_empty());
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
            association_tl: String::new(),
            consumed_bytes: 3,
            syllable_count: 1,
        },
        &config_tl(),
    );

    // Model B: mid-commit emits NO CommitTextReplacingPreedit — it only
    // re-renders the combined marked region (nailed prefix + new pending).
    assert_kinds(
        &resp.effect,
        [
            "UpdatePreedit",
            "NextWordUpdateLastSelectedWord",
            "PerformAutocomplete",
        ],
    );

    let state = e.snapshot_state();
    let Phase::Continuous { raw, nailed, .. } = state.phase else {
        panic!("still in Continuous");
    };
    assert_eq!(raw, "a");
    assert_eq!(nailed.len(), 1);
    assert_eq!(nailed[0].display_text, "珠");
    assert_eq!(nailed[0].raw_text, "tsu");
    assert_eq!(nailed[0].raw_span, (0, 3));
    assert_eq!(nailed[0].syllable_count, 1);

    // Model B: effect[0] is UpdatePreedit carrying the whole composition —
    // the just-nailed "珠" + derived pending "a" = "珠a".
    let preedit = resp.preedit.as_ref().expect("preedit");
    assert_eq!(preedit.display_text, "珠 a");
    assert_eq!(preedit.raw_input, "a");
    let Kind::UpdatePreedit(up) = resp.effect[0].kind.as_ref().unwrap() else {
        unreachable!();
    };
    assert_eq!(up.display, "珠 a");
    let Kind::NextWordUpdateLastSelectedWord(nw) = resp.effect[1].kind.as_ref().unwrap() else {
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
            association_tl: String::new(),
            consumed_bytes: 3,
            syllable_count: 1,
        },
        &config_tl(),
    );
    e.apply(
        Intent::CommitContinuous {
            display_text: "仔".to_string(),
            canonical_text: String::new(),
            association_tl: String::new(),
            consumed_bytes: 1,
            syllable_count: 1,
        },
        &config_tl(),
    );
    let state = e.snapshot_state();
    let Phase::Continuous { nailed, .. } = state.phase else {
        panic!();
    };
    assert_eq!(nailed[0].raw_span, (0, 3));
    assert_eq!(nailed[1].raw_span, (3, 4));
    assert_eq!(nailed[1].raw_text, "a");
}

// ---- Final commit --------------------------------------------------

#[test]
fn final_commit_exits_to_idle_emits_word_selected() {
    let mut e = engine_in_continuous("tsu");
    let resp = e.apply(
        Intent::CommitContinuous {
            display_text: "珠".to_string(),
            canonical_text: String::new(),
            association_tl: String::new(),
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
            association_tl: String::new(),
            consumed_bytes: 99,
            syllable_count: 1,
        },
        &config_tl(),
    );
    assert!(resp.effect.is_empty());
    let Phase::Continuous { raw, nailed, .. } = e.snapshot_state().phase else {
        panic!();
    };
    assert_eq!(raw, "tsua");
    assert!(nailed.is_empty());
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
            association_tl: String::new(),
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
            association_tl: String::new(),
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
            association_tl: String::new(),
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
            association_tl: String::new(),
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
            association_tl: String::new(),
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
            association_tl: String::new(),
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
    let Phase::Continuous { raw, nailed, .. } = e.snapshot_state().phase else {
        panic!();
    };
    assert_eq!(raw, "aguah");
    assert_eq!(nailed.len(), 1);
    assert_eq!(nailed[0].display_text, "珠");
    // Model B: preedit.display_text = whole composition (珠 + derived(aguah));
    // "aguah" has no tone digit / hyphen so derives verbatim. raw_input =
    // pending tail only.
    let preedit = resp.preedit.as_ref().expect("preedit");
    assert_eq!(preedit.raw_input, "aguah");
    assert_eq!(preedit.display_text, "珠 aguah");
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
            association_tl: String::new(),
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
    let Phase::Continuous { raw, nailed, .. } = e.snapshot_state().phase else {
        panic!();
    };
    assert_eq!(raw, "agui");
    assert_eq!(nailed.len(), 1);
    // Model B: combined preedit = 珠 + derived("agui") = "珠agui".
    let preedit = resp.preedit.as_ref().expect("preedit");
    assert_eq!(preedit.raw_input, "agui");
    assert_eq!(preedit.display_text, "珠 agui");
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
fn delete_backward_under_continuous_pops_nailed_when_pending_empty() {
    let mut e = engine_in_continuous("tsua");
    e.apply(
        Intent::CommitContinuous {
            display_text: "珠".to_string(),
            canonical_text: String::new(),
            association_tl: String::new(),
            consumed_bytes: 3,
            syllable_count: 1,
        },
        &config_tl(),
    );
    // pending = "a". Delete pending char first.
    e.apply(Intent::DeleteBackward, &config_tl());
    let Phase::Continuous { raw, nailed, .. } = e.snapshot_state().phase else {
        panic!();
    };
    assert_eq!(raw, "");
    assert_eq!(nailed.len(), 1);

    // Model B: next backspace UNNAILS the last segment, restoring its
    // raw_text ("tsu") as pending. ZERO DeleteBackwardFromDocument (the
    // nailed segment was never in the document). No nailed remains so
    // nextword's last_selected_word must clear (Codex post-impl finding #1),
    // then UpdatePreedit(combined) + PerformAutocomplete.
    let resp = e.apply(Intent::DeleteBackward, &config_tl());
    assert_kinds(
        &resp.effect,
        [
            "NextWordClearForNewComposing",
            "UpdatePreedit",
            "PerformAutocomplete",
        ],
    );
    let Phase::Continuous { raw, nailed, .. } = e.snapshot_state().phase else {
        panic!();
    };
    assert_eq!(raw, "tsu");
    assert!(nailed.is_empty());
    // Combined = (no nailed) + derived("tsu") = "tsu".
    let preedit = resp.preedit.as_ref().expect("preedit");
    assert_eq!(preedit.raw_input, "tsu");
    assert_eq!(preedit.display_text, "tsu");
}

#[test]
fn delete_backward_pop_with_remaining_nailed_emits_nextword_update() {
    // Two nailed segments → pop the latter → nextword's last_selected_word
    // should now correct to the segment behind, not clear.
    let mut e = engine_in_continuous("tsuagua");
    e.apply(
        Intent::CommitContinuous {
            display_text: "珠".to_string(),
            canonical_text: String::new(),
            association_tl: String::new(),
            consumed_bytes: 3,
            syllable_count: 1,
        },
        &config_tl(),
    );
    e.apply(
        Intent::CommitContinuous {
            display_text: "仔".to_string(),
            canonical_text: String::new(),
            association_tl: String::new(),
            consumed_bytes: 1,
            syllable_count: 1,
        },
        &config_tl(),
    );
    // pending = "gua", nailed = ["珠", "仔"].
    // Drain pending so next backspace pops "仔".
    e.apply(Intent::DeleteBackward, &config_tl()); // pending = "gu"
    e.apply(Intent::DeleteBackward, &config_tl()); // pending = "g"
    e.apply(Intent::DeleteBackward, &config_tl()); // pending = ""

    // Model B: unnail "仔" — ZERO DeleteBackwardFromDocument. The remaining
    // nailed "珠" makes the nextword correction an UpdateLastSelectedWord
    // (not a clear), then UpdatePreedit(combined) + PerformAutocomplete.
    let resp = e.apply(Intent::DeleteBackward, &config_tl());
    assert_kinds(
        &resp.effect,
        [
            "NextWordUpdateLastSelectedWord",
            "UpdatePreedit",
            "PerformAutocomplete",
        ],
    );
    // The NextWordUpdateLastSelectedWord payload should reflect "珠" / "tsu",
    // the segment still nailed in the marked region.
    let Kind::NextWordUpdateLastSelectedWord(nw) = resp.effect[0].kind.as_ref().unwrap() else {
        unreachable!();
    };
    assert_eq!(nw.text, "珠");
    assert_eq!(nw.roman, "tsu");

    let Phase::Continuous { raw, nailed, .. } = e.snapshot_state().phase else {
        panic!();
    };
    assert_eq!(raw, "a");
    assert_eq!(nailed.len(), 1);
    assert_eq!(nailed[0].display_text, "珠");
    // Combined = 珠 + derived("a") = "珠a".
    let preedit = resp.preedit.as_ref().expect("preedit");
    assert_eq!(preedit.raw_input, "a");
    assert_eq!(preedit.display_text, "珠 a");
}

#[test]
fn delete_backward_pops_multi_char_display_emits_no_document_deletes() {
    let mut e = engine_in_continuous("tsuagua");
    // Mid-commit a 2-syllable segment (display "珠仔" = 2 chars, raw "tsua" = 4 bytes).
    e.apply(
        Intent::CommitContinuous {
            display_text: "珠仔".to_string(),
            canonical_text: String::new(),
            association_tl: String::new(),
            consumed_bytes: 4,
            syllable_count: 2,
        },
        &config_tl(),
    );
    // Drain pending so next backspace pops the nailed segment.
    e.apply(Intent::DeleteBackward, &config_tl()); // pending = "gu"
    e.apply(Intent::DeleteBackward, &config_tl()); // pending = "g"
    e.apply(Intent::DeleteBackward, &config_tl()); // pending = ""

    let resp = e.apply(Intent::DeleteBackward, &config_tl());
    // Model B: unnail of a 2-char display segment emits ZERO
    // DeleteBackwardFromDocument (it was never in the document), then
    // NextWordClearForNewComposing (nailed list now empty), then
    // UpdatePreedit + PerformAutocomplete.
    assert_kinds(
        &resp.effect,
        [
            "NextWordClearForNewComposing",
            "UpdatePreedit",
            "PerformAutocomplete",
        ],
    );
    let Phase::Continuous { raw, nailed, .. } = e.snapshot_state().phase else {
        panic!();
    };
    assert_eq!(raw, "tsua");
    assert!(nailed.is_empty());
    // Combined = (no nailed) + derived("tsua") = "tsua".
    let preedit = resp.preedit.as_ref().expect("preedit");
    assert_eq!(preedit.raw_input, "tsua");
    assert_eq!(preedit.display_text, "tsua");
}

#[test]
fn delete_backward_under_continuous_with_empty_state_exits_to_idle() {
    // Engineered edge case: enter continuous on a single-char raw, then
    // delete that char with no nailed segments. Model B: the char only ever
    // lived in the marked region (never the document), so the engine exits
    // to Idle and clears the marked region — NO DeleteBackwardFromDocument.
    let mut e = engine_in_continuous("a");
    let resp = e.apply(Intent::DeleteBackward, &config_tl());
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
fn query_state_under_continuous_raw_input_pending_only_display_text_whole_composition() {
    let mut e = engine_in_continuous("tsuagua");
    e.apply(
        Intent::CommitContinuous {
            display_text: "珠".to_string(),
            canonical_text: String::new(),
            association_tl: String::new(),
            consumed_bytes: 3,
            syllable_count: 1,
        },
        &config_tl(),
    );
    let resp = e.apply(Intent::QueryState, &config_tl());
    assert!(resp.effect.is_empty());
    let preedit = resp.preedit.expect("preedit");
    // Model B: raw_input is still the pending tail only (NOT nailed.raw +
    // pending), but display_text is the whole composition: nailed "珠" +
    // derived("agua").
    assert_eq!(preedit.raw_input, "agua");
    assert_eq!(preedit.display_text, "珠 agua");
    assert!(resp.is_composing);
}

// ---- Continuous-phase routing of legacy text-bearing Intents -----
//
// Per Codex post-impl finding #3, the three Intents that carry user text
// (`Start`, `SelectSuggestion`, `CommitPreeditThenInsertExternal`) under
// Continuous must NOT silently drop the text. Under **Model B** they drop
// continuous state and route the text through a sane equivalent: `Start`
// becomes "abort continuous + begin fresh Composing"; `SelectSuggestion`
// becomes "commit Σ nailed.display_text + text"; `CommitPreeditThenInsert
// External` becomes "commit Σ nailed.display_text + pending derived +
// external atomically" (nailed segments were never in the document, so
// they ride the single commit). `CommitDerived` stays snapshot noop —
// Continuous auto-nails via mid-commit. `CommitRaw` (Enter) commits the
// whole composition `Phase::composing_display` (§10.10a Model B; see
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
    let Phase::Composing { raw, .. } = e.snapshot_state().phase else {
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
// the new contract: CommitRaw under Continuous mirrors commit_continuous's
// final-commit shape (4 effects, NextWordWordSelected fires). **Model B**:
// the commit text is the WHOLE composition — Σ nailed[i].display_text +
// derived display of the pending tail — because nailed segments were never
// written to the document; they lived in the marked region. With no prior
// mid-commit, combined == derived(raw) so single-segment Enter is unchanged.

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
fn commit_raw_under_continuous_after_mid_commit_commits_whole_composition() {
    // Setup: enter Continuous on "tsuali2", mid-commit "紙" consuming bytes 0..4
    // ("tsua"), leaving pending = "li2". Model B: "紙" was NOT written to the
    // document (it lived in the marked region), so Enter commits the WHOLE
    // composition = nailed "紙" + derived("li2") = "紙lí".
    let mut e = engine_in_continuous("tsuali2");
    e.apply(
        Intent::CommitContinuous {
            display_text: "紙".to_string(),
            canonical_text: String::new(),
            association_tl: String::new(),
            consumed_bytes: 4,
            syllable_count: 1,
        },
        &config_tl(),
    );
    let Phase::Continuous { raw, nailed, .. } = e.snapshot_state().phase else {
        panic!("expected Continuous after mid-commit");
    };
    assert_eq!(raw, "li2");
    assert_eq!(nailed.len(), 1);

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
    // Model B: whole composition — nailed "紙" + pending tail "li2" → "lí".
    assert_eq!(commit.text, "紙 lí");
    // Terminal NextWordWordSelected is for the pending tail "word".
    let Kind::NextWordWordSelected(nw) = resp.effect[3].kind.as_ref().unwrap() else {
        unreachable!();
    };
    assert_eq!(nw.text, "lí");
    assert_eq!(nw.roman, "li2");
    assert_eq!(e.snapshot_state().phase, Phase::Idle);
}

#[test]
fn commit_raw_under_continuous_with_translate_swapped_unchanged() {
    // F6.A regression: is_translate_swapped is a candidate-display swap that
    // does not influence the inline pre-edit / Enter-raw contract. Enter
    // commits the same derived display whether swap is on or off.
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
    // TPS glyphs are rendered as-is (derived_display short-circuits TPS to
    // the raw bopomofo string, minus separator markers). Enter commits the
    // same string.
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

// §41 — consuming the trailing separator marker along with the glyphs makes
// the commit FINAL: no phantom one-space buffer survives. This pins the
// transition half of the contract (given full consumption, the engine leaves
// Continuous); that the emitted candidate's span actually REACHES raw len is
// pinned end-to-end in `tps_space_pinned_tone.rs` and at the offset-map level
// in `shadow.rs`.
#[test]
fn full_span_commit_consumes_the_trailing_separator_marker() {
    let raw = "ㄒㄧ ";
    let mut e = engine_in_continuous(raw);
    let resp = e.apply(
        Intent::CommitContinuous {
            display_text: "詩".to_string(),
            canonical_text: String::new(),
            association_tl: String::new(),
            consumed_bytes: raw.len(),
            syllable_count: 1,
        },
        &config_tl(),
    );
    // Final commit → the engine leaves Continuous entirely.
    assert_eq!(e.snapshot_state().phase, Phase::Idle);
    assert!(
        resp.effect.iter().any(|ef| matches!(
            ef.kind.as_ref().unwrap(),
            Kind::CommitTextReplacingPreedit(_)
        )),
        "final commit must replace the preedit, got {:?}",
        resp.effect
    );
}

// §41 — the marker must not reach the document from the BARE-`Composing`
// commit path either. Platforms promote to Continuous after every mutation,
// so this arm is off the normal UX path, but the engine's contract cannot
// rest on the phase a caller happens to be in (Codex post-impl BLOCK
// 2026-08-21 — this arm used to commit `raw` verbatim).
#[test]
fn commit_raw_under_bare_composing_tps_hides_the_separator_marker() {
    let mut e = Engine::new();
    e.apply(
        Intent::Start {
            text: "ㄍㄠ ㄉㄞ ".to_string(),
        },
        &config_tl(),
    );
    // NO EnterContinuous — commit straight out of `Phase::Composing`.
    let resp = e.apply(Intent::CommitRaw, &config_tl());
    let Kind::CommitTextReplacingPreedit(commit) = resp.effect[0].kind.as_ref().unwrap() else {
        unreachable!();
    };
    assert_eq!(commit.text, "ㄍㄠㄉㄞ");
}

// §41 — the terminal NextWord effect keys the association on the committed
// word, so its romanization must be marker-free too: a row keyed `ㄍㄠ␣ㄉㄞ`
// would never be reconstructed by a later lookup of `ㄍㄠㄉㄞ`.
#[test]
fn terminal_nextword_roman_drops_the_separator_marker() {
    let mut e = Engine::new();
    e.apply(
        Intent::Start {
            text: "ㄍㄠ ㄉㄞ ".to_string(),
        },
        &config_tl(),
    );
    e.apply(Intent::EnterContinuous, &config_tl());
    let resp = e.apply(Intent::CommitRaw, &config_tl());
    let nw = resp
        .effect
        .iter()
        .find_map(|ef| match ef.kind.as_ref().unwrap() {
            Kind::NextWordWordSelected(nw) => Some(nw),
            _ => None,
        })
        .expect("terminal NextWordWordSelected");
    assert_eq!(nw.text, "ㄍㄠㄉㄞ");
    assert_eq!(nw.roman, "ㄍㄠㄉㄞ");
}

// §41 — the keyboard's tone-1 / boundary space is a marker, not text: it
// stays in the raw buffer (the lattice barrier and the §41 tone pin both
// read it) but must never reach the user — not in the pre-edit, not in what
// Enter commits.
#[test]
fn commit_raw_under_continuous_tps_hides_the_separator_marker() {
    let mut e = Engine::new();
    e.apply(
        Intent::Start {
            text: "ㄍㄠ ㄉㄞ ".to_string(),
        },
        &config_tl(),
    );
    e.apply(Intent::EnterContinuous, &config_tl());
    let resp = e.apply(Intent::CommitRaw, &config_tl());
    let Kind::CommitTextReplacingPreedit(commit) = resp.effect[0].kind.as_ref().unwrap() else {
        unreachable!();
    };
    assert_eq!(
        commit.text, "ㄍㄠㄉㄞ",
        "interior + trailing separator markers must not be committed"
    );
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

// ---- Model B: text-bearing Intents with a NAILED prefix ----------
// Codex post-impl P2: the zero-nailed cases above don't exercise the
// Model-B "Σ nailed.display_text + …" combine. These pin §10.8 #9.

#[test]
fn select_suggestion_under_continuous_with_nailed_prefix_commits_combined() {
    // Nail "珠" (raw "tsu") leaving pending "a", then SelectSuggestion.
    let mut e = engine_in_continuous("tsua");
    e.apply(
        Intent::CommitContinuous {
            display_text: "珠".to_string(),
            canonical_text: String::new(),
            association_tl: String::new(),
            consumed_bytes: 3,
            syllable_count: 1,
        },
        &config_tl(),
    );
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
    // Model B: Σ nailed.display_text ("珠") + text ("紙"). The nailed
    // prefix was never in the document, so it rides this single commit.
    assert_eq!(commit.text, "珠紙");
    assert_eq!(e.snapshot_state().phase, Phase::Idle);
}

#[test]
fn commit_preedit_then_insert_external_with_nailed_prefix_combines_all() {
    // Nail "珠" (raw "tsu") leaving pending "a", then commit-preedit + "!".
    let mut e = engine_in_continuous("tsua");
    e.apply(
        Intent::CommitContinuous {
            display_text: "珠".to_string(),
            canonical_text: String::new(),
            association_tl: String::new(),
            consumed_bytes: 3,
            syllable_count: 1,
        },
        &config_tl(),
    );
    let resp = e.apply(
        Intent::CommitPreeditThenInsertExternal {
            text: "!".to_string(),
        },
        &config_tl(),
    );
    let Kind::CommitTextReplacingPreedit(commit) = resp.effect[0].kind.as_ref().unwrap() else {
        unreachable!();
    };
    // Model B: Σ nailed.display_text ("珠") + derived(pending "a" → "a")
    // + external ("!") = "珠a!".
    assert_eq!(commit.text, "珠 a!");
    assert_eq!(e.snapshot_state().phase, Phase::Idle);
}

#[test]
fn commit_raw_under_continuous_raw_empty_after_unnail_commits_nailed_only() {
    // Nail "珠" (raw "tsu") leaving pending "a", backspace "a" away so
    // raw="" with one nailed segment, then Enter (CommitRaw). Pins the
    // raw-empty-but-nailed `commit_raw_continuous` branch (Codex P2).
    let mut e = engine_in_continuous("tsua");
    e.apply(
        Intent::CommitContinuous {
            display_text: "珠".to_string(),
            canonical_text: String::new(),
            association_tl: String::new(),
            consumed_bytes: 3,
            syllable_count: 1,
        },
        &config_tl(),
    );
    e.apply(Intent::DeleteBackward, &config_tl()); // pending "a" → ""
    let state = e.snapshot_state();
    let Phase::Continuous { raw, nailed, .. } = state.phase else {
        panic!("still Continuous with empty pending + 1 nailed");
    };
    assert_eq!(raw, "");
    assert_eq!(nailed.len(), 1);

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
    // Whole composition with empty tail = just the nailed prefix "珠".
    assert_eq!(commit.text, "珠");
    let Kind::NextWordWordSelected(nw) = resp.effect[3].kind.as_ref().unwrap() else {
        unreachable!();
    };
    // raw empty → terminal NextWord uses the last nailed segment's
    // canonical key + raw_text (canonical falls back to display_text "珠"
    // because the mid-commit passed empty canonical_text).
    assert_eq!(nw.text, "珠");
    assert_eq!(nw.roman, "tsu");
    assert!(nw.trigger_prediction);
    assert_eq!(e.snapshot_state().phase, Phase::Idle);
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
    // not transition to Continuous { raw: "", nailed: [] }.
    let resp = e.apply(Intent::EnterContinuous, &config_tl());
    assert!(resp.effect.is_empty());
    assert!(matches!(
        e.snapshot_state().phase,
        Phase::Composing { .. } | Phase::Idle
    ));
}

#[test]
fn replace_last_to_empty_pending_no_nailed_exits_to_idle() {
    // Engineered: Continuous with single-char pending and no nailed segments.
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

// ---- NailedSegment public surface ------------------------------

#[test]
fn nailed_segment_public_fields_round_trip() {
    let seg = NailedSegment {
        display_text: "珠仔".to_string(),
        canonical_text: "珠仔".to_string(),
        raw_text: "tsua".to_string(),
        association_tl: "tsu-á".to_string(),
        raw_span: (0, 4),
        syllable_count: 2,
    };
    assert_eq!(seg.display_text, "珠仔");
    assert_eq!(seg.canonical_text, "珠仔");
    assert_eq!(seg.raw_text, "tsua");
    assert_eq!(seg.association_tl, "tsu-á");
    assert_eq!(seg.raw_span, (0, 4));
    assert_eq!(seg.syllable_count, 2);
}

// ---- v3.5.8 Phase 9 Bug 1 (Option A) — swap-aware commit, Model B ----
// `display_text` = swap/TPS/both-scripts marked-region string; `canonical_text`
// = canonical key for NextWord. Empty canonical → falls back to display
// (legacy callers). These pin: NailedSegment.display_text holds the swapped
// display; NextWord uses `canonical`. Model B: a mid-commit emits NO
// CommitTextReplacingPreedit; the swapped display rides the eventual hard
// finalize's whole-composition write.

#[test]
fn bug1_mid_commit_marks_display_but_nextword_uses_canonical() {
    let mut e = engine_in_continuous("tsua");
    let resp = e.apply(
        Intent::CommitContinuous {
            display_text: "tāi-uân".to_string(), // swapped roman → marked region
            canonical_text: "臺灣".to_string(),  // canonical hanji → NextWord text
            association_tl: "tâi-uân".to_string(), // R2 canonical TL → NextWord roman
            consumed_bytes: 3,
            syllable_count: 1,
        },
        &config_tl(),
    );
    // Model B mid-commit: [UpdatePreedit(combined), NextWordUpdateLastSelectedWord,
    // PerformAutocomplete] — NO CommitTextReplacingPreedit.
    assert_kinds(
        &resp.effect,
        [
            "UpdatePreedit",
            "NextWordUpdateLastSelectedWord",
            "PerformAutocomplete",
        ],
    );
    let Kind::UpdatePreedit(up) = resp.effect[0].kind.as_ref().unwrap() else {
        unreachable!();
    };
    // Combined = swapped "tāi-uân" + derived("a") = "tāi-uâna".
    assert_eq!(up.display, "tāi-uân a");
    let Kind::NextWordUpdateLastSelectedWord(nw) = resp.effect[1].kind.as_ref().unwrap() else {
        unreachable!();
    };
    assert_eq!(nw.text, "臺灣");
    // R2: NextWord roman = the candidate's canonical TL (association_tl),
    // NOT the raw typed slice "tsu". This is the write-side fix that keeps
    // continuous-input `prev_tl`/`next_tl` aligned with a normal commit.
    assert_eq!(nw.roman, "tâi-uân");

    let Phase::Continuous { nailed, .. } = e.snapshot_state().phase else {
        panic!("still Continuous");
    };
    assert_eq!(nailed[0].display_text, "tāi-uân");
    assert_eq!(nailed[0].canonical_text, "臺灣");
    assert_eq!(nailed[0].association_tl, "tâi-uân");
}

#[test]
fn bug1_final_commit_documents_display_but_word_selected_uses_canonical() {
    let mut e = engine_in_continuous("tsu");
    let resp = e.apply(
        Intent::CommitContinuous {
            display_text: "tāi-uân".to_string(),
            canonical_text: "臺灣".to_string(),
            association_tl: "tâi-uân".to_string(),
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
    // R2: terminal WordSelected roman = canonical TL, not raw "tsu".
    assert_eq!(nw.roman, "tâi-uân");
    assert!(nw.trigger_prediction);
    assert_eq!(e.snapshot_state().phase, Phase::Idle);
}

#[test]
fn bug1_backspace_pop_correction_uses_canonical_no_document_delete() {
    // seg0 multi-char swapped display. Model B: unnailing seg1 emits ZERO
    // DeleteBackwardFromDocument (nailed segments were never in the
    // document); the NextWord correction still targets seg0's canonical.
    let mut e = engine_in_continuous("tsuagua");
    e.apply(
        Intent::CommitContinuous {
            display_text: "tāi-uân".to_string(), // 7 chars
            canonical_text: "臺灣".to_string(),
            association_tl: "tâi-uân".to_string(),
            consumed_bytes: 3,
            syllable_count: 1,
        },
        &config_tl(),
    );
    e.apply(
        Intent::CommitContinuous {
            display_text: "gí".to_string(), // 2 chars
            canonical_text: "語".to_string(),
            association_tl: "gí".to_string(),
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
    // Model B: unnail of seg1 — ZERO DeleteBackwardFromDocument, then the
    // nextword correction (seg0 still nailed) + UpdatePreedit + autocomplete.
    assert_kinds(
        &resp.effect,
        [
            "NextWordUpdateLastSelectedWord",
            "UpdatePreedit",
            "PerformAutocomplete",
        ],
    );
    // Correction targets seg0 — canonical, not the swapped display string.
    let Kind::NextWordUpdateLastSelectedWord(nw) = resp.effect[0].kind.as_ref().unwrap() else {
        unreachable!();
    };
    assert_eq!(nw.text, "臺灣");
    // R2: the backspace correction re-establishes seg0's context with its
    // canonical TL (association_tl), not the raw slice "tsu".
    assert_eq!(nw.roman, "tâi-uân");
    // Unnailing seg1 restores its raw_text "a" (consumed_bytes=1 from
    // pending "agua") as the new pending; combined = seg0 display "tāi-uân"
    // + derived("a") = "tāi-uâna".
    let preedit = resp.preedit.as_ref().expect("preedit");
    assert_eq!(preedit.raw_input, "a");
    assert_eq!(preedit.display_text, "tāi-uân a");
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
            association_tl: String::new(),
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
