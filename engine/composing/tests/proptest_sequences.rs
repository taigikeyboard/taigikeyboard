//! Property tests for append/delete/replace sequences. Per plan §7.3.
//!
//! Random Intent sequences must preserve:
//! - `Idle ⟹ raw_input == "" ∧ selected_candidate_index == -1`
//! - `Composing ⟹ selected_candidate_index >= 0`
//! - `is_composing` matches phase on every response
//!
//! Coverage target: 256 sequences × len 1-12.

// 中文: 對 append/delete/replace 隨機序列做 property 測試,確保階段不變式恆成立。

use composing::{Engine, Intent};
use proptest::prelude::*;
use protos::engine::AppConfig;

fn config_tl() -> AppConfig {
    AppConfig {
        tone_mode: String::new(),
        input_mode: "tl".to_string(),
        oo_doubletap_enabled: false,
        nn_doubletap_enabled: false,
        is_translate_swapped: false,
        is_association_recording_enabled: false,
        platform_id: 0,
        output_both_scripts: false,
    }
}

fn arb_intent() -> impl Strategy<Value = Intent> {
    prop_oneof![
        any::<u8>().prop_map(|b| Intent::Append {
            ch: ((b % 26 + b'a') as char).to_string()
        }),
        Just(Intent::AppendHyphen),
        any::<u8>().prop_map(|b| Intent::ReplaceLast {
            replacement: ((b % 26 + b'a') as char).to_string(),
        }),
        Just(Intent::DeleteBackward),
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
        intents in proptest::collection::vec(arb_intent(), 1..=12)
    ) {
        let cfg = config_tl();
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
