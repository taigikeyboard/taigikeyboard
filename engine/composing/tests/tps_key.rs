//! `TpsKey` — one TPS key at the caret: auto-correct, replacement and
//! insertion in a single step, and the Space separator. Every platform sends
//! it for every TPS key; the expected buffers in `typed_keys_*` were recorded
//! against the phones' former three-call path before it was removed.

use composing::api::CaretDirection;
use composing::{requests, Engine, Intent};
use protos::engine::composing_request::Method;
use protos::engine::{AppConfig, ComposingResponse, TpsKey};

use crate::common;
use crate::common::req;

fn config_tps() -> AppConfig {
    common::config("tps")
}

fn tps_key(engine: &mut Engine, key: &str) -> ComposingResponse {
    requests::handle(
        &req(Method::TpsKey(TpsKey { key: key.into() })),
        engine,
        &config_tps(),
    )
    .unwrap()
}

/// Types `keys` one glyph at a time and answers the last response.
fn type_keys(engine: &mut Engine, keys: &str) -> ComposingResponse {
    keys.chars()
        .map(|key| tps_key(engine, &key.to_string()))
        .last()
        .expect("at least one key")
}

fn raw_of(engine: &Engine) -> String {
    match engine.snapshot_state().phase {
        composing::Phase::Continuous { raw, .. } => raw,
        composing::Phase::Idle => String::new(),
    }
}

#[test]
fn glyph_from_idle_begins_the_composition() {
    let mut engine = Engine::new();
    let resp = tps_key(&mut engine, "ㄍ");
    assert!(resp.is_composing);
    let preedit = resp.preedit.unwrap();
    assert_eq!(preedit.raw_input, "ㄍ");
    assert_eq!(preedit.display_text, "ㄍ");
}

// INVARIANT_TPS_STOPCODA_PHONOTACTIC_GATE (§32)
// trace: tps_adjust tests — ("ㄉ", "ㄍㄚ") → "ㆵ" (kat is a valid final);
// "ㄍㄠ" + "ㄉ" keeps ㄉ (kaut is not), so ㄉ starts the next syllable.
#[test]
fn stop_after_a_vowel_folds_only_where_the_final_is_valid() {
    let mut engine = Engine::new();
    type_keys(&mut engine, "ㄍㄚㄉ");
    assert_eq!(raw_of(&engine), "ㄍㄚㆵ");

    let mut engine = Engine::new();
    type_keys(&mut engine, "ㄍㄠㄉ");
    assert_eq!(raw_of(&engine), "ㄍㄠㄉ");
}

// trace: palatalization_replacement — last ㄗ + incoming ㄧ → replace ㄗ with ㄐ,
// then ㄧ is inserted: one step, one preedit.
#[test]
fn replacement_and_insertion_are_one_step() {
    let mut engine = Engine::new();
    tps_key(&mut engine, "ㄗ");
    let resp = tps_key(&mut engine, "ㄧ");
    let preedit = resp.preedit.unwrap();
    assert_eq!(preedit.raw_input, "ㄐㄧ");
    assert_eq!(preedit.display_text, "ㄐㄧ");
}

// trace: syllabic_nasal_replacement — last ㄇ + an incoming tone mark → ㆬ.
#[test]
fn tone_mark_after_a_bare_nasal_makes_it_syllabic() {
    let mut engine = Engine::new();
    type_keys(&mut engine, "ㄇˋ");
    assert_eq!(raw_of(&engine), "ㆬˋ");
}

// INVARIANT_TPS_SPACE_SOFT_SEPARATOR (§31)
// trace: prefix "ㄍㄚ" ends in a vowel → the separator is inserted; the preedit
// shows the buffer without it (`derived_display`).
#[test]
fn space_after_an_open_syllable_is_the_separator() {
    let mut engine = Engine::new();
    type_keys(&mut engine, "ㄍㄚ");
    let resp = tps_key(&mut engine, " ");
    // Effects are the caller's signal that the key was taken.
    assert_eq!(
        common::effect_kinds(&resp.effect),
        vec!["UpdatePreedit", "RefreshCandidates"]
    );
    let preedit = resp.preedit.unwrap();
    assert_eq!(preedit.raw_input, "ㄍㄚ ");
    assert_eq!(preedit.display_text, "ㄍㄚ");
}

#[test]
fn space_on_a_closed_syllable_is_a_no_op_with_no_effects() {
    // After a separator.
    let mut engine = Engine::new();
    type_keys(&mut engine, "ㄍㄚ ");
    let resp = tps_key(&mut engine, " ");
    assert!(resp.effect.is_empty());
    assert_eq!(resp.preedit.unwrap().raw_input, "ㄍㄚ ");

    // After a tone mark.
    let mut engine = Engine::new();
    type_keys(&mut engine, "ㄍㄚˋ");
    let resp = tps_key(&mut engine, " ");
    assert!(resp.effect.is_empty());
    assert_eq!(resp.preedit.unwrap().raw_input, "ㄍㄚˋ");

    // After the combining tone-8 dot: the phone's Space key commits on this
    // empty answer. trace: "ㄒㄧㄍ˙" stores ㆻ + U+0307, a tone mark.
    let mut engine = Engine::new();
    type_keys(&mut engine, "ㄒㄧㄍ˙");
    assert!(tps_key(&mut engine, " ").effect.is_empty());
}

#[test]
fn space_and_an_empty_key_while_idle_are_no_ops() {
    let mut engine = Engine::new();
    for key in [" ", ""] {
        let resp = tps_key(&mut engine, key);
        assert!(!resp.is_composing);
        assert!(resp.effect.is_empty());
    }
}

// trace: "ㄗㄚ", caret stepped left to between ㄗ and ㄚ; the adjuster reads the
// prefix "ㄗ" (not the whole buffer, whose last char is ㄚ), so ㄧ palatalizes
// it, and ㄚ rides along after the caret.
#[test]
fn key_acts_on_the_chunk_before_the_caret() {
    let mut engine = Engine::new();
    type_keys(&mut engine, "ㄗㄚ");
    engine.apply(
        Intent::MoveCaret {
            direction: Some(CaretDirection::Left),
        },
        &config_tps(),
    );
    tps_key(&mut engine, "ㄧ");
    assert_eq!(raw_of(&engine), "ㄐㄧㄚ");
    assert!(matches!(
        engine.snapshot_state().phase,
        composing::Phase::Continuous { caret, .. } if caret == "ㄐㄧ".len()
    ));

    // Nothing precedes a caret at the start, so Space there is a no-op.
    let mut engine = Engine::new();
    type_keys(&mut engine, "ㄚ");
    engine.apply(
        Intent::MoveCaret {
            direction: Some(CaretDirection::Left),
        },
        &config_tps(),
    );
    assert!(tps_key(&mut engine, " ").effect.is_empty());
}

#[test]
fn nailed_segments_stay_untouched() {
    // trace: "ㄍㄚㄙ" — ㄙ has no coda form, so the buffer is as typed. Nail the
    // first two glyphs (6 bytes); the pending tail is "ㄙ", and ㄧ palatalizes it.
    let mut engine = engine_with_nailed_ka();
    let resp = tps_key(&mut engine, "ㄧ");
    assert_eq!(resp.preedit.unwrap().raw_input, "ㄒㄧ");
    match engine.snapshot_state().phase {
        composing::Phase::Continuous { nailed, .. } => {
            assert_eq!(nailed.len(), 1);
            assert_eq!(nailed[0].display_text, "家");
        }
        composing::Phase::Idle => panic!("expected Continuous"),
    }
}

/// Nails "ㄍㄚ" of "ㄍㄚㄙ" as 家, leaving the pending tail "ㄙ".
fn engine_with_nailed_ka() -> Engine {
    let mut engine = Engine::new();
    type_keys(&mut engine, "ㄍㄚㄙ");
    engine.apply(
        Intent::CommitContinuous {
            canonical_text: "家".to_string(),
            association_tl: String::new(),
            hanji: Some("家".to_string()),
            consumed_bytes: "ㄍㄚ".len(),
            syllable_count: 1,
            script: Some(composing::CommitScript::Lead),
            roman: "ka".to_string(),
        },
        &config_tps(),
    );
    engine
}

// The pending tail after typing `keys` one `TpsKey` at a time. Each expected
// buffer is what the phones' former three-call path (`TpsInputAdjust` →
// `ReplaceLast` → `Append`) produced, recorded before that path was removed
// (D3) — effects and preedit matched key for key.
#[test]
fn typed_keys_leave_the_expected_buffer() {
    for (keys, expected) in [
        ("ㄍㄠㄉㄞ", "ㄍㄠㄉㄞ"),          // §32: 交代 without a separator
        ("ㄍㄠ ㄉㄞ", "ㄍㄠ ㄉㄞ"),        // §31: with one
        ("ㄍㄨㄇㆦ", "ㄍㄨㄇㆦ"),          // §33: 龜毛
        ("ㄗㄧㄚˋ", "ㄐㄧㄚˋ"),            // palatalization
        ("ㄇˊㄫ˫", "ㆬˊㆭ˫"),              // syllabic nasals
        ("ㄗㄤ9", "ㄗㄤˆ"),                // tone 9 from the digit
        ("ㄍㄚㄉ  ㄅㄚ", "ㄍㄚㆵ ㄅㄚ"),   // a second Space changes nothing
        ("ㄍㄚㄉ˙ ㄅ", "ㄍㄚㆵ\u{307}ㄅ"), // tone 8 closes the syllable
        ("ㄏㄧㄫㄉㄧㄅ", "ㄏㄧㄥㄉㄧㆴ"),
        ("ㄚ0123456789", "ㄚ0123456789"), // digits after a digit stay literal
        ("ㄍㄚ-ㄇ", "ㄍㄚ-ㆬ"),           // hyphen is no syllable boundary
        ("-ㄍ", "ㄍ"),                    // leading hyphen went to the document
        ("ㄒㄧㄍ˙ ", "ㄒㄧㆻ\u{307}"),    // Space after the tone-8 dot refused
        ("ㆮㄧㆮ", "ㆮㄧㆯ"),
    ] {
        let mut engine = Engine::new();
        for key in keys.chars() {
            tps_key(&mut engine, &key.to_string());
        }
        assert_eq!(raw_of(&engine), expected, "{keys:?}");
    }
}

// As above, after a nailed segment: the pending tail starts at "ㄙ".
#[test]
fn typed_keys_after_a_nailed_segment_leave_the_expected_buffer() {
    for (keys, expected) in [
        ("ㄧ", "ㄒㄧ"),
        ("ㄧㄚˋ", "ㄒㄧㄚˋ"),
        (" ", "ㄙ "),
        ("ㄇˊ", "ㄙㆬˊ"),
        ("-ㄅ", "ㄙ-ㆴ"),
        ("ㄚ0123456789", "ㄙㄚ0123456789"),
    ] {
        let mut engine = engine_with_nailed_ka();
        for key in keys.chars() {
            tps_key(&mut engine, &key.to_string());
        }
        assert_eq!(raw_of(&engine), expected, "{keys:?}");
    }
}

// INVARIANT_TPS_SPACE_PINS_UNMARKED_TONE (§41) — a stop coda is a consonant,
// not a tone mark, so Space after it is the separator that pins tone 4.
// trace: "ㄍㄚㄉ" → "ㄍㄚㆵ" (above); ㆵ is not in `is_tps_tone_mark`.
#[test]
fn space_after_a_stop_coda_is_the_separator() {
    let mut engine = Engine::new();
    type_keys(&mut engine, "ㄍㄚㄉ");
    let resp = tps_key(&mut engine, " ");
    assert_eq!(resp.preedit.unwrap().raw_input, "ㄍㄚㆵ ");
}

// trace: "ㄍㄚㄙㄚ" (ㄙ has no coda form), caret stepped left twice to after
// "ㄍㄚ"; Space lands there and the tail "ㄙㄚ" rides along.
#[test]
fn space_is_inserted_at_a_caret_inside_the_tail() {
    let mut engine = Engine::new();
    type_keys(&mut engine, "ㄍㄚㄙㄚ");
    for _ in 0..2 {
        engine.apply(
            Intent::MoveCaret {
                direction: Some(CaretDirection::Left),
            },
            &config_tps(),
        );
    }
    tps_key(&mut engine, " ");
    assert_eq!(raw_of(&engine), "ㄍㄚ ㄙㄚ");
}

// INVARIANT_KHINSIANN_LEADING_MARKER_LITERAL (§21) — as `Append` does.
#[test]
fn leading_hyphen_from_idle_is_a_document_literal() {
    let mut engine = Engine::new();
    let resp = tps_key(&mut engine, "-");
    assert!(!resp.is_composing);
    assert_eq!(common::commit_text(&resp), Some("-".to_string()));
}

// trace: "ㄍㄚˋ", caret stepped left to before ˋ — a separator there would part
// the syllable from its tone mark. "ㄍㄚ ㄙㄚ", caret stepped left to before the
// separator — a second one beside it adds nothing. Both refuse.
#[test]
fn space_before_a_tone_mark_or_a_separator_is_a_no_op() {
    for (keys, steps_left) in [("ㄍㄚˋ", 1), ("ㄍㄚ ㄙㄚ", 3)] {
        let mut engine = Engine::new();
        type_keys(&mut engine, keys);
        for _ in 0..steps_left {
            engine.apply(
                Intent::MoveCaret {
                    direction: Some(CaretDirection::Left),
                },
                &config_tps(),
            );
        }
        assert!(tps_key(&mut engine, " ").effect.is_empty(), "{keys:?}");
        assert_eq!(raw_of(&engine), keys);
    }
}
