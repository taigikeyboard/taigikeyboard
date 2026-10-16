//! Learned phrases (§50) — a phrase the user composed segment by segment
//! in continuous input becomes a whole-buffer candidate next time.
//!
//! Two halves, pinned together here:
//!
//! 1. **Learning** — the final `CommitContinuous` of a composition whose
//!    every nailed segment was a hanji pick teaches a phrase (`Applied.learned`)
//!    with the concatenated hanji and the `-`-joined canonical TL. Nothing
//!    is taught mid-commit, for a single segment, for a hanji-less
//!    segment, or past six syllables.
//! 2. **Recall** — a `UserRows.learned` row whose key equals the
//!    typed buffer leads the hanji candidates when the dictionary has no
//!    word under that key, competes (and loses) against a dictionary
//!    homophone unless `user_frequency` prefers it, dedupes against the
//!    same pair from the dictionary, and never-selected never outranks a
//!    never-selected manual custom row (Codex 2026-09-20 F5: learned =
//!    competitor; custom rows compete too since USER 2026-10-16, ahead of
//!    the never-selected rest).
//!
//! Fixture mirrors the USER report (2026-09-20): 記起來 `kì--khí-lâi` is a
//! 台日-only dictionary word (default off), so a default user composes it as
//! 記 `ki` + 起來 `khilai`; the dictionary here carries 機/ki, 記/kì, 起來 and
//! NOT 記起來.

use composing::api::Engine;
use composing::{requests, CommitScript, Intent, Phase};
use protos::engine::composing_request::Method;
use protos::engine::{CandidateMessage, CommitContinuous, FetchAtPos, Start};

use crate::common;
use crate::common::Fetch;
use crate::common::{
    build_dictionary_fst, build_syllables_fst, build_tkdb_v4, config_tl, effect_kinds,
    empty_association_bin, engine_in_continuous, fetch_at_pos_response, fetch_cells, fetch_hanji,
    install_lexicon, req, selected, Cell, Row, NOW_MS,
};
use lexicon::{CustomEntry, LearnedEntry};
use test_support::{engine_install_lock, write_temp};

// ---- Learning ---------------------------------------------------------------

/// One `CommitContinuous`: `(hanji, canonical_tl, consumed_bytes, syllable_count)`.
type Pick<'a> = (Option<&'a str>, &'a str, usize, u8);

fn pick((hanji, tl, consumed_bytes, syllable_count): Pick<'_>) -> Intent {
    Intent::CommitContinuous {
        canonical_text: hanji.unwrap_or(tl).to_string(),
        association_tl: tl.to_string(),
        hanji: hanji.map(str::to_string),
        consumed_bytes,
        syllable_count,
        script: Some(CommitScript::Roman),
        roman: hanji.unwrap_or(tl).to_string(),
    }
}

/// Apply every pick over `raw` and return what the LAST commit learned,
/// against the fixture lexicon: the commit and the learned reading both
/// group words through its compound oracle (起來 is a word, 記起 is not).
fn learn(raw: &str, picks: &[Pick<'_>]) -> Option<LearnedEntry> {
    let _lock = engine_install_lock();
    install(&fixture_rows(), FIXTURE_SYLLABLES);
    learn_installed(raw, picks)
}

/// [`learn`] against the lexicon the caller installed under the lock.
fn learn_installed(raw: &str, picks: &[Pick<'_>]) -> Option<LearnedEntry> {
    let mut e = engine_in_continuous(raw);
    let mut last = None;
    for p in picks {
        last = Some(e.apply_learning(pick(*p), &config_tl()));
    }
    last.expect("at least one pick").learned
}

fn phrase(hanji: &str, canonical_tl: &str) -> Option<LearnedEntry> {
    Some(LearnedEntry {
        hanji: hanji.into(),
        canonical_tl: canonical_tl.into(),
    })
}

#[test]
fn final_commit_of_hanji_picks_learns_the_joined_phrase() {
    let _lock = engine_install_lock();
    install(&fixture_rows(), FIXTURE_SYLLABLES);
    let mut e = engine_in_continuous("kikhilai");
    let mid = e.apply_learning(pick((Some("記"), "kì", 2, 1)), &config_tl());
    assert!(
        mid.learned.is_none(),
        "mid-commit must not learn; got {:?}",
        mid.learned
    );
    assert!(matches!(e.snapshot_state().phase, Phase::Continuous { .. }));

    let fin = e.apply_learning(pick((Some("起來"), "khí-lâi", 6, 2)), &config_tl());
    assert_eq!(
        effect_kinds(&fin.response.effect),
        vec![
            "CommitTextReplacingPreedit",
            "ClearCandidates",
            "ResetCandidateContext",
            "NextWordWordSelected",
        ]
    );
    // Two words, as the commit renders them: 記 + the dictionary word 起來.
    assert_eq!(fin.learned, phrase("記起來", "kì khí-lâi"));
    assert!(matches!(e.snapshot_state().phase, Phase::Idle));
}

#[test]
fn only_a_dictionary_compound_run_learns_its_hyphens() {
    // USER 2026-09-23: `tsotsintshutkhau` picked 做 → 進 → 出 → 口 learned
    // `tsò-tsìn-tshut-kháu`, but 做 and 進出口 are two words.
    // trace: longest_compound_run from 做: 做進出口 / 做進出 / 做進 not in
    // the fixture → 1, space; from 進: 進出口 (n = 3) is → `tsìn-tshut-kháu`.
    let rows = [
        ("tso", "做", "tsò", 1, 5000),
        ("tsin", "進", "tsìn", 1, 3000),
        ("tshut", "出", "tshut", 1, 8000),
        ("khau", "口", "kháu", 1, 4000),
        ("tsintshutkhau", "進出口", "tsìn-tshut-kháu", 3, 100),
    ]
    .map(|(toneless_key, hanji, tl, syll, freq)| Row {
        toneless_key,
        hanji,
        tl,
        syll,
        freq,
    });
    let _lock = engine_install_lock();
    install(&rows, &["tso3", "tsin3", "tshut4", "khau2"]);
    assert_eq!(
        learn_installed(
            "tsotsintshutkhau",
            &[
                (Some("做"), "tsò", 3, 1),
                (Some("進"), "tsìn", 4, 1),
                (Some("出"), "tshut", 5, 1),
                (Some("口"), "kháu", 4, 1),
            ],
        ),
        phrase("做進出口", "tsò tsìn-tshut-kháu")
    );
    // Picked as two words, the same reading.
    assert_eq!(
        learn_installed(
            "tsotsintshutkhau",
            &[
                (Some("做"), "tsò", 3, 1),
                (Some("進出口"), "tsìn-tshut-kháu", 13, 3),
            ],
        ),
        phrase("做進出口", "tsò tsìn-tshut-kháu")
    );
}

#[test]
fn khinsiann_segment_keeps_its_double_hyphen() {
    // 記 + --起來: the second piece already opens with the neutral-tone
    // marker, so the join must not add a third hyphen.
    assert_eq!(
        learn(
            "kikhilai",
            &[(Some("記"), "kì", 2, 1), (Some("起來"), "--khí-lâi", 6, 2)]
        ),
        phrase("記起來", "kì--khí-lâi")
    );
}

#[test]
fn typed_separator_before_a_segment_is_the_joiner() {
    // The `-` run the user typed folds into the NEXT segment's raw
    // prefix (記 over `ki` leaves `--khilai` pending), so it is read there:
    // `--` learns the khinsiann, `-` the hyphen, nothing the word space.
    for (raw, consumed, tl) in [
        ("ki--khilai", 8, "kì--khí-lâi"),
        ("ki-khilai", 7, "kì-khí-lâi"),
        ("kikhilai", 6, "kì khí-lâi"),
        // A longer run is still the khinsiann marker, never stored verbatim.
        ("ki---khilai", 9, "kì--khí-lâi"),
    ] {
        assert_eq!(
            learn(
                raw,
                &[
                    (Some("記"), "kì", 2, 1),
                    (Some("起來"), "khí-lâi", consumed, 2)
                ]
            ),
            phrase("記起來", tl),
            "raw {raw:?}"
        );
    }
}

#[test]
fn dictionary_khinsiann_piece_wins_over_the_typed_separator() {
    // The segment's own dictionary form already opens with `--`; the
    // typed `-` is not stacked on it (mirrors the walker's dictionary-
    // owned-edge rule).
    assert_eq!(
        learn(
            "ki-khilai",
            &[(Some("記"), "kì", 2, 1), (Some("起來"), "--khí-lâi", 7, 2)]
        ),
        phrase("記起來", "kì--khí-lâi")
    );
}

/// `CommitContinuous` for the candidate the platform would pick: its own
/// `consumed_span_end` as `consumed_bytes`, every sidechannel echoed —
/// decoded as the wire request is.
fn commit(cand: &CandidateMessage) -> Intent {
    let method = Method::CommitContinuous(CommitContinuous {
        script: protos::engine::CommitScript::Lead as i32,
        roman: cand.roman.clone(),
        canonical_text: cand.display_text.clone(),
        association_tl: cand.canonical_tl.clone(),
        hanji: cand.hanji.clone(),
        consumed_bytes: cand.consumed_span_end,
        syllable_count: cand.syllable_count,
    });
    requests::decode_intent(&req(method)).expect("CommitContinuous decodes")
}

fn fetch_candidate(engine: &mut Engine, hanji: &str) -> CandidateMessage {
    let resp = requests::handle(
        &req(Method::FetchAtPos(FetchAtPos::default())),
        engine,
        &config_tl(),
    )
    .expect("FetchAtPos");
    let cands = resp.continuous.expect("continuous carrier").candidates;
    cands
        .iter()
        .find(|c| c.hanji.as_deref() == Some(hanji))
        .cloned()
        .unwrap_or_else(|| panic!("no candidate {hanji}; got {cands:?}"))
}

#[test]
fn typed_khinsiann_survives_the_real_fetch_and_commit_path() {
    // End to end: the `--` typed between `ki` and `khilai` stays in the
    // pending buffer when 記 is picked (its span ends before the run) and
    // is consumed with 起來 (the run folds into that span), so the final
    // commit learns `kì--khí-lâi`.
    let _lock = engine_install_lock();
    install(&fixture_rows(), FIXTURE_SYLLABLES);
    let cfg = config_tl();
    let mut engine = Engine::new();
    requests::handle(
        &req(Method::Start(Start {
            text: "ki--khilai".into(),
        })),
        &mut engine,
        &cfg,
    )
    .expect("setup");

    let ki = fetch_candidate(&mut engine, "記");
    assert_eq!(ki.consumed_span_end, 2, "記 ends before the typed run");
    let mid = engine.apply_learning(commit(&ki), &cfg);
    assert!(mid.learned.is_none());

    let khilai = fetch_candidate(&mut engine, "起來");
    assert_eq!(
        khilai.consumed_span_end, 8,
        "the run folds into 起來's span"
    );
    let fin = engine.apply_learning(commit(&khilai), &cfg);
    assert_eq!(fin.learned, phrase("記起來", "kì--khí-lâi"));
}

#[test]
fn three_single_picks_learn_too() {
    assert_eq!(
        learn(
            "kikhilai",
            &[
                (Some("記"), "kì", 2, 1),
                (Some("起"), "khí", 3, 1),
                (Some("來"), "lâi", 3, 1)
            ],
        ),
        // trace: 記起來 / 記起 not in the fixture → space; 起來 is → `-`.
        phrase("記起來", "kì khí-lâi")
    );
}

#[test]
fn single_segment_commit_learns_nothing() {
    assert_eq!(learn("khilai", &[(Some("起來"), "khí-lâi", 6, 2)]), None);
}

#[test]
fn a_hanji_less_segment_blocks_learning() {
    // §34 literal / OOV pick in the middle: the platform sends no `hanji`;
    // an empty hanji is stored as absent, not as a pick.
    for first in [None, Some("")] {
        assert_eq!(
            learn(
                "kikhilai",
                &[(first, "kì", 2, 1), (Some("起來"), "khí-lâi", 6, 2)]
            ),
            None,
            "first hanji {first:?}"
        );
    }
}

#[test]
fn missing_canonical_tl_blocks_learning() {
    // Legacy caller / TPS-OOV: `association_tl` empty → no key to learn under.
    assert_eq!(
        learn(
            "kikhilai",
            &[(Some("記"), "", 2, 1), (Some("起來"), "khí-lâi", 6, 2)]
        ),
        None
    );
}

#[test]
fn seven_syllables_is_a_clause_not_a_word() {
    const TL: [&str; 7] = ["a", "b", "c", "d", "e", "f", "g"];
    const HANJI: [&str; 7] = ["甲", "乙", "丙", "丁", "戊", "己", "庚"];
    let picks = |n: usize| -> Vec<Pick<'static>> {
        (0..n).map(|i| (Some(HANJI[i]), TL[i], 1, 1)).collect()
    };
    assert_eq!(
        learn("abcdefg", &picks(7)),
        None,
        "7 syllables must not learn"
    );
    assert_eq!(
        learn("abcdef", &picks(6)),
        phrase("甲乙丙丁戊己", "a b c d e f")
    );
}

#[test]
fn syllable_count_comes_from_the_joined_tl_not_the_segment_echo() {
    // The echoed `syllable_count` is platform input (here understated as
    // 1 for a 6-syllable word); the cap must follow the TL itself.
    assert_eq!(
        learn(
            "abcdefg",
            &[
                (Some("甲"), "a", 1, 1),
                (Some("乙丙丁戊己庚"), "b-c-d-e-f-g", 6, 1)
            ]
        ),
        None
    );
}

#[test]
fn a_multi_word_dictionary_tl_learns_with_its_space() {
    assert_eq!(
        learn(
            "guaiasi",
            &[(Some("我"), "guá", 3, 1), (Some("也是"), "iā sī", 4, 2)]
        ),
        phrase("我也是", "guá iā sī")
    );
}

// ---- Recall -----------------------------------------------------------------

fn fixture_rows() -> Vec<Row> {
    vec![
        Row {
            toneless_key: "ki",
            hanji: "機",
            tl: "ki",
            syll: 1,
            freq: 6318,
        },
        Row {
            toneless_key: "ki",
            hanji: "記",
            tl: "kì",
            syll: 1,
            freq: 2000,
        },
        Row {
            toneless_key: "khilai",
            hanji: "起來",
            tl: "khí-lâi",
            syll: 2,
            freq: 3000,
        },
        Row {
            toneless_key: "khi",
            hanji: "起",
            tl: "khí",
            syll: 1,
            freq: 16425,
        },
        Row {
            toneless_key: "lai",
            hanji: "來",
            tl: "lâi",
            syll: 1,
            freq: 30000,
        },
    ]
}

const FIXTURE_SYLLABLES: &[&str] = &["ki1", "ki3", "khi2", "lai5"];

fn install(rows: &[Row], syllables: &[&str]) {
    let dict_path = write_temp("dictionary.bin", &build_tkdb_v4(rows));
    let fst_path = build_dictionary_fst(rows);
    let association_path = write_temp("association.bin", &empty_association_bin());
    let syllables_path = build_syllables_fst(syllables);
    install_lexicon(&fst_path, &dict_path, &association_path, &syllables_path);
}

fn learned(hanji: &str, canonical_tl: &str) -> LearnedEntry {
    LearnedEntry {
        hanji: hanji.into(),
        canonical_tl: canonical_tl.into(),
    }
}

fn with_learned(rows: Vec<LearnedEntry>) -> Fetch {
    Fetch {
        learned: rows,
        ..Default::default()
    }
}

#[test]
fn learned_phrase_leads_when_the_dictionary_has_no_word_under_the_key() {
    let _lock = engine_install_lock();
    install(&fixture_rows(), FIXTURE_SYLLABLES);
    let cold = fetch_hanji("kikhilai", "tl", Fetch::default());
    assert_eq!(
        cold[0], "機起來",
        "cold start synthesizes the split; got {cold:?}"
    );
    assert!(!cold.contains(&"記起來".to_string()));

    let hanji = fetch_hanji(
        "kikhilai",
        "tl",
        with_learned(vec![learned("記起來", "kì-khí-lâi")]),
    );
    assert_eq!(hanji[0], "記起來", "learned phrase leads; got {hanji:?}");
    assert_eq!(
        hanji.iter().filter(|h| *h == "記起來").count(),
        1,
        "walker slot 0 and the span-local row collapse to one; got {hanji:?}"
    );
}

#[test]
fn learned_phrase_with_khinsiann_key_matches_the_toneless_buffer() {
    let _lock = engine_install_lock();
    install(&fixture_rows(), FIXTURE_SYLLABLES);
    let hanji = fetch_hanji(
        "kikhilai",
        "tl",
        with_learned(vec![learned("記起來", "kì--khí-lâi")]),
    );
    assert_eq!(hanji[0], "記起來", "got {hanji:?}");
}

/// The romanizations listed for `hanji`, in display order.
fn readings_for<'a>(cells: &'a [Cell], hanji: &str) -> Vec<&'a str> {
    cells
        .iter()
        .filter(|c| c.0.as_deref() == Some(hanji))
        .map(|c| c.1.as_str())
        .collect()
}

#[test]
fn learned_rows_differing_only_in_separator_collapse_to_the_first() {
    // `UNIQUE(hanji, roman)` keeps a slip and its correction as two rows;
    // the platform lists them `learn_count DESC, updated_at DESC`, and only
    // that first row surfaces — whichever separator it carries.
    let _lock = engine_install_lock();
    install(&fixture_rows(), FIXTURE_SYLLABLES);
    for order in [["kì--khí-lâi", "kì-khí-lâi"], ["kì-khí-lâi", "kì--khí-lâi"]] {
        let rows = order.iter().map(|tl| learned("記起來", tl)).collect();
        let cells = fetch_cells(&config_tl(), "kikhilai", with_learned(rows));
        let readings = readings_for(&cells, "記起來");
        assert_eq!(readings, vec![order[0]], "rows {order:?}; got {cells:?}");
    }
}

#[test]
fn learned_rows_differing_in_tone_are_different_words() {
    // Separator folding is tone-preserving: 記起來/kì vs 機起來/ki are two
    // readings of the same hanji (Core Principle #6), both listed.
    let _lock = engine_install_lock();
    install(&fixture_rows(), FIXTURE_SYLLABLES);
    let cells = fetch_cells(
        &config_tl(),
        "kikhilai",
        with_learned(vec![
            learned("記起來", "kì-khí-lâi"),
            learned("記起來", "ki-khí-lâi"),
        ]),
    );
    let readings = readings_for(&cells, "記起來");
    assert_eq!(readings, vec!["kì-khí-lâi", "ki-khí-lâi"], "got {cells:?}");
}

#[test]
fn learned_rows_differing_in_space_vs_hyphen_collapse_too() {
    // No `-` typed either way: 我 + 也是 learns `guá-iā sī` (the dictionary's
    // multi-word space), 我 + 也 + 是 learns `guá-iā-sī`. Same reading, so
    // an install that holds both rows now lists 我也是 once (Codex 2026-09-22
    // post-impl F1: this is the one recall change a user who never types
    // `-` can see).
    let _lock = engine_install_lock();
    install(&guaiasi_rows(), &["gua2", "ia7", "i1", "si7", "a1"]);
    let cells = fetch_cells(
        &config_tl(),
        "guaiasi",
        with_learned(vec![
            learned("我也是", "guá-iā-sī"),
            learned("我也是", "guá-iā sī"),
        ]),
    );
    assert_eq!(
        readings_for(&cells, "我也是"),
        vec!["guá-iā-sī"],
        "got {cells:?}"
    );
}

#[test]
fn learned_phrase_matches_a_poj_typed_buffer() {
    // The learned row is canonical TL; the POJ user types the POJ spelling.
    let _lock = engine_install_lock();
    let rows = vec![
        Row {
            toneless_key: "tshia",
            hanji: "車",
            tl: "tshia",
            syll: 1,
            freq: 5000,
        },
        Row {
            toneless_key: "thau",
            hanji: "頭",
            tl: "thâu",
            syll: 1,
            freq: 9000,
        },
    ];
    install(&rows, &["tshia1", "thau5"]);
    let hanji = fetch_hanji(
        "chhiathau",
        "poj",
        with_learned(vec![learned("車頭", "tshia-thâu")]),
    );
    assert_eq!(hanji[0], "車頭", "got {hanji:?}");
}

// INVARIANT_LEARNED_PHRASE_COMPETES_NEVER_OVERRIDES (behavioral-invariants.md §50)
#[test]
fn dictionary_homophone_beats_a_learned_row_until_the_user_prefers_it() {
    // Same key, different word: the fixture gains a real 3-syllable word
    // under `kikhilai`; the learned pair must not override it (Codex F5),
    // only compete.
    let _lock = engine_install_lock();
    let mut rows = fixture_rows();
    rows.push(Row {
        toneless_key: "kikhilai",
        hanji: "機器來",
        tl: "ki-khì-lâi",
        syll: 3,
        freq: 12,
    });
    install(&rows, FIXTURE_SYLLABLES);

    let hanji = fetch_hanji(
        "kikhilai",
        "tl",
        with_learned(vec![learned("記起來", "kì-khí-lâi")]),
    );
    assert_eq!(
        hanji[0], "機器來",
        "dictionary word keeps the edge; got {hanji:?}"
    );
    assert!(
        hanji.contains(&"記起來".to_string()),
        "learned row still listed; got {hanji:?}"
    );

    // One pick of the learned phrase → it leads (user weight is the
    // leading CandidateSortKey dimension, same as #69).
    let hanji = fetch_hanji(
        "kikhilai",
        "tl",
        Fetch {
            learned: vec![learned("記起來", "kì-khí-lâi")],
            frequency: vec![selected("記起來", "kì-khí-lâi", 1, 1_000)],
            now_ms: NOW_MS,
            ..Default::default()
        },
    );
    assert_eq!(hanji[0], "記起來", "got {hanji:?}");
    assert_eq!(hanji[1], "機器來", "got {hanji:?}");
}

#[test]
fn learned_pair_the_dictionary_also_carries_is_listed_once_from_the_dictionary() {
    let _lock = engine_install_lock();
    let mut rows = fixture_rows();
    rows.push(Row {
        toneless_key: "kikhilai",
        hanji: "記起來",
        tl: "kì-khí-lâi",
        syll: 3,
        freq: 12,
    });
    install(&rows, FIXTURE_SYLLABLES);
    let hanji = fetch_hanji(
        "kikhilai",
        "tl",
        with_learned(vec![learned("記起來", "kì-khí-lâi")]),
    );
    assert_eq!(hanji[0], "記起來", "got {hanji:?}");
    assert_eq!(
        hanji.iter().filter(|h| *h == "記起來").count(),
        1,
        "got {hanji:?}"
    );
}

#[test]
fn custom_row_whose_reading_has_a_word_space_leads_the_dictionary_word() {
    // A custom roman committed from a multi-word candidate keeps the space
    // between its words (`kì khí-lâi`). The walker key folds that space like
    // the learned key always did, so the never-selected row still leads the
    // never-selected dictionary word under the same key (機器來), toneless
    // and toned, listed once.
    let _lock = engine_install_lock();
    let mut rows = fixture_rows();
    rows.push(Row {
        toneless_key: "kikhilai",
        hanji: "機器來",
        tl: "ki-khì-lâi",
        syll: 3,
        freq: 12,
    });
    install(&rows, FIXTURE_SYLLABLES);
    let custom = |roman: &str, hanji: &str| Fetch {
        custom: vec![CustomEntry {
            roman: roman.into(),
            hanji: Some(hanji.into()),
        }],
        ..Default::default()
    };
    for raw in ["kikhilai", "ki3khi2lai5"] {
        let hanji = fetch_hanji(raw, "tl", custom("kì khí-lâi", "既起來"));
        assert_eq!(
            hanji[0], "既起來",
            "{raw}: the custom row wins the edge; got {hanji:?}"
        );
        assert_eq!(
            hanji.iter().filter(|h| *h == "既起來").count(),
            1,
            "{raw}: walker slot 0 and the span-local row collapse to one; got {hanji:?}"
        );
    }
    // The fold changes the key only: a wrong-tone spaced row is still
    // refused by the typed tone.
    let hanji = fetch_hanji("ki3khi2lai5", "tl", custom("kí khí-lâi", "忌起來"));
    assert!(
        !hanji.contains(&"忌起來".to_string()),
        "typed kì must not admit kí; got {hanji:?}"
    );
}

#[test]
fn manual_custom_row_outranks_a_learned_row_under_the_same_key() {
    let _lock = engine_install_lock();
    install(&fixture_rows(), FIXTURE_SYLLABLES);
    let hanji = fetch_hanji(
        "kikhilai",
        "tl",
        Fetch {
            custom: vec![CustomEntry {
                roman: "kì-khí-lâi".into(),
                hanji: Some("既起來".into()),
            }],
            learned: vec![learned("記起來", "kì-khí-lâi")],
            ..Default::default()
        },
    );
    assert_eq!(
        hanji[0], "既起來",
        "the never-selected custom row wins the edge; got {hanji:?}"
    );
    assert!(
        hanji.contains(&"記起來".to_string()),
        "learned row still listed; got {hanji:?}"
    );
}

#[test]
fn empty_or_half_empty_learned_rows_are_ignored() {
    let _lock = engine_install_lock();
    install(&fixture_rows(), FIXTURE_SYLLABLES);
    let hanji = fetch_hanji(
        "kikhilai",
        "tl",
        with_learned(vec![learned("", "kì-khí-lâi"), learned("記起來", "")]),
    );
    assert_eq!(hanji[0], "機起來", "got {hanji:?}");
}

fn guaiasi_rows() -> Vec<Row> {
    vec![
        Row {
            toneless_key: "gua",
            hanji: "我",
            tl: "guá",
            syll: 1,
            freq: 50000,
        },
        Row {
            toneless_key: "ia",
            hanji: "也",
            tl: "iā",
            syll: 1,
            freq: 20000,
        },
        Row {
            toneless_key: "i",
            hanji: "伊",
            tl: "i",
            syll: 1,
            freq: 40000,
        },
        Row {
            toneless_key: "si",
            hanji: "是",
            tl: "sī",
            syll: 1,
            freq: 60000,
        },
        Row {
            toneless_key: "iasi",
            hanji: "也是",
            tl: "iā sī",
            syll: 2,
            freq: 3000,
        },
    ]
}

#[test]
fn learned_row_with_a_space_in_its_tl_matches_the_typed_buffer() {
    let _lock = engine_install_lock();
    install(&guaiasi_rows(), &["gua2", "ia7", "i1", "si7", "a1"]);

    let resp = fetch_at_pos_response(
        &common::config("tl"),
        "guaiasi",
        with_learned(vec![learned("我也是", "guá-iā sī")]),
    );
    let candidates = resp.continuous.expect("continuous carrier").candidates;
    let hanji: Vec<&str> = candidates
        .iter()
        .filter_map(|c| c.hanji.as_deref())
        .collect();
    assert_eq!(hanji[0], "我也是", "got {hanji:?}");
    let row = candidates
        .iter()
        .find(|c| c.hanji.as_deref() == Some("我也是"))
        .expect("learned row present");
    assert_eq!(row.syllable_count, 3, "space-separated syllables count");
}

#[test]
fn learned_row_answers_the_nasal_oo_alias_key() {
    // Typing 好玄 as `hoonn…` / `ho͘ⁿ…` produces the alias edge key the
    // dictionary build indexes beside the canonical `honn…`; a learned row
    // stored canonically must answer both, like a custom row does.
    let _lock = engine_install_lock();
    let rows = vec![
        Row {
            toneless_key: "honn",
            hanji: "好",
            tl: "hònn",
            syll: 1,
            freq: 9000,
        },
        Row {
            toneless_key: "hian",
            hanji: "玄",
            tl: "hiân",
            syll: 1,
            freq: 4000,
        },
    ];
    install(&rows, &["honn3", "hoonn3", "hian5"]);

    for typed in ["honnhian", "hoonnhian"] {
        let hanji = fetch_hanji(
            typed,
            "tl",
            with_learned(vec![learned("好玄", "hònn-hiân")]),
        );
        assert_eq!(hanji[0], "好玄", "typed {typed}; got {hanji:?}");
    }
}

#[test]
fn a_custom_word_keeps_its_syllables_in_the_compound_run() {
    // A custom list row's syllable count comes from its TL, not a fixed 1
    // (which fused 來 + 記 `kì-khí` into the compound 來記, `lâi-kì-khí`).
    // Reached through its abbreviation `kkh`: the walker's slot 0 counts a
    // custom edge itself, so only the list's custom rows carried the 1.
    // trace: segments 來 (1) + custom 記 (tl_syllables("kì-khí") = 2) →
    // longest_compound_run stops at 來 → space; the commit learns
    // `lâi kì-khí`.
    let rows = [
        ("lai", "來", "lâi", 1, 5000),
        ("ki", "記", "kì", 1, 4000),
        ("khi", "起", "khí", 1, 3000),
        ("laiki", "來記", "lâi-kì", 2, 100),
    ]
    .map(|(toneless_key, hanji, tl, syll, freq)| Row {
        toneless_key,
        hanji,
        tl,
        syll,
        freq,
    });
    let _lock = engine_install_lock();
    install(&rows, &["lai5", "ki3", "khi2"]);
    let cfg = config_tl();
    let mut engine = engine_in_continuous("laikkh");
    let find = |engine: &mut Engine, hanji: &str, consumed: u32| {
        let fetch = Fetch {
            custom: vec![CustomEntry {
                roman: "kì-khí".into(),
                hanji: Some("記".into()),
            }],
            ..Fetch::default()
        };
        let resp = requests::apply(fetch.intent(), engine, &cfg);
        let cands = resp.continuous.expect("continuous carrier").candidates;
        cands
            .iter()
            .find(|c| c.hanji.as_deref() == Some(hanji) && c.consumed_span_end == consumed)
            .cloned()
            .unwrap_or_else(|| panic!("no {hanji} ending at {consumed}; got {cands:?}"))
    };

    let lai = find(&mut engine, "來", 3);
    assert!(engine.apply_learning(commit(&lai), &cfg).learned.is_none());
    let custom = find(&mut engine, "記", 3);
    assert_eq!(custom.syllable_count, 2, "kì-khí is two syllables");
    let fin = engine.apply_learning(commit(&custom), &cfg);
    assert_eq!(fin.learned, phrase("來記", "lâi kì-khí"));
}
