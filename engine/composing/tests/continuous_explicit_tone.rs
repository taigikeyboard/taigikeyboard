//! Explicit-tone filtering integration test — covers the bug where a
//! continuous-input query carrying an explicit numeric tone (e.g. `tsua2`)
//! surfaced candidates for EVERY tone of the syllable, because the
//! continuous path stripped the tone digit and looked up the fused
//! toneless FST key (`tl:tsua`). After the fix a fully-toned span keeps
//! its digits (`tl:tsua2`) so `lookup_exact` / `lookup_prefix` filters to
//! exactly the typed tone — `tsua2` → 紙 (tsuá) only, NOT 蛇 (tsuâ).
//!
//! Toneless input is unchanged: `tsua` still surfaces every tone (the
//! continuous-input no-tone affordance). Pure same-syllable homophones
//! (紙/tsuá tone2 vs 蛇/tsuâ tone5) make the tone axis the ONLY difference,
//! so a leak from the span-local, walker slot-0, OR Step 4b prefix-scan
//! paths would show the wrong-tone row and fail the assertion.
//!
//! Hermetic install of `LexiconHandle` mirrors `golden_fetch_at_pos.rs` /
//! `tps_display_dedup.rs` (this binary is its own process with its own
//! singleton; the lock guards in-binary `#[test]` parallelism). The
//! fixture's `dictionary.fst` emits BOTH the toneless `tl:<tl_notone>` and
//! the toned `tl:<tl_num>` key families, matching production
//! `dictionary/build/create_fst.py:127-130` — without the toned keys the
//! tone filter would have nothing to hit.

// 中文: 明確聲調過濾整合測試 — 對齊使用者回報的 tai5 → tai2/tai3 全聲調 bug。
// 中文:   紙/tsuá(tone2) 與 蛇/tsuâ(tone5) 同音節不同調,共用去調鍵 tl:tsua。
// 中文:   修復後 tsua2 保留數字 → 只出 紙;tsua(去調) → 兩調皆出(去調輸入行為不變)。
// 中文:   fixture dict.fst 同發去調 + 含調家族(對齊 create_fst.py)。

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
    let path = std::env::temp_dir().join(format!("composing-explicit-tone-{pid}-{name}"));
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
        payload.extend_from_slice(&0u16.to_le_bytes()); // kautian_subtag (v3); 0 = none
        payload.extend_from_slice(row.hanzi.as_bytes());
        payload.extend_from_slice(row.tl.as_bytes());
    }
    for off in &offsets {
        out.extend_from_slice(&off.to_le_bytes());
    }
    out.extend_from_slice(&payload);
    out
}

/// Emits the toneless `tl:<tl_notone>` family AND the toned `tl:<tl_num>`
/// family (derived from the display `tl` via `phonetics::normalize_input`
/// — the SAME normalizer the runtime applies to numeric-tone input, so a
/// `tl:tsua2` runtime query byte-matches the fixture key). Production
/// `create_fst.py:127-130` emits both; without the toned keys the fix's
/// tone filter would have nothing to hit.
fn build_dictionary_fst(rows: &[Row]) -> PathBuf {
    let mut entries: Vec<Vec<u8>> = Vec::with_capacity(rows.len() * 2);
    for (idx, row) in rows.iter().enumerate() {
        let rowid = (idx + 1) as u32;
        let mut push_key = |body: &str| {
            let mut e = Vec::with_capacity(body.len() + 4 + 5);
            e.extend_from_slice(b"tl:");
            e.extend_from_slice(body.as_bytes());
            e.push(SEPARATOR);
            e.extend_from_slice(&rowid.to_le_bytes());
            entries.push(e);
        };
        push_key(row.toneless_key);
        let tl_num = phonetics::normalize_input(row.tl);
        if tl_num.bytes().any(|b| b.is_ascii_digit()) {
            push_key(&tl_num);
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

/// 紙/tsuá (tone2) and 蛇/tsuâ (tone5) share the toneless key `tsua` and
/// differ ONLY by tone — the minimal fixture that makes "explicit tone
/// filters" provable. 珠/tsu is an unrelated control row that must never
/// appear for a `tsua*` query.
fn fixture_rows() -> Vec<Row> {
    vec![
        Row {
            toneless_key: "tsua",
            hanzi: "紙",
            tl: "tsuá", // tone 2 → tl_num tsua2
            syll: 1,
            freq: 100,
        },
        Row {
            toneless_key: "tsua",
            hanzi: "蛇",
            tl: "tsuâ", // tone 5 → tl_num tsua5
            syll: 1,
            freq: 70,
        },
        Row {
            toneless_key: "tsu",
            hanzi: "珠",
            tl: "tsu",
            syll: 1,
            freq: 90,
        },
    ]
}

fn install_fixture() {
    let rows = fixture_rows();
    let dict_path = write_temp("dictionary.bin", &build_tkdb_v3(&rows));
    let fst_path = build_dictionary_fst(&rows);
    let assoc_path = write_temp("association.bin", &empty_association_bin());
    let syllables_path = build_syllables_fst(&["tsua2", "tsua5", "tsu"]);
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
    }
}

fn req(method: Method) -> ComposingRequest {
    ComposingRequest {
        method: Some(method),
    }
}

/// Drive `raw` through `Start → EnterContinuous → FetchAtPos` and return
/// the candidate hanji set.
fn fetch_hanji(raw: &str) -> Vec<String> {
    let cfg = config("tl");
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
                .filter_map(|cand| cand.hanji)
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn explicit_tone_filters_to_typed_tone() {
    let _lock = engine_install_lock();
    install_fixture();
    // The headline bug: `tsua2` (tone 2) must surface 紙 (tsuá) ONLY —
    // 蛇 (tsuâ, tone 5) shares the toneless key `tsua` but is the wrong
    // tone, so it must NOT appear via span-local, walker slot-0, or the
    // Step 4b prefix scan.
    let hanji = fetch_hanji("tsua2");
    assert!(
        hanji.iter().any(|h| h == "紙"),
        "tsua2 must surface 紙 (tsuá); got {hanji:?}"
    );
    assert!(
        !hanji.iter().any(|h| h == "蛇"),
        "tsua2 must NOT surface 蛇 (tsuâ, wrong tone); got {hanji:?}"
    );

    // Symmetric: tone 5 surfaces 蛇 only.
    let hanji5 = fetch_hanji("tsua5");
    assert!(
        hanji5.iter().any(|h| h == "蛇"),
        "tsua5 must surface 蛇 (tsuâ); got {hanji5:?}"
    );
    assert!(
        !hanji5.iter().any(|h| h == "紙"),
        "tsua5 must NOT surface 紙 (tsuá, wrong tone); got {hanji5:?}"
    );
}

#[test]
fn toneless_input_still_surfaces_all_tones() {
    let _lock = engine_install_lock();
    install_fixture();
    // Regression guard for the no-tone affordance: toneless `tsua` must
    // still surface BOTH 紙 (tsuá) and 蛇 (tsuâ). The fix only changes
    // fully-toned spans; toneless input keeps the all-tone fused-key
    // behavior.
    let hanji = fetch_hanji("tsua");
    assert!(
        hanji.iter().any(|h| h == "紙") && hanji.iter().any(|h| h == "蛇"),
        "toneless tsua must surface both 紙 and 蛇; got {hanji:?}"
    );
}

#[test]
fn longest_match_suppresses_shorter_prefix_syllable() {
    let _lock = engine_install_lock();
    install_fixture();
    // INVARIANT_CONTINUOUS_LONGEST_MATCH_PREFIX (USER 2026-05-31「免調也壓制」).
    // 珠/tsu is a SHORTER single syllable that is a strict prefix of tsua —
    // the reported bug's `ta` ⊂ `tai` / `tai5` shape. The span-local candidate
    // strip surfaces only the LONGEST single syllable at offset 0, so 珠 (tsu,
    // span 0–3) must NOT appear for either a toned or a toneless tsua* query.
    // The fixture's `syllables.fst` carries `tsua2` (build_syllables_fst), so
    // `tsua2` resolves to the toned single syllable exactly as production does.
    //
    // Toned: `tsua2` keeps 紙 (tsuá, tone 2) and drops BOTH 蛇 (wrong tone,
    // §17) AND 珠 (shorter prefix syllable, §18).
    let toned = fetch_hanji("tsua2");
    assert!(
        toned.iter().any(|h| h == "紙"),
        "tsua2 must surface 紙 (tsuá); got {toned:?}"
    );
    assert!(
        !toned.iter().any(|h| h == "珠"),
        "tsua2 must NOT surface 珠 (shorter prefix syllable); got {toned:?}"
    );

    // Toneless: `tsua` surfaces all tones of the longest syllable (紙 + 蛇)
    // but still drops the shorter 珠 — the suppression keys on span length,
    // not tone.
    let toneless = fetch_hanji("tsua");
    assert!(
        toneless.iter().any(|h| h == "紙") && toneless.iter().any(|h| h == "蛇"),
        "toneless tsua must surface both 紙 and 蛇; got {toneless:?}"
    );
    assert!(
        !toneless.iter().any(|h| h == "珠"),
        "toneless tsua must NOT surface 珠 (shorter prefix syllable); got {toneless:?}"
    );
}
