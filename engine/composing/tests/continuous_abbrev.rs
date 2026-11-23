//! Whole-buffer abbreviation candidates end to end (`Start →
//! FetchAtPos`) — `behavioral-invariants.md` §46, USER
//! 2026-09-18 `ss` → 鎖匙 `só-sî`. Step 4c of
//! `composing::continuous::assemble_candidates` + `shadow::abbrev_query_key` +
//! `lexicon::fetch_abbrev_candidates`; the per-family face checks and the
//! sort live in `lexicon/tests/whole_buffer_abbrev.rs`, this file pins what
//! only the assembled vector shows: block order, cross-batch dedupe, the
//! span-local branch, recase, POJ render, the custom merge.
//!
//! `common::build_dictionary_fst` emits the `*_abbrev` keys like
//! `create_fst.py` does.

use composing::api::{CaretDirection, CommitScript, ContinuousPick};
use composing::{requests, Engine, Intent};
use protos::engine::composing_request::Method;
use protos::engine::{
    AppConfig, Append, CandidateDisplayMode, CandidateMessage, CellScript, CommitContinuous,
    FetchAtPos, Start,
};

use crate::common::Fetch;
use crate::common::{
    build_dictionary_fst, build_syllables_fst, build_tkdb_v4, config, config_with_display_mode,
    continuous, empty_association_bin, fetch_at_pos_response, fetch_hanji, fetch_hanji_with_custom,
    fetched_engine, install_lexicon, pick_of, send, Row,
};
use lexicon::CustomEntry;
use test_support::{engine_install_lock, write_temp};

/// Abbreviation-only rows under `mk` beyond the ones that are also partial
/// extensions — enough that the block would overflow the output cap.
const MK_ABBREV_ONLY_ROWS: usize = 30;

fn fixture_rows() -> Vec<Row> {
    let mut rows = vec![
        // trace: abbrev "ss" (só + sî), toneless "sosi".
        Row {
            toneless_key: "sosi",
            hanji: "鎖匙",
            tl: "só-sî",
            syll: 2,
            freq: 100,
        },
        // trace: abbrev "ss" too, higher freq → ranks ahead of 鎖匙.
        Row {
            toneless_key: "siansenn",
            hanji: "先生",
            tl: "sian-senn",
            syll: 2,
            freq: 400,
        },
        // trace: single syllable, toneless "tse" — a partial-prefix hit for
        // `ts` that must stay ahead of every abbreviation hit.
        Row {
            toneless_key: "tse",
            hanji: "這",
            tl: "tse",
            syll: 1,
            freq: 27_958,
        },
        // trace: abbrev "tss" (leading units ts + s — `ts` is one unit).
        Row {
            toneless_key: "tsasi",
            hanji: "早時",
            tl: "tsá-sî",
            syll: 2,
            freq: 50,
        },
        // trace: `m` is a valid left-anchored syllable (毋), so buffer `mk`
        // takes the span-local branch; 物件 (abbrev "mk") must still surface.
        Row {
            toneless_key: "m",
            hanji: "毋",
            tl: "m̄",
            syll: 1,
            freq: 39_475,
        },
        Row {
            toneless_key: "mihkiann",
            hanji: "物件",
            tl: "mi̍h-kiānn",
            syll: 2,
            freq: 244,
        },
        // trace: TL abbrev "tsp" (leading units ts + p); POJ display
        // `chia̍h-pn̄g` → POJ abbrev "chp".
        Row {
            toneless_key: "tsiahpng",
            hanji: "食飯",
            tl: "tsia̍h-pn̄g",
            syll: 2,
            freq: 144,
        },
        // trace: abbrev "si" (sóo + í: a vowel-initial syllable keeps its
        // vowel, `derive_abbrev`), toneless "sooi" — `si` lists it as an
        // abbreviation row only.
        Row {
            toneless_key: "sooi",
            hanji: "所以",
            tl: "sóo-í",
            syll: 2,
            freq: 300,
        },
        // trace: abbrev "ai" AND toneless "ai" — `ai` lists it as a reading
        // row and drops the abbreviation twin (`retain_absent_from`).
        Row {
            toneless_key: "ai",
            hanji: "阿姨",
            tl: "a-î",
            syll: 2,
            freq: 300,
        },
        // trace: abbrev "am", toneless "ama" — `am` is a strict prefix of
        // the toneless key, so `am` lists it as a Step 4b prefix row and
        // drops the abbreviation twin.
        Row {
            toneless_key: "ama",
            hanji: "阿嬤",
            tl: "a-má",
            syll: 2,
            freq: 300,
        },
    ];
    // trace: toneless `mk…` AND abbrev "mk" — under buffer `mk` each of
    // these is a Step 4b partial extension (`mk` reaches one glyph into
    // the second syllable, §43 admits it) AND an abbreviation hit. High
    // freq so they lead the abbreviation pool and would eat a pre-exclude
    // cap.
    for (hanji, tl, toneless_key) in [
        ("毋肯", "m̄-khíng", "mkhing"),
        ("毋甘", "m̄-kam", "mkam"),
        ("毋管", "m̄-kuán", "mkuan"),
        ("毋驚", "m̄-kiann", "mkiann"),
        ("毋見", "m̄-kìnn", "mkinn"),
        ("毋過", "m̄-koh", "mkoh"),
        ("毋捌", "m̄-kok", "mkok"),
    ] {
        rows.push(Row {
            toneless_key,
            hanji,
            tl,
            syll: 2,
            freq: 5_000,
        });
    }
    // trace: abbrev "mk" only (toneless `moka` does not start with `mk`).
    for i in 0..MK_ABBREV_ONLY_ROWS {
        rows.push(Row {
            toneless_key: "moka",
            hanji: Box::leak(format!("詞{i}").into_boxed_str()),
            tl: "mó-ka",
            syll: 2,
            freq: 10,
        });
    }
    rows
}

fn install_fixture() {
    let rows = fixture_rows();
    let dict_path = write_temp("dictionary.bin", &build_tkdb_v4(&rows));
    let fst_path = build_dictionary_fst(&rows);
    let association_path = write_temp("association.bin", &empty_association_bin());
    // `m7` makes `m` a lattice syllable (the span-local branch for `mk`).
    let syllables_path = build_syllables_fst(&["m7", "tse1", "so2", "si5"]);
    install_lexicon(&fst_path, &dict_path, &association_path, &syllables_path);
}

fn fetch(raw: &str, input_mode: &str) -> Vec<CandidateMessage> {
    fetch_at_pos_response(&config(input_mode), raw, Fetch::default())
        .continuous
        .map(|c| c.candidates)
        .unwrap_or_default()
}

fn hanji(raw: &str, input_mode: &str) -> Vec<String> {
    fetch_hanji(raw, input_mode, Fetch::default())
}

// INVARIANT_CONTINUOUS_WHOLE_BUFFER_ABBREV (behavioral-invariants.md §46)
#[test]
fn ss_surfaces_abbreviated_words_after_the_literal() {
    let _lock = engine_install_lock();
    install_fixture();
    let cands = fetch("ss", "tl");
    // §34 literal keeps index 0; the abbreviation block follows, higher
    // dictionary frequency first, each committing the whole buffer.
    assert_eq!(
        (cands[0].roman.as_str(), cands[0].hanji.as_deref()),
        ("ss", None)
    );
    assert_eq!(hanji("ss", "tl"), vec!["先生", "鎖匙"], "{cands:?}");
    let sosi = &cands[2];
    assert_eq!(sosi.roman, "só-sî");
    assert_eq!(sosi.consumed_span_end, 2);
}

#[test]
fn abbreviation_block_trails_partial_hits() {
    let _lock = engine_install_lock();
    install_fixture();
    // `ts` is a valid onset and one leading unit: the single-syllable
    // partial hit 這 keeps its place; 早時 abbreviates to `tss`, not `ts`,
    // and 食飯 to `tsp`, so neither joins the `ts` strip.
    assert_eq!(hanji("ts", "tl"), vec!["這"]);
    // `tss` has no syllabic reading: the abbreviation block is all there is.
    assert_eq!(hanji("tss", "tl"), vec!["早時"]);
    assert_eq!(hanji("tsp", "tl"), vec!["食飯"]);
}

#[test]
fn span_local_branch_dedupes_partial_overlap_before_capping() {
    let _lock = engine_install_lock();
    install_fixture();
    // `mk`: `m` is a lattice syllable so the fetch takes the span-local
    // branch (毋); the seven `m̄-k…` rows arrive as Step 4b partial
    // extensions AND as abbreviation hits and must show once; the
    // abbreviation block is then filled to the cap from the rows the
    // exclude did not touch (物件 + the 30 `mó-ka` rows → 30 survive).
    // Every two-syllable row covers the whole input, so 毋 — covering only
    // `m` — follows them all (tier first, USER 2026-10-12). An abbreviation
    // ties a full syllable on `coverage_kind`, so the abbreviation block
    // leads the seven partial extensions.
    let cands = fetch("mk", "tl");
    let hanji: Vec<&str> = cands.iter().filter_map(|c| c.hanji.as_deref()).collect();
    assert_eq!(hanji.last(), Some(&"毋"), "{hanji:?}");
    assert_eq!(hanji[0], "物件", "abbreviation first: {hanji:?}");
    let partials = &hanji[hanji.len() - 8..hanji.len() - 1];
    assert!(
        partials
            .iter()
            .all(|word| word.starts_with('毋') && word.chars().count() == 2),
        "the partial extensions follow the abbreviations: {hanji:?}"
    );
    let distinct: std::collections::HashSet<&str> = hanji.iter().copied().collect();
    assert_eq!(
        distinct.len(),
        hanji.len(),
        "no candidate shown twice: {hanji:?}"
    );
    let two_syllable = cands.iter().filter(|c| c.syllable_count == 2).count();
    assert_eq!(
        two_syllable,
        7 + lexicon::PARTIAL_PREFIX_OUTPUT_CAP,
        "7 partial overlaps + a full abbreviation block; got {hanji:?}"
    );
    let mihkiann = cands
        .iter()
        .find(|c| c.hanji.as_deref() == Some("物件"))
        .expect("物件 via the span-local branch");
    assert_eq!(mihkiann.consumed_span_end, 2);
    assert_eq!(mihkiann.syllable_count, 2, "record's own syllable count");
}

#[test]
fn uppercase_buffer_recases_the_abbreviated_roman() {
    let _lock = engine_install_lock();
    install_fixture();
    let cands = fetch("SS", "tl");
    let sosi = cands
        .iter()
        .find(|c| c.hanji.as_deref() == Some("鎖匙"))
        .expect("鎖匙");
    assert_eq!(sosi.roman, "SÓ-SÎ");
}

#[test]
fn syllabic_and_separated_buffers_get_no_abbreviation_hits() {
    let _lock = engine_install_lock();
    install_fixture();
    // `s` alone: one glyph, no word abbreviates to it; `s-s` / `s s` / `ss2`
    // carry a non-consonant and stay on the syllable paths.
    for raw in ["s", "s-s", "s s", "ss2"] {
        let hanji = hanji(raw, "tl");
        assert!(
            !hanji.iter().any(|h| h == "鎖匙" || h == "先生"),
            "{raw:?} must not surface abbreviation hits; got {hanji:?}"
        );
    }
    // `sosi` (the syllables) reaches 鎖匙 through the toneless key as before.
    assert!(hanji("sosi", "tl").iter().any(|h| h == "鎖匙"));
}

#[test]
fn poj_mode_matches_the_poj_abbreviation_and_renders_poj() {
    let _lock = engine_install_lock();
    install_fixture();
    // POJ `chia̍h-pn̄g` abbreviates to `chp` (TL `tsp` is not a POJ key).
    let cands = fetch("chp", "poj");
    let tsiahpng = cands
        .iter()
        .find(|c| c.hanji.as_deref() == Some("食飯"))
        .unwrap_or_else(|| panic!("食飯 via poj-abbrev:chp; got {cands:?}"));
    assert_eq!(tsiahpng.roman, "chia̍h-pn̄g", "POJ presentation");
    assert_eq!(tsiahpng.canonical_tl, "tsia̍h-pn̄g", "identity stays TL");
    assert!(!hanji("tsp", "poj").iter().any(|h| h == "食飯"));
    assert!(!hanji("chp", "tl").iter().any(|h| h == "食飯"));
    // `ss` is the same acronym in both scripts.
    assert!(hanji("ss", "poj").iter().any(|h| h == "鎖匙"));
}

#[test]
fn tps_initials_match_the_tps_abbreviation() {
    let _lock = engine_install_lock();
    install_fixture();
    let cands = fetch("ㄙㄒ", "tps");
    let sosi = cands
        .iter()
        .find(|c| c.hanji.as_deref() == Some("鎖匙"))
        .unwrap_or_else(|| panic!("鎖匙 via tps:ㄙㄒ; got {cands:?}"));
    assert_eq!(sosi.consumed_span_end, 6, "TPS span is UTF-8 bytes");
    // A tone mark on the tail disqualifies the buffer.
    assert!(!hanji("ㄙㄒˊ", "tps").iter().any(|h| h == "鎖匙"));
}

#[test]
fn custom_entry_sharing_the_abbreviated_word_shows_once() {
    let _lock = engine_install_lock();
    install_fixture();
    // A custom 鎖匙 `só-sî` answers `ss` in the abbreviation block, by the
    // exact-face rule a dictionary row answers to (whole-buffer synth,
    // source rank 0). The dictionary's own 鎖匙 collapses onto it — same
    // `(roman, hanji, span)` — and, as the user's own word, it leads the
    // block (`custom_lead`, USER 2026-10-12).
    let hanji = fetch_hanji_with_custom(
        "ss",
        "tl",
        vec![CustomEntry {
            roman: "só-sî".into(),
            hanji: Some("鎖匙".into()),
        }],
    );
    assert_eq!(
        hanji,
        vec!["鎖匙", "先生"],
        "custom first, dictionary 鎖匙 deduped onto it"
    );
}

//
// A pick is an abbreviation pick when it names a row the last fetch listed
// as a whole-buffer abbreviation (`COVERAGE_KIND_ABBREV`) for the same
// composing state — the row whose rank the pick should raise. These fetch
// the list, then pick a row from it, as a platform does.

/// A fresh engine composing `raw` under `config`, and the list a fetch with
/// `fetch` answered (recorded on the engine, `requests::apply`).
fn listed(raw: &str, config: &AppConfig, fetch: Fetch) -> (Engine, Vec<CandidateMessage>) {
    let (engine, response) = fetched_engine(config, raw, fetch);
    let candidates = response
        .continuous
        .map(|c| c.candidates)
        .unwrap_or_default();
    (engine, candidates)
}

/// The first listed cell of `hanji` consuming `end` bytes.
fn row_of<'a>(candidates: &'a [CandidateMessage], hanji: &str, end: u32) -> &'a CandidateMessage {
    candidates
        .iter()
        .find(|c| c.hanji.as_deref() == Some(hanji) && c.consumed_span_end == end)
        .unwrap_or_else(|| panic!("no {hanji} ending at {end}: {candidates:?}"))
}

/// Whether the pick `intent` counted as an abbreviation pick; `None` when it
/// counted nothing.
fn abbreviation(engine: &mut Engine, intent: Intent, config: &AppConfig) -> Option<bool> {
    engine
        .apply_learning(intent, config)
        .usage
        .map(|usage| usage.abbreviation)
}

#[test]
fn a_pick_of_a_listed_abbreviation_row_is_an_abbreviation_pick() {
    let _lock = engine_install_lock();
    install_fixture();
    let config = config("tl");
    let (mut engine, candidates) = listed("si", &config, Fetch::default());
    let sooi = pick_of(row_of(&candidates, "所以", 2), CommitScript::Lead);
    assert_eq!(abbreviation(&mut engine, sooi, &config), Some(true));
}

#[test]
fn a_pick_of_the_words_full_row_is_a_reading_pick() {
    let _lock = engine_install_lock();
    install_fixture();
    let config = config("tl");
    let (mut engine, candidates) = listed("ai", &config, Fetch::default());
    let ai = pick_of(row_of(&candidates, "阿姨", 2), CommitScript::Lead);
    assert_eq!(abbreviation(&mut engine, ai, &config), Some(false));
}

// The root cause (PR #1057 follow-up): `am` is both 阿嬤's abbreviation and a
// strict prefix of its toneless key `ama`; the list shows the prefix row, so
// the pick must raise the reading weight that row ranks by.
#[test]
fn a_pick_of_the_words_prefix_row_is_a_reading_pick() {
    let _lock = engine_install_lock();
    install_fixture();
    let config = config("tl");
    let (mut engine, candidates) = listed("am", &config, Fetch::default());
    let ama = pick_of(row_of(&candidates, "阿嬤", 2), CommitScript::Lead);
    assert_eq!(abbreviation(&mut engine, ama, &config), Some(false));
}

#[test]
fn a_word_typed_in_full_is_a_reading_pick() {
    let _lock = engine_install_lock();
    install_fixture();
    let config = config("tl");
    let (mut engine, candidates) = listed("sooi", &config, Fetch::default());
    let sooi = pick_of(row_of(&candidates, "所以", 4), CommitScript::Lead);
    assert_eq!(abbreviation(&mut engine, sooi, &config), Some(false));
}

#[test]
fn a_pick_with_no_fetch_before_it_is_a_reading_pick() {
    let _lock = engine_install_lock();
    install_fixture();
    let config = config("tl");
    let (_, candidates) = listed("si", &config, Fetch::default());
    let sooi = pick_of(row_of(&candidates, "所以", 2), CommitScript::Lead);
    let mut unfetched = Engine::new();
    unfetched.apply(Intent::Start { text: "si".into() }, &config);
    assert_eq!(abbreviation(&mut unfetched, sooi, &config), Some(false));
}

// The record is keyed on what the list was built from (tail, nailed, list
// start), not on the keystrokes that led there: typing `a` after `si`
// leaves a record for `si` that a pick under `sia` does not match; deleting
// back to `si` restores the state the list was fetched for, whose list a
// refetch would answer the same, so the pick counts as an abbreviation.
#[test]
fn the_record_matches_the_state_it_was_listed_for() {
    let _lock = engine_install_lock();
    install_fixture();
    let config = config("tl");
    let (mut engine, candidates) = listed("si", &config, Fetch::default());
    let sooi = pick_of(row_of(&candidates, "所以", 2), CommitScript::Lead);
    let mut typed_on = engine.clone();
    typed_on.apply(Intent::Append { ch: "a".into() }, &config);
    assert_eq!(
        abbreviation(&mut typed_on, sooi.clone(), &config),
        Some(false)
    );
    engine.apply(Intent::Append { ch: "a".into() }, &config);
    engine.apply(Intent::DeleteBackward, &config);
    assert_eq!(abbreviation(&mut engine, sooi, &config), Some(true));
}

// A desktop caret move keeps the list on screen without a refetch; outside a
// conversion the list does not depend on the caret, so the record holds.
#[test]
fn a_caret_move_keeps_the_record() {
    let _lock = engine_install_lock();
    install_fixture();
    let config = config("tl");
    let (mut engine, candidates) = listed("si", &config, Fetch::default());
    let sooi = pick_of(row_of(&candidates, "所以", 2), CommitScript::Lead);
    engine.apply(
        Intent::MoveCaret {
            direction: Some(CaretDirection::Left),
        },
        &config,
    );
    assert_eq!(abbreviation(&mut engine, sooi, &config), Some(true));
}

// Hanji with Romanization splits a row into a Hanji cell and a roman cell;
// both carry the row's identity, so the roman cell counts as the row does.
#[test]
fn the_roman_cell_of_an_abbreviation_row_is_an_abbreviation_pick() {
    let _lock = engine_install_lock();
    install_fixture();
    let config = config_with_display_mode("tl", CandidateDisplayMode::Combined as i32);
    let (mut engine, candidates) = listed("si", &config, Fetch::default());
    let roman_cell = candidates
        .iter()
        .find(|c| c.display_text == "所以" && c.cell_script() == CellScript::Roman)
        .unwrap_or_else(|| panic!("a roman cell of 所以: {candidates:?}"));
    let pick = pick_of(roman_cell, CommitScript::Roman);
    assert_eq!(abbreviation(&mut engine, pick, &config), Some(true));
}

// trace: a custom 你好 `lí-hó` abbreviates to `lh` (`derive_custom_search_keys`
// abbrev form); the custom merge lists it as a whole-buffer
// `COVERAGE_KIND_ABBREV` synth (`push_custom_entries`).
#[test]
fn a_custom_abbreviation_row_is_an_abbreviation_pick() {
    let _lock = engine_install_lock();
    install_fixture();
    let config = config("tl");
    let fetch = Fetch {
        custom: vec![CustomEntry {
            roman: "lí-hó".into(),
            hanji: Some("你好".into()),
        }],
        ..Fetch::default()
    };
    let (mut engine, candidates) = listed("lh", &config, fetch);
    let pick = pick_of(row_of(&candidates, "你好", 2), CommitScript::Lead);
    assert_eq!(abbreviation(&mut engine, pick, &config), Some(true));
}

#[test]
fn a_pick_confirmed_with_a_literal_counts_as_the_row_does() {
    let _lock = engine_install_lock();
    install_fixture();
    let config = config("tl");
    let (mut engine, candidates) = listed("si", &config, Fetch::default());
    let row = row_of(&candidates, "所以", 2);
    let confirm = Intent::ConfirmCompositionThenInsert {
        candidate: Some(ContinuousPick {
            canonical_text: row.display_text.clone(),
            association_tl: row.canonical_tl.clone(),
            hanji: row.hanji.clone(),
            consumed_bytes: row.consumed_span_end as usize,
            syllable_count: row.syllable_count as u8,
            script: Some(CommitScript::Lead),
            roman: row.roman.clone(),
        }),
        text: "，".into(),
        auto_space_enabled: false,
        candidate_spacing_enabled: false,
    };
    assert_eq!(abbreviation(&mut engine, confirm, &config), Some(true));
}

/// The wire pick of 所以's whole-buffer row under `si`.
fn sooi_pick(config: &AppConfig) -> Method {
    let (_, candidates) = listed("si", config, Fetch::default());
    let row = row_of(&candidates, "所以", 2);
    Method::CommitContinuous(CommitContinuous {
        canonical_text: row.display_text.clone(),
        association_tl: row.canonical_tl.clone(),
        hanji: row.hanji.clone(),
        consumed_bytes: row.consumed_span_end,
        syllable_count: row.syllable_count,
        script: protos::engine::CommitScript::Lead as i32,
        roman: row.roman.clone(),
    })
}

#[test]
fn a_new_generation_forgets_the_record() {
    let _lock = engine_install_lock();
    install_fixture();
    let config = config("tl");
    let sooi = sooi_pick(&config);
    let fetched = |generation_after: u64| {
        let handle = continuous("si", 1, &config);
        send(
            &handle,
            1,
            Method::FetchAtPos(FetchAtPos::default()),
            &config,
        );
        // A new context composes the same buffer again (the drop resets).
        send(
            &handle,
            generation_after,
            Method::Start(Start { text: "si".into() }),
            &config,
        );
        handle
    };
    let same = fetched(1);
    let applied = send(&same, 1, sooi.clone(), &config);
    assert_eq!(applied.usage.map(|u| u.abbreviation), Some(true));
    let reset = fetched(2);
    let applied = send(&reset, 2, sooi, &config);
    assert_eq!(applied.usage.map(|u| u.abbreviation), Some(false));
}

// A worker-thread fetch answers from a clone; one for `s` finishing after the
// main thread typed `i` (and the `si` fetch landed) must not replace the `si`
// record — `si`'s 所以 abbreviation row would be forgotten.
#[test]
fn a_fetch_landing_after_the_state_moved_on_is_dropped() {
    let _lock = engine_install_lock();
    install_fixture();
    let config = config("tl");
    let sooi = sooi_pick(&config);
    let handle = continuous("s", 1, &config);
    let stale = handle.engine_at(1).expect("generation 1");
    send(
        &handle,
        1,
        Method::Append(Append { char: "i".into() }),
        &config,
    );
    send(
        &handle,
        1,
        Method::FetchAtPos(FetchAtPos::default()),
        &config,
    );
    let (_, listing) = requests::query_listing(&Fetch::default().intent(), &stale, &config);
    let listing = listing.expect("composing `s`");
    handle.remember_listing(1, Some(listing), &config);
    let applied = send(&handle, 1, sooi, &config);
    assert_eq!(applied.usage.map(|u| u.abbreviation), Some(true));
}

// A fetch from an older generation is dropped even when the new generation
// composes the same buffer: the `si` listing it carries matches the state, so
// only the generation check stops it.
#[test]
fn a_fetch_from_an_older_generation_is_dropped() {
    let _lock = engine_install_lock();
    install_fixture();
    let config = config("tl");
    let sooi = sooi_pick(&config);
    let handle = continuous("si", 1, &config);
    let older = handle.engine_at(1).expect("generation 1");
    let (_, listing) = requests::query_listing(&Fetch::default().intent(), &older, &config);
    let listing = listing.expect("composing `si`");
    // A new context composes the same buffer again (the drop resets).
    send(
        &handle,
        2,
        Method::Start(Start { text: "si".into() }),
        &config,
    );
    handle.remember_listing(1, Some(listing), &config);
    let applied = send(&handle, 2, sooi, &config);
    assert_eq!(applied.usage.map(|u| u.abbreviation), Some(false));
}
