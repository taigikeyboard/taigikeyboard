//! Shared helpers for composing integration tests.
//!
//! Cargo compiles each `tests/*.rs` as a separate crate, so a `tests/common/`
//! module declared via `mod common;` from each test file is the standard way
//! to share helpers without leaking them into the production crate (same
//! pattern as `engine/lexicon/tests/common/mod.rs`, which is test-private to
//! `lexicon` and therefore not importable from here).
//!
//! Only helper *definitions* live here — hermetic fixture serializers
//! (TKDB v3 / `dictionary.fst` / `syllables.fst` / `association.bin`), the
//! per-binary install lock, temp-path namespacing, the `AppConfig` /
//! `ComposingRequest` constructors, and the `Start → EnterContinuous →
//! FetchAtPos` driver. Every test file keeps its own fixture rows, syllable
//! samples, and assertions.

#![allow(dead_code)] // Different test files use different subsets.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};

use composing::api::Engine;
use composing::dispatch;
use fst::SetBuilder;
use lexicon::{EngineHandle as LexiconHandle, LexiconPaths, SyllableInventory};
use phonetics::{canonicalize_poj_syllable, canonicalize_syllable};
use protos::engine::composing_request::Method;
use protos::engine::{
    AppConfig, ComposingRequest, ComposingResponse, EnterContinuous, FetchAtPos, Start,
};

pub const SEPARATOR: u8 = 0xFF;
const RANK_NEUTRAL_BITMASK: u16 = 1u16 << 11;
const TKDB_HEADER_SIZE: usize = 16;

/// Serializes `LexiconHandle::install` vs. assertion within ONE test binary.
/// Each `tests/*.rs` is its own process with its own `lexicon::EngineHandle`
/// singleton, so the lock only guards cargo's in-binary `#[test]`
/// parallelism from swapping the singleton mid-assertion.
pub fn engine_install_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}

/// One dictionary fixture row. Empty `hanzi` ⇒ TAILO (no hanji). `rowid` is
/// the 1-based slice index, shared between `dictionary.bin` (offset-table
/// order) and the `dictionary.fst` keys (`<family>:<key> + 0xFF + rowid_le`).
/// `toneless_key` feeds only the `tl:` family; TPS-only fixtures leave it
/// empty because their families derive from `tl`.
pub struct Row {
    pub toneless_key: &'static str,
    pub hanzi: &'static str,
    pub tl: &'static str,
    pub syll: u8,
    pub freq: u32,
}

/// Per-process temp namespace (pid) so a concurrent invocation of the same
/// test binary cannot truncate/rewrite another's fixture files mid
/// install/read.
pub fn write_temp(name: &str, bytes: &[u8]) -> PathBuf {
    let pid = std::process::id();
    let path = std::env::temp_dir().join(format!("composing-test-{pid}-{name}"));
    std::fs::write(&path, bytes).expect("write temp fixture");
    path
}

/// Unique `.fst` temp path for the `SyllableInventory` builders: pid plus a
/// per-process atomic counter so parallel tests in one binary never collide.
pub fn unique_temp_path() -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    std::env::temp_dir().join(format!("composing-test-{pid}-{n}.fst"))
}

/// TKDB v3 byte layout — verbatim logic from
/// `engine/lexicon/tests/common/mod.rs::build_tkdb_bin` (`build_tkdb_v3`
/// path: every row carries a `syllable_count` byte + a `kautian_subtag` u16).
pub fn build_tkdb_v3(rows: &[Row]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"TKDB");
    out.extend_from_slice(&3u32.to_le_bytes()); // version
    out.extend_from_slice(&(rows.len() as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // build_ts

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

/// Write sorted, deduped FST `entries` to a fresh temp file and return it.
pub fn write_fst_set(name: &str, mut entries: Vec<Vec<u8>>) -> PathBuf {
    entries.sort();
    entries.dedup();
    let path = write_temp(name, &[]);
    let file = std::fs::File::create(&path).unwrap_or_else(|e| panic!("create {name}: {e}"));
    let mut builder = SetBuilder::new(std::io::BufWriter::new(file)).expect("fst builder");
    for entry in &entries {
        builder.insert(entry).expect("fst insert");
    }
    builder.finish().expect("fst finish");
    path
}

/// `<family>:<body> + 0xFF + rowid_le_u32` — the production `dictionary.fst`
/// entry shape (`dictionary/build/create_fst.py`).
pub fn fst_entry(family: &[u8], body: &str, rowid: u32) -> Vec<u8> {
    let mut e = Vec::with_capacity(family.len() + body.len() + 5);
    e.extend_from_slice(family);
    e.extend_from_slice(body.as_bytes());
    e.push(SEPARATOR);
    e.extend_from_slice(&rowid.to_le_bytes());
    e
}

/// `dictionary.fst` with the toneless `tl:` family, the `poj:` family
/// (derived per row via `derive_poj_notone`) and the `tps:` family plus its
/// er↔or variant — the same chain as `create_fst.py`.
pub fn build_dictionary_fst(rows: &[Row]) -> PathBuf {
    let mut entries: Vec<Vec<u8>> = Vec::with_capacity(rows.len());
    for (idx, row) in rows.iter().enumerate() {
        let rowid = (idx + 1) as u32;
        entries.push(fst_entry(b"tl:", row.toneless_key, rowid));
        if let Some(poj_notone) = derive_poj_notone(row.tl) {
            entries.push(fst_entry(b"poj:", &poj_notone, rowid));
        }
        let tps_notone = phonetics::tps_notone_from_tl(row.tl);
        if !tps_notone.is_empty() {
            entries.push(fst_entry(b"tps:", &tps_notone, rowid));
            let tps_notone_var = phonetics::tps_notone_or_variant(&tps_notone);
            if !tps_notone_var.is_empty() {
                entries.push(fst_entry(b"tps:", &tps_notone_var, rowid));
            }
        }
    }
    write_fst_set("dictionary.fst", entries)
}

/// `dictionary.fst` with the toneless `tl:<tl_notone>` family AND the toned
/// `tl:<tl_num>` family (derived from the display `tl` via
/// `phonetics::normalize_input` — the SAME normalizer the runtime applies to
/// numeric-tone input, so a `tl:tsua2` runtime query byte-matches the fixture
/// key). Production `create_fst.py:127-130` emits both; without the toned
/// keys the explicit-tone filter would have nothing to hit. The toned key is
/// emitted only when the result carries a tone digit (display-unmarked
/// tone-1/4 syllables collapse onto the toneless key).
pub fn build_dictionary_fst_tl_toned(rows: &[Row]) -> PathBuf {
    let mut entries: Vec<Vec<u8>> = Vec::with_capacity(rows.len() * 2);
    for (idx, row) in rows.iter().enumerate() {
        let rowid = (idx + 1) as u32;
        entries.push(fst_entry(b"tl:", row.toneless_key, rowid));
        let tl_num = phonetics::normalize_input(row.tl);
        if tl_num.bytes().any(|b| b.is_ascii_digit()) {
            entries.push(fst_entry(b"tl:", &tl_num, rowid));
        }
    }
    write_fst_set("dictionary.fst", entries)
}

/// Both TPS key families per row (`tps:<tps_notone>` + `tps:<tps_num>`),
/// mirroring `create_fst.py:126-141`. For a tone-1 or tone-4 row the two are
/// byte-identical (no mark to add) and dedupe to one key.
pub fn build_dictionary_fst_tps(rows: &[Row]) -> PathBuf {
    let mut entries: Vec<Vec<u8>> = Vec::with_capacity(rows.len() * 2);
    for (idx, row) in rows.iter().enumerate() {
        let rowid = (idx + 1) as u32;
        entries.push(fst_entry(
            b"tps:",
            &phonetics::tps_notone_from_tl(row.tl),
            rowid,
        ));
        entries.push(fst_entry(
            b"tps:",
            &phonetics::tps_num_from_tl(row.tl),
            rowid,
        ));
    }
    write_fst_set("dictionary-tps.fst", entries)
}

/// Derive `poj_notone` from `record.tl` at fixture-build time, mirroring the
/// production `dictionary/build/create_fst.py:124-127` pipeline (TL display
/// → POJ display → per-syllable `canonicalize_poj_syllable` → concat).
/// Returns `None` when any non-empty syllable fails phonotactic gating (the
/// production pipeline would have omitted that row's POJ keys).
pub fn derive_poj_notone(tl_display: &str) -> Option<String> {
    let poj_display = phonetics::api::tl_display_to_poj_display(tl_display);
    let mut out = String::new();
    for token in poj_display.split(['-', ' ']) {
        if token.is_empty() {
            continue;
        }
        let (toneless, _) = canonicalize_poj_syllable(token)?;
        out.push_str(&toneless);
    }
    (!out.is_empty()).then_some(out)
}

/// Tagged-single-FST syllable keys for one canonical syllable: the toned key
/// (when `tone` is non-empty) plus the toneless key — the per-row dual emit
/// of the production `create_syllables_fst.py`.
fn push_syllable_keys(keys: &mut Vec<String>, family: &str, canonical: &str, tone: &str) {
    if tone.is_empty() {
        keys.push(format!("{family}:{canonical}"));
    } else {
        keys.push(format!("{family}:{canonical}{tone}"));
        keys.push(format!("{family}:{canonical}"));
    }
}

/// `tl:` family keys for TL samples (POJ→TL fold + nasal/o-dot ASCII fold via
/// `canonicalize_syllable`). `.expect()` (not silent skip) so a wrong
/// grounding assumption fails loudly.
pub fn tl_syllable_keys(samples: &[&str]) -> Vec<String> {
    let mut keys = Vec::new();
    for s in samples {
        let (canonical, tone) = canonicalize_syllable(s)
            .unwrap_or_else(|| panic!("tl sample {s:?} failed canonicalize_syllable"));
        push_syllable_keys(&mut keys, "tl", &canonical, &tone);
    }
    keys
}

/// `poj:` family keys for POJ samples (encoding-only fold via
/// `canonicalize_poj_syllable`; POJ ASCII spelling such as `chiah` preserved).
pub fn poj_syllable_keys(samples: &[&str]) -> Vec<String> {
    let mut keys = Vec::new();
    for s in samples {
        let (canonical, tone) = canonicalize_poj_syllable(s)
            .unwrap_or_else(|| panic!("poj sample {s:?} failed canonicalize_poj_syllable"));
        push_syllable_keys(&mut keys, "poj", &canonical, &tone);
    }
    keys
}

/// Hermetic `SyllableInventory` over an explicit key list (sorted + deduped
/// here), written to a `unique_temp_path()` FST.
pub fn inventory_from_keys(keys: Vec<String>) -> SyllableInventory {
    let mut keys = keys;
    keys.sort();
    keys.dedup();
    let path = unique_temp_path();
    let file = std::fs::File::create(&path).expect("create fst");
    let mut builder = SetBuilder::new(std::io::BufWriter::new(file)).expect("builder");
    for key in &keys {
        builder.insert(key.as_bytes()).expect("insert");
    }
    builder.finish().expect("finish");
    SyllableInventory::open(&path).expect("open inventory")
}

/// Inventory carrying BOTH the `tl:` and `poj:` families — mirrors the
/// production `fst-builder build-syllables --tl-input --poj-input` pipeline.
pub fn build_dual_inventory(tl_samples: &[&str], poj_samples: &[&str]) -> SyllableInventory {
    let mut keys = tl_syllable_keys(tl_samples);
    keys.extend(poj_syllable_keys(poj_samples));
    inventory_from_keys(keys)
}

/// TL-only inventory so `contains_in(InputMode::Tl, …)` probes hit.
pub fn build_inventory(samples: &[&str]) -> SyllableInventory {
    build_dual_inventory(samples, &[])
}

/// POJ-only inventory (`poj:chiah`, not `poj:tsiah`).
pub fn build_poj_inventory(samples: &[&str]) -> SyllableInventory {
    build_dual_inventory(&[], samples)
}

/// `syllables.fst` with the `tl:` family only.
pub fn build_syllables_fst_tl(samples: &[&str]) -> PathBuf {
    write_fst_set(
        "syllables.fst",
        tl_syllable_keys(samples)
            .into_iter()
            .map(String::into_bytes)
            .collect(),
    )
}

/// `syllables.fst` with `tl:` / `poj:` / `tps:` family keys (numeric AND
/// toneless canonical forms) for every sample, so production lookups via
/// `contains_in(mode, …)` resolve under any mode.
pub fn build_syllables_fst(samples: &[&str]) -> PathBuf {
    let mut keys: Vec<String> = Vec::new();
    for s in samples {
        keys.extend(tl_syllable_keys(&[s]));
        // POJ-form sample derived from the TL-form input (`tsua7` → POJ
        // `chua7`), then normalized through `canonicalize_poj_syllable`;
        // samples that fail the POJ phonotactic gate contribute no `poj:` key.
        let poj_display = phonetics::api::tl_display_to_poj_display(s);
        if let Some((poj_canonical, poj_tone)) = canonicalize_poj_syllable(&poj_display) {
            push_syllable_keys(&mut keys, "poj", &poj_canonical, &poj_tone);
        }
        // TPS via `tl_numeric_token_to_tps` with `or_maps_to_er=true`
        // (Node bridge default). `to_zhuyin` emits a trailing space marker
        // for tone-1 inputs and joins multi-token output with `-`; the
        // sample is a single syllable so strip both.
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
    write_fst_set(
        "syllables.fst",
        keys.into_iter().map(String::into_bytes).collect(),
    )
}

/// Per-syllable TPS inventory, toned + toneless, for every syllable of
/// every fixture row (a phrase row contributes each of its syllables).
pub fn build_syllables_fst_tps(rows: &[Row]) -> PathBuf {
    let mut keys: Vec<String> = Vec::new();
    for row in rows {
        for token in row.tl.split(['-', ' ']) {
            if token.is_empty() {
                continue;
            }
            keys.push(format!("tps:{}", phonetics::tps_notone_from_tl(token)));
            keys.push(format!("tps:{}", phonetics::tps_num_from_tl(token)));
        }
    }
    write_fst_set(
        "syllables-tps.fst",
        keys.into_iter().map(String::into_bytes).collect(),
    )
}

/// Empty `association.bin` — `TKWA` + version 1 + 0 keys/entries/ts.
pub fn empty_association_bin() -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"TKWA");
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // key_count
    out.extend_from_slice(&0u32.to_le_bytes()); // entry_count
    out.extend_from_slice(&0u32.to_le_bytes()); // build_ts
    out
}

/// Validate the four fixture paths and swap them into the process-global
/// `lexicon::EngineHandle` singleton.
pub fn install_lexicon(
    fst_path: &Path,
    dict_path: &Path,
    assoc_path: &Path,
    syllables_path: &Path,
) {
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

/// `AppConfig` with every toggle off (proto defaults); `candidate_display_mode`
/// is the proto enum value (`0` = side-by-side default).
pub fn config_with_display_mode(input_mode: &str, candidate_display_mode: i32) -> AppConfig {
    AppConfig {
        input_mode: input_mode.to_string(),
        candidate_display_mode,
        ..Default::default()
    }
}

pub fn config(input_mode: &str) -> AppConfig {
    config_with_display_mode(input_mode, 0)
}

pub fn config_tl() -> AppConfig {
    config("tl")
}

pub fn req(method: Method) -> ComposingRequest {
    ComposingRequest {
        method: Some(method),
    }
}

/// Drive `raw` through `Start → EnterContinuous → FetchAtPos` on a fresh
/// `Engine` and return the `FetchAtPos` response (its `continuous` carrier is
/// `None` when the engine never entered the continuous phase — callers that
/// care distinguish that from an empty candidate list).
pub fn fetch_at_pos_response(
    config: &AppConfig,
    raw: &str,
    fetch: FetchAtPos,
) -> ComposingResponse {
    let mut engine = Engine::new();
    dispatch::handle(
        &req(Method::Start(Start { text: raw.into() })),
        &mut engine,
        config,
    )
    .expect("Start");
    dispatch::handle(
        &req(Method::EnterContinuous(EnterContinuous {})),
        &mut engine,
        config,
    )
    .expect("EnterContinuous");
    dispatch::handle(&req(Method::FetchAtPos(fetch)), &mut engine, config).expect("FetchAtPos")
}
