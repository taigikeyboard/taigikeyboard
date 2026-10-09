//! Phase 4 — `Phase::Continuous` integration tests.
//!
//! Tests exercise the engine through the in-process `Engine::apply` API and
//! inspect `Phase::Continuous` internals via `Engine::snapshot_state`. **Model B**: nailed segments are NOT in the
//! document; `ComposingResponse.preedit.display_text` carries the whole
//! composition (Σ nailed display + derived pending tail) while
//! `preedit.raw_input` stays the pending tail only.
//!
//! Each test pins the **exact effect order**, not just membership — the
//! `transition.rs` doc-comment promises proto-ordered effect consumption,
//! so order regressions must surface here.

use composing::CommitScript;
use composing::{Engine, Intent, NailedSegment, Phase};
use protos::engine::effect::Kind;
use protos::engine::Effect;

use crate::common::{config, config_tl, effect_kinds, engine_in_continuous};

fn assert_kinds<'a, K>(effects: &'a [Effect], expected: K)
where
    K: IntoIterator<Item = &'a str>,
{
    let expected: Vec<&str> = expected.into_iter().collect();
    assert_eq!(effect_kinds(effects), expected, "effect kinds (ordered)");
}

// ---- Enter ---------------------------------------------------------

#[test]
fn start_lands_in_continuous_with_nothing_nailed() {
    // R12: the first keystroke already composes in Continuous; there is no
    // single-segment phase for a platform to promote out of.
    let mut e = Engine::new();
    let resp = e.apply(
        Intent::Start {
            text: "tsua".to_string(),
        },
        &config_tl(),
    );
    assert_kinds(&resp.effect, ["UpdatePreedit", "RefreshCandidates"]);
    assert!(resp.is_composing);
    match e.snapshot_state().phase {
        Phase::Continuous {
            raw, caret, nailed, ..
        } => {
            assert_eq!(raw, "tsua");
            assert_eq!(caret, 4);
            assert!(nailed.is_empty());
        }
        other => panic!("expected Continuous, got {other:?}"),
    }
}

// ---- Mid-commit ----------------------------------------------------

#[test]
fn mid_commit_pushes_segment_and_emits_ordered_effects() {
    let mut e = engine_in_continuous("tsua");
    let resp = e.apply(
        Intent::CommitContinuous {
            canonical_text: "珠".to_string(),
            association_tl: String::new(),
            hanji: None,
            consumed_bytes: 3,
            syllable_count: 1,
            script: Some(CommitScript::Roman),
            roman: "珠".to_string(),
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
            "RefreshCandidates",
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
            canonical_text: "珠".to_string(),
            association_tl: String::new(),
            hanji: None,
            consumed_bytes: 3,
            syllable_count: 1,
            script: Some(CommitScript::Roman),
            roman: "珠".to_string(),
        },
        &config_tl(),
    );
    e.apply(
        Intent::CommitContinuous {
            canonical_text: "仔".to_string(),
            association_tl: String::new(),
            hanji: None,
            consumed_bytes: 1,
            syllable_count: 1,
            script: Some(CommitScript::Roman),
            roman: "仔".to_string(),
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
            canonical_text: "珠".to_string(),
            association_tl: String::new(),
            hanji: None,
            consumed_bytes: 3,
            syllable_count: 1,
            script: Some(CommitScript::Roman),
            roman: "珠".to_string(),
        },
        &config_tl(),
    );
    assert_kinds(
        &resp.effect,
        [
            "CommitTextReplacingPreedit",
            "ClearCandidates",
            "ResetCandidateContext",
            "NextWordWordSelected",
        ],
    );
    assert!(!resp.is_composing);
    assert_eq!(e.snapshot_state().phase, Phase::Idle);

    let Kind::NextWordWordSelected(nw) = resp.effect[3].kind.as_ref().unwrap() else {
        unreachable!();
    };
    assert_eq!(nw.text, "珠");
    assert_eq!(nw.roman, "tsu");
    assert!(nw.trigger_prediction);
    assert!(nw.preceding.is_empty(), "no nailed segment before 珠");
}

// INVARIANT_NEXTWORD_COMMIT_SEQUENCE_LEARNING (behavioral-invariants §40): the
// final commit carries every nailed segment before the terminal word, in
// document order, keyed canonically (canonical text + association TL, raw
// slice as fallback) — the same identity the terminal word uses.
#[test]
fn final_commit_carries_nailed_segments_as_preceding() {
    let mut e = engine_in_continuous("guabehtsiah");
    for (display, canonical, association_tl, consumed) in
        [("我", "我", "guá", 3), ("欲", "欲", "", 3)]
    {
        e.apply(
            Intent::CommitContinuous {
                canonical_text: canonical.to_string(),
                association_tl: association_tl.to_string(),
                hanji: None,
                consumed_bytes: consumed,
                syllable_count: 1,
                script: Some(CommitScript::Roman),
                roman: display.to_string(),
            },
            &config_tl(),
        );
    }
    let resp = e.apply(
        Intent::CommitContinuous {
            canonical_text: "食".to_string(),
            association_tl: "tsia̍h".to_string(),
            hanji: None,
            consumed_bytes: 5,
            syllable_count: 1,
            script: Some(CommitScript::Roman),
            roman: "食".to_string(),
        },
        &config_tl(),
    );
    let Some(Kind::NextWordWordSelected(nw)) = resp.effect.last().and_then(|e| e.kind.as_ref())
    else {
        panic!(
            "final commit ends with NextWordWordSelected, got {:?}",
            resp.effect
        );
    };
    assert_eq!((nw.text.as_str(), nw.roman.as_str()), ("食", "tsia̍h"));
    let preceding: Vec<(&str, &str)> = nw
        .preceding
        .iter()
        .map(|w| (w.text.as_str(), w.roman.as_str()))
        .collect();
    assert_eq!(preceding, vec![("我", "guá"), ("欲", "beh")]);
}

// ---- CommitContinuous validation ----------------------------------

#[test]
fn commit_continuous_out_of_range_is_noop() {
    let mut e = engine_in_continuous("tsua");
    let resp = e.apply(
        Intent::CommitContinuous {
            canonical_text: "X".to_string(),
            association_tl: String::new(),
            hanji: None,
            consumed_bytes: 99,
            syllable_count: 1,
            script: Some(CommitScript::Roman),
            roman: "X".to_string(),
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
    let resp = e.apply(
        Intent::CommitContinuous {
            canonical_text: "X".to_string(),
            association_tl: String::new(),
            hanji: None,
            consumed_bytes: 1, // mid-codepoint
            syllable_count: 1,
            script: Some(CommitScript::Roman),
            roman: "X".to_string(),
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
            canonical_text: "X".to_string(),
            association_tl: String::new(),
            hanji: None,
            consumed_bytes: 0,
            syllable_count: 1,
            script: Some(CommitScript::Roman),
            roman: "X".to_string(),
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
            canonical_text: String::new(),
            association_tl: String::new(),
            hanji: None,
            consumed_bytes: 3,
            syllable_count: 1,
            script: Some(CommitScript::Roman),
            roman: String::new(),
        },
        &config_tl(),
    );
    assert!(resp.effect.is_empty());
}

// ---- Reset under Continuous ---------------------------------------

#[test]
fn reset_after_a_nail_discards_the_whole_composition() {
    // Model B (Codex risk (ii)): the nailed segment never reached the
    // document, so the abort trio clears the whole marked region and the
    // nailed state goes with it — nothing is written.
    let mut e = engine_in_continuous("tsuatsua");
    e.apply(
        Intent::CommitContinuous {
            canonical_text: "珠".to_string(),
            association_tl: String::new(),
            hanji: None,
            consumed_bytes: 3,
            syllable_count: 1,
            script: Some(CommitScript::Roman),
            roman: "珠".to_string(),
        },
        &config_tl(),
    );
    let resp = e.apply(Intent::Reset, &config_tl());
    assert_kinds(
        &resp.effect,
        [
            "ClearPreeditWithoutCommit",
            "ClearCandidates",
            "NextWordClearForNewComposing",
        ],
    );
    assert_eq!(e.snapshot_state().phase, Phase::Idle);
}

#[test]
fn reset_under_continuous_emits_the_abort_trio() {
    let mut e = engine_in_continuous("tsua");
    let resp = e.apply(Intent::Reset, &config_tl());
    assert_kinds(
        &resp.effect,
        [
            "ClearPreeditWithoutCommit",
            "ClearCandidates",
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
            canonical_text: "珠".to_string(),
            association_tl: String::new(),
            hanji: None,
            consumed_bytes: 3,
            syllable_count: 1,
            script: Some(CommitScript::Roman),
            roman: "珠".to_string(),
        },
        &config_tl(),
    );
    // After mid-commit, raw="" — but the seed used "tsu" so pending is empty
    // here. Re-prime with a non-final mid-commit using a longer raw.
    let mut e = engine_in_continuous("tsuagua");
    e.apply(
        Intent::CommitContinuous {
            canonical_text: "珠".to_string(),
            association_tl: String::new(),
            hanji: None,
            consumed_bytes: 3,
            syllable_count: 1,
            script: Some(CommitScript::Roman),
            roman: "珠".to_string(),
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
    assert_kinds(&resp.effect, ["UpdatePreedit", "RefreshCandidates"]);
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
    assert_kinds(&resp.effect, ["UpdatePreedit", "RefreshCandidates"]);
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
            canonical_text: "珠".to_string(),
            association_tl: String::new(),
            hanji: None,
            consumed_bytes: 3,
            syllable_count: 1,
            script: Some(CommitScript::Roman),
            roman: "珠".to_string(),
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
    assert_kinds(&resp.effect, ["UpdatePreedit", "RefreshCandidates"]);
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
    assert_kinds(&resp.effect, ["UpdatePreedit", "RefreshCandidates"]);
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
            canonical_text: "珠".to_string(),
            association_tl: String::new(),
            hanji: None,
            consumed_bytes: 3,
            syllable_count: 1,
            script: Some(CommitScript::Roman),
            roman: "珠".to_string(),
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
    // then UpdatePreedit(combined) + RefreshCandidates.
    let resp = e.apply(Intent::DeleteBackward, &config_tl());
    assert_kinds(
        &resp.effect,
        [
            "NextWordClearForNewComposing",
            "UpdatePreedit",
            "RefreshCandidates",
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
            canonical_text: "珠".to_string(),
            association_tl: String::new(),
            hanji: None,
            consumed_bytes: 3,
            syllable_count: 1,
            script: Some(CommitScript::Roman),
            roman: "珠".to_string(),
        },
        &config_tl(),
    );
    e.apply(
        Intent::CommitContinuous {
            canonical_text: "仔".to_string(),
            association_tl: String::new(),
            hanji: None,
            consumed_bytes: 1,
            syllable_count: 1,
            script: Some(CommitScript::Roman),
            roman: "仔".to_string(),
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
    // (not a clear), then UpdatePreedit(combined) + RefreshCandidates.
    let resp = e.apply(Intent::DeleteBackward, &config_tl());
    assert_kinds(
        &resp.effect,
        [
            "NextWordUpdateLastSelectedWord",
            "UpdatePreedit",
            "RefreshCandidates",
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
            canonical_text: "珠仔".to_string(),
            association_tl: String::new(),
            hanji: None,
            consumed_bytes: 4,
            syllable_count: 2,
            script: Some(CommitScript::Roman),
            roman: "珠仔".to_string(),
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
    // UpdatePreedit + RefreshCandidates.
    assert_kinds(
        &resp.effect,
        [
            "NextWordClearForNewComposing",
            "UpdatePreedit",
            "RefreshCandidates",
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
            "ClearCandidates",
            "NextWordClearForNewComposing",
        ],
    );
    assert_eq!(e.snapshot_state().phase, Phase::Idle);
}

// ---- Snapshot under Continuous -----

#[test]
fn snapshot_under_continuous_raw_input_pending_only_display_text_whole_composition() {
    let mut e = engine_in_continuous("tsuagua");
    e.apply(
        Intent::CommitContinuous {
            canonical_text: "珠".to_string(),
            association_tl: String::new(),
            hanji: None,
            consumed_bytes: 3,
            syllable_count: 1,
            script: Some(CommitScript::Roman),
            roman: "珠".to_string(),
        },
        &config_tl(),
    );
    let resp = e.snapshot(&config_tl());
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
// Per Codex post-impl finding #3, the two Intents that carry user text
// (`Start`, `CommitPreeditThenInsertExternal`) under
// Continuous must NOT silently drop the text. Under **Model B** they drop
// continuous state and route the text through a sane equivalent: `Start`
// becomes "abort continuous + begin fresh Composing"; `CommitPreeditThenInsert
// External` becomes "commit Σ nailed.display_text + pending derived +
// external atomically" (nailed segments were never in the document, so
// they ride the single commit). `CommitRaw` (Enter) commits the
// whole composition `Phase::composing_display` (§10.10a Model B; see
// `commit_raw_under_continuous_*` tests below and
// `docs/engine/continuous-commit-and-display.md` §10.3).

#[test]
fn start_under_continuous_aborts_then_begins_fresh_composition() {
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
            "ClearCandidates",
            "NextWordClearForNewComposing",
            "UpdatePreedit",
            "RefreshCandidates",
        ],
    );
    let Phase::Continuous { raw, nailed, .. } = e.snapshot_state().phase else {
        panic!("Start under Continuous must begin a fresh composition");
    };
    assert_eq!(raw, "abc");
    assert!(nailed.is_empty());
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
    // with roman = canonical TL of the raw tail and trigger_prediction = true.
    // trace: canonical_tl_form("li2", Tl) — tone digit 2 → TL acute → "lí".
    let mut e = engine_in_continuous("li2");
    let resp = e.apply(Intent::CommitRaw, &config_tl());
    assert_kinds(
        &resp.effect,
        [
            "CommitTextReplacingPreedit",
            "ClearCandidates",
            "ResetCandidateContext",
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
    assert_eq!(nw.roman, "lí");
    assert!(nw.trigger_prediction);
    assert_eq!(e.snapshot_state().phase, Phase::Idle);
}

// Enter commits the permissive rendering but learns the tail under the same
// key a literal-candidate pick learns (behavioral-invariants §34), so a
// hidden tone boundary does not fork the association record.
#[test]
fn commit_raw_learns_the_literal_key_not_the_permissive_rendering() {
    // trace: tai5gi2 — render tâigí; learning text keeps `tai5gi2` (old
    // to_tone_marks: one chunk, not a single syllable → verbatim).
    //        tai1 — render tai; learning text `tai1` (tone 1 digit kept).
    //        li2 — valid syllable, render == learning text == lí.
    for (raw, committed, learned) in [
        ("tai5gi2", "tâigí", "tai5gi2"),
        ("tai1", "tai", "tai1"),
        ("li2", "lí", "lí"),
    ] {
        let mut e = engine_in_continuous(raw);
        let resp = e.apply(Intent::CommitRaw, &config_tl());
        let Kind::CommitTextReplacingPreedit(commit) = resp.effect[0].kind.as_ref().unwrap() else {
            unreachable!();
        };
        assert_eq!(commit.text, committed, "{raw}");
        let nw = resp
            .effect
            .iter()
            .find_map(|effect| match effect.kind.as_ref() {
                Some(Kind::NextWordWordSelected(nw)) => Some(nw),
                _ => None,
            })
            .expect("Enter learns the tail");
        assert_eq!(nw.text, learned, "{raw}");
    }
}

// The Enter tail is the one NextWord reading the engine itself derives from
// typed text, so it is put in canonical TL form before NextWord learns it
// verbatim: TL keeps its special finals `eng` / `ek` (§3.2.6), POJ folds to
// TL (POJ `eng` IS TL `ing`), TPS and English pass through.
#[test]
fn commit_raw_tail_reading_is_canonical_tl() {
    // trace: canonical_tl_form(tail, mode) —
    //   tl  "keng"    keep_tl_finals, tone 1 → "keng" (a full fold gave "king")
    //   tl  "tek"     keep_tl_finals, stop → tone 4 unmarked → "tek"
    //   poj "cheng"   ch→ts, eng→ing, tone 1 → "tsing"
    //   poj "chiah"   ch→ts, stop → tone 4 unmarked → "tsiah"
    //   english "chat" identity (a full fold gave "tsat")
    for (mode, raw, reading) in [
        ("tl", "keng", "keng"),
        ("tl", "tek", "tek"),
        ("poj", "cheng", "tsing"),
        ("poj", "chiah", "tsiah"),
        ("english", "chat", "chat"),
    ] {
        let mode_config = config(mode);
        let mut e = Engine::new();
        e.apply(
            Intent::Start {
                text: raw.to_string(),
            },
            &mode_config,
        );
        let resp = e.apply(Intent::CommitRaw, &mode_config);
        let nw = resp
            .effect
            .iter()
            .find_map(|effect| match effect.kind.as_ref() {
                Some(Kind::NextWordWordSelected(nw)) => Some(nw),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{mode} {raw:?}: no NextWordWordSelected"));
        assert_eq!(nw.roman, reading, "{mode} {raw:?}");
    }
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
            canonical_text: "紙".to_string(),
            association_tl: String::new(),
            hanji: None,
            consumed_bytes: 4,
            syllable_count: 1,
            script: Some(CommitScript::Roman),
            roman: "紙".to_string(),
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
            "ClearCandidates",
            "ResetCandidateContext",
            "NextWordWordSelected",
        ],
    );
    let Kind::CommitTextReplacingPreedit(commit) = resp.effect[0].kind.as_ref().unwrap() else {
        unreachable!();
    };
    // Model B: whole composition — nailed "紙" + pending tail "li2" → "lí".
    assert_eq!(commit.text, "紙 lí");
    // Terminal NextWordWordSelected is for the pending tail "word"; the
    // nailed 紙 precedes it (raw slice — no association TL was sent).
    let Kind::NextWordWordSelected(nw) = resp.effect[3].kind.as_ref().unwrap() else {
        unreachable!();
    };
    assert_eq!(nw.text, "lí");
    assert_eq!(nw.roman, "lí");
    let preceding: Vec<(&str, &str)> = nw
        .preceding
        .iter()
        .map(|w| (w.text.as_str(), w.roman.as_str()))
        .collect();
    assert_eq!(preceding, vec![("紙", "tsua")]);
    assert_eq!(e.snapshot_state().phase, Phase::Idle);
}

#[test]
fn commit_raw_under_continuous_with_translate_swapped_unchanged() {
    // F6.A regression: is_hanji_first is a candidate-display swap that
    // does not influence the inline pre-edit / Enter-raw contract. Enter
    // commits the same derived display whether swap is on or off.
    let mut config = config_tl();
    config.is_hanji_first = true;
    let mut e = Engine::new();
    e.apply(
        Intent::Start {
            text: "li2".to_string(),
        },
        &config,
    );
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
            canonical_text: "詩".to_string(),
            association_tl: String::new(),
            hanji: None,
            consumed_bytes: raw.len(),
            syllable_count: 1,
            script: Some(CommitScript::Roman),
            roman: "詩".to_string(),
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
    let resp = e.apply(Intent::CommitRaw, &config_tl());
    let Kind::CommitTextReplacingPreedit(commit) = resp.effect[0].kind.as_ref().unwrap() else {
        unreachable!();
    };
    assert_eq!(
        commit.text, "ㄍㄠㄉㄞ",
        "interior + trailing separator markers must not be committed"
    );
}

// INVARIANT_COMPOSING_EXTERNAL_INSERT_COMMITS_PREEDIT_ATOMICALLY (behavioral-invariants.md §13)
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
            "ClearCandidates",
            "ResetCandidateContext",
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
fn commit_preedit_then_insert_external_with_nailed_prefix_combines_all() {
    // Nail "珠" (raw "tsu") leaving pending "a", then commit-preedit + "!".
    let mut e = engine_in_continuous("tsua");
    e.apply(
        Intent::CommitContinuous {
            canonical_text: "珠".to_string(),
            association_tl: String::new(),
            hanji: None,
            consumed_bytes: 3,
            syllable_count: 1,
            script: Some(CommitScript::Roman),
            roman: "珠".to_string(),
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
            canonical_text: "珠".to_string(),
            association_tl: String::new(),
            hanji: None,
            consumed_bytes: 3,
            syllable_count: 1,
            script: Some(CommitScript::Roman),
            roman: "珠".to_string(),
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
            "ClearCandidates",
            "ResetCandidateContext",
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

// INVARIANT_NEXTWORD_COMMIT_SEQUENCE_LEARNING (behavioral-invariants §40):
// Enter with an empty tail makes the LAST nailed segment the terminal word
// and every earlier one `preceding` — none dropped, none sent twice.
#[test]
fn commit_raw_with_empty_tail_carries_earlier_nails_as_preceding() {
    let mut e = engine_in_continuous("tsuguaa");
    for (display, consumed) in [("珠", 3), ("我", 3)] {
        e.apply(
            Intent::CommitContinuous {
                canonical_text: display.to_string(),
                association_tl: String::new(),
                hanji: None,
                consumed_bytes: consumed,
                syllable_count: 1,
                script: Some(CommitScript::Roman),
                roman: display.to_string(),
            },
            &config_tl(),
        );
    }
    e.apply(Intent::DeleteBackward, &config_tl()); // pending "a" → ""
    let Phase::Continuous { raw, nailed, .. } = e.snapshot_state().phase else {
        panic!("still Continuous with empty pending + 2 nailed");
    };
    assert_eq!((raw.as_str(), nailed.len()), ("", 2));

    let resp = e.apply(Intent::CommitRaw, &config_tl());
    let Some(Kind::NextWordWordSelected(nw)) = resp.effect.last().and_then(|e| e.kind.as_ref())
    else {
        panic!(
            "Enter ends with NextWordWordSelected, got {:?}",
            resp.effect
        );
    };
    assert_eq!((nw.text.as_str(), nw.roman.as_str()), ("我", "gua"));
    let preceding: Vec<(&str, &str)> = nw
        .preceding
        .iter()
        .map(|w| (w.text.as_str(), w.roman.as_str()))
        .collect();
    assert_eq!(preceding, vec![("珠", "tsu")]);
}

// ---- Empty-Continuous invariant guards (Codex finding #2) --------

#[test]
fn empty_start_stays_idle() {
    // A composition is never empty in both pending and nailed: an empty
    // Start from Idle is a no-op, not a degenerate Continuous { raw: "" }.
    let mut e = Engine::new();
    let resp = e.apply(
        Intent::Start {
            text: String::new(),
        },
        &config_tl(),
    );
    assert!(resp.effect.is_empty());
    assert!(!resp.is_composing);
    assert_eq!(e.snapshot_state().phase, Phase::Idle);
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
            "ClearCandidates",
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
        hanji: None,
        raw_span: (0, 4),
        syllable_count: 2,
        is_picked: true,
    };
    assert_eq!(seg.display_text, "珠仔");
    assert_eq!(seg.canonical_text, "珠仔");
    assert_eq!(seg.raw_text, "tsua");
    assert_eq!(seg.association_tl, "tsu-á");
    assert_eq!(seg.raw_span, (0, 4));
    assert_eq!(seg.syllable_count, 2);
    assert!(seg.is_picked);
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
            canonical_text: "臺灣".to_string(), // canonical hanji → NextWord text
            association_tl: "tâi-uân".to_string(), // R2 canonical TL → NextWord roman
            hanji: None,
            consumed_bytes: 3,
            syllable_count: 1,
            script: Some(CommitScript::Roman),
            roman: "tāi-uân".to_string(),
        },
        &config_tl(),
    );
    // Model B mid-commit: [UpdatePreedit(combined), NextWordUpdateLastSelectedWord,
    // RefreshCandidates] — NO CommitTextReplacingPreedit.
    assert_kinds(
        &resp.effect,
        [
            "UpdatePreedit",
            "NextWordUpdateLastSelectedWord",
            "RefreshCandidates",
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

// INVARIANT_NEXTWORD_CONTINUOUS_CANONICAL_TL (behavioral-invariants.md §25)
#[test]
fn bug1_final_commit_documents_display_but_word_selected_uses_canonical() {
    let mut e = engine_in_continuous("tsu");
    let resp = e.apply(
        Intent::CommitContinuous {
            canonical_text: "臺灣".to_string(),
            association_tl: "tâi-uân".to_string(),
            hanji: None,
            consumed_bytes: 3,
            syllable_count: 1,
            script: Some(CommitScript::Roman),
            roman: "tāi-uân".to_string(),
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
            canonical_text: "臺灣".to_string(),
            association_tl: "tâi-uân".to_string(),
            hanji: None,
            consumed_bytes: 3,
            syllable_count: 1,
            script: Some(CommitScript::Roman),
            roman: "tāi-uân".to_string(),
        },
        &config_tl(),
    );
    e.apply(
        Intent::CommitContinuous {
            canonical_text: "語".to_string(),
            association_tl: "gí".to_string(),
            hanji: None,
            consumed_bytes: 1,
            syllable_count: 1,
            script: Some(CommitScript::Roman),
            roman: "gí".to_string(),
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
            "RefreshCandidates",
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
            canonical_text: "珠".to_string(),
            association_tl: String::new(),
            hanji: None,
            consumed_bytes: 3,
            syllable_count: 1,
            script: Some(CommitScript::Roman),
            roman: "珠".to_string(),
        },
        &config_tl(),
    );
    let Kind::NextWordWordSelected(nw) = resp.effect[3].kind.as_ref().unwrap() else {
        unreachable!();
    };
    assert_eq!(nw.text, "珠");
}
