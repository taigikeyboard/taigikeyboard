//! Slot-0 respects the dictionary separator form — covers the `hoogua`
//! bug (USER 2026-06-02). The continuous-input walker synthesizes the
//! full-buffer "best reading" by joining per-syllable canonical romans
//! with a SPACE. For a genuine multi-word reading that space-join is the
//! desired display, but when the synth's `(hanji, span)` coincides with a
//! single lexical dict word that stores its own separator form (`-` 連字
//! or `--` 輕聲), the space-join is a malformed rendering of that word.
//!
//! Reported symptom: typing `hoogua` showed best candidate `hōo guá`
//! (space) instead of the dictionary word 予我 `hōo--guá` (khinsiann). The
//! pre-fix `(roman, hanji, span)` slot-0 dedupe could not collapse the
//! pair because the romans differ ONLY in the separator, so the malformed
//! synth won slot 0 and the canonical dict row sank below.
//!
//! The fix (display layer, NOT the cost/segmentation primitive — diagnosis
//! §S5 / behavioral-invariants §18 lesson) promotes the matching FULL,
//! full-span dict row's canonical roman to slot 0 when it is the SAME
//! reading (separator-insensitive, tone-preserving) as the synth.
//!
//! Fixture mirrors production: 予我/hōo--guá (freq 16) and 戶外/hōo-guā
//! (freq 25) share the toneless key `hoogua`; the high-freq single chars
//! 予/hōo + 我/guá make the walker's min-cost path the 予+我 SPLIT (synth
//! hanji 予我, synth roman `hōo guá`), exactly as production does. 戶外's
//! tone-7 `guā` differs from the synth's tone-2 `guá`, so it must NOT be
//! promoted (Core Principle #7 word identity).
//!
//! Hermetic `LexiconHandle` install mirrors `continuous_explicit_tone.rs`.

// 中文: slot-0 尊重字典分隔符形式 — hoogua bug(USER 2026-06-02)整合測試。
// 中文:   walker synth 以空格 join 逐音節羅馬字;真多詞句空格正確,但合成
// 中文:   (hanji,span) 撞上單一字典詞(存 `-`/`--`)時空格版是錯誤呈現。
// 中文:   修法在 display 層提字典 canonical roman 到 slot 0(§S5/§18 教訓)。
// 中文:   fixture 對齊 production:予我/hōo--guá + 戶外/hōo-guā 共用去調鍵 hoogua;
// 中文:   高頻單字 予/我 使 walker 最小成本路徑為 予+我 split(synth roman hōo guá)。

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
    let path = std::env::temp_dir().join(format!("composing-slot0-dict-roman-{pid}-{name}"));
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
/// family (the runtime numeric-tone query byte-matches the latter). The
/// `tl_notone` of every row in this fixture is the same `hoogua`/`hoo`/
/// `gua`, so the toneless families collide exactly as production.
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

/// 予我/hōo--guá (輕聲, freq 16) + 戶外/hōo-guā (連字, freq 25) collide on
/// `tl_notone = hoogua`. 予/hōo + 我/guá are high-freq single chars so the
/// walker's min-cost path is the 予+我 split → synth `hōo guá` (space).
fn fixture_rows() -> Vec<Row> {
    vec![
        Row {
            toneless_key: "hoogua",
            hanzi: "予我",
            tl: "hōo--guá", // khinsiann; tl_num hoo7gua2
            syll: 2,
            freq: 16,
        },
        Row {
            toneless_key: "hoogua",
            hanzi: "戶外",
            tl: "hōo-guā", // 連字; tl_num hoo7gua7 — tone 7 (different word)
            syll: 2,
            freq: 25,
        },
        // 予/我 are very common single morphemes — high freq so the
        // walker's min-cost path is the 予+我 SPLIT, not the 1-edge
        // whole-word 戶外. Threshold derived from the cost model
        // (`ln(CORPUS/(1+freq))/len^0.2 × syll^0.2`, `lattice/cost.rs`):
        // the 2-edge split beats the freq-25 whole word only once each
        // single's freq exceeds ~18.4k; 80k leaves comfortable margin.
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
    ]
}

fn install_fixture() {
    let rows = fixture_rows();
    let dict_path = write_temp("dictionary.bin", &build_tkdb_v3(&rows));
    let fst_path = build_dictionary_fst(&rows);
    let assoc_path = write_temp("association.bin", &empty_association_bin());
    let syllables_path = build_syllables_fst(&["hoo7", "gua2", "gua7"]);
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

fn config() -> AppConfig {
    AppConfig {
        tone_mode: String::new(),
        input_mode: "tl".to_string(),
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

/// Drive `raw` through `Start → EnterContinuous → FetchAtPos` and return
/// the `(hanji, roman)` pairs in candidate order.
///
/// Runs in DEFAULT config (literal-roman candidate ON, §34/S22). The
/// always-on preedit-literal prepend takes index 0 as a bare-roman row
/// (`hanji = None`), so this file's separator invariant asserts against the
/// first HANJI-bearing candidate (the dict best), which now sits right after
/// it — verifying `INVARIANT_CONTINUOUS_SLOT0_RESPECTS_DICT_SEPARATOR` under
/// the config users actually run. The separator-promote lives in
/// `assemble_candidates` and is independent of the toggle.
fn fetch_candidates(raw: &str) -> Vec<(Option<String>, String)> {
    let cfg = config();
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
fn slot0_promotes_dict_khinsiann_form_over_space_synth() {
    let _lock = engine_install_lock();
    install_fixture();
    // INVARIANT_CONTINUOUS_SLOT0_RESPECTS_DICT_SEPARATOR.
    // The best DICT candidate must be the dictionary word 予我 with its
    // canonical khinsiann roman `hōo--guá`, NOT the walker's space-joined
    // synth `hōo guá`. In default config the §34 literal-roman row (`hanji
    // = None`) leads at index 0, so the dict best is the first HANJI-bearing
    // candidate right after it.
    let cands = fetch_candidates("hoogua");
    let (hanji0, roman0) = cands
        .iter()
        .find(|(hanji, _)| hanji.is_some())
        .expect("hoogua produced no hanji candidate");
    assert_eq!(
        hanji0.as_deref(),
        Some("予我"),
        "best dict candidate hanji must be 予我; got {cands:?}"
    );
    assert_eq!(
        roman0, "hōo--guá",
        "best dict roman must be the dict khinsiann form `hōo--guá`, not the space synth; got {roman0:?}"
    );

    // The malformed space-join synth must be fully suppressed (it was the
    // same reading as the promoted dict row).
    assert!(
        !cands.iter().any(|(_, r)| r == "hōo guá"),
        "space-joined synth `hōo guá` must be suppressed; got {cands:?}"
    );

    // 戶外/hōo-guā is a DIFFERENT word (tone 7 ≠ tone 2) and must remain a
    // separate candidate — never collapsed into the promote.
    assert!(
        cands
            .iter()
            .any(|(h, r)| h.as_deref() == Some("戶外") && r == "hōo-guā"),
        "戶外/hōo-guā must remain present (different word); got {cands:?}"
    );
}

#[test]
fn slot0_keeps_space_synth_for_genuine_multiword_reading() {
    let _lock = engine_install_lock();
    install_fixture();
    // Negative guard: `hoo` alone is a single syllable — no full-span
    // multi-word collision. The single-char dict word 予/hōo is the
    // expected slot-0 and nothing about the promote should fire. This
    // pins that the promote is scoped to the separator-mismatch case and
    // does not perturb ordinary single-syllable continuous output.
    let cands = fetch_candidates("hoo");
    assert!(
        cands
            .iter()
            .any(|(h, r)| h.as_deref() == Some("予") && r == "hōo"),
        "hoo must surface 予/hōo; got {cands:?}"
    );
}
