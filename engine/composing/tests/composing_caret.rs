//! Desktop composing caret (roadmap § Desktop composing caret, P1b):
//! `MoveCaret` steps inside the pending tail, every mutator edits at the
//! caret, and `Preedit.caret_utf16` projects it into the display.

use composing::api::{CaretDirection, Phase};
use composing::{dispatch, Engine, Intent};
use protos::engine::composing_request::Method;
use protos::engine::composing_response::Preedit;
use protos::engine::effect::Kind;
use protos::engine::{ComposingResponse, MoveCaret};

mod common;
use common::{config, config_tl, effect_kinds, req};

fn start(text: &str) -> Engine {
    let mut engine = Engine::new();
    engine.apply(
        Intent::Start {
            text: text.to_string(),
        },
        &config_tl(),
    );
    engine
}

fn move_caret(engine: &mut Engine, direction: CaretDirection) -> ComposingResponse {
    engine.apply(
        Intent::MoveCaret {
            direction: Some(direction),
        },
        &config_tl(),
    )
}

fn preedit(resp: &ComposingResponse) -> &Preedit {
    resp.preedit.as_ref().expect("preedit")
}

fn kinds(resp: &ComposingResponse) -> Vec<&'static str> {
    effect_kinds(&resp.effect)
}

fn caret_of(engine: &Engine) -> usize {
    match engine.snapshot_state().phase {
        Phase::Composing { caret, .. } | Phase::Continuous { caret, .. } => caret,
        Phase::Idle => panic!("idle has no caret"),
    }
}

#[test]
fn move_caret_steps_one_char_emits_only_update_preedit_and_keeps_selection() {
    // trace: "ka2" → display "ká" (k, á). Start puts the caret at 3 (end);
    // the display caret is the end, 2 units.
    let mut engine = start("ka2");
    engine.apply(Intent::SetSelectedCandidateIndex { index: 2 }, &config_tl());

    // ← : caret 3 → 2 (before the digit). Display: k→1, a matches á→2; 2.
    let resp = move_caret(&mut engine, CaretDirection::Left);
    assert_eq!(kinds(&resp), vec!["UpdatePreedit"]);
    assert_eq!(
        resp.selected_candidate_index, 2,
        "selection survives a move"
    );
    assert!(resp.is_composing);
    assert_eq!(caret_of(&engine), 2);
    let p = preedit(&resp);
    assert_eq!(
        (p.raw_input.as_str(), p.display_text.as_str()),
        ("ka2", "k\u{e1}")
    );
    assert_eq!(p.caret_utf16, 2);
    let Some(Kind::UpdatePreedit(update)) = resp.effect[0].kind.as_ref() else {
        panic!("UpdatePreedit");
    };
    assert_eq!(
        (update.display.as_str(), update.caret_utf16),
        ("k\u{e1}", 2)
    );

    // ← ← : 2 → 1 (between k and a, display 1) → 0 (display 0).
    assert_eq!(
        preedit(&move_caret(&mut engine, CaretDirection::Left)).caret_utf16,
        1
    );
    assert_eq!(
        preedit(&move_caret(&mut engine, CaretDirection::Left)).caret_utf16,
        0
    );
    assert_eq!(caret_of(&engine), 0);

    // ← at the start: nothing moves, no effect.
    let resp = move_caret(&mut engine, CaretDirection::Left);
    assert!(resp.effect.is_empty());
    assert_eq!(caret_of(&engine), 0);

    // → → → back to the end, then → is a no-op.
    for _ in 0..3 {
        move_caret(&mut engine, CaretDirection::Right);
    }
    assert_eq!(caret_of(&engine), 3);
    let resp = move_caret(&mut engine, CaretDirection::Right);
    assert!(resp.effect.is_empty());
    assert_eq!(preedit(&resp).caret_utf16, 2);
}

#[test]
fn move_caret_when_idle_is_a_snapshot() {
    let mut engine = Engine::new();
    let resp = move_caret(&mut engine, CaretDirection::Left);
    assert!(!resp.is_composing);
    assert!(resp.effect.is_empty());
}

#[test]
fn append_inserts_at_the_caret_and_refetches() {
    // USER's example: "ka2", ← ← (caret 1, between k and a), type h →
    // "kha2", display "khá", caret after the h. trace: k→1, h→2; 2.
    let mut engine = start("ka2");
    move_caret(&mut engine, CaretDirection::Left);
    move_caret(&mut engine, CaretDirection::Left);
    let resp = engine.apply(
        Intent::Append {
            ch: "h".to_string(),
        },
        &config_tl(),
    );
    assert_eq!(kinds(&resp), vec!["UpdatePreedit", "PerformAutocomplete"]);
    assert_eq!(resp.selected_candidate_index, 0);
    let p = preedit(&resp);
    assert_eq!(
        (p.raw_input.as_str(), p.display_text.as_str()),
        ("kha2", "kh\u{e1}")
    );
    assert_eq!(p.caret_utf16, 2);
    assert_eq!(caret_of(&engine), 2);
}

#[test]
fn delete_backward_removes_the_char_before_the_caret_and_stops_at_the_start() {
    // "kha2" with the caret after "kh" (2): backspace drops the h → "ka2",
    // caret 1.
    let mut engine = start("kha2");
    move_caret(&mut engine, CaretDirection::Left);
    move_caret(&mut engine, CaretDirection::Left);
    let resp = engine.apply(Intent::DeleteBackward, &config_tl());
    assert_eq!(kinds(&resp), vec!["UpdatePreedit", "PerformAutocomplete"]);
    assert_eq!(preedit(&resp).raw_input, "ka2");
    assert_eq!(caret_of(&engine), 1);

    // Caret at 0 with text after it: nothing to delete, nothing happens —
    // the composition is NOT ended.
    move_caret(&mut engine, CaretDirection::Left);
    let resp = engine.apply(Intent::DeleteBackward, &config_tl());
    assert!(resp.effect.is_empty());
    assert!(resp.is_composing);
    assert_eq!(preedit(&resp).raw_input, "ka2");
    assert_eq!(caret_of(&engine), 0);
}

#[test]
fn replace_last_swaps_the_char_before_the_caret_and_keeps_the_selection() {
    // "tai" caret 2 (after "ta"), ReplaceLast "o": drop a → "ti" caret 1,
    // insert o → "toi" caret 2. Selection index untouched (TPS auto-correct
    // contract).
    let mut engine = start("tai");
    engine.apply(Intent::SetSelectedCandidateIndex { index: 1 }, &config_tl());
    move_caret(&mut engine, CaretDirection::Left);
    let resp = engine.apply(
        Intent::ReplaceLast {
            replacement: "o".to_string(),
        },
        &config_tl(),
    );
    assert_eq!(preedit(&resp).raw_input, "toi");
    assert_eq!(resp.selected_candidate_index, 1);
    assert_eq!(caret_of(&engine), 2);
}

#[test]
fn telex_key_acts_on_the_chunk_before_the_caret() {
    // "tai" caret 2, Telex `v` (tone 2): apply_telex_key("ta") → "ta2", tail
    // "i" rides along → "ta2i", caret 3. A digit mid-buffer is not a
    // syllable end, so the display stays literal and the caret is 3.
    let mut engine = start("tai");
    move_caret(&mut engine, CaretDirection::Left);
    let resp = engine.apply(
        Intent::TelexKey {
            key: "v".to_string(),
        },
        &config_tl(),
    );
    let p = preedit(&resp);
    assert_eq!(
        (p.raw_input.as_str(), p.display_text.as_str()),
        ("ta2i", "ta2i")
    );
    assert_eq!(p.caret_utf16, 3);
    assert_eq!(caret_of(&engine), 3);
}

#[test]
fn continuous_keeps_the_caret_on_promotion_resets_it_on_nail_and_never_enters_a_segment() {
    // "tsua" caret 3 (← once) → EnterContinuous keeps 3.
    let mut engine = start("tsua");
    move_caret(&mut engine, CaretDirection::Left);
    let resp = engine.apply(Intent::EnterContinuous, &config_tl());
    assert!(resp.effect.is_empty());
    assert_eq!(caret_of(&engine), 3);
    assert_eq!(preedit(&resp).caret_utf16, 3);

    // Nail 珠 over "tsu" (3 bytes): pending "a", caret at its end (1).
    // Display under TL (roman spacing): "珠 a" — prefix 2 units, so the
    // caret is 3.
    let resp = engine.apply(
        Intent::CommitContinuous {
            display_text: "珠".to_string(),
            canonical_text: String::new(),
            association_tl: String::new(),
            consumed_bytes: 3,
            syllable_count: 1,
        },
        &config_tl(),
    );
    let p = preedit(&resp);
    assert_eq!(
        (p.raw_input.as_str(), p.display_text.as_str()),
        ("a", "珠 a")
    );
    assert_eq!(p.caret_utf16, 3);
    assert_eq!(caret_of(&engine), 1);

    // ← puts the caret before the a (display 2, after "珠 "); ← again does
    // not enter the nailed segment.
    assert_eq!(
        preedit(&move_caret(&mut engine, CaretDirection::Left)).caret_utf16,
        2
    );
    let resp = move_caret(&mut engine, CaretDirection::Left);
    assert!(resp.effect.is_empty());
    assert_eq!(caret_of(&engine), 0);

    // Backspace with the caret at 0 and a pending char after it: no-op,
    // no unnail.
    let resp = engine.apply(Intent::DeleteBackward, &config_tl());
    assert!(resp.effect.is_empty());
    assert!(matches!(
        engine.snapshot_state().phase,
        Phase::Continuous { ref raw, caret: 0, ref nailed } if raw == "a" && nailed.len() == 1
    ));

    // → then backspace empties the pending tail (still Continuous, as
    // before this round); the next backspace unnails 珠 — its raw text comes
    // back as the tail with the caret at its end. One segment popped leaves
    // none, so NextWord is cleared rather than re-pointed.
    move_caret(&mut engine, CaretDirection::Right);
    let resp = engine.apply(Intent::DeleteBackward, &config_tl());
    assert_eq!(kinds(&resp), vec!["UpdatePreedit", "PerformAutocomplete"]);
    assert_eq!(preedit(&resp).raw_input, "");
    assert_eq!(caret_of(&engine), 0);
    let resp = engine.apply(Intent::DeleteBackward, &config_tl());
    assert_eq!(
        kinds(&resp),
        vec![
            "NextWordClearForNewComposing",
            "UpdatePreedit",
            "PerformAutocomplete"
        ]
    );
    assert_eq!(preedit(&resp).raw_input, "tsu");
    assert_eq!(caret_of(&engine), 3);
}

#[test]
fn start_puts_the_caret_at_the_end() {
    let mut engine = start("ka2");
    move_caret(&mut engine, CaretDirection::Left);
    engine.apply(
        Intent::Start {
            text: "li2".to_string(),
        },
        &config_tl(),
    );
    assert_eq!(caret_of(&engine), 3);
}

#[test]
fn continuous_mid_tail_append_keeps_the_nailed_prefix_and_projects_the_caret() {
    // "tsua" → nail 珠 over "tsu" → pending "a" (caret 1). Type "i" then ←
    // then "h": "ai" caret 2 → ← caret 1 → insert h at 1 → "ahi" caret 2.
    // Display "珠 ahi": prefix "珠 " is 2 units, a→1, h→2; caret 4.
    let mut engine = start("tsua");
    engine.apply(Intent::EnterContinuous, &config_tl());
    engine.apply(
        Intent::CommitContinuous {
            display_text: "珠".to_string(),
            canonical_text: String::new(),
            association_tl: String::new(),
            consumed_bytes: 3,
            syllable_count: 1,
        },
        &config_tl(),
    );
    engine.apply(
        Intent::Append {
            ch: "i".to_string(),
        },
        &config_tl(),
    );
    move_caret(&mut engine, CaretDirection::Left);
    let resp = engine.apply(
        Intent::Append {
            ch: "h".to_string(),
        },
        &config_tl(),
    );
    assert_eq!(kinds(&resp), vec!["UpdatePreedit", "PerformAutocomplete"]);
    let p = preedit(&resp);
    assert_eq!(
        (p.raw_input.as_str(), p.display_text.as_str()),
        ("ahi", "珠 ahi")
    );
    assert_eq!(p.caret_utf16, 4);
    assert_eq!(caret_of(&engine), 2);
}

#[test]
fn hanji_first_prefix_has_no_space_before_the_tail() {
    // Same nail under hanji-first (swapped, single script): "珠a", no
    // separator, so the caret after the a is 2 and before it 1.
    let mut config = config("tl");
    config.is_translate_swapped = true;
    let mut engine = Engine::new();
    engine.apply(
        Intent::Start {
            text: "tsua".to_string(),
        },
        &config,
    );
    engine.apply(Intent::EnterContinuous, &config);
    let resp = engine.apply(
        Intent::CommitContinuous {
            display_text: "珠".to_string(),
            canonical_text: String::new(),
            association_tl: String::new(),
            consumed_bytes: 3,
            syllable_count: 1,
        },
        &config,
    );
    let p = preedit(&resp);
    assert_eq!((p.display_text.as_str(), p.caret_utf16), ("珠a", 2));
    let resp = engine.apply(
        Intent::MoveCaret {
            direction: Some(CaretDirection::Left),
        },
        &config,
    );
    assert_eq!(preedit(&resp).caret_utf16, 1);
}

#[test]
fn move_caret_on_the_wire_decodes_left_right_and_treats_unspecified_as_a_no_op() {
    let mut engine = start("ka2");
    let apply = |engine: &mut Engine, direction: i32| {
        dispatch::handle(
            &req(Method::MoveCaret(MoveCaret { direction })),
            engine,
            &config_tl(),
        )
        .expect("dispatch ok")
    };
    let left = protos::engine::CaretDirection::Left as i32;
    let right = protos::engine::CaretDirection::Right as i32;
    assert_eq!(preedit(&apply(&mut engine, left)).caret_utf16, 2);
    assert_eq!(caret_of(&engine), 2);
    // Unspecified (0) and a value this build does not know: nothing moves,
    // no effects, still composing.
    for unknown in [0, 99] {
        let resp = apply(&mut engine, unknown);
        assert!(resp.effect.is_empty());
        assert!(resp.is_composing);
        assert_eq!(caret_of(&engine), 2);
    }
    assert_eq!(kinds(&apply(&mut engine, right)), vec!["UpdatePreedit"]);
    assert_eq!(caret_of(&engine), 3);
}
