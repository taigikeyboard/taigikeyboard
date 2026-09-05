//! 候選詞顯示 = 羅馬字 display-dedup integration test (§44 /
//! `INVARIANT_ROMAN_ONLY_CELLS_COLLAPSE_SAME_ROMAN`). Roman-only cells hide
//! the hanji, so rows that differ only in hanji — 同音異字 `食/tsia̍h` +
//! `𤆬/tsia̍h`, and the §34 literal `tsiah` beside dict `隻/tsiah` — are
//! visible duplicates. `composing::dispatch` collapses them by the RENDERED
//! ROMAN alone AFTER the literal prepend; first-seen wins (top-ranked sorted
//! row, or the literal). Side-by-side (explicit, proto default `0`, or an
//! unknown value) keeps every row; TPS ignores the setting.
//!
//! The consumed span left the key on 2026-09-03 (§44). No fixture here can
//! produce the case it used to separate — two rows rendering the SAME roman
//! over DIFFERENT spans — because a row's toneless FST key is derived from
//! its own reading, so equal romans mean equal keys and equal consumed
//! slices; an attempt to install a hand-built row with a longer key and a
//! shorter reading does not surface at all. The span-agnostic key is pinned
//! on the helper instead
//! (`composing::dispatch::tests::dedupe_display_roman_collapses_same_roman_across_spans`).
//!
//! Hermetic `LexiconHandle` install mirrors `tps_display_dedup.rs`.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};

use composing::api::Engine;
use composing::dispatch;
use fst::SetBuilder;
use lexicon::{EngineHandle as LexiconHandle, LexiconPaths};
use phonetics::canonicalize_syllable;
use protos::engine::composing_request::Method;
use protos::engine::{
    AppConfig, CandidateDisplayMode, ComposingRequest, EnterContinuous, FetchAtPos, Start,
};

const SEPARATOR: u8 = 0xFF;
const RANK_NEUTRAL_BITMASK: u16 = 1u16 << 11;
const TKDB_HEADER_SIZE: usize = 16;

fn engine_install_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}

struct Row {
    toneless_key: &'static str,
    hanzi: &'static str,
    tl: &'static str,
    syll: u8,
    freq: u32,
}

fn write_temp(name: &str, bytes: &[u8]) -> PathBuf {
    let pid = std::process::id();
    let path = std::env::temp_dir().join(format!("composing-roman-only-dedup-{pid}-{name}"));
    std::fs::write(&path, bytes).expect("write temp fixture");
    path
}

fn build_tkdb_v3(rows: &[Row]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"TKDB");
    out.extend_from_slice(&3u32.to_le_bytes());
    out.extend_from_slice(&(rows.len() as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    let offset_table_size = rows.len() * 4;
    let mut offsets = Vec::<u32>::with_capacity(rows.len());
    let mut payload = Vec::<u8>::new();
    for row in rows {
        offsets.push((TKDB_HEADER_SIZE + offset_table_size + payload.len()) as u32);
        payload.extend_from_slice(&RANK_NEUTRAL_BITMASK.to_le_bytes());
        payload.extend_from_slice(&row.freq.to_le_bytes());
        payload.push(row.hanzi.len() as u8);
        payload.push(row.tl.len() as u8);
        payload.push(row.syll);
        payload.extend_from_slice(&0u16.to_le_bytes()); // kautian_subtag (v3); 0 = no kautian provenance
        payload.extend_from_slice(row.hanzi.as_bytes());
        payload.extend_from_slice(row.tl.as_bytes());
    }
    for off in &offsets {
        out.extend_from_slice(&off.to_le_bytes());
    }
    out.extend_from_slice(&payload);
    out
}

fn build_dictionary_fst(rows: &[Row]) -> PathBuf {
    let mut entries: Vec<Vec<u8>> = Vec::with_capacity(rows.len());
    for (idx, row) in rows.iter().enumerate() {
        let rowid = (idx + 1) as u32;
        // tl: family
        let mut e = Vec::with_capacity(row.toneless_key.len() + 4 + 5);
        e.extend_from_slice(b"tl:");
        e.extend_from_slice(row.toneless_key.as_bytes());
        e.push(SEPARATOR);
        e.extend_from_slice(&rowid.to_le_bytes());
        entries.push(e);
        // poj: family — derived per row via canonicalize_poj_syllable
        if let Some(poj_notone) = derive_poj_notone(row.tl) {
            let mut e2 = Vec::with_capacity(poj_notone.len() + 5 + 5);
            e2.extend_from_slice(b"poj:");
            e2.extend_from_slice(poj_notone.as_bytes());
            e2.push(SEPARATOR);
            e2.extend_from_slice(&rowid.to_le_bytes());
            entries.push(e2);
        }
        // tps: family + er↔or variant — same chain as create_fst.py
        let tps_notone = phonetics::tps_notone_from_tl(row.tl);
        if !tps_notone.is_empty() {
            let mut e3 = Vec::with_capacity(tps_notone.len() + 4 + 5);
            e3.extend_from_slice(b"tps:");
            e3.extend_from_slice(tps_notone.as_bytes());
            e3.push(SEPARATOR);
            e3.extend_from_slice(&rowid.to_le_bytes());
            entries.push(e3);
            let tps_notone_var = phonetics::tps_notone_or_variant(&tps_notone);
            if !tps_notone_var.is_empty() {
                let mut e4 = Vec::with_capacity(tps_notone_var.len() + 4 + 5);
                e4.extend_from_slice(b"tps:");
                e4.extend_from_slice(tps_notone_var.as_bytes());
                e4.push(SEPARATOR);
                e4.extend_from_slice(&rowid.to_le_bytes());
                entries.push(e4);
            }
        }
    }
    entries.sort();
    entries.dedup();
    let path = write_temp("dictionary.fst", &[]);
    let file = std::fs::File::create(&path).expect("create dictionary.fst");
    let mut builder = SetBuilder::new(std::io::BufWriter::new(file)).expect("fst builder");
    for entry in &entries {
        builder.insert(entry).expect("fst insert");
    }
    builder.finish().expect("fst finish");
    path
}

fn derive_poj_notone(tl_display: &str) -> Option<String> {
    let poj_display = phonetics::api::tl_display_to_poj_display(tl_display);
    let mut out = String::new();
    for token in poj_display.split(['-', ' ']) {
        if token.is_empty() {
            continue;
        }
        let (toneless, _) = phonetics::canonicalize_poj_syllable(token)?;
        out.push_str(&toneless);
    }
    (!out.is_empty()).then_some(out)
}

fn build_syllables_fst(samples: &[&str]) -> PathBuf {
    let mut keys: Vec<String> = Vec::new();
    for s in samples {
        let (canonical, tone) = canonicalize_syllable(s)
            .unwrap_or_else(|| panic!("syllable sample {s:?} failed canonicalize_syllable"));
        if tone.is_empty() {
            keys.push(format!("tl:{canonical}"));
        } else {
            keys.push(format!("tl:{canonical}{tone}"));
            keys.push(format!("tl:{canonical}"));
        }
        let poj_display = phonetics::api::tl_display_to_poj_display(s);
        if let Some((poj_canonical, poj_tone)) = phonetics::canonicalize_poj_syllable(&poj_display)
        {
            if poj_tone.is_empty() {
                keys.push(format!("poj:{poj_canonical}"));
            } else {
                keys.push(format!("poj:{poj_canonical}{poj_tone}"));
                keys.push(format!("poj:{poj_canonical}"));
            }
        }
        let numeric = phonetics::to_tone_number(s);
        let tps_with_tone = phonetics::tl_numeric_token_to_tps(&numeric, false, true);
        let tps_clean: String = tps_with_tone
            .chars()
            .filter(|&c| !c.is_whitespace() && c != '-')
            .collect();
        if !tps_clean.is_empty() {
            let tps_toneless: String = tps_clean
                .chars()
                .filter(|&c| !phonetics::is_tps_tone_mark(c))
                .collect();
            keys.push(format!("tps:{tps_clean}"));
            if !tps_toneless.is_empty() {
                keys.push(format!("tps:{tps_toneless}"));
            }
        }
    }
    keys.sort();
    keys.dedup();
    let path = write_temp("syllables.fst", &[]);
    let file = std::fs::File::create(&path).expect("create syllables.fst");
    let mut builder = SetBuilder::new(std::io::BufWriter::new(file)).expect("fst builder");
    for key in &keys {
        builder.insert(key.as_bytes()).expect("insert");
    }
    builder.finish().expect("finish");
    path
}

fn empty_association_bin() -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"TKWA");
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out
}

fn fixture_rows() -> Vec<Row> {
    // Toneless key `tsiah` carries three production-shaped rows: two 同音異字
    // readings `tsia̍h` (食 high-freq, 𤆬 low-freq) and the tone-4 `tsiah`
    // (隻) whose roman equals the §34 literal for raw input `tsiah`.
    vec![
        Row {
            toneless_key: "tsiah",
            hanzi: "食",
            tl: "tsia̍h",
            syll: 1,
            freq: 9000,
        },
        Row {
            toneless_key: "tsiah",
            hanzi: "𤆬",
            tl: "tsia̍h",
            syll: 1,
            freq: 3000,
        },
        Row {
            toneless_key: "tsiah",
            hanzi: "隻",
            tl: "tsiah",
            syll: 1,
            freq: 5000,
        },
    ]
}

fn install_fixture() {
    let rows = fixture_rows();
    let dict_path = write_temp("dictionary.bin", &build_tkdb_v3(&rows));
    let fst_path = build_dictionary_fst(&rows);
    let assoc_path = write_temp("association.bin", &empty_association_bin());
    let syllables_path = build_syllables_fst(&["tsiah8", "tsiah4"]);
    let paths = LexiconPaths::validated(
        fst_path.to_str().unwrap(),
        dict_path.to_str().unwrap(),
        assoc_path.to_str().unwrap(),
        syllables_path.to_str().unwrap(),
        2,
    )
    .expect("LexiconPaths::validated");
    LexiconHandle::install(paths).expect("EngineHandle::install");
}

const SIDE_BY_SIDE: i32 = CandidateDisplayMode::SideBySide as i32;
const ROMAN_ONLY: i32 = CandidateDisplayMode::RomanOnly as i32;

fn config(input_mode: &str, candidate_display_mode: i32) -> AppConfig {
    AppConfig {
        tone_mode: String::new(),
        input_mode: input_mode.to_string(),
        oo_doubletap_enabled: false,
        nn_doubletap_enabled: false,
        is_translate_swapped: false,
        is_association_recording_enabled: false,
        platform_id: 0,
        output_both_scripts: false,
        candidate_display_mode,
    }
}

fn req(method: Method) -> ComposingRequest {
    ComposingRequest {
        method: Some(method),
    }
}

/// `(hanji, roman)` per candidate, strip order.
fn fetch(
    raw: &str,
    input_mode: &str,
    candidate_display_mode: i32,
) -> Vec<(Option<String>, String)> {
    let cfg = config(input_mode, candidate_display_mode);
    let mut engine = Engine::new();
    dispatch::handle(
        &req(Method::Start(Start { text: raw.into() })),
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
    let resp = dispatch::handle(
        &req(Method::FetchAtPos(FetchAtPos {
            position: 0,
            frequency_entries: Vec::new(),
            now_ms: 0,
            custom_entries: Vec::new(),
            enabled_sources_bitmask: 0,
            literal_roman_candidate_disabled: false,
        })),
        &mut engine,
        &cfg,
    )
    .expect("FetchAtPos");
    resp.continuous
        .map(|c| {
            c.candidates
                .into_iter()
                .map(|cand| (cand.hanji, cand.roman))
                .collect()
        })
        .unwrap_or_default()
}

fn rows_with_roman<'a>(
    candidates: &'a [(Option<String>, String)],
    roman: &str,
) -> Vec<&'a (Option<String>, String)> {
    candidates.iter().filter(|(_, r)| r == roman).collect()
}

#[test]
fn roman_only_collapses_same_roman_rows_keeping_the_top_ranked_one() {
    let _lock = engine_install_lock();
    install_fixture();
    let candidates = fetch("tsiah", "tl", ROMAN_ONLY);

    // 食 (freq 9000) outranks 𤆬 (freq 3000) → 食 is the survivor.
    let tsiah8 = rows_with_roman(&candidates, "tsia̍h");
    assert_eq!(
        tsiah8.len(),
        1,
        "羅馬字 must collapse 食/𤆬 `tsia̍h` to one cell; got {candidates:?}"
    );
    assert_eq!(
        tsiah8[0].0.as_deref(),
        Some("食"),
        "survivor = top-ranked row"
    );

    // The §34 literal `tsiah` is inserted first, so it absorbs dict 隻/tsiah.
    let literal = rows_with_roman(&candidates, "tsiah");
    assert_eq!(
        literal.len(),
        1,
        "literal `tsiah` and dict 隻/tsiah must be one cell; got {candidates:?}"
    );
    assert_eq!(
        literal[0].0, None,
        "the literal keeps slot 0 and wins the collapse"
    );
    assert_eq!(candidates[0].1, "tsiah", "literal stays at index 0");
    assert_eq!(
        candidates.len(),
        2,
        "exactly two visible cells; got {candidates:?}"
    );
}

#[test]
fn side_by_side_keeps_every_row_for_explicit_default_and_unknown_values() {
    let _lock = engine_install_lock();
    install_fixture();
    let explicit = fetch("tsiah", "tl", SIDE_BY_SIDE);

    // Today's list: literal + 隻 + 食 + 𤆬, hanji-bearing rows all distinct.
    assert_eq!(
        explicit.len(),
        4,
        "side-by-side keeps all rows; got {explicit:?}"
    );
    assert_eq!(rows_with_roman(&explicit, "tsia̍h").len(), 2);
    assert_eq!(rows_with_roman(&explicit, "tsiah").len(), 2);

    // proto3 default (un-wired builds) and an unknown value from a newer
    // platform both normalise to side-by-side — one fallback, one place.
    assert_eq!(
        fetch("tsiah", "tl", 0),
        explicit,
        "UNSPECIFIED = side-by-side"
    );
    assert_eq!(
        fetch("tsiah", "tl", 99),
        explicit,
        "unknown value = side-by-side"
    );
    // 漢羅合用 keeps every row too — a one-label cell is distinct by (hanji, roman).
    assert_eq!(
        fetch("tsiah", "tl", CandidateDisplayMode::Combined as i32),
        explicit,
        "COMBINED = no engine collapse"
    );
}

#[test]
fn poj_roman_only_collapses_on_the_rendered_poj_roman() {
    let _lock = engine_install_lock();
    install_fixture();
    // POJ renders `tsia̍h` as `chia̍h`; the dedupe keys on what the user sees.
    let candidates = fetch("chiah", "poj", ROMAN_ONLY);
    let chiah8 = rows_with_roman(&candidates, "chia̍h");
    assert_eq!(
        chiah8.len(),
        1,
        "POJ 羅馬字 collapses 食/𤆬 `chia̍h`; got {candidates:?}"
    );
    assert_eq!(chiah8[0].0.as_deref(), Some("食"));
    assert_eq!(
        rows_with_roman(&candidates, "chiah").len(),
        1,
        "literal absorbs 隻/chiah"
    );
}

#[test]
fn tps_ignores_the_roman_only_setting() {
    let _lock = engine_install_lock();
    install_fixture();
    // Toneless Bopomofo for `tsiah`: `ㄐㄧㄚㆷ` (dispatch promotes the mode to
    // TPS from the buffer, so the "tl" string is irrelevant). TPS cells are
    // hanji-first, so 食 and 𤆬 stay distinct whatever the picker says.
    let tps: String = phonetics::tl_numeric_token_to_tps("tsiah4", false, true)
        .chars()
        .filter(|&c| !c.is_whitespace() && c != '-' && !phonetics::is_tps_tone_mark(c))
        .collect();
    assert!(!tps.is_empty());
    let roman_only = fetch(&tps, "tl", ROMAN_ONLY);
    let side_by_side = fetch(&tps, "tl", SIDE_BY_SIDE);
    assert_eq!(
        roman_only, side_by_side,
        "TPS output must not depend on 候選詞顯示"
    );
    let hanji: Vec<&str> = roman_only
        .iter()
        .filter_map(|(h, _)| h.as_deref())
        .collect();
    assert!(
        hanji.contains(&"食") && hanji.contains(&"𤆬"),
        "both 同音異字 stay in TPS; got {roman_only:?}"
    );
}
