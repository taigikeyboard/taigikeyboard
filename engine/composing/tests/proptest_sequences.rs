//! Property tests for append/delete/replace sequences. Per plan §7.3.
//!
//! Random Intent sequences must preserve:
//! - `Idle ⟹ raw_input == "" ∧ selected_candidate_index == -1`
//! - `Composing ⟹ selected_candidate_index >= 0`
//! - `is_composing` matches phase on every response
//! - the caret is a char boundary of the pending tail and its display
//!   projection never exceeds the display
//!
//! Coverage target: 256 sequences × len 1-12.

use composing::api::{CaretDirection, Phase};
use composing::{Engine, Intent};
use proptest::prelude::*;
use protos::engine::effect::Kind;
use protos::engine::AppConfig;

mod common;
use common::{config, config_tl};

/// TL, or POJ with both double-tap folds on — the display then drops and
/// folds characters, which is what the caret projection has to survive.
fn arb_config() -> impl Strategy<Value = AppConfig> {
    prop_oneof![
        Just(config_tl()),
        Just(AppConfig {
            oo_doubletap_enabled: true,
            nn_doubletap_enabled: true,
            ..config("poj")
        }),
    ]
}

/// Letters, tone digits, a multi-byte glyph and the POJ dot — so the caret
/// crosses folds, marks and char boundaries wider than one byte.
fn arb_char() -> impl Strategy<Value = String> {
    prop_oneof![
        any::<u8>().prop_map(|b| ((b % 26 + b'a') as char).to_string()),
        (1u8..=9).prop_map(|d| d.to_string()),
        Just("\u{3109}".to_string()),
        Just("\u{358}".to_string()),
    ]
}

fn arb_intent() -> impl Strategy<Value = Intent> {
    prop_oneof![
        arb_char().prop_map(|ch| Intent::Append { ch }),
        arb_char().prop_map(|text| Intent::Start { text }),
        Just(Intent::AppendHyphen),
        arb_char().prop_map(|replacement| Intent::ReplaceLast { replacement }),
        Just(Intent::DeleteBackward),
        Just(Intent::MoveCaret {
            direction: Some(CaretDirection::Left)
        }),
        Just(Intent::MoveCaret {
            direction: Some(CaretDirection::Right)
        }),
        any::<u8>().prop_map(|b| Intent::TelexKey {
            key: ["v", "y", "d", "z", "f"][usize::from(b % 5)].to_string(),
        }),
        Just(Intent::EnterContinuous),
        Just(Intent::CommitDerived),
        Just(Intent::CommitRaw),
        Just(Intent::Reset),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        ..ProptestConfig::default()
    })]

    #[test]
    fn random_sequences_preserve_phase_index_invariants(
        intents in proptest::collection::vec(arb_intent(), 1..=12),
        cfg in arb_config(),
    ) {
        let mut engine = Engine::new();
        for intent in intents {
            let resp = engine.apply(intent, &cfg);
            // Idle ⟹ empty raw + -1 index
            if !resp.is_composing {
                let raw = resp.preedit.as_ref().map(|p| p.raw_input.as_str()).unwrap_or("");
                prop_assert_eq!(raw, "");
                prop_assert_eq!(resp.selected_candidate_index, -1);
            } else {
                // Composing ⟹ non-negative index
                prop_assert!(resp.selected_candidate_index >= 0);
            }
            let preedit = resp.preedit.clone().unwrap_or_default();
            let display_utf16 = preedit.display_text.encode_utf16().count();
            prop_assert!(preedit.caret_utf16 as usize <= display_utf16);
            // Every UpdatePreedit in the response shows the same caret the
            // preedit reports.
            for effect in &resp.effect {
                if let Some(Kind::UpdatePreedit(update)) = &effect.kind {
                    prop_assert_eq!(update.caret_utf16, preedit.caret_utf16);
                    prop_assert_eq!(&update.display, &preedit.display_text);
                }
            }
            match engine.snapshot_state().phase {
                Phase::Idle => prop_assert_eq!(preedit.caret_utf16, 0),
                Phase::Composing { raw, caret } | Phase::Continuous { raw, caret, .. } => {
                    prop_assert!(caret <= raw.len());
                    prop_assert!(raw.is_char_boundary(caret));
                    prop_assert_eq!(preedit.raw_input, raw);
                }
            }
        }
    }

    #[test]
    fn snapshot_idempotent(
        intents in proptest::collection::vec(arb_intent(), 1..=10)
    ) {
        let cfg = config_tl();
        let mut engine = Engine::new();
        for intent in intents {
            let _ = engine.apply(intent, &cfg);
        }
        let snap_a = engine.snapshot(&cfg);
        let snap_b = engine.snapshot(&cfg);
        prop_assert_eq!(snap_a.is_composing, snap_b.is_composing);
        prop_assert_eq!(snap_a.selected_candidate_index, snap_b.selected_candidate_index);
        prop_assert_eq!(
            snap_a.preedit.unwrap().raw_input,
            snap_b.preedit.unwrap().raw_input
        );
    }
}
