//! Learned phrases (§50) — a phrase the user composed segment by segment
//! in continuous input becomes a whole-buffer candidate next time.
//!
//! Two halves, pinned together here:
//!
//! 1. **Learning** — the final `CommitContinuous` of a composition whose
//!    every nailed segment was a hanji pick emits `Effect.PhraseLearned`
//!    with the concatenated hanji and the `-`-joined canonical TL. Nothing
//!    is emitted mid-commit, for a single segment, for a hanji-less
//!    segment, or past six syllables.
//! 2. **Recall** — a `FetchAtPos.learned_entries` row whose key equals the
//!    typed buffer leads the hanji candidates when the dictionary has no
//!    word under that key, competes (and loses) against a dictionary
//!    homophone unless `user_frequency` prefers it, dedupes against the
//!    same pair from the dictionary, and never outranks a manual custom
//!    row (Codex 2026-09-20 F5: learned = competitor, custom = override).
//!
//! Fixture mirrors the USER report (2026-09-20): 記起來 `kì--khí-lâi` is a
//! 台日-only dictionary word (default off), so a default user composes it as
//! 記 `ki` + 起來 `khilai`; the dictionary here carries 機/ki, 記/kì, 起來 and
//! NOT 記起來.

use composing::api::Engine;
use composing::{dispatch, Intent, Phase};
use protos::engine::composing_request::Method;
use protos::engine::effect::Kind;
use protos::engine::{
    CandidateMessage, CommitContinuous, ComposingResponse, CustomDictEntry, EnterContinuous,
    FetchAtPos, LearnedEntry, PhraseLearned, Start,
};

mod common;
use common::{
    build_dictionary_fst, build_syllables_fst, build_tkdb_v3, config_tl, effect_kinds,
    empty_association_bin, engine_in_continuous, engine_install_lock, fetch_at_pos_response,
    fetch_cells, fetch_hanji, install_lexicon, req, selected, write_temp, Cell, Row, NOW_MS,
};

// ---- Learning ---------------------------------------------------------------

/// One `CommitContinuous`: `(hanji, canonical_tl, consumed_bytes, syllable_count)`.
type Pick<'a> = (Option<&'a str>, &'a str, usize, u8);

fn pick((hanji, tl, consumed_bytes, syllable_count): Pick<'_>) -> Intent {
    Intent::CommitContinuous {
        display_text: hanji.unwrap_or(tl).to_string(),
        canonical_text: hanji.unwrap_or(tl).to_string(),
        association_tl: tl.to_string(),
        hanji: hanji.map(str::to_string),
        consumed_bytes,
        syllable_count,
    }
}

fn learned_effect(resp: &ComposingResponse) -> Option<PhraseLearned> {
    resp.effect.iter().find_map(|e| match e.kind.as_ref() {
        Some(Kind::PhraseLearned(p)) => Some(p.clone()),
        _ => None,
    })
}

/// Apply every pick over `raw` and return what the LAST commit learned,
/// against the fixture lexicon: the commit and the learned reading both
/// group words through its compound oracle (起來 is a word, 記起 is not).
fn learn(raw: &str, picks: &[Pick<'_>]) -> Option<PhraseLearned> {
    let _lock = engine_install_lock();
    install(&fixture_rows(), FIXTURE_SYLLABLES);
    learn_installed(raw, picks)
}

/// [`learn`] against the lexicon the caller installed under the lock.
fn learn_installed(raw: &str, picks: &[Pick<'_>]) -> Option<PhraseLearned> {
    let mut e = engine_in_continuous(raw);
    let mut last = None;
    for p in picks {
        last = Some(e.apply(pick(*p), &config_tl()));
    }
    learned_effect(last.as_ref().expect("at least one pick"))
}

fn phrase(hanji: &str, canonical_tl: &str) -> Option<PhraseLearned> {
    Some(PhraseLearned {
        hanji: hanji.into(),
        canonical_tl: canonical_tl.into(),
    })
}

#[test]
fn final_commit_of_hanji_picks_learns_the_joined_phrase() {
    let _lock = engine_install_lock();
    install(&fixture_rows(), FIXTURE_SYLLABLES);
    let mut e = engine_in_continuous("kikhilai");
    let mid = e.apply(pick((Some("記"), "kì", 2, 1)), &config_tl());
    assert!(
        learned_effect(&mid).is_none(),
        "mid-commit must not learn; got {:?}",
        effect_kinds(&mid.effect)
    );
    assert!(matches!(e.snapshot_state().phase, Phase::Continuous { .. }));

    let fin = e.apply(pick((Some("起來"), "khí-lâi", 6, 2)), &config_tl());
    assert_eq!(
        effect_kinds(&fin.effect),
        vec![
            "CommitTextReplacingPreedit",
            "ResetAutocomplete",
            "ResetAutocompleteContext",
            "NextWordWordSelected",
            "PhraseLearned",
        ]
    );
    // Two words, as the commit renders them: 記 + the dictionary word 起來.
    assert_eq!(learned_effect(&fin), phrase("記起來", "kì khí-lâi"));
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
    .map(|(toneless_key, hanzi, tl, syll, freq)| Row {
        toneless_key,
        hanzi,
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
/// `consumed_span_end` as `consumed_bytes`, every sidechannel echoed.
fn commit(cand: &CandidateMessage) -> Method {
    Method::CommitContinuous(CommitContinuous {
        display_text: cand.display_text.clone(),
        canonical_text: cand.display_text.clone(),
        association_tl: cand.canonical_tl.clone(),
        hanji: cand.hanji.clone(),
        consumed_bytes: cand.consumed_span_end,
        syllable_count: cand.syllable_count,
    })
}

fn fetch_candidate(engine: &mut Engine, hanji: &str) -> CandidateMessage {
    let resp = dispatch::handle(
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
    for method in [
        Method::Start(Start {
            text: "ki--khilai".into(),
        }),
        Method::EnterContinuous(EnterContinuous {}),
    ] {
        dispatch::handle(&req(method), &mut engine, &cfg).expect("setup");
    }

    let ki = fetch_candidate(&mut engine, "記");
    assert_eq!(ki.consumed_span_end, 2, "記 ends before the typed run");
    let mid = dispatch::handle(&req(commit(&ki)), &mut engine, &cfg).expect("mid-commit");
    assert!(learned_effect(&mid).is_none());

    let khilai = fetch_candidate(&mut engine, "起來");
    assert_eq!(
        khilai.consumed_span_end, 8,
        "the run folds into 起來's span"
    );
    let fin = dispatch::handle(&req(commit(&khilai)), &mut engine, &cfg).expect("final commit");
    assert_eq!(learned_effect(&fin), phrase("記起來", "kì--khí-lâi"));
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
    // A custom-dictionary pick echoes `syllable_count = 1` whatever its
    // length; the cap must follow the TL itself.
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
            hanzi: "機",
            tl: "ki",
            syll: 1,
            freq: 6318,
        },
        Row {
            toneless_key: "ki",
            hanzi: "記",
            tl: "kì",
            syll: 1,
            freq: 2000,
        },
        Row {
            toneless_key: "khilai",
            hanzi: "起來",
            tl: "khí-lâi",
            syll: 2,
            freq: 3000,
        },
        Row {
            toneless_key: "khi",
            hanzi: "起",
            tl: "khí",
            syll: 1,
            freq: 16425,
        },
        Row {
            toneless_key: "lai",
            hanzi: "來",
            tl: "lâi",
            syll: 1,
            freq: 30000,
        },
    ]
}

const FIXTURE_SYLLABLES: &[&str] = &["ki1", "ki3", "khi2", "lai5"];

fn install(rows: &[Row], syllables: &[&str]) {
    let dict_path = write_temp("dictionary.bin", &build_tkdb_v3(rows));
    let fst_path = build_dictionary_fst(rows);
    let assoc_path = write_temp("association.bin", &empty_association_bin());
    let syllables_path = build_syllables_fst(syllables);
    install_lexicon(&fst_path, &dict_path, &assoc_path, &syllables_path);
}

fn learned(hanji: &str, canonical_tl: &str) -> LearnedEntry {
    LearnedEntry {
        hanji: hanji.into(),
        canonical_tl: canonical_tl.into(),
    }
}

fn with_learned(rows: Vec<LearnedEntry>) -> FetchAtPos {
    FetchAtPos {
        learned_entries: rows,
        ..Default::default()
    }
}

#[test]
fn learned_phrase_leads_when_the_dictionary_has_no_word_under_the_key() {
    let _lock = engine_install_lock();
    install(&fixture_rows(), FIXTURE_SYLLABLES);
    let cold = fetch_hanji("kikhilai", "tl", FetchAtPos::default());
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
    // `UNIQUE(hanzi, roman)` keeps a slip and its correction as two rows;
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
            hanzi: "車",
            tl: "tshia",
            syll: 1,
            freq: 5000,
        },
        Row {
            toneless_key: "thau",
            hanzi: "頭",
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

#[test]
fn dictionary_homophone_beats_a_learned_row_until_the_user_prefers_it() {
    // Same key, different word: the fixture gains a real 3-syllable word
    // under `kikhilai`; the learned pair must not override it (Codex F5),
    // only compete.
    let _lock = engine_install_lock();
    let mut rows = fixture_rows();
    rows.push(Row {
        toneless_key: "kikhilai",
        hanzi: "機器來",
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
    // leading SortKey dimension, same as #69).
    let hanji = fetch_hanji(
        "kikhilai",
        "tl",
        FetchAtPos {
            learned_entries: vec![learned("記起來", "kì-khí-lâi")],
            frequency_entries: vec![selected("記起來", "kì-khí-lâi", 1, 1_000)],
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
        hanzi: "記起來",
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
fn manual_custom_row_outranks_a_learned_row_under_the_same_key() {
    let _lock = engine_install_lock();
    install(&fixture_rows(), FIXTURE_SYLLABLES);
    let hanji = fetch_hanji(
        "kikhilai",
        "tl",
        FetchAtPos {
            custom_entries: vec![CustomDictEntry {
                roman: "kì-khí-lâi".into(),
                hanji: Some("既起來".into()),
            }],
            learned_entries: vec![learned("記起來", "kì-khí-lâi")],
            ..Default::default()
        },
    );
    assert_eq!(
        hanji[0], "既起來",
        "custom override wins the edge; got {hanji:?}"
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
            hanzi: "我",
            tl: "guá",
            syll: 1,
            freq: 50000,
        },
        Row {
            toneless_key: "ia",
            hanzi: "也",
            tl: "iā",
            syll: 1,
            freq: 20000,
        },
        Row {
            toneless_key: "i",
            hanzi: "伊",
            tl: "i",
            syll: 1,
            freq: 40000,
        },
        Row {
            toneless_key: "si",
            hanzi: "是",
            tl: "sī",
            syll: 1,
            freq: 60000,
        },
        Row {
            toneless_key: "iasi",
            hanzi: "也是",
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
            hanzi: "好",
            tl: "hònn",
            syll: 1,
            freq: 9000,
        },
        Row {
            toneless_key: "hian",
            hanzi: "玄",
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
