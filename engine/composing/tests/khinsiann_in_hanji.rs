//! Integration tests for `AppConfig.khinsiann_in_hanji` (opt-in neutral tone `--`
//! in Hanji output), same-commit-Hanji candidate merging, user-vs-builtin collisions,
//! `display_text` identity preservation, `learned_phrase` stripping, and
//! typed `--` boundary rules.

use crate::common::{
    build_dictionary_fst, build_tkdb_v4, cell_with_hanji, commit_text, config,
    empty_association_bin, fetch_at_pos_response, install_lexicon, Fetch, Row,
};
use composing::api::{CommitScript, Engine};
use composing::{requests, Intent};
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
        Row {
            toneless_key: "sia",
            hanji: "寫",
            tl: "siá",
            syll: 1,
            freq: 2000,
        },
        Row {
            toneless_key: "tshutlai",
            hanji: "出來",
            tl: "tshut-lâi",
            syll: 2,
            freq: 1500,
        },
        Row {
            toneless_key: "tshim",
            hanji: "深",
            tl: "tshim",
            syll: 1,
            freq: 2000,
        },
        Row {
            toneless_key: "tsittiama",
            hanji: "一點仔",
            tl: "tsi̍t-tiám-á",
            syll: 3,
            freq: 1500,
        },
        Row {
            toneless_key: "kong",
            hanji: "講",
            tl: "kóng",
            syll: 1,
            freq: 2000,
        },
        Row {
            toneless_key: "tsit",
            hanji: "一",
            tl: "tsi̍t",
            syll: 1,
            freq: 2000,
        },
        Row {
            toneless_key: "siann",
            hanji: "聲",
            tl: "siann",
            syll: 1,
            freq: 2000,
        },
        Row {
            toneless_key: "kue",
            hanji: "過",
            tl: "kuè",
            syll: 1,
            freq: 2000,
        },
        Row {
            toneless_key: "bo",
            hanji: "無",
            tl: "bô",
            syll: 1,
            freq: 2000,
        },
        Row {
            toneless_key: "kui",
            hanji: "幾",
            tl: "kuí",
            syll: 1,
            freq: 2000,
        },
        Row {
            toneless_key: "kang",
            hanji: "工",
            tl: "kang",
            syll: 1,
            freq: 2000,
        },
        Row {
            toneless_key: "pangtanglai",
            hanji: "放重利",
            tl: "pàng tāng-lāi",
            syll: 3,
            freq: 1500,
        },
        Row {
            toneless_key: "tshintsiann",
            hanji: "親tsiânn",
            tl: "tshin-tsiânn",
            syll: 2,
            freq: 1500,
        },
        Row {
            toneless_key: "sikhae",
            hanji: "四腳--ê",
            tl: "sì-kha--ê",
            syll: 3,
            freq: 1500,
        },
        Row {
            toneless_key: "kiann",
            hanji: "行",
            tl: "kiânn",
            syll: 1,
            freq: 2000,
        },
        Row {
            toneless_key: "kuelai",
            hanji: "過來",
            tl: "kuè--lâi",
            syll: 2,
            freq: 1500,
        },
        Row {
            toneless_key: "kuelai",
            hanji: "過來",
            tl: "kuè-lâi",
            syll: 2,
            freq: 1200,
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
            "sia2", "tshut4", "lai5", "tshim1", "tsit8", "tiam2", "a2", "kong2", "siann1", "kue3",
            "bo5", "kui2", "kang1", "pang3", "tang7", "lai7", "tshin1", "tsiann5", "si3", "kha1",
            "e5", "kiann5", "tai5", "pak4", "lang5",
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

// INVARIANT_LEADING_HYPHENS_COMPOSE_IN_BUFFER (§21) — typing `--` from Idle
// stays in the buffer (`--` emits zero candidates, then `--tngkhi` lists
// `--轉-去` and `--轉--去` without duplicating `----`).
#[test]
fn typed_leading_double_dash_from_idle_composes_in_buffer() {
    let _lock = engine_install_lock();
    install_fixture();
    let on = hanji_config(true, false);

    let mut engine = Engine::new();
    let first = engine.apply(Intent::AppendHyphen, &on);
    assert!(first.is_composing);
    assert_eq!(commit_text(&first), None);
    assert_eq!(first.preedit.unwrap().raw_input, "-");

    let second = engine.apply(Intent::AppendHyphen, &on);
    assert!(second.is_composing);
    assert_eq!(commit_text(&second), None);
    assert_eq!(second.preedit.unwrap().raw_input, "--");

    let empty_fetch = requests::apply(Fetch::default().intent(), &mut engine, &on)
        .continuous
        .unwrap()
        .candidates;
    assert!(empty_fetch.is_empty(), "hyphen-only buffer lists nothing");

    for ch in "tngkhi".chars() {
        engine.apply(Intent::Append { ch: ch.to_string() }, &on);
    }
    assert_eq!(engine.snapshot(&on).preedit.unwrap().raw_input, "--tngkhi");

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
    assert_eq!(hanjis, vec!["--轉-去", "--轉--去"]);
    let romans: Vec<&str> = cands.iter().map(|c| c.roman.as_str()).collect();
    assert_eq!(romans, vec!["--tńg-khì", "--tńg--khì"]);

    let off = hanji_config(false, false);
    let cands_off = requests::apply(
        Fetch {
            literal_roman_candidate_disabled: true,
            ..Default::default()
        }
        .intent(),
        &mut engine,
        &off,
    )
    .continuous
    .unwrap()
    .candidates;
    let hanjis_off: Vec<&str> = cands_off
        .iter()
        .filter_map(|c| c.hanji.as_deref())
        .collect();
    assert_eq!(hanjis_off, vec!["--轉去"]);
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

fn fetch_with(cfg: &AppConfig, raw: &str, fetch: Fetch) -> Vec<CandidateMessage> {
    fetch_at_pos_response(
        cfg,
        raw,
        Fetch {
            literal_roman_candidate_disabled: true,
            ..fetch
        },
    )
    .continuous
    .expect("continuous response")
    .candidates
}

fn start_and_fetch(engine: &mut Engine, raw: &str, cfg: &AppConfig) -> Vec<CandidateMessage> {
    engine.apply(Intent::Start { text: raw.into() }, cfg);
    requests::apply(
        Fetch {
            literal_roman_candidate_disabled: true,
            ..Default::default()
        }
        .intent(),
        engine,
        cfg,
    )
    .continuous
    .expect("continuous response")
    .candidates
}

fn fetch_pending(engine: &mut Engine, cfg: &AppConfig) -> Vec<CandidateMessage> {
    requests::apply(
        Fetch {
            literal_roman_candidate_disabled: true,
            ..Default::default()
        }
        .intent(),
        engine,
        cfg,
    )
    .continuous
    .expect("continuous response")
    .candidates
}

fn find_hanji<'a>(cands: &'a [CandidateMessage], hanji: &str) -> &'a CandidateMessage {
    cands
        .iter()
        .find(|c| c.hanji.as_deref() == Some(hanji))
        .unwrap_or_else(|| panic!("missing {hanji:?} in {cands:?}"))
}

#[test]
fn non_khinsiann_dictionary_words_prepended_with_double_dash_add_middle_hyphens() {
    let _lock = engine_install_lock();
    install_fixture();

    // 1. Walker + nailing + learned phrase on 寫--出-來 and 深--一-點-仔.
    for (raw, head_hanji, expected_tail, expected_sentence, expected_learned_hanji, expected_tl) in [
        (
            "sia--tshutlai",
            "寫",
            "--出-來",
            "寫--出-來",
            "寫出來",
            "siá--tshut-lâi",
        ),
        (
            "tshim--tsittiama",
            "深",
            "--一-點-仔",
            "深--一-點-仔",
            "深一點仔",
            "tshim--tsi̍t-tiám-á",
        ),
    ] {
        let on = hanji_config(true, false);
        let cands_on = fetch_candidates(&on, raw, vec![]);
        assert_eq!(cands_on[0].hanji.as_deref(), Some(expected_sentence));
        assert_eq!(cands_on[0].display_text, expected_learned_hanji);
        assert_eq!(cands_on[0].roman, expected_tl);
        assert_eq!(cands_on[0].canonical_tl, expected_tl.replace("--", " "));

        let off = hanji_config(false, false);
        let cands_off = fetch_candidates(&off, raw, vec![]);
        assert_eq!(cands_off[0].hanji.as_deref(), Some(expected_learned_hanji));

        let mut engine = Engine::new();
        let list1 = start_and_fetch(&mut engine, raw, &on);
        let head = find_hanji(&list1, head_hanji);
        let mid = pick_candidate(&mut engine, head, &on, CommitScript::Lead);
        assert!(mid.response.is_composing);

        let list2 = fetch_pending(&mut engine, &on);
        let tail = find_hanji(&list2, expected_tail);
        assert_eq!(tail.hanji.as_deref(), Some(expected_tail));
        assert_ne!(tail.display_text, expected_tail);
        let fin = pick_candidate(&mut engine, tail, &on, CommitScript::Lead);
        assert_eq!(
            commit_text(&fin.response).as_deref(),
            Some(expected_sentence)
        );
        let learned = fin.learned.expect("learned phrase");
        assert_eq!(learned.hanji, expected_learned_hanji);
        assert_eq!(learned.canonical_tl, expected_tl);
    }

    // 2. Annotate in Brackets (both Hanji-first and Roman-first).
    for (hanji_first, expected) in [
        (true, "寫 (siá)--出-來 (--tshut-lâi)"),
        (false, "siá (寫)--tshut-lâi (--出-來)"),
    ] {
        let mut cfg = hanji_config(true, true);
        cfg.is_hanji_first = hanji_first;
        let mut engine = Engine::new();
        let list1 = start_and_fetch(&mut engine, "sia--tshutlai", &cfg);
        let head = find_hanji(&list1, "寫");
        pick_candidate(&mut engine, head, &cfg, CommitScript::Lead);
        let list2 = fetch_pending(&mut engine, &cfg);
        let tail = find_hanji(&list2, "--出-來");
        let fin = pick_candidate(&mut engine, tail, &cfg, CommitScript::Lead);
        assert_eq!(commit_text(&fin.response).as_deref(), Some(expected));
    }

    // 3. POJ mode renders Han-Lo romanization in POJ alongside `--` / `-`.
    let mut poj_cfg = hanji_config(true, false);
    poj_cfg.input_mode = "poj".into();
    let poj_cands = fetch_candidates(&poj_cfg, "sia--chhinchiann", vec![]);
    assert_eq!(
        poj_cands[0].hanji.as_deref(),
        Some("寫--親-chi\u{e2}\u{207f}")
    );
    let mut engine = Engine::new();
    let list1 = start_and_fetch(&mut engine, "sia--chhinchiann", &poj_cfg);
    pick_candidate(
        &mut engine,
        find_hanji(&list1, "寫"),
        &poj_cfg,
        CommitScript::Lead,
    );
    let list2 = fetch_pending(&mut engine, &poj_cfg);
    let tail = find_hanji(&list2, "--親-chi\u{e2}\u{207f}");
    assert_eq!(tail.display_text, "親tsiânn");
    let fin = pick_candidate(&mut engine, tail, &poj_cfg, CommitScript::Lead);
    assert_eq!(
        commit_text(&fin.response).as_deref(),
        Some("寫--親-chi\u{e2}\u{207f}")
    );
}

#[test]
fn non_dictionary_hyphen_chained_neutral_spans_render_in_walker_and_nailing() {
    let _lock = engine_install_lock();
    install_fixture();
    let on = hanji_config(true, false);

    // 1. 講--一-聲 (一聲 is not in the fixture): walker slot 0 = 講--一-聲.
    let walker = fetch_candidates(&on, "kong--tsit-siann", vec![]);
    assert_eq!(walker[0].hanji.as_deref(), Some("講--一-聲"));

    // Nailing segment-by-segment: list2 shows `--一`, list3 (`-siann`) shows `-聲`,
    // commits `講--一-聲`, and learns `("講一聲", "kóng--tsi̍t-siann")`.
    let mut engine = Engine::new();
    let list1 = start_and_fetch(&mut engine, "kong--tsit-siann", &on);
    pick_candidate(
        &mut engine,
        find_hanji(&list1, "講"),
        &on,
        CommitScript::Lead,
    );
    let list2 = fetch_pending(&mut engine, &on);
    pick_candidate(
        &mut engine,
        find_hanji(&list2, "--一"),
        &on,
        CommitScript::Lead,
    );
    let list3 = fetch_pending(&mut engine, &on);
    let siann = find_hanji(&list3, "-聲");
    assert_eq!(siann.display_text, "聲");
    let fin = pick_candidate(&mut engine, siann, &on, CommitScript::Lead);
    assert_eq!(commit_text(&fin.response).as_deref(), Some("講--一-聲"));
    let learned = fin.learned.expect("learned phrase emitted");
    assert_eq!(learned.hanji, "講一聲");
    assert_eq!(learned.canonical_tl, "kóng--tsi̍t-siann");

    // Recalling the learned phrase from `kongtsitsiann` surfaces `講--一-聲`.
    let recalled = fetch_with(
        &on,
        "kongtsitsiann",
        Fetch {
            learned: vec![learned],
            ..Default::default()
        },
    );
    let _ = find_hanji(&recalled, "講--一-聲");

    // 2. No typed `-` resets neutral tone at the word boundary.
    let reset = fetch_candidates(&on, "kong--tsitsiann", vec![]);
    assert_eq!(reset[0].hanji.as_deref(), Some("講--一聲"));

    // 3. 3-word chained neutral span: 過--無-幾-工 (walker + nailing).
    assert_eq!(
        fetch_candidates(&on, "kue--bo-kui-kang", vec![])[0]
            .hanji
            .as_deref(),
        Some("過--無-幾-工")
    );
    let mut eng3 = Engine::new();
    let l1 = start_and_fetch(&mut eng3, "kue--bo-kui-kang", &on);
    pick_candidate(&mut eng3, find_hanji(&l1, "過"), &on, CommitScript::Lead);
    let l2 = fetch_pending(&mut eng3, &on);
    pick_candidate(&mut eng3, find_hanji(&l2, "--無"), &on, CommitScript::Lead);
    let l3 = fetch_pending(&mut eng3, &on);
    pick_candidate(&mut eng3, find_hanji(&l3, "-幾"), &on, CommitScript::Lead);
    let l4 = fetch_pending(&mut eng3, &on);
    let fin3 = pick_candidate(&mut eng3, find_hanji(&l4, "-工"), &on, CommitScript::Lead);
    assert_eq!(commit_text(&fin3.response).as_deref(), Some("過--無-幾-工"));

    // 4. Continuing with `-` into a multi-syllable dictionary word (`講--一-出-來`).
    assert_eq!(
        fetch_candidates(&on, "kong--tsit-tshutlai", vec![])[0]
            .hanji
            .as_deref(),
        Some("講--一-出-來")
    );
    let mut eng4 = Engine::new();
    let l1 = start_and_fetch(&mut eng4, "kong--tsit-tshutlai", &on);
    pick_candidate(&mut eng4, find_hanji(&l1, "講"), &on, CommitScript::Lead);
    let l2 = fetch_pending(&mut eng4, &on);
    pick_candidate(&mut eng4, find_hanji(&l2, "--一"), &on, CommitScript::Lead);
    let l3 = fetch_pending(&mut eng4, &on);
    let fin4 = pick_candidate(
        &mut eng4,
        find_hanji(&l3, "-出-來"),
        &on,
        CommitScript::Lead,
    );
    assert_eq!(commit_text(&fin4.response).as_deref(), Some("講--一-出-來"));

    // 5. Continuing with `-` after a dictionary `--`-ending word (`會記--得-矣`).
    assert_eq!(
        fetch_candidates(&on, "ekitit-ah", vec![])[0]
            .hanji
            .as_deref(),
        Some("會記--得-矣")
    );
    let mut eng5 = Engine::new();
    let l1 = start_and_fetch(&mut eng5, "ekitit-ah", &on);
    pick_candidate(
        &mut eng5,
        find_hanji(&l1, "會記--得"),
        &on,
        CommitScript::Lead,
    );
    let l2 = fetch_pending(&mut eng5, &on);
    let fin5 = pick_candidate(&mut eng5, find_hanji(&l2, "-矣"), &on, CommitScript::Lead);
    assert_eq!(commit_text(&fin5.response).as_deref(), Some("會記--得-矣"));

    // 6. Plain `-` without an open neutral tone adds no hyphens in Hanji.
    assert_eq!(
        fetch_candidates(&on, "kong-tsit-siann", vec![])[0]
            .hanji
            .as_deref(),
        Some("講一聲")
    );
    assert_eq!(
        fetch_candidates(&on, "sia-tshutlai", vec![])[0]
            .hanji
            .as_deref(),
        Some("寫出來")
    );
}

#[test]
fn internal_double_dash_stays_distinct_and_custom_learned_space_rules_align() {
    let _lock = engine_install_lock();
    install_fixture();
    let on = hanji_config(true, false);

    // 1. Internal `--` is kept separate from `-`: `kuè--lâi` (1500) vs `kuè-lâi` (1200).
    let kuelai = fetch_candidates(&on, "kiann--kuelai", vec![]);
    assert_eq!(kuelai[0].hanji.as_deref(), Some("行--過--來"));
    let mut engine = Engine::new();
    let l1 = start_and_fetch(&mut engine, "kiann--kuelai", &on);
    pick_candidate(&mut engine, find_hanji(&l1, "行"), &on, CommitScript::Lead);
    let l2 = fetch_pending(&mut engine, &on);
    let tail_hanjis: Vec<&str> = l2.iter().filter_map(|c| c.hanji.as_deref()).collect();
    assert_eq!(tail_hanjis, vec!["--過--來", "--過-來", "--過"]);

    // Han-Lo with internal `--` (`四腳--ê` `sì-kha--ê`).
    assert_eq!(
        fetch_candidates(&on, "sia--sikhae", vec![])[0]
            .hanji
            .as_deref(),
        Some("寫--四-腳--ê")
    );

    // 2. Builtin space-separated entry resets neutral tone after the space.
    assert_eq!(
        fetch_candidates(&on, "sia--pangtanglai", vec![])[0]
            .hanji
            .as_deref(),
        Some("寫--放重利")
    );

    // 3. Custom and learned entries follow the same alignment and space-reset rules.
    let custom_point = vec![CustomEntry {
        roman: "tsi̍t-tiám-á".into(),
        hanji: Some("一点仔".into()),
    }];
    assert_eq!(
        fetch_with(
            &on,
            "tshim--tsittiama",
            Fetch {
                custom: custom_point.clone(),
                edge_custom: custom_point,
                ..Default::default()
            }
        )[0]
        .hanji
        .as_deref(),
        Some("深--一-点-仔")
    );

    let custom_internal_dd = vec![CustomEntry {
        roman: "ē-kì--tit".into(),
        hanji: Some("會--記得".into()),
    }];
    assert_eq!(
        fetch_with(
            &on,
            "sia--ekitit",
            Fetch {
                custom: custom_internal_dd.clone(),
                edge_custom: custom_internal_dd,
                ..Default::default()
            }
        )[0]
        .hanji
        .as_deref(),
        Some("寫--會--記得")
    );

    for (custom_hanji, expected) in [("台北人", "寫--台-北人"), ("台北 人", "寫--台-北 人")]
    {
        let row = vec![CustomEntry {
            roman: "tâi-pak lâng".into(),
            hanji: Some(custom_hanji.into()),
        }];
        assert_eq!(
            fetch_with(
                &on,
                "sia--taipaklang",
                Fetch {
                    custom: row.clone(),
                    edge_custom: row,
                    ..Default::default()
                }
            )[0]
            .hanji
            .as_deref(),
            Some(expected)
        );
    }

    // 4. Custom `--出-來` collapses with builtin `出來` under `--tshutlai`, even with brackets on.
    // Nail `寫` first so `--tshutlai` is mid-composing.
    let brackets_on = hanji_config(true, true);
    let mut eng_mid = Engine::new();
    let first = start_and_fetch(&mut eng_mid, "sia--tshutlai", &brackets_on);
    pick_candidate(
        &mut eng_mid,
        find_hanji(&first, "寫"),
        &brackets_on,
        CommitScript::Lead,
    );
    let custom_dup = vec![CustomEntry {
        roman: "tshut-lâi".into(),
        hanji: Some("--出-來".into()),
    }];
    let tail_cands = requests::apply(
        Fetch {
            literal_roman_candidate_disabled: true,
            custom: custom_dup,
            ..Default::default()
        }
        .intent(),
        &mut eng_mid,
        &brackets_on,
    )
    .continuous
    .expect("continuous")
    .candidates;
    assert_eq!(
        tail_cands
            .iter()
            .filter(|c| c.hanji.as_deref() == Some("--出-來"))
            .count(),
        1,
        "custom --出-來 must collapse with rendered builtin --出-來: {tail_cands:?}"
    );
}

#[test]
fn explain_mechanism_matches_mid_composing_khinsiann_words_and_slot0() {
    let _lock = engine_install_lock();
    install_fixture();
    let on = hanji_config(true, false);

    for (raw, expected_words, expected_sentence) in [
        ("sia--tshutlai", vec!["寫", "--出-來"], "寫--出-來"),
        ("kong--tsit-siann", vec!["講", "--一", "-聲"], "講--一-聲"),
    ] {
        let mut engine = Engine::new();
        engine.apply(Intent::Start { text: raw.into() }, &on);
        let mech = composing::explain::mechanism(
            &engine,
            0,
            &composing::UserRows::default(),
            ranking::ContextRanks::empty(),
            true,
            &on,
        )
        .expect("mechanism");
        let word_hanjis: Vec<&str> = mech
            .words
            .iter()
            .filter_map(|w| w.hanji.as_deref())
            .collect();
        assert_eq!(word_hanjis, expected_words);
        assert_eq!(mech.candidates[0].role, "sentence");
        assert_eq!(mech.candidates[0].hanji.as_deref(), Some(expected_sentence));
    }
}

#[test]
fn buffer_initial_double_dash_regression_matrix_never_duplicates() {
    let _lock = engine_install_lock();
    install_fixture();

    for khinsiann in [true, false] {
        for literal_disabled in [false, true] {
            for (input_mode, separator, display_mode, hanji_first, han_lo_mixed, both_scripts) in [
                (
                    "tl",
                    SyllableSeparator::Hyphen,
                    CandidateDisplayMode::SideBySide,
                    true,
                    false,
                    false,
                ),
                (
                    "tl",
                    SyllableSeparator::Hyphen,
                    CandidateDisplayMode::SideBySide,
                    false,
                    false,
                    false,
                ),
                (
                    "tl",
                    SyllableSeparator::Hyphen,
                    CandidateDisplayMode::RomanOnly,
                    false,
                    false,
                    false,
                ),
                (
                    "tl",
                    SyllableSeparator::Hyphen,
                    CandidateDisplayMode::Combined,
                    true,
                    false,
                    false,
                ),
                (
                    "tl",
                    SyllableSeparator::Hyphen,
                    CandidateDisplayMode::SideBySide,
                    true,
                    true,
                    true,
                ),
                (
                    "poj",
                    SyllableSeparator::Hyphen,
                    CandidateDisplayMode::SideBySide,
                    true,
                    false,
                    false,
                ),
                (
                    "tl",
                    SyllableSeparator::Space,
                    CandidateDisplayMode::SideBySide,
                    false,
                    false,
                    false,
                ),
                (
                    "tl",
                    SyllableSeparator::None,
                    CandidateDisplayMode::SideBySide,
                    false,
                    false,
                    false,
                ),
                (
                    "tps",
                    SyllableSeparator::Hyphen,
                    CandidateDisplayMode::SideBySide,
                    true,
                    false,
                    false,
                ),
            ] {
                let mut cfg = config(input_mode);
                cfg.khinsiann_in_hanji = khinsiann;
                cfg.candidate_display_mode = display_mode as i32;
                cfg.syllable_separator = separator as i32;
                cfg.is_hanji_first = hanji_first;
                cfg.han_lo_mixed = han_lo_mixed;
                cfg.output_both_scripts = both_scripts;

                for raw in ["--ah", "--tngkhi"] {
                    let cands = fetch_with(
                        &cfg,
                        raw,
                        Fetch {
                            literal_roman_candidate_disabled: literal_disabled,
                            ..Default::default()
                        },
                    );
                    assert!(
                        !cands.is_empty(),
                        "{raw} {input_mode} khinsiann={khinsiann}"
                    );
                    let mut seen_roman = std::collections::HashSet::new();
                    for c in &cands {
                        assert!(
                            !c.roman.contains("----"),
                            "roman duplicated `--`: {raw} {c:?}"
                        );
                        if let Some(h) = &c.hanji {
                            assert!(!h.contains("----"), "hanji duplicated `--`: {raw} {c:?}");
                        }
                        if !c.tps_text.is_empty() {
                            assert!(
                                !c.tps_text.contains("\u{02d9}\u{02d9}"),
                                "tps_text duplicated neutral mark: {raw} {c:?}"
                            );
                        }
                        if display_mode == CandidateDisplayMode::RomanOnly {
                            assert!(
                                seen_roman.insert(c.roman.clone()),
                                "duplicate roman cell under RomanOnly for {raw}: {cands:?}"
                            );
                        }
                    }

                    if raw == "--ah" {
                        let expected_roman = match separator {
                            SyllableSeparator::Space | SyllableSeparator::None
                                if input_mode != "tps" =>
                            {
                                "\u{00b7}ah"
                            }
                            _ => "--ah",
                        };
                        assert!(
                            cands
                                .iter()
                                .any(|c| c.hanji.as_deref() == Some("--矣") && c.roman == expected_roman),
                            "missing rendered --矣 / {expected_roman} for {raw} ({input_mode}, {separator:?}): {cands:?}"
                        );
                        if !literal_disabled && input_mode != "tps" {
                            assert_eq!(cands[0].roman, expected_roman);
                        }
                    }
                }
            }
        }
    }

    // Space / None separators also shape `--tngkhi`, and custom `--返-去` never doubles `--`.
    for (separator, expected_roman) in [
        (SyllableSeparator::Hyphen, "--tńg-khì"),
        (SyllableSeparator::Space, "\u{00b7}tńg khì"),
        (SyllableSeparator::None, "\u{00b7}tńgkhì"),
    ] {
        let mut cfg = hanji_config(true, false);
        cfg.syllable_separator = separator as i32;
        let cands = fetch_candidates(&cfg, "--tngkhi", vec![]);
        assert_eq!(cands[0].hanji.as_deref(), Some("--轉-去"));
        assert_eq!(cands[0].roman, expected_roman);
    }

    for khinsiann in [true, false] {
        let cfg = hanji_config(khinsiann, false);
        let custom = vec![CustomEntry {
            roman: "--tńg-khì".into(),
            hanji: Some("--返-去".into()),
        }];
        let cands = fetch_candidates(&cfg, "--tngkhi", custom);
        assert!(
            cands
                .iter()
                .all(|c| !c.hanji.as_deref().unwrap_or_default().contains("----")),
            "khinsiann={khinsiann}: {cands:?}"
        );
        assert!(
            cands.iter().any(|c| c.hanji.as_deref() == Some("--返-去")),
            "custom --返-去 missing for khinsiann={khinsiann}: {cands:?}"
        );
    }

    // TPS layout carries `--` onto `tps_text` (`--矣` for Hanji, `˙ㄉㆭˋㄎㄧ˪` for a roman-only row).
    let tps_cfg = config("tps");
    let tps_cands = fetch_candidates(
        &tps_cfg,
        "--tngkhi",
        vec![CustomEntry {
            roman: "tńg-khì".into(),
            hanji: None,
        }],
    );
    assert!(
        tps_cands
            .iter()
            .any(|c| c.tps_text == "\u{02d9}ㄉㆭˋ \u{02d9}ㄎㄧ˪"),
        "expected ˙ㄉㆭˋ ˙ㄎㄧ˪ in tps_text: {tps_cands:?}"
    );
    assert!(
        tps_cands.iter().any(|c| c.tps_text == "--轉去"),
        "expected --轉去 in tps_text: {tps_cands:?}"
    );
}

#[test]
fn buffer_initial_double_dash_matches_pr_1171_mid_composing_rules() {
    let _lock = engine_install_lock();
    install_fixture();

    // 1. `--tshutlai` across khinsiann toggle, POJ, Space, and None.
    let on = hanji_config(true, false);
    let off = hanji_config(false, false);
    let cands_on = fetch_candidates(&on, "--tshutlai", vec![]);
    assert_eq!(cands_on[0].hanji.as_deref(), Some("--出-來"));
    assert_eq!(cands_on[0].roman, "--tshut-lâi");
    let cands_off = fetch_candidates(&off, "--tshutlai", vec![]);
    assert_eq!(cands_off[0].hanji.as_deref(), Some("--出來"));
    assert_eq!(cands_off[0].roman, "--tshut-lâi");

    let mut poj_cfg = hanji_config(true, false);
    poj_cfg.input_mode = "poj".into();
    let poj_cands = fetch_candidates(&poj_cfg, "--chhutlai", vec![]);
    assert_eq!(poj_cands[0].hanji.as_deref(), Some("--出-來"));
    assert_eq!(poj_cands[0].roman, "--chhut-lâi");

    for (sep, expected_roman) in [
        (SyllableSeparator::Space, "\u{00b7}tshut lâi"),
        (SyllableSeparator::None, "\u{00b7}tshutlâi"),
    ] {
        let mut sep_cfg = hanji_config(true, false);
        sep_cfg.syllable_separator = sep as i32;
        let c = fetch_candidates(&sep_cfg, "--tshutlai", vec![]);
        assert_eq!(c[0].roman, expected_roman);
        assert_eq!(c[0].hanji.as_deref(), Some("--出-來"));
    }

    // 2. `--tsit-siann`: slot 0 chains `-` (`--一-聲`), nailing learns `--tsi̍t-siann`.
    assert_eq!(
        fetch_candidates(&on, "--tsit-siann", vec![])[0]
            .hanji
            .as_deref(),
        Some("--一-聲")
    );
    assert_eq!(
        fetch_candidates(&off, "--tsit-siann", vec![])[0]
            .hanji
            .as_deref(),
        Some("--一聲")
    );
    let mut engine = Engine::new();
    let list1 = start_and_fetch(&mut engine, "--tsit-siann", &on);
    let first = find_hanji(&list1, "--一");
    let mid = pick_candidate(&mut engine, first, &on, CommitScript::Lead);
    assert!(mid.response.is_composing);
    let list2 = fetch_pending(&mut engine, &on);
    let second = find_hanji(&list2, "-聲");
    let fin = pick_candidate(&mut engine, second, &on, CommitScript::Lead);
    assert_eq!(commit_text(&fin.response).as_deref(), Some("--一-聲"));
    let learned = fin.learned.expect("learned phrase");
    assert_eq!(learned.hanji, "一聲");
    assert_eq!(learned.canonical_tl, "--tsi̍t-siann");

    // 3. `--kuelai` keeps internal `--` distinct; `--pangtanglai` resets on word space.
    let kuelai: Vec<Option<String>> = fetch_candidates(&on, "--kuelai", vec![])
        .into_iter()
        .map(|c| c.hanji)
        .collect();
    assert_eq!(
        &kuelai[..2],
        &[Some("--過--來".to_owned()), Some("--過-來".to_owned())]
    );
    assert_eq!(
        fetch_candidates(&on, "--pangtanglai", vec![])[0]
            .hanji
            .as_deref(),
        Some("--放重利")
    );

    // 4. POJ Han-Lo at buffer start: `--chhinchiann` -> `--親-chiâⁿ`.
    let chhin = fetch_candidates(&poj_cfg, "--chhinchiann", vec![]);
    assert_eq!(chhin[0].hanji.as_deref(), Some("--親-chi\u{e2}\u{207f}"));

    // 5. Casing preserves leading `--` (`--Tngkhi` -> `--Tńg-khì`, `--TNGKHI` -> `--TŃG-KHÌ`).
    let title = fetch_candidates(&on, "--Tngkhi", vec![]);
    assert_eq!(title[0].roman, "--Tńg-khì");
    let upper = fetch_candidates(&on, "--TNGKHI", vec![]);
    assert_eq!(upper[0].roman, "--T\u{0143}G-KH\u{00cc}");
}

#[test]
fn buffer_initial_single_and_triple_hyphens_and_mid_composing_control() {
    let _lock = engine_install_lock();
    install_fixture();
    let on = hanji_config(true, false);
    let off = hanji_config(false, false);

    // Buffer-initial single `-`: keeps literal `-` prefix, no neutral-tone interior `-`.
    for cfg in [&on, &off] {
        let cands = fetch_candidates(cfg, "-tshutlai", vec![]);
        assert_eq!(cands[0].hanji.as_deref(), Some("-出來"));
        assert_eq!(cands[0].roman, "-tshut-lâi");
        assert_eq!(cands[0].display_text, "出來");
    }

    // `-tngkhi`: rejects `--tńg-khì` (§52), keeps `-轉去` and `-轉--去`.
    let tng_on: Vec<String> = fetch_candidates(&on, "-tngkhi", vec![])
        .into_iter()
        .filter_map(|c| c.hanji)
        .collect();
    assert!(
        !tng_on.iter().any(|h| h == "--轉-去"),
        "single `-` must reject --tńg-khì: {tng_on:?}"
    );
    assert_eq!(tng_on, vec!["-轉--去".to_owned(), "-轉去".to_owned()]);

    // Triple `---ah`: roman keeps `---ah`; Hanji shows `--矣` (`true`) / `---矣` (`false`).
    let triple_on = fetch_candidates(&on, "---ah", vec![]);
    assert_eq!(triple_on[0].hanji.as_deref(), Some("--矣"));
    assert_eq!(triple_on[0].roman, "---ah");
    let triple_off = fetch_candidates(&off, "---ah", vec![]);
    assert_eq!(triple_off[0].hanji.as_deref(), Some("---矣"));
    assert_eq!(triple_off[0].roman, "---ah");

    // Control: mid-composing single `-` remains a syllable separator (`寫出來`, not `寫-出來`).
    for cfg in [&on, &off] {
        let cands = fetch_candidates(cfg, "sia-tshutlai", vec![]);
        assert_eq!(cands[0].hanji.as_deref(), Some("寫出來"));
    }
}

#[test]
fn hyphen_only_buffers_emit_zero_candidates_and_commit_cleanly() {
    let _lock = engine_install_lock();
    install_fixture();
    let cfg = config("tl");

    for count in [1usize, 2, 3] {
        let expected_raw = "-".repeat(count);
        let mut engine = Engine::new();
        for _ in 0..count {
            let resp = engine.apply(Intent::AppendHyphen, &cfg);
            assert!(resp.is_composing);
            assert_eq!(commit_text(&resp), None);
        }
        assert_eq!(
            engine.snapshot(&cfg).preedit.unwrap().raw_input,
            expected_raw
        );

        // Zero candidates even when literal roman candidate is enabled.
        let fetch = requests::apply(Fetch::default().intent(), &mut engine, &cfg)
            .continuous
            .unwrap();
        assert!(
            fetch.candidates.is_empty(),
            "hyphen-only {expected_raw:?} must emit 0 candidates: {:?}",
            fetch.candidates
        );

        // `CommitRaw` commits the exact hyphen run.
        let committed = engine.apply(Intent::CommitRaw, &cfg);
        assert!(!committed.is_composing);
        assert_eq!(
            commit_text(&committed).as_deref(),
            Some(expected_raw.as_str())
        );
    }

    // Backspace from `"-"` returns cleanly to Idle with no document commit.
    let mut back_eng = Engine::new();
    back_eng.apply(Intent::AppendHyphen, &cfg);
    let back = back_eng.apply(Intent::DeleteBackward, &cfg);
    assert!(!back.is_composing);
    assert_eq!(commit_text(&back), None);

    // Space confirm-then-insert on `"-"` writes `"- "` without arming an auto-space swap.
    let mut space_eng = Engine::new();
    space_eng.apply(Intent::AppendHyphen, &cfg);
    let space_resp = space_eng.apply(
        Intent::ConfirmCompositionThenInsert {
            candidate: None,
            text: " ".into(),
            auto_space_enabled: true,
            candidate_spacing_enabled: true,
        },
        &cfg,
    );
    assert!(!space_resp.is_composing);
    assert_eq!(commit_text(&space_resp).as_deref(), Some("- "));
    assert!(!space_resp.commit.unwrap().leaves_trailing_auto_space);
}

#[test]
fn explain_mechanism_matches_buffer_initial_leading_hyphens() {
    let _lock = engine_install_lock();
    install_fixture();
    let on = hanji_config(true, false);

    for (raw, expected_words, expected_sentence) in [
        ("--tshutlai", vec!["--出-來"], "--出-來"),
        ("--tsit-siann", vec!["--一", "-聲"], "--一-聲"),
        ("-tshutlai", vec!["-出來"], "-出來"),
    ] {
        let mut engine = Engine::new();
        engine.apply(Intent::Start { text: raw.into() }, &on);
        let mech = composing::explain::mechanism(
            &engine,
            0,
            &composing::UserRows::default(),
            ranking::ContextRanks::empty(),
            true,
            &on,
        )
        .expect("mechanism");
        let word_hanjis: Vec<&str> = mech
            .words
            .iter()
            .filter_map(|w| w.hanji.as_deref())
            .collect();
        assert_eq!(word_hanjis, expected_words, "{raw}");
        assert_eq!(
            mech.candidates[0].hanji.as_deref(),
            Some(expected_sentence),
            "{raw}"
        );
    }
}
