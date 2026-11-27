//! Integration tests for `AppConfig.khinsiann_in_hanji` (opt-in neutral tone `--`
//! in Hanji output), same-commit-Hanji candidate merging, user-vs-builtin collisions,
//! `display_text` identity preservation, `learned_phrase` stripping, and
//! typed `--` boundary rules.

use crate::common::{
    build_dictionary_fst, build_tkdb_v4, cell_with_hanji, commit_text, config,
    empty_association_bin, fetch_at_pos_response, install_lexicon, Fetch, Row,
};
use composing::api::{CommitScript, Engine};
use composing::{requests, Intent, Phase};
use lexicon::{CustomEntry, LearnedEntry};
use protos::engine::{AppConfig, CandidateDisplayMode, CandidateMessage, SyllableSeparator};
use test_support::{engine_install_lock, write_temp};

fn fixture_rows() -> Vec<Row> {
    vec![
        Row {
            toneless_key: "tngkhi",
            hanji: "轉去",
            tl: "tńg--khì",
            syll: 2,
            freq: 500,
        },
        Row {
            toneless_key: "tngkhi",
            hanji: "轉去",
            tl: "tńg-khì",
            syll: 2,
            freq: 400,
        },
        Row {
            toneless_key: "tngkhi",
            hanji: "轉去",
            tl: "--tńg-khì",
            syll: 2,
            freq: 900,
        },
        Row {
            toneless_key: "ekitit",
            hanji: "會記得",
            tl: "ē-kì--tit",
            syll: 3,
            freq: 800,
        },
        Row {
            toneless_key: "ekilit",
            hanji: "會記得",
            tl: "ē-kì--lit",
            syll: 3,
            freq: 700,
        },
        Row {
            toneless_key: "ekilit",
            hanji: "會記得",
            tl: "ē-kì--lìt",
            syll: 3,
            freq: 600,
        },
        Row {
            toneless_key: "tsau",
            hanji: "走",
            tl: "tsáu",
            syll: 1,
            freq: 2000,
        },
        Row {
            toneless_key: "hoo",
            hanji: "予",
            tl: "hōo",
            syll: 1,
            freq: 2000,
        },
        Row {
            toneless_key: "gua",
            hanji: "我",
            tl: "guá",
            syll: 1,
            freq: 2000,
        },
        Row {
            toneless_key: "ah",
            hanji: "矣",
            tl: "ah",
            syll: 1,
            freq: 2000,
        },
    ]
}

fn install_fixture() {
    let rows = fixture_rows();
    let dict_path = write_temp("khinsiann-dict.bin", &build_tkdb_v4(&rows));
    let fst_path = build_dictionary_fst(&rows);
    let assoc_path = write_temp("khinsiann-assoc.bin", &empty_association_bin());
    let inv_path = write_temp(
        "khinsiann-syll.fst",
        &std::fs::read(crate::common::build_syllables_fst(&[
            "tng2", "khi3", "e7", "ki3", "tit4", "lit4", "lit8", "tsau2", "hoo7", "gua2", "ah4",
        ]))
        .unwrap(),
    );
    install_lexicon(&fst_path, &dict_path, &assoc_path, &inv_path);
}

fn hanji_config(khinsiann_in_hanji: bool, output_both_scripts: bool) -> AppConfig {
    AppConfig {
        is_hanji_first: true,
        output_both_scripts,
        khinsiann_in_hanji,
        candidate_display_mode: CandidateDisplayMode::SideBySide as i32,
        ..config("tl")
    }
}

fn fetch_candidates(cfg: &AppConfig, raw: &str, custom: Vec<CustomEntry>) -> Vec<CandidateMessage> {
    fetch_at_pos_response(
        cfg,
        raw,
        Fetch {
            literal_roman_candidate_disabled: true,
            custom,
            ..Default::default()
        },
    )
    .continuous
    .expect("continuous response")
    .candidates
}

fn pick_candidate(
    engine: &mut Engine,
    cand: &CandidateMessage,
    cfg: &AppConfig,
    script: CommitScript,
) -> composing::Applied {
    engine.apply_learning(
        Intent::CommitContinuous {
            canonical_text: cand.display_text.clone(),
            association_tl: cand.canonical_tl.clone(),
            hanji: cand.hanji.clone(),
            consumed_bytes: cand.consumed_span_end as usize,
            syllable_count: cand.syllable_count as u8,
            script: Some(script),
            roman: cand.roman.clone(),
        },
        cfg,
    )
}

#[test]
fn leading_khinsiann_matches_without_a_typed_marker_and_commits_its_identity() {
    let _lock = engine_install_lock();
    install_fixture();
    for mode in ["tl", "poj"] {
        let mut cfg = hanji_config(true, false);
        cfg.input_mode = mode.into();
        for (raw, expected) in [
            ("tngkhi", vec!["--tńg-khì", "tńg--khì", "tńg-khì"]),
            ("tng-khi", vec!["--tńg-khì", "tńg-khì"]),
            ("tng--khi", vec!["tńg--khì"]),
            ("tkh", vec!["--tńg-khì", "tńg--khì", "tńg-khì"]),
        ] {
            let cands = fetch_candidates(&cfg, raw, vec![]);
            let mut actual: Vec<_> = cands.iter().map(|c| c.canonical_tl.as_str()).collect();
            actual.sort_unstable();
            assert_eq!(actual, expected, "{mode} {raw}: {cands:?}");
        }
        let cand = fetch_candidates(&cfg, "tngkhi", vec![])
            .into_iter()
            .find(|c| c.canonical_tl == "--tńg-khì")
            .expect("leading neutral reading reachable without typing --");
        assert_eq!(cand.hanji.as_deref(), Some("--轉-去"));
        assert_eq!(cand.roman, "--tńg-khì");
        assert_eq!(cand.display_text, "轉去");
        for (separator, expected_roman) in [
            (SyllableSeparator::None, "·tńgkhì"),
            (SyllableSeparator::Space, "·tńg khì"),
        ] {
            cfg.syllable_separator = separator as i32;
            let rendered = fetch_candidates(&cfg, "tngkhi", vec![])
                .into_iter()
                .find(|c| c.canonical_tl == "--tńg-khì")
                .expect("leading neutral reading survives separator rendering");
            assert_eq!(rendered.roman, expected_roman);
            assert_eq!(rendered.hanji.as_deref(), Some("--轉-去"));
        }
        cfg.syllable_separator = SyllableSeparator::Hyphen as i32;
        for (script, expected) in [
            (CommitScript::Hanji, "--轉-去"),
            (CommitScript::Roman, "--tńg-khì"),
        ] {
            let mut engine = Engine::new();
            engine.apply(
                Intent::Start {
                    text: "tngkhi".into(),
                },
                &cfg,
            );
            let committed = pick_candidate(&mut engine, &cand, &cfg, script);
            assert_eq!(commit_text(&committed.response).as_deref(), Some(expected));
        }
        cfg.khinsiann_in_hanji = false;
        cfg.is_hanji_first = false;
        let roman_first = fetch_candidates(&cfg, "tngkhi", vec![]);
        assert_eq!(roman_first.len(), 3, "{mode}: {roman_first:?}");
        assert!(roman_first.iter().any(|c| {
            c.canonical_tl == "--tńg-khì"
                && c.roman == "--tńg-khì"
                && c.hanji.as_deref() == Some("轉去")
        }));
    }
}

#[test]
fn leading_khinsiann_matches_custom_and_learned_entries_without_a_plain_counterpart() {
    let _lock = engine_install_lock();
    install_fixture();
    let cfg = hanji_config(true, false);
    // The fixture has 轉去 but no 返去: only the supplied user row can answer.
    for user_rows in [
        Fetch {
            custom: vec![CustomEntry {
                roman: "--tńg-khì".into(),
                hanji: Some("返去".into()),
            }],
            ..Default::default()
        },
        Fetch {
            learned: vec![LearnedEntry {
                hanji: "返去".into(),
                canonical_tl: "--tńg-khì".into(),
            }],
            ..Default::default()
        },
    ] {
        let cands = fetch_at_pos_response(&cfg, "tngkhi", user_rows)
            .continuous
            .expect("continuous response")
            .candidates;
        let cand = cands
            .iter()
            .find(|c| c.hanji.as_deref() == Some("--返-去"))
            .expect("user row reachable without typing --");
        assert_eq!(cand.canonical_tl, "--tńg-khì");
        assert_eq!(cand.display_text, "返去");
    }
}

#[test]
fn khinsiann_in_hanji_renders_merges_and_commits_with_plain_identity() {
    let _lock = engine_install_lock();
    install_fixture();

    // khinsiann_in_hanji = true, brackets off:
    // - `tngkhi`: all three readings, including `--tńg-khì`, have distinct
    //   displayed Hanji. No leading `--` needs to be typed.
    let on = hanji_config(true, false);
    let tng_on = fetch_candidates(&on, "tngkhi", vec![]);
    let tng_hanjis: Vec<&str> = tng_on.iter().filter_map(|c| c.hanji.as_deref()).collect();
    assert_eq!(tng_hanjis, vec!["--轉-去", "轉--去", "轉去"]);
    assert_eq!(
        tng_on[0].display_text, "轉去",
        "display_text learning identity stays plain Hanji"
    );

    // - `ekilit`: `ē-kì--lit` and `ē-kì--lìt` both render `會記--得` -> merged into 1 cell!
    let eki_on = fetch_candidates(&on, "ekilit", vec![]);
    let eki_hanjis: Vec<&str> = eki_on.iter().filter_map(|c| c.hanji.as_deref()).collect();
    assert_eq!(eki_hanjis, vec!["會記--得"]);
    assert_eq!(eki_on[0].display_text, "會記得");

    // khinsiann_in_hanji = false, brackets off:
    // - `tngkhi`: all three readings display `轉去` -> merged into one cell.
    let off = hanji_config(false, false);
    let tng_off = fetch_candidates(&off, "tngkhi", vec![]);
    let tng_off_hanjis: Vec<&str> = tng_off.iter().filter_map(|c| c.hanji.as_deref()).collect();
    assert_eq!(tng_off_hanjis, vec!["轉去"]);

    // Commit with `output_both_scripts = true` + `khinsiann_in_hanji = true` -> `會記--得 (ē-kì--tit)`
    let brackets_on = hanji_config(true, true);
    let mut engine = Engine::new();
    engine.apply(
        Intent::Start {
            text: "ekitit".into(),
        },
        &brackets_on,
    );
    let cands = requests::apply(
        Fetch {
            literal_roman_candidate_disabled: true,
            ..Default::default()
        }
        .intent(),
        &mut engine,
        &brackets_on,
    )
    .continuous
    .unwrap()
    .candidates;
    let cand = cands
        .iter()
        .find(|c| c.hanji.as_deref() == Some("會記--得"))
        .expect("會記--得 present");
    let committed = pick_candidate(&mut engine, cand, &brackets_on, CommitScript::Lead);
    assert_eq!(
        commit_text(&committed.response).as_deref(),
        Some("會記--得 (ē-kì--tit)")
    );
}

#[test]
fn custom_khinsiann_collision_respects_toggle() {
    let _lock = engine_install_lock();
    install_fixture();
    let custom = vec![CustomEntry {
        roman: "ē-kì--tit".into(),
        hanji: Some("會記--得".into()),
    }];

    let on = hanji_config(true, false);
    let cands_on = fetch_candidates(&on, "ekitit", custom.clone());
    let hanjis_on: Vec<&str> = cands_on.iter().filter_map(|c| c.hanji.as_deref()).collect();
    assert_eq!(
        hanjis_on,
        vec!["會記--得"],
        "custom 會記--得 collapses with builtin 會記--得 when khinsiann_in_hanji is on"
    );

    let off = hanji_config(false, false);
    let cands_off = fetch_candidates(&off, "ekitit", custom);
    let hanjis_off: Vec<&str> = cands_off
        .iter()
        .filter_map(|c| c.hanji.as_deref())
        .collect();
    assert_eq!(
        hanjis_off,
        vec!["會記--得", "會記得"],
        "when off, custom 會記--得 stays first and builtin 會記得 remains second"
    );
}

#[test]
fn typed_leading_double_dash_from_idle_still_commits_literally() {
    let _lock = engine_install_lock();
    install_fixture();
    let on = hanji_config(true, false);

    let mut engine = Engine::new();
    let first = engine.apply(Intent::AppendHyphen, &on);
    assert_eq!(commit_text(&first).as_deref(), Some("-"));
    assert!(matches!(engine.snapshot_state().phase, Phase::Idle));

    let second = engine.apply(Intent::AppendHyphen, &on);
    assert_eq!(commit_text(&second).as_deref(), Some("-"));
    assert!(matches!(engine.snapshot_state().phase, Phase::Idle));

    // Subsequent `tngkhi` composes without the already committed hyphens.
    // The dictionary's leading `--` remains optional for matching.
    engine.apply(
        Intent::Start {
            text: "tngkhi".into(),
        },
        &on,
    );
    let cands = requests::apply(
        Fetch {
            literal_roman_candidate_disabled: true,
            ..Default::default()
        }
        .intent(),
        &mut engine,
        &on,
    )
    .continuous
    .unwrap()
    .candidates;
    let hanjis: Vec<&str> = cands.iter().filter_map(|c| c.hanji.as_deref()).collect();
    assert_eq!(hanjis, vec!["--轉-去", "轉--去", "轉去"]);
}

#[test]
fn typed_double_dash_in_composition_walk_and_nailing_and_learned_phrase() {
    let _lock = engine_install_lock();
    install_fixture();

    for (khinsiann, expected_sentence, expected_tail_pick) in
        [(true, "走--轉-去", "--轉-去"), (false, "走轉去", "轉去")]
    {
        let cfg = hanji_config(khinsiann, false);
        // 1. Full-sentence walk over `tsau--tngkhi`.
        let cells = crate::common::fetch_cells(
            &cfg,
            "tsau--tngkhi",
            Fetch {
                literal_roman_candidate_disabled: true,
                ..Default::default()
            },
        );
        assert_eq!(
            cells[0].0.as_deref(),
            Some(expected_sentence),
            "khinsiann={khinsiann}: full-sentence walker slot0"
        );
        // Check `cell_with_hanji` helper sanity (not strictly needed since cells[0] matches).
        let _ = cell_with_hanji(&cells, expected_sentence);

        // 2. Nail `走` first, then select from `--tngkhi` tail.
        let mut engine = Engine::new();
        engine.apply(
            Intent::Start {
                text: "tsau--tngkhi".into(),
            },
            &cfg,
        );
        let list1 = requests::apply(
            Fetch {
                literal_roman_candidate_disabled: true,
                ..Default::default()
            }
            .intent(),
            &mut engine,
            &cfg,
        )
        .continuous
        .unwrap()
        .candidates;
        let tsau = list1
            .iter()
            .find(|c| c.hanji.as_deref() == Some("走"))
            .expect("走 candidate");
        let mid = pick_candidate(&mut engine, tsau, &cfg, CommitScript::Lead);
        assert!(mid.response.is_composing);

        // Now raw pending tail is `--tngkhi`: `--tńg-khì` is admitted alongside
        // `tńg--khì` and `tńg-khì`. With `khinsiann_in_hanji` on, `--tńg-khì`
        // and `tńg-khì` both render `--轉-去` and merge into 1 cell while
        // `tńg--khì` renders `--轉--去`; with it off, all 3 merge into `轉去`.
        let list2 = requests::apply(
            Fetch {
                literal_roman_candidate_disabled: true,
                ..Default::default()
            }
            .intent(),
            &mut engine,
            &cfg,
        )
        .continuous
        .unwrap()
        .candidates;
        let tail_hanjis: Vec<&str> = list2.iter().filter_map(|c| c.hanji.as_deref()).collect();
        if khinsiann {
            assert_eq!(tail_hanjis, vec!["--轉-去", "--轉--去"]);
        } else {
            assert_eq!(tail_hanjis, vec!["轉去"]);
        }
        let tail_cand = list2
            .iter()
            .find(|c| {
                c.hanji.as_deref() == Some(expected_tail_pick)
                    && (!khinsiann || c.canonical_tl == "--tńg-khì")
            })
            .unwrap_or_else(|| panic!("missing {expected_tail_pick} in {list2:?}"));
        let fin = pick_candidate(&mut engine, tail_cand, &cfg, CommitScript::Lead);
        assert_eq!(
            commit_text(&fin.response).as_deref(),
            Some(expected_sentence),
            "khinsiann={khinsiann}: nailed prefix commit"
        );
        let learned = fin.learned.expect("learned phrase emitted");
        assert_eq!(
            learned.hanji, "走轉去",
            "learned_phrase always strips pure-Hanji neutral-tone hyphens"
        );
        assert_eq!(learned.canonical_tl, "tsáu--tńg-khì");
    }
}

#[test]
fn typed_double_dash_between_normal_words_emits_marker_when_enabled() {
    let _lock = engine_install_lock();
    install_fixture();

    for (khinsiann, expected) in [(true, "予--我"), (false, "予我")] {
        let cfg = hanji_config(khinsiann, false);
        let cands = fetch_candidates(&cfg, "hoo--gua", vec![]);
        assert_eq!(
            cands[0].hanji.as_deref(),
            Some(expected),
            "khinsiann={khinsiann}: walker slot0 on hoo--gua"
        );

        let mut engine = Engine::new();
        engine.apply(
            Intent::Start {
                text: "hoo--gua".into(),
            },
            &cfg,
        );
        let list1 = requests::apply(
            Fetch {
                literal_roman_candidate_disabled: true,
                ..Default::default()
            }
            .intent(),
            &mut engine,
            &cfg,
        )
        .continuous
        .unwrap()
        .candidates;
        let hoo = list1
            .iter()
            .find(|c| c.hanji.as_deref() == Some("予"))
            .expect("予");
        pick_candidate(&mut engine, hoo, &cfg, CommitScript::Lead);

        let list2 = requests::apply(
            Fetch {
                literal_roman_candidate_disabled: true,
                ..Default::default()
            }
            .intent(),
            &mut engine,
            &cfg,
        )
        .continuous
        .unwrap()
        .candidates;
        let expected_tail_gua = if khinsiann { "--我" } else { "我" };
        let gua = list2
            .iter()
            .find(|c| c.hanji.as_deref() == Some(expected_tail_gua))
            .unwrap_or_else(|| panic!("missing {expected_tail_gua} in {list2:?}"));
        assert_eq!(gua.display_text, "我");
        let fin = pick_candidate(&mut engine, gua, &cfg, CommitScript::Lead);
        assert_eq!(
            commit_text(&fin.response).as_deref(),
            Some(expected),
            "khinsiann={khinsiann}: nailed hoo + --gua"
        );
    }
}
