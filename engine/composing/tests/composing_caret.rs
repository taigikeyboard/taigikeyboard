//! Desktop composing caret (roadmap § Desktop composing caret, P1b):
//! `MoveCaret` steps inside the pending tail, every mutator edits at the
//! caret, and `Preedit.caret_utf16` projects it into the display.

use composing::api::{CaretDirection, Phase};
use composing::CommitScript;
use composing::{requests, Engine, Intent};
use protos::engine::composing_request::Method;
use protos::engine::composing_response::Preedit;
use protos::engine::effect::Kind;
use protos::engine::{ComposingResponse, MoveCaret};

use crate::common::{config, config_tl, effect_kinds, req};

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
        Phase::Continuous { caret, .. } => caret,
        Phase::Idle => panic!("idle has no caret"),
    }
}

#[test]
fn move_caret_steps_one_char_emits_only_update_preedit() {
    // trace: "ka2" → display "ká" (k, á). Start puts the caret at 3 (end);
    // the display caret is the end, 2 units.
    let mut engine = start("ka2");

    // ← : caret 3 → 2 (before the digit). Display: k→1, a matches á→2; 2.
    let resp = move_caret(&mut engine, CaretDirection::Left);
    assert_eq!(kinds(&resp), vec!["UpdatePreedit"]);
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
fn move_caret_jumps_to_the_start_and_end_of_the_tail() {
    // trace: "ka2" → display "ká", caret 3 (end, display 2). End at the end:
    // no move, no effect. Start → 0 (display 0), one UpdatePreedit; Start
    // again → no effect. End → 3; the folded tone digit draws it at 2.
    let mut engine = start("ka2");
    let resp = move_caret(&mut engine, CaretDirection::End);
    assert!(resp.effect.is_empty());
    assert_eq!(caret_of(&engine), 3);

    let resp = move_caret(&mut engine, CaretDirection::Start);
    assert_eq!(kinds(&resp), vec!["UpdatePreedit"]);
    assert_eq!(caret_of(&engine), 0);
    assert_eq!(preedit(&resp).caret_utf16, 0);
    assert!(move_caret(&mut engine, CaretDirection::Start)
        .effect
        .is_empty());

    // From the middle (← once from the end: caret 2) End goes to 3.
    move_caret(&mut engine, CaretDirection::End);
    move_caret(&mut engine, CaretDirection::Left);
    assert_eq!(caret_of(&engine), 2);
    let resp = move_caret(&mut engine, CaretDirection::End);
    assert_eq!(kinds(&resp), vec!["UpdatePreedit"]);
    assert_eq!(caret_of(&engine), 3);
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
    assert_eq!(kinds(&resp), vec!["UpdatePreedit", "RefreshCandidates"]);
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
    assert_eq!(kinds(&resp), vec!["UpdatePreedit", "RefreshCandidates"]);
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
fn replace_last_swaps_the_char_before_the_caret() {
    // "tai" caret 2 (after "ta"), ReplaceLast "o": drop a → "ti" caret 1,
    // insert o → "toi" caret 2.
    let mut engine = start("tai");
    move_caret(&mut engine, CaretDirection::Left);
    let resp = engine.apply(
        Intent::ReplaceLast {
            replacement: "o".to_string(),
        },
        &config_tl(),
    );
    assert_eq!(preedit(&resp).raw_input, "toi");
    assert_eq!(caret_of(&engine), 2);
}

#[test]
fn telex_key_acts_on_the_chunk_before_the_caret() {
    // "tai" caret 2, Telex `v` (tone 2): apply_telex_key("ta") → "ta2", tail
    // "i" rides along → "ta2i", raw caret 3. The internal tone digit
    // is consumed, so the display is "tái" with its caret after "á".
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
        ("ta2i", "tái")
    );
    assert_eq!(p.caret_utf16, 2);
    assert_eq!(caret_of(&engine), 3);
}

#[test]
fn continuous_resets_the_caret_on_nail_and_never_enters_a_segment() {
    // "tsua" caret 3 (← once).
    let mut engine = start("tsua");
    let resp = move_caret(&mut engine, CaretDirection::Left);
    assert_eq!(caret_of(&engine), 3);
    assert_eq!(preedit(&resp).caret_utf16, 3);

    // Nail "tsu" (珠) over "tsu" (3 bytes): pending "a", caret at its end (1).
    // Display under TL (roman spacing): "tsu a" — prefix "tsu " is 4 units, so
    // the caret is 5.
    let resp = engine.apply(
        Intent::CommitContinuous {
            canonical_text: "珠".to_string(),
            association_tl: "tsu".to_string(),
            hanji: Some("珠".to_string()),
            consumed_bytes: 3,
            syllable_count: 1,
            script: Some(CommitScript::Roman),
            roman: "tsu".to_string(),
        },
        &config_tl(),
    );
    let p = preedit(&resp);
    assert_eq!(
        (p.raw_input.as_str(), p.display_text.as_str()),
        ("a", "tsu a")
    );
    assert_eq!(p.caret_utf16, 5);
    assert_eq!(caret_of(&engine), 1);

    // ← puts the caret before the a (display 4, after "tsu "); ← again does
    // not enter the nailed segment.
    assert_eq!(
        preedit(&move_caret(&mut engine, CaretDirection::Left)).caret_utf16,
        4
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
        Phase::Continuous { ref raw, caret: 0, ref nailed, .. } if raw == "a" && nailed.len() == 1
    ));

    // → then backspace empties the pending tail (still Continuous, as
    // before this round); the next backspace unnails "tsu" — its raw text comes
    // back as the tail with the caret at its end. One segment popped leaves
    // none, so NextWord is cleared rather than re-pointed.
    move_caret(&mut engine, CaretDirection::Right);
    let resp = engine.apply(Intent::DeleteBackward, &config_tl());
    assert_eq!(kinds(&resp), vec!["UpdatePreedit", "RefreshCandidates"]);
    assert_eq!(preedit(&resp).raw_input, "");
    assert_eq!(caret_of(&engine), 0);
    let resp = engine.apply(Intent::DeleteBackward, &config_tl());
    assert_eq!(
        kinds(&resp),
        vec![
            "NextWordClearForNewComposing",
            "UpdatePreedit",
            "RefreshCandidates"
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
    // "tsua" → nail "tsu" over "tsu" → pending "a" (caret 1). Type "i" then ←
    // then "h": "ai" caret 2 → ← caret 1 → insert h at 1 → "ahi" caret 2.
    // Display "tsu ahi": prefix "tsu " is 4 units, a→1, h→2; caret 6.
    let mut engine = start("tsua");
    engine.apply(
        Intent::CommitContinuous {
            canonical_text: "珠".to_string(),
            association_tl: "tsu".to_string(),
            hanji: Some("珠".to_string()),
            consumed_bytes: 3,
            syllable_count: 1,
            script: Some(CommitScript::Roman),
            roman: "tsu".to_string(),
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
    assert_eq!(kinds(&resp), vec!["UpdatePreedit", "RefreshCandidates"]);
    let p = preedit(&resp);
    assert_eq!(
        (p.raw_input.as_str(), p.display_text.as_str()),
        ("ahi", "tsu ahi")
    );
    assert_eq!(p.caret_utf16, 6);
    assert_eq!(caret_of(&engine), 2);
}

#[test]
fn hanji_first_prefix_has_no_space_before_the_tail() {
    // Hanji nail under hanji-first: "珠a", no separator, so the caret after
    // the a is 2 and before it 1.
    let mut config = config("tl");
    config.is_hanji_first = true;
    let mut engine = Engine::new();
    engine.apply(
        Intent::Start {
            text: "tsua".to_string(),
        },
        &config,
    );
    let resp = engine.apply(
        Intent::CommitContinuous {
            canonical_text: "珠".to_string(),
            association_tl: "tsu".to_string(),
            hanji: Some("珠".to_string()),
            consumed_bytes: 3,
            syllable_count: 1,
            script: Some(CommitScript::Lead),
            roman: "tsu".to_string(),
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
fn hanji_first_mixed_han_lo_prefix_projects_caret_past_the_boundary_space() {
    // #853: under hanji-first, nailing `啥mih` over "siannmih" (8 bytes) out of
    // "siannmihtaitsi" renders `"啥mih taitsi"` — prefix `"啥mih "` is 5 UTF-16
    // units (`啥` + `m` + `i` + `h` + ` `), so Start lands at 5 and End at 11.
    let mut config = config("tl");
    config.is_hanji_first = true;
    let mut engine = Engine::new();
    engine.apply(
        Intent::Start {
            text: "siannmihtaitsi".to_string(),
        },
        &config,
    );
    let resp = engine.apply(
        Intent::CommitContinuous {
            canonical_text: "啥mih".to_string(),
            association_tl: "siánn-mih".to_string(),
            hanji: Some("啥mih".to_string()),
            consumed_bytes: 8,
            syllable_count: 2,
            script: Some(CommitScript::Lead),
            roman: "siánn-mih".to_string(),
        },
        &config,
    );
    let p = preedit(&resp);
    assert_eq!(
        (p.display_text.as_str(), p.caret_utf16),
        ("啥mih taitsi", 11)
    );
    let resp = engine.apply(
        Intent::MoveCaret {
            direction: Some(CaretDirection::Start),
        },
        &config,
    );
    assert_eq!(preedit(&resp).caret_utf16, 5);
}

#[test]
fn start_stops_at_the_tail_after_a_nailed_segment() {
    // trace: "tsua", nail "tsu" over "tsu" → pending "a", display "tsu a"
    // (prefix 4 units). Start → raw caret 0 = display 4, right after the
    // prefix; Start again: no effect, still one segment nailed (Start never
    // re-opens). End → back to 1 (display 5).
    let mut engine = start("tsua");
    engine.apply(
        Intent::CommitContinuous {
            canonical_text: "珠".to_string(),
            association_tl: "tsu".to_string(),
            hanji: Some("珠".to_string()),
            consumed_bytes: 3,
            syllable_count: 1,
            script: Some(CommitScript::Roman),
            roman: "tsu".to_string(),
        },
        &config_tl(),
    );
    let resp = move_caret(&mut engine, CaretDirection::Start);
    assert_eq!(kinds(&resp), vec!["UpdatePreedit"]);
    assert_eq!(caret_of(&engine), 0);
    assert_eq!(preedit(&resp).caret_utf16, 4);
    let resp = move_caret(&mut engine, CaretDirection::Start);
    assert!(resp.effect.is_empty());
    assert_eq!(preedit(&resp).display_text, "tsu a");

    let resp = move_caret(&mut engine, CaretDirection::End);
    assert_eq!(caret_of(&engine), 1);
    assert_eq!(preedit(&resp).caret_utf16, 5);
}

#[test]
fn move_caret_on_the_wire_decodes_start_and_end() {
    let mut engine = start("ka2");
    let apply = |engine: &mut Engine, direction: protos::engine::CaretDirection| {
        requests::handle(
            &req(Method::MoveCaret(MoveCaret {
                direction: direction as i32,
            })),
            engine,
            &config_tl(),
        )
        .expect("dispatch ok")
    };
    apply(&mut engine, protos::engine::CaretDirection::Start);
    assert_eq!(caret_of(&engine), 0);
    apply(&mut engine, protos::engine::CaretDirection::End);
    assert_eq!(caret_of(&engine), 3);
}

#[test]
fn move_caret_on_the_wire_decodes_left_right_and_treats_unspecified_as_a_no_op() {
    let mut engine = start("ka2");
    let apply = |engine: &mut Engine, direction: i32| {
        requests::handle(
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

#[test]
fn backspace_across_mixed_boundary_restores_the_raw_pick() {
    // #853: under hanji-first, nailing `啥mih` over "siannmih" (8 bytes) out of
    // "siannmihtaitsi" renders `"啥mih taitsi"` — prefix `"啥mih "` is 5 UTF-16
    // units (`啥` + `m` + `i` + `h` + ` `), so Start lands at 5 and End at 11.
    let mut config = config("tl");
    config.is_hanji_first = true;
    let mut engine = Engine::new();
    engine.apply(
        Intent::Start {
            text: "siannmihtaitsi".to_string(),
        },
        &config,
    );
    let resp = engine.apply(
        Intent::CommitContinuous {
            canonical_text: "啥mih".to_string(),
            association_tl: "siánn-mih".to_string(),
            hanji: Some("啥mih".to_string()),
            consumed_bytes: 8,
            syllable_count: 2,
            script: Some(CommitScript::Lead),
            roman: "siánn-mih".to_string(),
        },
        &config,
    );
    let p = preedit(&resp);
    assert_eq!(
        (p.display_text.as_str(), p.caret_utf16),
        ("啥mih taitsi", 11)
    );
    for remaining in (0..6).rev() {
        let resp = engine.apply(Intent::DeleteBackward, &config);
        let expected = if remaining == 0 {
            "啥mih".to_string()
        } else {
            format!("啥mih {}", &"taitsi"[..remaining])
        };
        assert_eq!(preedit(&resp).display_text, expected);
        assert_eq!(
            preedit(&resp).caret_utf16 as usize,
            expected.encode_utf16().count()
        );
    }
    let resp = engine.apply(Intent::DeleteBackward, &config);
    assert_eq!(preedit(&resp).raw_input, "siannmih");
    assert_eq!(preedit(&resp).display_text, "siannmih");
    assert_eq!(preedit(&resp).caret_utf16, 8);
    assert_eq!(caret_of(&engine), 8);
}
