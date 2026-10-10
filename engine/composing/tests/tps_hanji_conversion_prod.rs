//! Hanji conversion in the TPS preedit over the production lexicon
//! (`docs/architecture/desktop-tps-hanji-conversion-roadmap.md` H1): the
//! sentence the plan was grounded on (`candidate_dump`, `DUMP_MODE=tps`,
//! 2026-10-05) converts as its readings close.

use composing::{requests, Engine, Intent};
use protos::engine::composing_request::Method;
use protos::engine::composing_response::Preedit;
use protos::engine::{AppConfig, TpsKey};

use crate::common::{config_converting, converted_words, production_lexicon_ready, req};

// trace (candidate_dump slot 0, the same neutral walk): `ㄍㄧㄣ ㄚˋ` → roman
// `kin-á`, one edge (0, 15): the dictionary word 巾仔 (walker_cost 12,348 →
// 10.75 with the length terms) beats the 今 / 根 (10,402 → 8.35) + 仔
// (7,643) split since E1 P3. Closing ㆢㄧㆵ˙ (11 bytes) →
// `kin-á-ji̍t`, one edge (0, 26): the two words become 今仔日. The rest of
// the sentence, typed key by key, ends on 今仔日天氣真好; before ㄏㄛ's tone
// mark the reading is open and shows as glyphs.
#[test]
fn a_production_sentence_converts_and_resegments_as_readings_close() {
    if !production_lexicon_ready() {
        eprintln!("production artifacts absent — run `make dict`; skipping.");
        return;
    }
    let config = config_converting("tps");
    let mut engine = Engine::new();

    engine.apply(
        Intent::Start {
            text: "ㄍㄧㄣ ㄚˋ".into(),
        },
        &config,
    );
    assert_eq!(
        converted_words(&engine),
        vec![((0, 15), "巾仔".to_string())]
    );

    let preedit = type_keys(&mut engine, "ㆢㄧㆵ˙", &config);
    assert_eq!(preedit.raw_input, "ㄍㄧㄣ ㄚˋㆢㄧㆵ˙");
    assert_eq!(
        converted_words(&engine),
        vec![((0, 26), "今仔日".to_string())]
    );

    let preedit = type_keys(&mut engine, "ㄊㆪ ㄎㄧ˪ㄐㄧㄣ ㄏㄛ", &config);
    assert_eq!(preedit.raw_input, "ㄍㄧㄣ ㄚˋㆢㄧㆵ˙ㄊㆪ ㄎㄧ˪ㄐㄧㄣ ㄏㄛ");
    assert_eq!(preedit.display_text, "今仔日天氣真ㄏㄛ");

    let preedit = type_keys(&mut engine, "ˋ", &config);
    assert_eq!(preedit.display_text, "今仔日天氣真好");
    assert_eq!(preedit.caret_utf16, 7);
}

/// Types `keys` one `TpsKey` at a time (the desktop key path, adjuster
/// included) and answers the last preedit.
fn type_keys(engine: &mut Engine, keys: &str, config: &AppConfig) -> Preedit {
    keys.chars()
        .map(|key| {
            let request = req(Method::TpsKey(TpsKey {
                key: key.to_string(),
            }));
            requests::handle(&request, engine, config)
                .expect("TpsKey")
                .preedit
                .expect("preedit")
        })
        .last()
        .expect("at least one key")
}

/// `(span end, Hanji, canonical TL)` of each row `FetchAtPos` lists.
fn listed_rows(engine: &mut Engine, config: &AppConfig) -> Vec<(u32, String, String)> {
    requests::apply(crate::common::Fetch::default().intent(), engine, config)
        .continuous
        .map(|continuous| continuous.candidates)
        .unwrap_or_default()
        .into_iter()
        .map(|c| {
            (
                c.consumed_span_end,
                c.hanji.unwrap_or_default(),
                c.canonical_tl,
            )
        })
        .collect()
}

// trace (production lexicon, 2026-10-10): toneless `ㄌㄧㄏㄛ` typed key by
// key → raw `ㄌㄧㆷㄛ` (the adjuster folds ㄏ after ㄌㄧ since lih is a
// syllable). Single ends at 0: ㄌㄧ (6, li) and ㄌㄧㆷ (9, li̍h); the syllable
// ㆷㄛ (ho through the {ㄏㆷ} family) leaves 6 and ends at 12 > 9, so §18
// guard (e) keeps end 6. Before the fix the list held 你好 汝好 字號 理會
// (12) and 裂 哩 揤 列 致 塊 得 (9) only. `ㄌㄧㄏㆦ`: no 12-byte word, the
// li̍h rows plus now li. `ㄍㄚㄉㄚ`: kat (9) plus now ka (6).
#[test]
fn a_toneless_first_syllable_a_longer_one_straddles_is_listed() {
    if !production_lexicon_ready() {
        eprintln!("production artifacts absent — run `make dict`; skipping.");
        return;
    }
    let row = |end: u32, hanji: &str, tl: &str| (end, hanji.to_string(), tl.to_string());
    for config in [config_converting("tps"), crate::common::config("tps")] {
        for (typed, short, long) in [
            ("ㄌㄧㄏㄛ", row(6, "你", "lí"), row(9, "裂", "li̍h")),
            ("ㄌㄧㄏㆦ", row(6, "你", "lí"), row(9, "裂", "li̍h")),
            ("ㄍㄚㄉㄚ", row(6, "家", "ka"), row(9, "結", "kat")),
        ] {
            let mut engine = Engine::new();
            type_keys(&mut engine, typed, &config);
            let rows = listed_rows(&mut engine, &config);
            assert!(
                rows.contains(&short),
                "{typed}: {short:?} must be listed, got {rows:?}"
            );
            assert!(
                rows.contains(&long),
                "{typed}: {long:?} must stay listed, got {rows:?}"
            );
        }
    }

    // A pick of 你 consumes its 6 bytes only; arm B keeps composing.
    let config = config_converting("tps");
    let mut engine = Engine::new();
    type_keys(&mut engine, "ㄌㄧㄏㄛ", &config);
    listed_rows(&mut engine, &config);
    let picked = requests::apply(
        Intent::CommitContinuous {
            canonical_text: "你".into(),
            association_tl: "lí".into(),
            hanji: Some("你".into()),
            consumed_bytes: 6,
            syllable_count: 1,
            script: Some(composing::api::CommitScript::Lead),
            roman: "lí".into(),
        },
        &mut engine,
        &config,
    );
    // trace: the tail keeps the adjuster's ㆷ glyph (display only — the
    // {ㄏㆷ} family reads it as the initial of ho); Space closes it on tone 1
    // → 熇 ho.
    let preedit = picked.preedit.expect("preedit after the pick");
    assert_eq!(
        (preedit.raw_input.as_str(), preedit.display_text.as_str()),
        ("ㆷㄛ", "你ㆷㄛ")
    );
    assert_eq!(type_keys(&mut engine, " ", &config).display_text, "你熇");
}
