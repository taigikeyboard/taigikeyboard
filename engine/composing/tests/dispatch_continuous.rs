//! v3.5.8 連續輸入 Phase 6 — dispatch decoding + degraded-path tests.
//!
//! Covers:
//! - Decoding `EnterContinuous` / `FetchAtPos` / `CommitContinuous` /
//!   `ResetContinuous` from `ComposingRequest.method` oneof variants.
//! - `Intent::FetchAtPos` short-circuit behavior in `dispatch::handle`:
//!   - Idle / Composing phase → `continuous = None` snapshot.
//!   - `Phase::Continuous` + lexicon NOT installed → `continuous =
//!     Some(empty)`.
//!   - `position != 0` → `continuous = Some(empty)` (reserved for future
//!     partial-fetch capability).
//! - Round-trip through dispatch for `EnterContinuous`, `CommitContinuous`,
//!   `ResetContinuous` (state changes match `transition.rs` Phase-4
//!   contract).
//!
//! The full TL/TPS lexicon-backed FetchAtPos integration (with hermetic
//! `dictionary.fst` + `dictionary.bin` + `syllables.fst`) lives next to
//! the lexicon parity suite — composing-side tests stay focused on the
//! dispatch wiring + decode contract.

// 中文: Phase 6 dispatch 的 decode 與降級路徑測試;TL/TPS 的 lexicon 整合測試交給 lexicon 端的 hermetic install fixture。

use composing::api::{Engine, Intent, Phase};
use composing::dispatch;
use protos::engine::composing_request::Method;
use protos::engine::{
    AppConfig, CommitContinuous, ComposingRequest, EnterContinuous, FetchAtPos, ResetContinuous,
};

fn config() -> AppConfig {
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

fn req(method: Method) -> ComposingRequest {
    ComposingRequest {
        method: Some(method),
    }
}

// ---- Decode tests --------------------------------------------------------

#[test]
fn decode_enter_continuous() {
    let mut engine = Engine::new();
    let _ = dispatch::handle(
        &req(Method::EnterContinuous(EnterContinuous {})),
        &mut engine,
        &config(),
    )
    .expect("dispatch ok");
    // EnterContinuous from Idle is no-op — phase stays Idle.
    assert!(matches!(engine.snapshot_state().phase, Phase::Idle));
}

#[test]
fn decode_fetch_at_pos_idle_returns_no_continuous_carrier() {
    let mut engine = Engine::new();
    let resp = dispatch::handle(
        &req(Method::FetchAtPos(FetchAtPos {
            position: 0,
            frequency_entries: vec![],
            now_ms: 0,
            custom_entries: vec![],
        })),
        &mut engine,
        &config(),
    )
    .expect("dispatch ok");
    assert!(
        resp.continuous.is_none(),
        "expected None, got {:?}",
        resp.continuous
    );
}

#[test]
fn decode_fetch_at_pos_position_nonzero_returns_empty_carrier() {
    let mut engine = Engine::new();
    // Get into Phase::Continuous via Start + EnterContinuous.
    dispatch::handle(
        &req(Method::Start(protos::engine::Start {
            text: "tsua".into(),
        })),
        &mut engine,
        &config(),
    )
    .unwrap();
    dispatch::handle(
        &req(Method::EnterContinuous(EnterContinuous {})),
        &mut engine,
        &config(),
    )
    .unwrap();
    assert!(matches!(
        engine.snapshot_state().phase,
        Phase::Continuous { .. }
    ));

    let resp = dispatch::handle(
        &req(Method::FetchAtPos(FetchAtPos {
            position: 1,
            frequency_entries: vec![],
            now_ms: 0,
            custom_entries: vec![],
        })),
        &mut engine,
        &config(),
    )
    .expect("dispatch ok");
    let cont = resp.continuous.expect("continuous carrier present");
    assert!(
        cont.candidates.is_empty(),
        "non-zero position must yield empty candidates"
    );
}

#[test]
fn decode_fetch_at_pos_continuous_lexicon_unavailable_returns_empty_carrier() {
    // Lexicon::EngineHandle::with_state returns NotInitialized in this
    // bare test process (we never call install). FetchAtPos must
    // degrade to an empty candidate carrier — NOT panic.
    let mut engine = Engine::new();
    dispatch::handle(
        &req(Method::Start(protos::engine::Start {
            text: "tsua".into(),
        })),
        &mut engine,
        &config(),
    )
    .unwrap();
    dispatch::handle(
        &req(Method::EnterContinuous(EnterContinuous {})),
        &mut engine,
        &config(),
    )
    .unwrap();

    let resp = dispatch::handle(
        &req(Method::FetchAtPos(FetchAtPos {
            position: 0,
            frequency_entries: vec![],
            now_ms: 0,
            custom_entries: vec![],
        })),
        &mut engine,
        &config(),
    )
    .expect("dispatch ok");
    let cont = resp.continuous.expect("continuous carrier present");
    assert!(
        cont.candidates.is_empty(),
        "lexicon NotInitialized must yield empty candidates"
    );
}

// v3.5.8 Phase 9 Item 11 — hanzi guard ported into the engine.
// Spec: `continuous-candidate-display.md` §15.3.E + §15.6
// (`hanzi_guard_in_engine`) + `continuous-input-ranking.md` §10.7.
// §15.6 nominally places this in the `dispatch.rs` mod test, but that
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
#[test]
fn decode_fetch_at_pos_hanzi_buffer_returns_empty_carrier() {
    // CJK accidentally in the composing buffer (paste / stale
    // selection residue). The only legitimate input modes are
    // TL/POJ/TPS romanization, so the engine must short-circuit to
    // an empty candidate carrier rather than syllabify garbage —
    // mirroring the platform D-8 guard this round ports inward.
    let mut engine = Engine::new();
    dispatch::handle(
        &req(Method::Start(protos::engine::Start {
            text: "我好".into(),
        })),
        &mut engine,
        &config(),
    )
    .unwrap();
    dispatch::handle(
        &req(Method::EnterContinuous(EnterContinuous {})),
        &mut engine,
        &config(),
    )
    .unwrap();
    assert!(matches!(
        engine.snapshot_state().phase,
        Phase::Continuous { .. }
    ));

    let resp = dispatch::handle(
        &req(Method::FetchAtPos(FetchAtPos {
            position: 0,
            frequency_entries: vec![],
            now_ms: 0,
            custom_entries: vec![],
        })),
        &mut engine,
        &config(),
    )
    .expect("dispatch ok");
    let cont = resp
        .continuous
        .expect("continuous carrier present (guard returns empty, not None)");
    assert!(
        cont.candidates.is_empty(),
        "hanzi in composing buffer must yield empty candidates, got {:?}",
        cont.candidates
    );
}

// `is_hanzi` is `.any()`, so a single stray CJK char anywhere in an
// otherwise-romanized buffer also fails closed (matches platform D-8
// `classify_input` Hanzi precedence). Spec §15.3.E.
#[test]
fn decode_fetch_at_pos_mixed_hanzi_buffer_returns_empty_carrier() {
    let mut engine = Engine::new();
    dispatch::handle(
        &req(Method::Start(protos::engine::Start {
            text: "a好b".into(),
        })),
        &mut engine,
        &config(),
    )
    .unwrap();
    dispatch::handle(
        &req(Method::EnterContinuous(EnterContinuous {})),
        &mut engine,
        &config(),
    )
    .unwrap();
    assert!(matches!(
        engine.snapshot_state().phase,
        Phase::Continuous { .. }
    ));

    let resp = dispatch::handle(
        &req(Method::FetchAtPos(FetchAtPos {
            position: 0,
            frequency_entries: vec![],
            now_ms: 0,
            custom_entries: vec![],
        })),
        &mut engine,
        &config(),
    )
    .expect("dispatch ok");
    let cont = resp
        .continuous
        .expect("continuous carrier present (guard returns empty, not None)");
    assert!(
        cont.candidates.is_empty(),
        "stray hanzi in mixed buffer must yield empty candidates, got {:?}",
        cont.candidates
    );
}

#[test]
fn decode_commit_continuous_mid_commit() {
    let mut engine = Engine::new();
    dispatch::handle(
        &req(Method::Start(protos::engine::Start {
            text: "tsua".into(),
        })),
        &mut engine,
        &config(),
    )
    .unwrap();
    dispatch::handle(
        &req(Method::EnterContinuous(EnterContinuous {})),
        &mut engine,
        &config(),
    )
    .unwrap();

    // Commit "珠" consuming 3 of 4 bytes → mid-commit, stay in Continuous.
    let resp = dispatch::handle(
        &req(Method::CommitContinuous(CommitContinuous {
            display_text: "珠".into(),
            canonical_text: String::new(),
            consumed_bytes: 3,
            syllable_count: 1,
        })),
        &mut engine,
        &config(),
    )
    .expect("dispatch ok");

    assert!(resp.is_composing, "mid-commit stays composing");
    let state = engine.snapshot_state();
    let Phase::Continuous { raw, nailed } = &state.phase else {
        panic!("expected Continuous phase, got {:?}", state.phase);
    };
    assert_eq!(raw, "a");
    assert_eq!(nailed.len(), 1);
    assert_eq!(nailed[0].display_text, "珠");
    assert_eq!(nailed[0].syllable_count, 1);
}

#[test]
fn decode_commit_continuous_final_commit_exits_to_idle() {
    let mut engine = Engine::new();
    dispatch::handle(
        &req(Method::Start(protos::engine::Start {
            text: "tsua".into(),
        })),
        &mut engine,
        &config(),
    )
    .unwrap();
    dispatch::handle(
        &req(Method::EnterContinuous(EnterContinuous {})),
        &mut engine,
        &config(),
    )
    .unwrap();

    // Commit "紙" consuming all 4 bytes → final commit, exit to Idle.
    let resp = dispatch::handle(
        &req(Method::CommitContinuous(CommitContinuous {
            display_text: "紙".into(),
            canonical_text: String::new(),
            consumed_bytes: 4,
            syllable_count: 1,
        })),
        &mut engine,
        &config(),
    )
    .expect("dispatch ok");

    assert!(!resp.is_composing, "final commit exits to idle");
    assert!(matches!(engine.snapshot_state().phase, Phase::Idle));
}

#[test]
fn decode_reset_continuous_aborts() {
    let mut engine = Engine::new();
    dispatch::handle(
        &req(Method::Start(protos::engine::Start {
            text: "tsua".into(),
        })),
        &mut engine,
        &config(),
    )
    .unwrap();
    dispatch::handle(
        &req(Method::EnterContinuous(EnterContinuous {})),
        &mut engine,
        &config(),
    )
    .unwrap();

    let resp = dispatch::handle(
        &req(Method::ResetContinuous(ResetContinuous {})),
        &mut engine,
        &config(),
    )
    .expect("dispatch ok");

    assert!(!resp.is_composing);
    assert!(matches!(engine.snapshot_state().phase, Phase::Idle));
    // ResetContinuous emits the standard abort effect trio.
    assert_eq!(
        resp.effect.len(),
        3,
        "expected 3 abort effects, got {:?}",
        resp.effect
    );
}

// ---- Intent shape sanity --------------------------------------------------

#[test]
fn fetch_at_pos_decodes_to_position_field() {
    // Sanity: the `position` field on FetchAtPos round-trips through
    // dispatch::decode_intent. We verify by constructing the Intent
    // and decoding via a public test helper. Since decode_intent is
    // pub(crate), the public smoke test is the dispatch::handle
    // path: send a request with position=1 and confirm the engine
    // ends up with `continuous = Some(empty)` (the position-nonzero
    // branch). This is already covered by
    // `decode_fetch_at_pos_position_nonzero_returns_empty_carrier`;
    // here we just lock the typed-Intent shape so a future field
    // rename keeps the test surface in sync.
    let _ = Intent::FetchAtPos {
        position: 7,
        frequency_entries: vec![],
        now_ms: 0,
        custom_entries: vec![],
    };
}

// ---- Optional-presence contract for `ContinuousResponse` -----------------
// Per `composing.proto:152-161`, only `FetchAtPos` populates the
// `continuous` carrier — the other 3 continuous-input methods MUST keep
// it absent so platform consumers don't accidentally re-render the
// candidate strip on a state-changing op. (Codex post-impl finding.)

#[test]
fn enter_continuous_response_omits_continuous_carrier() {
    let mut engine = Engine::new();
    dispatch::handle(
        &req(Method::Start(protos::engine::Start {
            text: "tsua".into(),
        })),
        &mut engine,
        &config(),
    )
    .unwrap();
    let resp = dispatch::handle(
        &req(Method::EnterContinuous(EnterContinuous {})),
        &mut engine,
        &config(),
    )
    .expect("dispatch ok");
    assert!(
        resp.continuous.is_none(),
        "EnterContinuous must NOT populate continuous"
    );
}

#[test]
fn commit_continuous_response_omits_continuous_carrier() {
    let mut engine = Engine::new();
    dispatch::handle(
        &req(Method::Start(protos::engine::Start {
            text: "tsua".into(),
        })),
        &mut engine,
        &config(),
    )
    .unwrap();
    dispatch::handle(
        &req(Method::EnterContinuous(EnterContinuous {})),
        &mut engine,
        &config(),
    )
    .unwrap();
    let resp = dispatch::handle(
        &req(Method::CommitContinuous(CommitContinuous {
            display_text: "珠".into(),
            canonical_text: String::new(),
            consumed_bytes: 3,
            syllable_count: 1,
        })),
        &mut engine,
        &config(),
    )
    .expect("dispatch ok");
    assert!(
        resp.continuous.is_none(),
        "CommitContinuous must NOT populate continuous"
    );
}

#[test]
fn reset_continuous_response_omits_continuous_carrier() {
    let mut engine = Engine::new();
    dispatch::handle(
        &req(Method::Start(protos::engine::Start {
            text: "tsua".into(),
        })),
        &mut engine,
        &config(),
    )
    .unwrap();
    dispatch::handle(
        &req(Method::EnterContinuous(EnterContinuous {})),
        &mut engine,
        &config(),
    )
    .unwrap();
    let resp = dispatch::handle(
        &req(Method::ResetContinuous(ResetContinuous {})),
        &mut engine,
        &config(),
    )
    .expect("dispatch ok");
    assert!(
        resp.continuous.is_none(),
        "ResetContinuous must NOT populate continuous"
    );
}

// ---- Phase-4 precondition retest at dispatch layer ------------------------
// Phase 4 transition tests cover this; mirror at dispatch boundary so a
// future intent-decode rename can't silently relax the precondition.
// (Codex post-impl finding.)

#[test]
fn empty_start_then_enter_continuous_stays_idle() {
    let mut engine = Engine::new();
    dispatch::handle(
        &req(Method::Start(protos::engine::Start { text: "".into() })),
        &mut engine,
        &config(),
    )
    .unwrap();
    let resp = dispatch::handle(
        &req(Method::EnterContinuous(EnterContinuous {})),
        &mut engine,
        &config(),
    )
    .expect("dispatch ok");
    assert!(
        !matches!(engine.snapshot_state().phase, Phase::Continuous { .. }),
        "EnterContinuous from empty raw must NOT enter Phase::Continuous"
    );
    assert!(resp.continuous.is_none());
}

// ---- v3.5.8 Phase 9.3a — FetchAtPos plumbs user-frequency snapshot ------
// Decode-only smoke test: confirm a `FetchAtPos` request carrying a
// non-empty `frequency_entries` + non-zero `now_ms` round-trips through
// `decode_intent` → `Intent::FetchAtPos` without panicking and that the
// lexicon-unavailable degraded path still returns an empty carrier
// (state-availability fallback, not a decode failure). The full
// boost-amplifies-score behaviour is pinned hermetically in
// `engine/lexicon/tests/user_freq_plumb.rs`; this test only locks the
// composing-side decode wiring.

#[test]
fn fetch_at_pos_carries_user_freq_snapshot_through_decode() {
    let mut engine = Engine::new();
    dispatch::handle(
        &req(Method::Start(protos::engine::Start {
            text: "tsua".into(),
        })),
        &mut engine,
        &config(),
    )
    .unwrap();
    dispatch::handle(
        &req(Method::EnterContinuous(EnterContinuous {})),
        &mut engine,
        &config(),
    )
    .unwrap();

    let resp = dispatch::handle(
        &req(Method::FetchAtPos(FetchAtPos {
            position: 0,
            frequency_entries: vec![
                protos::engine::FrequencyEntry {
                    display_text_key: "珠仔".into(),
                    count: 3,
                    last_used_ms: 1_700_000_000_000,
                },
                // Duplicate key exercises the `last-write-wins` policy
                // documented at `ranking::build_frequency_map`.
                protos::engine::FrequencyEntry {
                    display_text_key: "珠仔".into(),
                    count: 7,
                    last_used_ms: 1_700_000_001_000,
                },
            ],
            now_ms: 1_700_000_002_000,
            custom_entries: vec![],
        })),
        &mut engine,
        &config(),
    )
    .expect("dispatch ok");

    // Lexicon is not installed in this bare test process, so the
    // candidate carrier is still empty — but the carrier MUST be
    // present (proving the dispatcher reached `handle_fetch_at_pos`)
    // and the decode must not have panicked on the populated payload.
    let cont = resp.continuous.expect("continuous carrier present");
    assert!(
        cont.candidates.is_empty(),
        "lexicon not installed → empty candidates, but decode succeeded"
    );
}
