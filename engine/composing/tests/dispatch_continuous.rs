//! v3.5.8 Continuous Input Phase 6 — dispatch decoding + degraded-path tests.
//!
//! Covers:
//! - Decoding `FetchAtPos` / `CommitContinuous` from `ComposingRequest.method`
//!   oneof variants.
//! - `Intent::FetchAtPos` short-circuit behavior in `requests::handle`:
//!   - Idle → `continuous = None` snapshot.
//!   - right after the first keystroke → carrier.
//!   - `Phase::Continuous` + lexicon NOT installed → `continuous =
//!     Some(empty)`.
//! - `CommitContinuous` / `Reset` answers
//!   carry no `continuous` carrier (their transitions are pinned on
//!   `Engine::apply` in `continuous_phase.rs`).
//!
//! The full TL/TPS lexicon-backed FetchAtPos integration (with hermetic
//! `dictionary.fst` + `dictionary.bin` + `syllables.fst`) lives next to
//! the lexicon parity suite — composing-side tests stay focused on the
//! dispatch wiring + decode contract.

use composing::api::{Engine, Phase};
use composing::requests;
use protos::engine::composing_request::Method;
use protos::engine::CommitScript;
use protos::engine::{CommitContinuous, FetchAtPos};

use crate::common::{config_tl, req};

#[test]
fn permissive_tones_keep_preview_candidate_and_all_commits_identical() {
    use protos::engine::{effect::Kind, AppConfig, Start};
    for mode in ["tl", "poj"] {
        for (raw, expected) in [
            ("tai5gi2", "tâigí"),
            ("tai1", "tai"),
            ("bak4", "bak"),
            ("tai1bak4", "taibak"),
        ] {
            let config = AppConfig {
                input_mode: mode.into(),
                ..config_tl()
            };
            for method in [
                Method::CommitAsTyped(protos::engine::CommitAsTyped {}),
                Method::CommitAsShown(protos::engine::CommitAsShown {}),
            ] {
                let mut engine = Engine::new();
                let preview = requests::handle(
                    &req(Method::Start(Start { text: raw.into() })),
                    &mut engine,
                    &config,
                )
                .unwrap();
                assert_eq!(preview.preedit.unwrap().display_text, expected);
                let fetched = requests::handle(
                    &req(Method::FetchAtPos(FetchAtPos::default())),
                    &mut engine,
                    &config,
                )
                .unwrap();
                let candidate = &fetched.continuous.unwrap().candidates[0];
                assert_eq!(candidate.roman, expected);
                let commit = requests::handle(&req(method), &mut engine, &config).unwrap();
                assert!(commit.effect.iter().any(|effect| matches!(
                    &effect.kind,
                    Some(Kind::CommitTextReplacingPreedit(text)) if text.text == expected
                )));
            }
            let mut engine = Engine::new();
            requests::handle(
                &req(Method::Start(Start { text: raw.into() })),
                &mut engine,
                &config,
            )
            .unwrap();
            let fetched = requests::handle(
                &req(Method::FetchAtPos(FetchAtPos::default())),
                &mut engine,
                &config,
            )
            .unwrap();
            let candidate = &fetched.continuous.unwrap().candidates[0];
            let commit = requests::handle(
                &req(Method::CommitContinuous(CommitContinuous {
                    canonical_text: candidate.display_text.clone(),
                    association_tl: candidate.canonical_tl.clone(),
                    consumed_bytes: raw.len() as u32,
                    syllable_count: candidate.syllable_count,
                    script: CommitScript::Roman as i32,
                    roman: candidate.roman.clone(),
                    ..Default::default()
                })),
                &mut engine,
                &config,
            )
            .unwrap();
            assert_eq!(commit.commit.unwrap().document_text, expected);
            assert!(!commit.is_composing);
        }
    }
}

// ---- Decode tests --------------------------------------------------------

#[test]
fn fetch_at_pos_right_after_the_first_keystroke_answers_candidates() {
    // R12: the first keystroke's fetch already finds a composition. Before,
    // an iOS async fetch that beat the platform's promotion call saw
    // `Phase::Composing` and answered no carrier — an empty candidate bar.
    let mut engine = Engine::new();
    requests::handle(
        &req(Method::Start(protos::engine::Start {
            text: "tsua".into(),
        })),
        &mut engine,
        &config_tl(),
    )
    .unwrap();
    let resp = requests::handle(
        &req(Method::FetchAtPos(FetchAtPos {
            now_ms: 0,
            ..FetchAtPos::default()
        })),
        &mut engine,
        &config_tl(),
    )
    .expect("dispatch ok");
    let carrier = resp.continuous.expect("continuous carrier present");
    // §34 literal-roman candidate needs no lexicon.
    assert_eq!(carrier.candidates[0].display_text, "tsua");
}

#[test]
fn decode_fetch_at_pos_idle_returns_no_continuous_carrier() {
    let mut engine = Engine::new();
    let resp = requests::handle(
        &req(Method::FetchAtPos(FetchAtPos {
            now_ms: 0,
            literal_roman_candidate_disabled: false,
        })),
        &mut engine,
        &config_tl(),
    )
    .expect("dispatch ok");
    assert!(
        resp.continuous.is_none(),
        "expected None, got {:?}",
        resp.continuous
    );
}

#[test]
fn decode_fetch_at_pos_continuous_lexicon_unavailable_returns_empty_carrier() {
    // Lexicon::EngineHandle::with_state returns NotInitialized in this
    // bare test process (we never call install). FetchAtPos must
    // degrade to an empty candidate carrier — NOT panic.
    let mut engine = Engine::new();
    requests::handle(
        &req(Method::Start(protos::engine::Start {
            text: "tsua".into(),
        })),
        &mut engine,
        &config_tl(),
    )
    .unwrap();

    let resp = requests::handle(
        &req(Method::FetchAtPos(FetchAtPos {
            now_ms: 0,
            // §34/S22: disable the literal-roman prepend so this test isolates
            // the lexicon-degradation path. With it ON (default), the bare
            // `derived_display("tsua")` candidate is added regardless of the
            // lexicon, which is orthogonal to "lexicon NotInitialized yields no
            // DICT candidates". Doubles as OFF-gate coverage.
            literal_roman_candidate_disabled: true,
        })),
        &mut engine,
        &config_tl(),
    )
    .expect("dispatch ok");
    let cont = resp.continuous.expect("continuous carrier present");
    assert!(
        cont.candidates.is_empty(),
        "lexicon NotInitialized must yield empty candidates"
    );
}

// §34/S22 — the `literal_roman_candidate_disabled` toggle gates ONLY the
// forced index-0 preedit-literal prepend. Lexicon-free so the literal is the
// sole candidate: ON → it appears at index 0; OFF → it is gone (and here, the
// carrier is empty because no dict candidates exist without a lexicon). The
// toggle never touches `assemble_candidates`, so any naturally-produced
// candidate would survive OFF — that is covered by the lexicon-backed golden.
#[test]
fn fetch_at_pos_literal_roman_toggle_gates_index0_prepend() {
    fn fetch_tsua(disabled: bool) -> Vec<(Option<String>, String)> {
        let mut engine = Engine::new();
        requests::handle(
            &req(Method::Start(protos::engine::Start {
                text: "tsua".into(),
            })),
            &mut engine,
            &config_tl(),
        )
        .unwrap();
        let resp = requests::handle(
            &req(Method::FetchAtPos(FetchAtPos {
                now_ms: 0,
                literal_roman_candidate_disabled: disabled,
            })),
            &mut engine,
            &config_tl(),
        )
        .expect("dispatch ok");
        resp.continuous
            .expect("continuous carrier present")
            .candidates
            .into_iter()
            .map(|c| (c.hanji, c.roman))
            .collect()
    }

    // ON (default): the literal-roman candidate is prepended at index 0,
    // roman-only (`hanji == None`) and byte-identical to the preedit.
    let on = fetch_tsua(false);
    assert_eq!(
        on.first(),
        Some(&(None, "tsua".to_string())),
        "literal-roman ON must prepend the preedit literal at index 0"
    );

    // OFF: the forced prepend is skipped; lexicon-free leaves no candidates.
    let off = fetch_tsua(true);
    assert!(
        off.is_empty(),
        "literal-roman OFF must not prepend the literal (got {off:?})"
    );
}

// v3.5.8 Phase 9 Item 11 — hanji guard ported into the engine.
// Spec: `continuous-candidate-display.md` §15.3.E + §15.6
// (`hanji_guard_in_engine`) + `continuous-commit-and-display.md` §10.7.
// §15.6 nominally places this in the `requests.rs` mod test, but that
// module doc routes Engine-dependent / degraded-path checks here next
// to the sibling `decode_fetch_at_pos_*_returns_empty_carrier` tests.
//
// Attribution caveat (intentional): this file is lexicon-free by
// charter, so the empty carrier below is also what the
// lexicon-unavailable degrade path yields — these pin the §15.6
// contract but cannot, alone, attribute emptiness to the guard.
// Guard attribution (mixed `"a好b"` where a real inventory would
// otherwise surface `a`→阿, suppressed only by the guard) needs a
// hermetic installed `EngineHandle` and belongs to the lexicon-backed
// layer. This note keeps a guard removal from passing silently.
// INVARIANT_LEX_HANJI_GUARD (behavioral-invariants.md §14) — the one engine guard.
#[test]
fn decode_fetch_at_pos_hanji_buffer_returns_empty_carrier() {
    // CJK accidentally in the composing buffer (paste / stale
    // selection residue). The only legitimate input modes are
    // TL/POJ/TPS romanization, so the engine must short-circuit to
    // an empty candidate carrier rather than syllabify garbage —
    // mirroring the platform D-8 guard this round ports inward.
    let mut engine = Engine::new();
    requests::handle(
        &req(Method::Start(protos::engine::Start {
            text: "我好".into(),
        })),
        &mut engine,
        &config_tl(),
    )
    .unwrap();
    assert!(matches!(
        engine.snapshot_state().phase,
        Phase::Continuous { .. }
    ));

    let resp = requests::handle(
        &req(Method::FetchAtPos(FetchAtPos {
            now_ms: 0,
            literal_roman_candidate_disabled: false,
        })),
        &mut engine,
        &config_tl(),
    )
    .expect("dispatch ok");
    let cont = resp
        .continuous
        .expect("continuous carrier present (guard returns empty, not None)");
    assert!(
        cont.candidates.is_empty(),
        "hanji in composing buffer must yield empty candidates, got {:?}",
        cont.candidates
    );
}

// `is_hanji` is `.any()`, so a single stray CJK char anywhere in an
// otherwise-romanized buffer also fails closed — Extension G (𰣻 U+308FB)
// and both Compatibility blocks (U+2F801, U+FA0D) included. Spec §15.3.E.
#[test]
fn decode_fetch_at_pos_mixed_hanji_buffer_returns_empty_carrier() {
    for text in ["a好b", "a\u{308FB}", "a\u{2F801}", "a\u{FA0D}"] {
        let mut engine = Engine::new();
        requests::handle(
            &req(Method::Start(protos::engine::Start { text: text.into() })),
            &mut engine,
            &config_tl(),
        )
        .unwrap();
        assert!(matches!(
            engine.snapshot_state().phase,
            Phase::Continuous { .. }
        ));

        let resp = requests::handle(
            &req(Method::FetchAtPos(FetchAtPos {
                now_ms: 0,
                literal_roman_candidate_disabled: false,
            })),
            &mut engine,
            &config_tl(),
        )
        .expect("dispatch ok");
        let cont = resp
            .continuous
            .expect("continuous carrier present (guard returns empty, not None)");
        assert!(
            cont.candidates.is_empty(),
            "stray hanji in {text:?} must yield empty candidates, got {:?}",
            cont.candidates
        );
    }
}

// ---- Optional-presence contract for `ContinuousResponse` -----------------
// Per `composing.proto:152-161`, only `FetchAtPos` populates the
// `continuous` carrier — the other 3 continuous-input methods MUST keep
// it absent so platform consumers don't accidentally re-render the
// candidate strip on a state-changing op. (Codex post-impl finding.)

#[test]
fn commit_continuous_response_omits_continuous_carrier() {
    let mut engine = Engine::new();
    requests::handle(
        &req(Method::Start(protos::engine::Start {
            text: "tsua".into(),
        })),
        &mut engine,
        &config_tl(),
    )
    .unwrap();
    let resp = requests::handle(
        &req(Method::CommitContinuous(CommitContinuous {
            script: CommitScript::Roman as i32,
            roman: "珠".into(),
            canonical_text: "珠".into(),
            association_tl: String::new(),
            hanji: None,
            consumed_bytes: 3,
            syllable_count: 1,
        })),
        &mut engine,
        &config_tl(),
    )
    .expect("dispatch ok");
    assert!(
        resp.continuous.is_none(),
        "CommitContinuous must NOT populate continuous"
    );
}

#[test]
fn reset_response_omits_continuous_carrier() {
    let mut engine = Engine::new();
    requests::handle(
        &req(Method::Start(protos::engine::Start {
            text: "tsua".into(),
        })),
        &mut engine,
        &config_tl(),
    )
    .unwrap();
    let resp = requests::handle(
        &req(Method::Reset(protos::engine::Reset {})),
        &mut engine,
        &config_tl(),
    )
    .expect("dispatch ok");
    assert!(
        resp.continuous.is_none(),
        "Reset must NOT populate continuous"
    );
}

// ---- Empty-composition precondition at the dispatch layer -----------------
// Phase 4 transition tests cover this; mirror at dispatch boundary so a
// future intent-decode rename can't silently relax the precondition.
// (Codex post-impl finding.)

#[test]
fn empty_start_stays_idle_at_the_wire() {
    let mut engine = Engine::new();
    let resp = requests::handle(
        &req(Method::Start(protos::engine::Start { text: "".into() })),
        &mut engine,
        &config_tl(),
    )
    .expect("dispatch ok");
    assert!(
        !matches!(engine.snapshot_state().phase, Phase::Continuous { .. }),
        "an empty Start must NOT enter Phase::Continuous"
    );
    assert!(!resp.is_composing);
    assert!(resp.continuous.is_none());
}
