//! §52 — a typed `-` / `--` is a syllable boundary (USER 2026-09-22:
//! `jim--khi--ah` for 𠕇去矣 offered 隙 `khiah` after 𠕇 was picked; the
//! `--` the user typed says `khi` is a syllable, never `khiah`).
//!
//! Two layers, pinned together here: the TL / POJ syllabifier lets no
//! single syllable cross a typed hyphen (so the walker cannot read
//! `khi|ah` as one hop), and the span's lookup pin drops every reading
//! that does not end a syllable on the boundary (so 隙 / 屐, which share
//! the toneless key `khiah` with 去啊 `khì--ah`, never surface under it).
//!
//! Fixture mirrors production around the report: `khi` / `khia` / `khiah`
//! are all syllables, 隙 `khiah` (1 syllable) and 去啊 `khì--ah` (2) share
//! the key, and 去 / 起 / 矣 are the singles a walker path is built from.

use composing::api::Engine;
use composing::dispatch;
use protos::engine::composing_request::Method;
use protos::engine::{CommitContinuous, EnterContinuous, FetchAtPos, Start};

mod common;
use common::Fetch;
use common::{
    build_dictionary_fst, build_syllables_fst, build_tkdb_v3, cell_with_hanji, config,
    empty_association_bin, engine_install_lock, fetch_cells, install_lexicon, req, write_temp,
    Cell, Row,
};
use lexicon::CustomEntry;

fn fixture_rows() -> Vec<Row> {
    vec![
        Row {
            toneless_key: "jim",
            hanzi: "忍",
            tl: "jím",
            syll: 1,
            freq: 600,
        },
        // A plain-`-` compound over the report's first two syllables.
        Row {
            toneless_key: "jimkhi",
            hanzi: "忍氣",
            tl: "jím-khì",
            syll: 2,
            freq: 400,
        },
        Row {
            toneless_key: "khi",
            hanzi: "去",
            tl: "khì",
            syll: 1,
            freq: 38_000,
        },
        Row {
            toneless_key: "khi",
            hanzi: "起",
            tl: "khí",
            syll: 1,
            freq: 16_000,
        },
        Row {
            toneless_key: "khia",
            hanzi: "企",
            tl: "khiā",
            syll: 1,
            freq: 2_500,
        },
        Row {
            toneless_key: "khiah",
            hanzi: "隙",
            tl: "khiah",
            syll: 1,
            freq: 100,
        },
        Row {
            toneless_key: "khiah",
            hanzi: "屐",
            tl: "khia̍h",
            syll: 1,
            freq: 50,
        },
        Row {
            toneless_key: "khiah",
            hanzi: "去啊",
            tl: "khì--ah",
            syll: 2,
            freq: 16,
        },
        // Production stores the khinsiann single without its `--` (矣 `ah`).
        Row {
            toneless_key: "ah",
            hanzi: "矣",
            tl: "ah",
            syll: 1,
            freq: 20_000,
        },
        Row {
            toneless_key: "ah",
            hanzi: "啊",
            tl: "ah",
            syll: 1,
            freq: 9_000,
        },
    ]
}

fn install_fixture() {
    let rows = fixture_rows();
    let dict_path = write_temp("dictionary.bin", &build_tkdb_v3(&rows));
    let fst_path = build_dictionary_fst(&rows);
    let assoc_path = write_temp("association.bin", &empty_association_bin());
    // `lai5` is a syllable with no row: the OOV branch.
    let syllables_path = build_syllables_fst(&[
        "jim2", "khi3", "khi2", "khia7", "khiah4", "khiah8", "ah4", "a2", "lai5",
    ]);
    install_lexicon(&fst_path, &dict_path, &assoc_path, &syllables_path);
}

fn fetch(raw: &str, mode: &str, fetch: Fetch) -> Vec<Cell> {
    fetch_cells(&config(mode), raw, fetch)
}

fn hanji_of(cells: &[Cell]) -> Vec<&str> {
    cells.iter().filter_map(|c| c.0.as_deref()).collect()
}

// INVARIANT_TYPED_HYPHEN_IS_A_SYLLABLE_BOUNDARY (behavioral-invariants.md §52)
#[test]
fn typed_hyphen_drops_the_readings_that_do_not_end_a_syllable_there() {
    let _lock = engine_install_lock();
    install_fixture();
    for (raw, synth, khinsiann_word) in [("khi--ah", "khì--ah", true), ("khi-ah", "khì-ah", false)]
    {
        let cells = fetch(raw, "tl", Fetch::default());
        let hanji = hanji_of(&cells);
        assert!(
            !hanji.contains(&"隙") && !hanji.contains(&"屐"),
            "{raw}: one-syllable readings of `khiah` must not surface; got {cells:?}"
        );
        // 去啊 `khì--ah` ends a syllable at `khi` AND separates with `--`:
        // offered under the typed `--`, not under a plain `-` (kind).
        assert_eq!(
            hanji.contains(&"去啊"),
            khinsiann_word,
            "{raw}: 去啊 follows the typed kind; got {cells:?}"
        );
        assert!(
            hanji.contains(&"去"),
            "{raw}: the span-local single 去 is offered; got {cells:?}"
        );
        // Walker slot 0 (index 1, after the §34 literal) is the two-word path.
        assert_eq!(cells[1].0.as_deref(), Some("去矣"), "{raw}: got {cells:?}");
        assert_eq!(cells[1].1, synth, "{raw}: the typed run renders");
    }
}

#[test]
fn typed_run_kind_selects_between_a_compound_and_a_khinsiann_reading() {
    // `jim-khi` is the hyphenated compound 忍氣 `jím-khì`; `jim--khi` is not — the
    // typed `--` asks for a khinsiann boundary the compound does not have,
    // so the walker builds 忍 + 去 with the typed join instead.
    let _lock = engine_install_lock();
    install_fixture();
    let cells = fetch("jim-khi", "tl", Fetch::default());
    assert_eq!(cells[1].0.as_deref(), Some("忍氣"), "got {cells:?}");
    assert_eq!(cells[1].1, "jím-khì");
    let cells = fetch("jim--khi", "tl", Fetch::default());
    assert!(
        !hanji_of(&cells).contains(&"忍氣"),
        "a plain compound never answers a typed `--`; got {cells:?}"
    );
    assert_eq!(cells[1].0.as_deref(), Some("忍去"), "got {cells:?}");
    assert_eq!(cells[1].1, "jím--khì", "the typed run renders");
}

#[test]
fn no_hyphen_keeps_every_reading_of_the_key() {
    // Control: `khiah` with nothing typed between still offers the single
    // syllable readings and the two-syllable word alike.
    let _lock = engine_install_lock();
    install_fixture();
    let cells = fetch("khiah", "tl", Fetch::default());
    let hanji = hanji_of(&cells);
    for expected in ["隙", "屐", "去啊"] {
        assert!(
            hanji.contains(&expected),
            "{expected} missing; got {cells:?}"
        );
    }
    assert_eq!(
        cells[1].0.as_deref(),
        Some("隙"),
        "slot 0 stays the one-hop word; got {cells:?}"
    );
}

#[test]
fn boundary_holds_in_the_pending_buffer_after_a_pick() {
    // The report's shape: `jim--khi--ah`, pick 忍 over `jim`, then the
    // pending `--khi--ah` must offer 去 and never 隙.
    let _lock = engine_install_lock();
    install_fixture();
    let cfg = config("tl");
    let mut engine = Engine::new();
    dispatch::handle(
        &req(Method::Start(Start {
            text: "jim--khi--ah".into(),
        })),
        &mut engine,
        &cfg,
    )
    .expect("Start");
    dispatch::handle(
        &req(Method::EnterContinuous(EnterContinuous {})),
        &mut engine,
        &cfg,
    )
    .expect("EnterContinuous");
    let first = dispatch::handle(
        &req(Method::FetchAtPos(FetchAtPos::default())),
        &mut engine,
        &cfg,
    )
    .expect("FetchAtPos");
    let cands = first.continuous.expect("continuous").candidates;
    let jim = cands
        .iter()
        .find(|c| c.hanji.as_deref() == Some("忍") && c.syllable_count == 1)
        .cloned()
        .unwrap_or_else(|| panic!("no 忍; got {cands:?}"));
    assert_eq!(jim.consumed_span_end, 3);
    dispatch::handle(
        &req(Method::CommitContinuous(CommitContinuous {
            display_text: jim.roman.clone(),
            canonical_text: jim.display_text.clone(),
            association_tl: jim.canonical_tl.clone(),
            hanji: jim.hanji.clone(),
            consumed_bytes: jim.consumed_span_end,
            syllable_count: jim.syllable_count,
        })),
        &mut engine,
        &cfg,
    )
    .expect("mid-commit");
    let second = dispatch::handle(
        &req(Method::FetchAtPos(FetchAtPos::default())),
        &mut engine,
        &cfg,
    )
    .expect("FetchAtPos");
    let cands = second.continuous.expect("continuous").candidates;
    let hanji: Vec<&str> = cands.iter().filter_map(|c| c.hanji.as_deref()).collect();
    assert!(
        hanji.contains(&"去"),
        "pending `--khi--ah` offers 去; got {cands:?}"
    );
    assert!(
        !hanji.contains(&"隙") && !hanji.contains(&"屐"),
        "pending `--khi--ah` never reads `khiah`; got {cands:?}"
    );
    let khi = cands
        .iter()
        .find(|c| c.hanji.as_deref() == Some("去"))
        .expect("去");
    assert_eq!(
        khi.consumed_span_end, 5,
        "去 consumes the leading `--` and `khi`"
    );
}

#[test]
fn oov_reading_splits_on_the_typed_hyphen_beside_a_dictionary_word() {
    // Mixed dictionary + OOV synth: `khi-lai-lai` — 去 is a word, `lai` is
    // a syllable with no row. The OOV part must not fuse across the typed
    // `-` (`lailai`), so the synth reads `khì-lai-lai`.
    let _lock = engine_install_lock();
    install_fixture();
    let cells = fetch("khi-lai-lai", "tl", Fetch::default());
    assert!(
        cells.iter().any(|c| c.1 == "khì-lai-lai"),
        "typed joins inside the OOV run; got {cells:?}"
    );
    assert!(
        !cells.iter().any(|c| c.1.contains("lailai")),
        "no OOV blob across a typed `-`; got {cells:?}"
    );
}

#[test]
fn custom_and_capitalised_readings_answer_to_the_boundary_too() {
    let _lock = engine_install_lock();
    install_fixture();
    let custom = |roman: &str, hanji: &str| CustomEntry {
        roman: roman.into(),
        hanji: Some(hanji.into()),
    };
    let cells = fetch(
        "khi--ah",
        "tl",
        Fetch {
            custom: vec![custom("Khì--ah", "去矣"), custom("Khiah", "隙")],
            ..Default::default()
        },
    );
    let hanji = hanji_of(&cells);
    assert!(
        hanji.contains(&"去矣"),
        "capitalised custom `Khì--ah` passes; got {cells:?}"
    );
    assert!(
        !hanji.contains(&"隙"),
        "custom `Khiah` crosses the boundary; got {cells:?}"
    );
}

#[test]
fn a_custom_reading_opening_with_a_double_hyphen_needs_a_typed_double_hyphen() {
    // Codex post-impl 2026-09-22 P2: a custom 矣 stored as `--ah` is a
    // khinsiann reading from its first letter; under a plain `-` it must not
    // become the walker's second word (`khì---ah`), under `--` it is, with
    // the run rendered once.
    let _lock = engine_install_lock();
    install_fixture();
    let custom = Fetch {
        custom: vec![CustomEntry {
            roman: "--ah".into(),
            hanji: Some("矣".into()),
        }],
        ..Default::default()
    };
    let cells = fetch("khi-ah", "tl", custom.clone());
    assert!(
        !cells.iter().any(|c| c.1.contains("---")),
        "a plain `-` never stacks onto the custom `--ah`; got {cells:?}"
    );
    let cells = fetch("khi--ah", "tl", custom);
    assert!(
        !cells.iter().any(|c| c.1.contains("---")),
        "the typed `--` is never stacked onto the custom `--ah`; got {cells:?}"
    );
    // The whole-buffer dictionary word 去啊 still takes slot 0 (the
    // existing promotion); the reading it and the custom row share renders
    // the run once.
    assert_eq!(cells[1].1, "khì--ah", "got {cells:?}");
}

#[test]
fn poj_typed_hyphen_reads_the_same_boundary() {
    let _lock = engine_install_lock();
    install_fixture();
    let cells = fetch("khi--ah", "poj", Fetch::default());
    let hanji = hanji_of(&cells);
    assert!(!hanji.contains(&"隙"), "got {cells:?}");
    assert!(hanji.contains(&"去啊"), "got {cells:?}");
    assert_eq!(cell_with_hanji(&cells, "去啊").1, "khì--ah");
}
