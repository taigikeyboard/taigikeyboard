//! §17 case 3 — partial-tone candidate filtering integration test.
//!
//! Reported bug (USER 2026-09-14, all four platforms): POJ `teng5-sek`
//! surfaced 等式 `téng-sek` / 中式 `teng-sek` ahead of 程式 `têng-sek`. The
//! typed `5` was discarded: `create_fst.py` emits only a toneless and a
//! fully-toned key family per record, so a span with SOME syllables toned
//! falls back to the toneless key and nothing downstream re-applied the
//! digit (`docs/reports/2026-09-14-partial-tone-candidate-filter.md`).
//!
//! After the fix the toneless lookup stays (there is still no partial-tone
//! FST family) and a post-lookup [`lexicon::TonePin::TypedTones`] pins
//! every syllable the user DID tone, leaving un-toned syllables free — on
//! the span-local list, the walker's slot 0, the partial-prefix extension
//! scan and both custom-entry paths.
//!
//! Fixture shape follows `continuous_explicit_tone.rs` (hermetic
//! `LexiconHandle`, `dictionary.fst` with both `tl:` families). TL mode:
//! 程式 / 等式 / 中式 share the toneless key `tingsik` and differ ONLY by
//! the first syllable's tone, and 程式 carries the LOWEST frequency so a
//! ranking-only explanation cannot pass. Per the fixture rule in
//! `.claude/rules/taigi-incidents.md`, every strict prefix of a probed
//! syllable that is itself a production syllable is present as a control
//! row and asserted on: 豬 `ti` / 鎮 `tìn` under `ting`, 是 `sī` under `sik`.

mod common;
use common::{
    build_dictionary_fst_tl_toned, build_syllables_fst_tl, build_tkdb_v3, empty_association_bin,
    engine_install_lock, fetch_hanji_with_custom as fetch_hanji_in, install_lexicon, write_temp,
    Row,
};
use lexicon::CustomEntry;

fn fixture_rows() -> Vec<Row> {
    vec![
        // The three-way tone collision on `tingsik`. 程式 is deliberately
        // the LEAST frequent so it can only lead by tone.
        Row {
            toneless_key: "tingsik",
            hanzi: "程式",
            tl: "tîng-sik", // tone 5 + stop coda 4 → tl_num ting5sik4
            syll: 2,
            freq: 1,
        },
        Row {
            toneless_key: "tingsik",
            hanzi: "等式",
            tl: "tíng-sik", // tone 2 → ting2sik4
            syll: 2,
            freq: 100,
        },
        Row {
            toneless_key: "tingsik",
            hanzi: "中式",
            tl: "ting-sik", // tone 1 → ting1sik4
            syll: 2,
            freq: 100,
        },
        // Single-syllable rows the (0, 5) `ting5` span resolves to.
        Row {
            toneless_key: "ting",
            hanzi: "程",
            tl: "tîng",
            syll: 1,
            freq: 50,
        },
        Row {
            toneless_key: "ting",
            hanzi: "等",
            tl: "tíng",
            syll: 1,
            freq: 80,
        },
        // `ting` + `se`: 程世 (tone 5) vs 中西 (tone 1) — the partial-prefix
        // and second-syllable-free probes.
        Row {
            toneless_key: "tingse",
            hanzi: "程世",
            tl: "tîng-sè",
            syll: 2,
            freq: 10,
        },
        Row {
            toneless_key: "tingse",
            hanzi: "中西",
            tl: "ting-se",
            syll: 2,
            freq: 100,
        },
        // Strict-prefix controls (fixture rule).
        Row {
            toneless_key: "ti",
            hanzi: "豬",
            tl: "ti",
            syll: 1,
            freq: 90,
        },
        Row {
            toneless_key: "tin",
            hanzi: "鎮",
            tl: "tìn",
            syll: 1,
            freq: 90,
        },
        Row {
            toneless_key: "si",
            hanzi: "是",
            tl: "sī",
            syll: 1,
            freq: 900,
        },
    ]
}

fn install_fixture() {
    let rows = fixture_rows();
    let dict_path = write_temp("dictionary-partial-tone.bin", &build_tkdb_v3(&rows));
    let fst_path = build_dictionary_fst_tl_toned(&rows);
    let assoc_path = write_temp("association-partial-tone.bin", &empty_association_bin());
    let syllables_path = build_syllables_fst_tl(&[
        "ting5", "ting2", "ting", "sik4", "se", "se3", "ti", "tin3", "si7",
    ]);
    install_lexicon(&fst_path, &dict_path, &assoc_path, &syllables_path);
}

/// TL-mode candidate hanji for `raw`, with `custom_dictionary.db` entries
/// attached.
fn fetch_hanji_with_custom(raw: &str, custom: Vec<CustomEntry>) -> Vec<String> {
    fetch_hanji_in(raw, "tl", custom)
}

fn fetch_hanji(raw: &str) -> Vec<String> {
    fetch_hanji_with_custom(raw, Vec::new())
}

fn has(hanji: &[String], h: &str) -> bool {
    hanji.iter().any(|x| x == h)
}

#[test]
fn partial_tone_keeps_only_readings_with_the_typed_tone() {
    let _lock = engine_install_lock();
    install_fixture();
    // The headline bug. `ting5sik` (first syllable toned, second not):
    // 程式 (tone 5) must be the only phrase; 等式 / 中式 share the toneless
    // key but carry the wrong first-syllable tone. The (0, 5) span `ting5`
    // is fully toned and keeps only 程 (§17 case 1, unchanged).
    for raw in ["ting5sik", "ting5-sik", "TING5SIK"] {
        let hanji = fetch_hanji(raw);
        assert!(
            has(&hanji, "程式"),
            "{raw:?} must surface 程式; got {hanji:?}"
        );
        assert!(
            has(&hanji, "程"),
            "{raw:?} must surface 程 (ting5); got {hanji:?}"
        );
        for wrong in ["等式", "中式", "等", "豬", "鎮", "是"] {
            assert!(
                !has(&hanji, wrong),
                "{raw:?} must NOT surface {wrong}; got {hanji:?}"
            );
        }
    }
}

#[test]
fn toneless_and_fully_toned_are_unchanged() {
    let _lock = engine_install_lock();
    install_fixture();
    // §17 case 2 — no digit → every tone (the affordance).
    let toneless = fetch_hanji("tingsik");
    for h in ["程式", "等式", "中式", "程", "等"] {
        assert!(
            has(&toneless, h),
            "toneless tingsik must surface {h}; got {toneless:?}"
        );
    }
    // §17 case 1 — fully toned → verbatim key, unchanged.
    let toned = fetch_hanji("ting5sik4");
    assert!(
        has(&toned, "程式"),
        "ting5sik4 must surface 程式; got {toned:?}"
    );
    assert!(
        !has(&toned, "等式") && !has(&toned, "中式"),
        "ting5sik4 must NOT surface wrong tones; got {toned:?}"
    );
    // The strict-prefix controls never appear at the anchor for the probed
    // two-syllable inputs (§18 longest-match) — asserted so a fixture
    // regression cannot hide behind them.
    for raw in ["tingsik", "ting5sik4"] {
        let hanji = fetch_hanji(raw);
        for control in ["豬", "鎮", "是"] {
            assert!(
                !has(&hanji, control),
                "{raw:?} must NOT surface the prefix control {control}; got {hanji:?}"
            );
        }
    }
}

#[test]
fn untoned_second_syllable_stays_free() {
    let _lock = engine_install_lock();
    install_fixture();
    // `ting5se`: the second syllable carries no digit, so 程世 (sè, tone 3)
    // passes — only the FIRST syllable's tone is pinned. 中西 (ting, tone 1)
    // is rejected on that first syllable.
    let hanji = fetch_hanji("ting5se");
    assert!(
        has(&hanji, "程世"),
        "ting5se must surface 程世; got {hanji:?}"
    );
    assert!(
        !has(&hanji, "中西"),
        "ting5se must NOT surface 中西; got {hanji:?}"
    );
}

#[test]
fn leading_untoned_syllable_with_toned_tail_keys_toneless() {
    let _lock = engine_install_lock();
    install_fixture();
    // Reverse partial tone (USER 2026-09-15, `kokbin5tong2` lost 國民黨):
    // leading syllable untoned, tail toned. Must key toneless + pin, not
    // the non-existent verbatim `tl:tingsik4`, so all three readings
    // (`sik4` satisfied, first syllable free) surface.
    let hanji = fetch_hanji("tingsik4");
    for h in ["程式", "等式", "中式"] {
        assert!(has(&hanji, h), "tingsik4 must surface {h}; got {hanji:?}");
    }
    // The typed tail tone still pins on this path: `tingse3` keeps 程世
    // (sè) and drops 中西 (se, tone 1).
    let hanji = fetch_hanji("tingse3");
    assert!(
        has(&hanji, "程世") && !has(&hanji, "中西"),
        "tingse3 must surface 程世 only; got {hanji:?}"
    );
}

#[test]
fn partial_prefix_extensions_honor_the_typed_tone() {
    let _lock = engine_install_lock();
    install_fixture();
    // `ting5s`: `s` is not a syllable, so the whole buffer goes down the
    // Step 4b partial-prefix scan on the toneless prefix `tl:tings`. The
    // whole-buffer pin must still drop the wrong-tone extensions.
    let hanji = fetch_hanji("ting5s");
    assert!(
        has(&hanji, "程式") || has(&hanji, "程世"),
        "ting5s must surface a tone-5 extension; got {hanji:?}"
    );
    for wrong in ["等式", "中式", "中西"] {
        assert!(
            !has(&hanji, wrong),
            "ting5s must NOT surface {wrong}; got {hanji:?}"
        );
    }
}

#[test]
fn custom_entries_answer_the_same_pin_on_both_paths() {
    let _lock = engine_install_lock();
    install_fixture();
    // Custom matching is toneless, so without the pin a custom entry with
    // the wrong tone would surface — through the lexicon whole-buffer merge
    // AND the walker's per-edge override (slot 0). Two entries under the
    // same toneless key, wrong tone FIRST: the walker must pick the first
    // ELIGIBLE entry, not filter its first-wins pick down to nothing.
    let custom = vec![
        CustomEntry {
            roman: "tíng-sik".into(),
            hanji: Some("甲".into()),
        },
        CustomEntry {
            roman: "tîng-sik".into(),
            hanji: Some("乙".into()),
        },
    ];
    let hanji = fetch_hanji_with_custom("ting5sik", custom.clone());
    assert!(
        has(&hanji, "乙"),
        "ting5sik must surface the tone-5 custom entry; got {hanji:?}"
    );
    assert!(
        !has(&hanji, "甲"),
        "ting5sik must NOT surface the tone-2 custom entry; got {hanji:?}"
    );
    assert_eq!(
        hanji.first().map(String::as_str),
        Some("乙"),
        "slot 0 (walker custom override) must be the first ELIGIBLE entry; got {hanji:?}"
    );

    // Fully toned input pins custom entries too — the verbatim key only
    // filters dictionary hits.
    let toned = fetch_hanji_with_custom("ting5sik4", custom.clone());
    assert!(
        has(&toned, "乙") && !has(&toned, "甲"),
        "ting5sik4 custom: got {toned:?}"
    );

    // Toneless: both entries visible (the affordance is unchanged for custom).
    let toneless = fetch_hanji_with_custom("tingsik", custom);
    assert!(
        has(&toneless, "甲") && has(&toneless, "乙"),
        "toneless tingsik must surface both custom entries; got {toneless:?}"
    );
}
