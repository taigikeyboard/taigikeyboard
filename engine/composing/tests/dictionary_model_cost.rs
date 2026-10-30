//! Reinstalling a dictionary updates both user-row and selected-word pricing.

use crate::common::{
    build_dictionary_fst_tl_toned, build_syllables_fst_tl, empty_association_bin, fetch_hanji,
    install_lexicon, selected, Fetch, Row, NOW_MS,
};
use lexicon::{CustomEntry, LearnedEntry};
use test_support::{
    build_tkdb_with_model, engine_install_lock, write_temp, TkdbModel, TkdbRow, TKDB_VERSION,
};

#[test]
fn loaded_model_prices_selected_custom_and_learned_words() {
    let _lock = engine_install_lock();
    for kind in ["selected", "custom", "learned"] {
        // Fixed record costs isolate the header model's effect. Include the
        // shorter syllables so the test also exercises those competing cuts.
        let mut rows = vec![
            Row {
                toneless_key: "king",
                hanji: "經",
                tl: "king",
                syll: 1,
                freq: 1,
            },
            Row {
                toneless_key: "sin",
                hanji: "身",
                tl: "sin",
                syll: 1,
                freq: 1,
            },
            Row {
                toneless_key: "ki",
                hanji: "基",
                tl: "ki",
                syll: 1,
                freq: 1,
            },
            Row {
                toneless_key: "kin",
                hanji: "巾",
                tl: "kin",
                syll: 1,
                freq: 1,
            },
            Row {
                toneless_key: "si",
                hanji: "詩",
                tl: "si",
                syll: 1,
                freq: 1,
            },
        ];
        if kind == "selected" {
            rows.push(Row {
                toneless_key: "kingsin",
                hanji: "更新",
                tl: "king-sin",
                syll: 2,
                freq: 1,
            });
        }
        let records: Vec<_> = rows
            .iter()
            .map(|row| TkdbRow {
                bitmask: 1,
                frequency: 1,
                syllable_count: Some(row.syll),
                kautian_subtag: Some(0),
                walker_cost: Some(if row.syll == 1 { 5_000 } else { 13_000 }),
                hanji: row.hanji,
                tl: row.tl,
            })
            .collect();
        let fst = build_dictionary_fst_tl_toned(&rows);
        let syllables = build_syllables_fst_tl(&["ki1", "kin1", "king1", "si1", "sin1"]);
        let associations = write_temp("model-cost-associations.bin", &empty_association_bin());
        for (tokens, expected) in [
            (3_627_713, "更新"),
            (1_000_000_000, "經身"),
            (3_627_713, "更新"),
        ] {
            let model = TkdbModel {
                tokens,
                ..Default::default()
            };
            let dictionary = write_temp(
                "model-cost-dictionary.bin",
                &build_tkdb_with_model(b"TKDB", TKDB_VERSION, &records, model),
            );
            install_lexicon(&fst, &dictionary, &associations, &syllables);
            let mut fetch = Fetch {
                now_ms: NOW_MS,
                ..Default::default()
            };
            match kind {
                "selected" => fetch.frequency = vec![selected("更新", "king-sin", 1, 0)],
                "custom" => {
                    fetch.custom = vec![CustomEntry {
                        roman: "king-sin".into(),
                        hanji: Some("更新".into()),
                    }]
                }
                "learned" => {
                    fetch.learned = vec![LearnedEntry {
                        canonical_tl: "king-sin".into(),
                        hanji: "更新".into(),
                    }]
                }
                _ => unreachable!(),
            }
            let hanji = fetch_hanji("kingsin", "tl", fetch);
            assert_eq!(hanji[0], expected, "{kind}, tokens={tokens}: {hanji:?}");
        }
    }
}
