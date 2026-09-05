//! TPS visual-dedup integration test — covers the bug where TPS input
//! `ㄨㄢ` returned two `灣` candidates because two `dict.bin` rows
//! (`灣/uan` and `灣/uân`) share the toneless TPS key `tps:ㄨㄢ` and the
//! pre-sort `(roman, hanji, consumed_span)` dedupe legitimately keeps
//! both for TL/POJ. TPS mode hides romanization (hanji-only UI), so
//! `composing::continuous::dedupe_display_hanji_for_tps` collapses the
//! visible duplicate. TL/POJ paths must NOT collapse — distinct
//! romanizations are distinct rows in their UI.
//!
//! Hermetic install of `LexiconHandle` mirrors `golden_fetch_at_pos.rs`
//! (this binary is its own process with its own singleton; the lock
//! guards in-binary `#[test]` parallelism). Fixture builders copy the
//! same TKDB v3 / dictionary.fst / syllables.fst byte layout — composing
//! tests cannot import `lexicon/tests/common/mod.rs` (test-private).

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};

use composing::api::Engine;
use composing::dispatch;
use fst::SetBuilder;
use lexicon::{EngineHandle as LexiconHandle, LexiconPaths};
use phonetics::canonicalize_syllable;
use protos::engine::composing_request::Method;
use protos::engine::{AppConfig, ComposingRequest, EnterContinuous, FetchAtPos, Start};

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
    let path = std::env::temp_dir().join(format!("composing-tps-dedup-{pid}-{name}"));
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
    // The reported bug: two 灣 rows differ only on tone (and hence
    // romanization), share toneless TPS key `tps:ㄨㄢ`. Mirrors production
    // `dictionary.csv:lines 灣,uan` + `灣,uân`.
    vec![
        Row {
            toneless_key: "uan",
            hanzi: "灣",
            tl: "uan",
            syll: 1,
            freq: 9981,
        },
        Row {
            toneless_key: "uan",
            hanzi: "灣",
            tl: "uân",
            syll: 1,
            freq: 7984,
        },
        // Sanity row to prove the dedupe is hanji-keyed and not roman-keyed:
        // distinct hanji on the same toneless key — must always survive.
        Row {
            toneless_key: "uan",
            hanzi: "彎",
            tl: "uan",
            syll: 1,
            freq: 100,
        },
    ]
}

fn install_fixture() {
    let rows = fixture_rows();
    let dict_path = write_temp("dictionary.bin", &build_tkdb_v3(&rows));
    let fst_path = build_dictionary_fst(&rows);
    let assoc_path = write_temp("association.bin", &empty_association_bin());
    let syllables_path = build_syllables_fst(&["uan1", "uan5"]);
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

fn config(input_mode: &str) -> AppConfig {
    AppConfig {
        tone_mode: String::new(),
        input_mode: input_mode.to_string(),
        oo_doubletap_enabled: false,
        nn_doubletap_enabled: false,
        is_translate_swapped: false,
        is_association_recording_enabled: false,
        platform_id: 0,
        output_both_scripts: false,
        candidate_display_mode: 0,
    }
}

fn req(method: Method) -> ComposingRequest {
    ComposingRequest {
        method: Some(method),
    }
}

fn fetch(raw: &str, input_mode: &str) -> Vec<(Option<String>, String)> {
    let cfg = config(input_mode);
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

#[test]
fn tps_input_collapses_duplicate_hanji() {
    let _lock = engine_install_lock();
    install_fixture();
    // Raw `ㄨㄢ` (3+3=6 bytes UTF-8). `dispatch::handle` upgrades mode to
    // InputMode::Tps via `contains_tps`, so the `input_mode` string only
    // pins the non-Bopomofo path — we still pass "tl" to exercise the
    // production auto-detect.
    let candidates = fetch("\u{3128}\u{3122}", "tl");
    let wan_count = candidates
        .iter()
        .filter(|(h, _)| h.as_deref() == Some("灣"))
        .count();
    let wan2_count = candidates
        .iter()
        .filter(|(h, _)| h.as_deref() == Some("彎"))
        .count();
    assert_eq!(
        wan_count, 1,
        "TPS mode must collapse the two 灣 rows (uan + uân) to one visible \
         candidate; got {candidates:?}"
    );
    assert_eq!(
        wan2_count, 1,
        "distinct hanji 彎 must always survive the TPS dedupe; got {candidates:?}"
    );
}

#[test]
fn tl_input_keeps_both_uan_and_uan_diacritic_rows() {
    let _lock = engine_install_lock();
    install_fixture();
    // Same fixture, TL toneless input `uan` — both 灣 rows differ on the
    // displayed `roman` (`uan` vs `uân`) so the TL UI shows distinct rows
    // and must NOT collapse. Exact-count + roman-set pin guards against a
    // duplicate explosion or a roman-render regression sneaking past.
    let candidates = fetch("uan", "tl");
    let mut wan_romans: Vec<&str> = candidates
        .iter()
        .filter(|(h, _)| h.as_deref() == Some("灣"))
        .map(|(_, r)| r.as_str())
        .collect();
    wan_romans.sort();
    assert_eq!(
        wan_romans,
        vec!["uan", "uân"],
        "TL mode must keep exactly the two 灣 rows with romans {{uan, uân}}; \
         got {candidates:?}"
    );
}

#[test]
fn poj_input_keeps_both_oan_and_oan_diacritic_rows() {
    let _lock = engine_install_lock();
    install_fixture();
    // POJ input `oan` — same fixture, two 灣 rows derive POJ display
    // `oan` / `oân`. POJ UI shows romanization so both must survive.
    let candidates = fetch("oan", "poj");
    let mut wan_romans: Vec<&str> = candidates
        .iter()
        .filter(|(h, _)| h.as_deref() == Some("灣"))
        .map(|(_, r)| r.as_str())
        .collect();
    wan_romans.sort();
    assert_eq!(
        wan_romans,
        vec!["oan", "oân"],
        "POJ mode must keep exactly the two 灣 rows with romans {{oan, oân}}; \
         got {candidates:?}"
    );
}
