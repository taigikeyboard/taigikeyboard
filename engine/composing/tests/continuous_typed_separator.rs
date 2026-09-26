//! Typed word separator in the continuous-input romanization (USER
//! 2026-09-21, 2026-09-22). The separator at every typed boundary is the
//! one the user typed — `-` hyphen or `--` neutral tone — whether the boundary falls
//! between two walker segments or inside one dictionary word whose record
//! stores a space (§55); a space stays only where nothing was typed. The
//! kind of the typed run still decides which words are offered (§52:
//! `hoo--gua` reads 予我 `hōo--guá`, `hoo-gua` does not).
//!
//! The typed separator is presentation only: `canonical_tl` /
//! `display_text` (the `(hanji, canonical_tl)` identity) keep the record's
//! or the space-joined form, so the same phrase typed three ways keys
//! `user_frequency.db` once.
//!
//! Fixture: 我/guá + 是/sī are high-freq singles with no `guasi` word, so
//! the walker path is the genuine two-word split; 予/hōo + 我/guá share the
//! same shape but 予我/hōo--guá exists as a word; `lai` is a syllable with
//! no dictionary row (the all-OOV branch).

use composing::api::Engine;
use composing::dispatch;
use protos::engine::composing_request::Method;
use protos::engine::effect::Kind;
use protos::engine::{
    AppConfig, CandidateMessage, CommitContinuous, CommitRaw, ComposingResponse, DeleteBackward,
    EnterContinuous, FetchAtPos, Start,
};

mod common;
use common::Fetch;
use common::{
    build_dictionary_fst, build_syllables_fst, build_tkdb_v3, cell_with_hanji, commit_text, config,
    empty_association_bin, engine_install_lock, fetch_cells, install_lexicon, req, write_temp,
    Cell, Row,
};

fn fixture_rows() -> Vec<Row> {
    vec![
        Row {
            toneless_key: "hoo",
            hanzi: "予",
            tl: "hōo",
            syll: 1,
            freq: 80_000,
        },
        Row {
            toneless_key: "gua",
            hanzi: "我",
            tl: "guá",
            syll: 1,
            freq: 80_000,
        },
        Row {
            toneless_key: "si",
            hanzi: "是",
            tl: "sī",
            syll: 1,
            freq: 80_000,
        },
        Row {
            toneless_key: "hoogua",
            hanzi: "予我",
            tl: "hōo--guá",
            syll: 2,
            freq: 16,
        },
    ]
}

fn install_rows(rows: &[Row], syllables: &[&str]) {
    let dict_path = write_temp("dictionary.bin", &build_tkdb_v3(rows));
    let fst_path = build_dictionary_fst(rows);
    let assoc_path = write_temp("association.bin", &empty_association_bin());
    let syllables_path = build_syllables_fst(syllables);
    install_lexicon(&fst_path, &dict_path, &assoc_path, &syllables_path);
}

fn install_fixture() {
    install_rows(&fixture_rows(), &["hoo7", "gua2", "si7", "lai5"]);
}

fn fetch(raw: &str, input_mode: &str, hyphenless: bool) -> Vec<Cell> {
    let cfg = AppConfig {
        hyphenless_roman: hyphenless,
        ..config(input_mode)
    };
    fetch_cells(&cfg, raw, Fetch::default())
}

#[test]
fn typed_separator_joins_the_two_word_reading() {
    let _lock = engine_install_lock();
    install_fixture();
    for (raw, roman) in [
        ("guasi", "guá sī"),
        ("gua-si", "guá-sī"),
        ("gua--si", "guá--sī"),
    ] {
        let cells = fetch(raw, "tl", false);
        let cell = cell_with_hanji(&cells, "我是");
        assert_eq!(cell.1, roman, "{raw}: rendered roman");
        assert_eq!(cell.2, "我是", "{raw}: display_text (詞頻 key)");
        assert_eq!(
            cell.3, "guá sī",
            "{raw}: canonical_tl (identity) never follows the typed separator"
        );
        assert_eq!(cells[0].1, raw, "{raw}: the §34 literal is what was typed");
    }
}

#[test]
fn typed_separator_kind_selects_the_dictionary_word() {
    let _lock = engine_install_lock();
    install_fixture();
    // The untyped `hoogua` promotion is pinned by `continuous_slot0_dict_roman`.
    // §52: the typed `--` is the khinsiann 予我 carries, so the word wins
    // (and §55 has nothing to rewrite — typed and stored agree); a plain
    // `-` is a different boundary kind and the word is not offered under
    // it — the typed join stands.
    let cells = fetch("hoo--gua", "tl", false);
    let cell = cell_with_hanji(&cells, "予我");
    assert_eq!(cell.1, "hōo--guá", "the record's own khinsiann form");
    assert_eq!(cell.3, "hōo--guá", "the record's identity");
    assert!(
        !cells.iter().any(|c| c.1 == "hōo guá" || c.1 == "hōo-guá"),
        "no synth join beside the dictionary word; got {cells:?}"
    );
    let cells = fetch("hoo-gua", "tl", false);
    assert!(
        !cells.iter().any(|c| c.3 == "hōo--guá"),
        "a plain `-` never reads the khinsiann record; got {cells:?}"
    );
    // The walker's 予 + 我 keeps the pair's hanji with the typed join.
    assert_eq!(cells[1].0.as_deref(), Some("予我"), "got {cells:?}");
    assert_eq!(cells[1].1, "hōo-guá", "the typed join; got {cells:?}");
}

#[test]
fn typed_separator_joins_the_all_oov_reading() {
    let _lock = engine_install_lock();
    install_fixture();
    let cells = fetch("lailai", "tl", false);
    assert!(
        cells.iter().any(|c| c.0.is_none() && c.1 == "lai lai"),
        "untyped OOV join stays the space; got {cells:?}"
    );
    let cells = fetch("lai-lai", "tl", false);
    // The typed join reads exactly like the §34 literal, which absorbs the
    // identical bare-roman synth (`dispatch::handle_fetch_at_pos`) — one
    // `lai-lai` cell, no `lai lai` cell.
    assert_eq!(
        cells
            .iter()
            .filter(|c| c.0.is_none() && c.1 == "lai-lai")
            .count(),
        1,
        "one typed-join cell; got {cells:?}"
    );
    assert!(
        !cells.iter().any(|c| c.1 == "lai lai"),
        "no space join once a separator was typed; got {cells:?}"
    );
}

#[test]
fn typed_separator_renders_under_poj() {
    let _lock = engine_install_lock();
    install_fixture();
    let cells = fetch("goa--si", "poj", false);
    let cell = cell_with_hanji(&cells, "我是");
    assert_eq!(cell.1, "góa--sī", "POJ spelling, typed khinsiann");
    assert_eq!(cell.3, "guá sī", "canonical TL keeps the space");
}

#[test]
fn typed_separator_follows_the_hyphenless_setting() {
    let _lock = engine_install_lock();
    install_fixture();
    let cells = fetch("gua--si", "tl", true);
    assert_eq!(cell_with_hanji(&cells, "我是").1, "guá\u{00b7}sī");
    let cells = fetch("gua-si", "tl", true);
    assert_eq!(cell_with_hanji(&cells, "我是").1, "guásī");
    assert_eq!(cells[0].1, "gua-si", "the literal keeps the typed hyphen");
}

// ---- A typed `-` inside a dictionary word (USER 2026-09-22, §55) ----
// `pang-tang-lai` read 放重利 as the record stores it, `pàng tāng-lāi`
// (a MOE dictionary phrase with a space); the user typed the hyphen, so the rendered
// separator is the hyphen. Fixture: 放/pàng + 重利/tāng-lāi singles and the
// phrase 放重利/`pàng tāng-lāi`, so the whole buffer is one dictionary edge.

fn pang_tang_lai_rows() -> Vec<Row> {
    vec![
        Row {
            toneless_key: "pang",
            hanzi: "放",
            tl: "pàng",
            syll: 1,
            freq: 80_000,
        },
        Row {
            toneless_key: "tanglai",
            hanzi: "重利",
            tl: "tāng-lāi",
            syll: 2,
            freq: 25,
        },
        Row {
            toneless_key: "pangtanglai",
            hanzi: "放重利",
            tl: "pàng tāng-lāi",
            syll: 3,
            freq: 16,
        },
    ]
}

#[test]
fn typed_hyphen_replaces_the_records_space() {
    let _lock = engine_install_lock();
    install_rows(&pang_tang_lai_rows(), &["pang3", "tang7", "lai7"]);
    for (raw, roman) in [
        ("pangtanglai", "pàng tāng-lāi"),
        ("pang-tang-lai", "pàng-tāng-lāi"),
        ("pang3-tang7-lai", "pàng-tāng-lāi"),
        ("PANG-TANG-LAI", "PÀNG-TĀNG-LĀI"),
        // `--` is not the record's kind: the walker builds 放 + 重利 and
        // the typed run is its join.
        ("pang--tang-lai", "pàng--tāng-lāi"),
    ] {
        let cells = fetch(raw, "tl", false);
        let cell = cell_with_hanji(&cells, "放重利");
        assert_eq!(cell.1, roman, "{raw}: rendered roman; got {cells:?}");
        assert_eq!(
            cell.3.to_lowercase(),
            "pàng tāng-lāi",
            "{raw}: canonical_tl (identity) keeps the record's space"
        );
        assert_eq!(
            cells
                .iter()
                .filter(|c| c.0.as_deref() == Some("放重利"))
                .count(),
            1,
            "{raw}: one 放重利 cell; got {cells:?}"
        );
    }
    let cells = fetch("pang-tang-lai", "poj", false);
    assert_eq!(cell_with_hanji(&cells, "放重利").1, "pàng-tāng-lāi");
    // No Hyphens drops the typed `-` like any other (§49).
    let cells = fetch("pang-tang-lai", "tl", true);
    assert_eq!(cell_with_hanji(&cells, "放重利").1, "pàngtānglāi");
}

// ---- Segment-by-segment commit keeps the typed run (USER 2026-09-22) ----
// `tng--lai` → pick 轉 → pick 來 committed `tńg-lâi`: the run folded into
// 來's `raw_text` and the nailed-prefix join never read it (the compound
// oracle then supplied its own `-`). Fixture: 轉/tńg + 來/lâi singles and
// 轉來/tńg--lâi as the compound the oracle finds.

fn tng_lai_rows() -> Vec<Row> {
    vec![
        Row {
            toneless_key: "tng",
            hanzi: "轉",
            tl: "tńg",
            syll: 1,
            freq: 50_000,
        },
        Row {
            toneless_key: "lai",
            hanzi: "來",
            tl: "lâi",
            syll: 1,
            freq: 60_000,
        },
        Row {
            toneless_key: "tnglai",
            hanzi: "轉來",
            tl: "tńg--lâi",
            syll: 2,
            freq: 3_000,
        },
    ]
}

fn install_tng_lai() {
    install_rows(&tng_lai_rows(), &["tng2", "lai5"]);
}

fn started(raw: &str, cfg: &AppConfig) -> Engine {
    let mut engine = Engine::new();
    for method in [
        Method::Start(Start { text: raw.into() }),
        Method::EnterContinuous(EnterContinuous {}),
    ] {
        dispatch::handle(&req(method), &mut engine, cfg).expect("setup");
    }
    engine
}

/// The single-syllable candidate for `hanji`, as the strip lists it.
fn single(engine: &mut Engine, cfg: &AppConfig, hanji: &str) -> CandidateMessage {
    let resp = dispatch::handle(&req(Method::FetchAtPos(FetchAtPos::default())), engine, cfg)
        .expect("FetchAtPos");
    let cands = resp.continuous.expect("continuous").candidates;
    cands
        .iter()
        .find(|c| c.hanji.as_deref() == Some(hanji) && c.syllable_count == 1)
        .cloned()
        .unwrap_or_else(|| panic!("no single {hanji}; got {cands:?}"))
}

/// `CommitContinuous` the way a platform tap sends it: `display_text` is
/// the candidate's roman under romanization output, its hanji under Hanji output.
fn pick(engine: &mut Engine, cfg: &AppConfig, hanji: &str) -> ComposingResponse {
    let c = single(engine, cfg, hanji);
    let display_text = if cfg.is_translate_swapped {
        c.display_text.clone()
    } else {
        c.roman.clone()
    };
    dispatch::handle(
        &req(Method::CommitContinuous(CommitContinuous {
            display_text,
            canonical_text: c.display_text.clone(),
            association_tl: c.canonical_tl.clone(),
            hanji: c.hanji.clone(),
            consumed_bytes: c.consumed_span_end,
            syllable_count: c.syllable_count,
        })),
        engine,
        cfg,
    )
    .expect("CommitContinuous")
}

fn preedit(resp: &ComposingResponse) -> Option<String> {
    resp.effect.iter().find_map(|e| match e.kind.as_ref() {
        Some(Kind::UpdatePreedit(p)) => Some(p.display.clone()),
        _ => None,
    })
}

fn roman_cfg(hyphenless: bool) -> AppConfig {
    AppConfig {
        is_translate_swapped: false,
        hyphenless_roman: hyphenless,
        ..config("tl")
    }
}

#[test]
fn two_picks_commit_the_typed_run_between_them() {
    let _lock = engine_install_lock();
    install_tng_lai();
    for (raw, mid, fin) in [
        ("tng--lai", "tńg--lai", "tńg--lâi"),
        ("tng-lai", "tńg-lai", "tńg-lâi"),
        // Nothing typed: 轉來 is a dictionary compound, so the oracle's
        // own `-` joins the two picks (the pending tail keeps the space).
        ("tnglai", "tńg lai", "tńg-lâi"),
    ] {
        let cfg = roman_cfg(false);
        let mut engine = started(raw, &cfg);
        let first = pick(&mut engine, &cfg, "轉");
        assert_eq!(
            preedit(&first).as_deref(),
            Some(mid),
            "{raw}: preedit after 轉"
        );
        let second = pick(&mut engine, &cfg, "來");
        assert_eq!(
            commit_text(&second).as_deref(),
            Some(fin),
            "{raw}: committed"
        );
    }
}

#[test]
fn two_picks_under_hyphenless_render_the_run_as_a_dot() {
    let _lock = engine_install_lock();
    install_tng_lai();
    for (raw, fin) in [
        ("tng--lai", "tńg·lâi"),
        ("tng-lai", "tńglâi"),
        // The oracle's joiner is the hyphen No Hyphens drops.
        ("tnglai", "tńglâi"),
    ] {
        let cfg = roman_cfg(true);
        let mut engine = started(raw, &cfg);
        pick(&mut engine, &cfg, "轉");
        let second = pick(&mut engine, &cfg, "來");
        assert_eq!(commit_text(&second).as_deref(), Some(fin), "{raw}");
    }
}

#[test]
fn two_picks_under_hanji_output_carry_no_separator() {
    let _lock = engine_install_lock();
    install_tng_lai();
    let cfg = AppConfig {
        is_translate_swapped: true,
        ..config("tl")
    };
    let mut engine = started("tng--lai", &cfg);
    let first = pick(&mut engine, &cfg, "轉");
    assert_eq!(preedit(&first).as_deref(), Some("轉--lai"));
    let second = pick(&mut engine, &cfg, "來");
    assert_eq!(commit_text(&second).as_deref(), Some("轉來"));
}

#[test]
fn enter_after_the_first_pick_commits_the_typed_tail_as_is() {
    let _lock = engine_install_lock();
    install_tng_lai();
    let cfg = roman_cfg(false);
    let mut engine = started("tng--lai", &cfg);
    pick(&mut engine, &cfg, "轉");
    let resp = dispatch::handle(&req(Method::CommitRaw(CommitRaw {})), &mut engine, &cfg)
        .expect("CommitRaw");
    assert_eq!(commit_text(&resp).as_deref(), Some("tńg--lai"));
}

#[test]
fn unnail_restores_the_run_and_a_repick_keeps_it() {
    // 轉 + 來 nailed over `tng--lai-khi`, pending `-khi`; delete the tail,
    // then one more backspace pops 來 and its raw `--lai` comes back as the
    // pending tail with its run — repicking 來 commits `tńg--lâi`.
    let _lock = engine_install_lock();
    let mut rows = tng_lai_rows();
    rows.push(Row {
        toneless_key: "khi",
        hanzi: "去",
        tl: "khì",
        syll: 1,
        freq: 40_000,
    });
    install_rows(&rows, &["tng2", "lai5", "khi3"]);

    let cfg = roman_cfg(false);
    let mut engine = started("tng--lai-khi", &cfg);
    pick(&mut engine, &cfg, "轉");
    let mid = pick(&mut engine, &cfg, "來");
    assert_eq!(preedit(&mid).as_deref(), Some("tńg--lâi-khi"));
    let mut last = None;
    for _ in 0..5 {
        last = Some(
            dispatch::handle(
                &req(Method::DeleteBackward(DeleteBackward {})),
                &mut engine,
                &cfg,
            )
            .expect("DeleteBackward"),
        );
    }
    assert_eq!(
        preedit(last.as_ref().unwrap()).as_deref(),
        Some("tńg--lai"),
        "來 popped, its raw run back in the tail"
    );
    let fin = pick(&mut engine, &cfg, "來");
    assert_eq!(commit_text(&fin).as_deref(), Some("tńg--lâi"));
}
