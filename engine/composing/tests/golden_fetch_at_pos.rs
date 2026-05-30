//! S0 — golden `FetchAtPos` snapshot harness (v3.5.9 refactor gate).
//!
//! Pins the full proto candidate vector that
//! `composing::dispatch::handle(FetchAtPos)` returns for a representative
//! input matrix, so every later behavior-neutral refactor slice's
//! acceptance reduces to "empty golden diff". The golden is frozen against
//! post-v3.5.8 `main` (v3.5.8 already shipped at tag `61df3028`; behavior
//! is frozen — the design-spec §0 precondition is satisfied).
//!
//! Design source of truth:
//! `docs/reports/2026-05-18-v358-refactor-design-spec.md` §1 + ⭐ amendment
//! block (B2 lifecycle invariant, B3 expanded matrix). Codex pre-impl
//! ANALYSIS-ONLY review applied (both BLOCKs resolved by tightening
//! grounding — no invented dictionary content; the ≥6-syllable case uses
//! known-valid syllables with `.expect()` canonicalization, not silent
//! skip).
//!
//! ## Why this lives in `composing` and re-implements the fixture builders
//!
//! `dispatch::handle(FetchAtPos)` resolves candidates through the
//! process-global `lexicon::EngineHandle` singleton, NOT injected fixtures.
//! The only pattern that drives real candidates through it is
//! `EngineHandle::install(LexiconPaths)` (mirrors
//! `engine/lexicon/tests/parity.rs`). This file must call BOTH
//! `composing::dispatch::handle` AND `lexicon::EngineHandle::install`, so
//! it lives in `composing` (which depends on `lexicon`). The hermetic
//! fixture builders are re-implemented here because `lexicon/tests/common`
//! is a lexicon-test-private module not visible to composing tests —
//! accepted duplication, the same pattern `user_freq_plumb.rs` already
//! uses. `parity.rs` passes `""` for `syllables_fst` → inventory `None` →
//! the whole-sentence walker path is silently skipped; S0 therefore builds
//! a REAL `syllables.fst` so the walker slot-0 path is exercised.
//!
//! ## Grounding (no invented dictionary content)
//!
//! Every dictionary triple `(toneless_key, hanzi, tl)` is sourced verbatim
//! from either an existing hermetic test or production `dictionary.csv`:
//!
//! - `span_local_fetch.rs` — tsua/taigikhipuann/taiuantaigi/taibak/
//!   mode-carrier/partial-prefix rows.
//! - `user_freq_plumb.rs` — the `台` user-freq row.
//! - production `dictionary.csv:8` — `有,ū,53685,ū,u7,u7,ㄨ˫,...` (added
//!   2026-05-27 with v3.5.9 D Fork 7b for the TPS standalone-syllable
//!   regression guard; mirrors a real row, not invented content).
//!
//! The OOV-tail syllables (`lang`/`kang`/`tan`/`lai`) are proven
//! `VALID_SAMPLES` in `engine/lexicon/tests/syllables_fst.rs`. The POJ
//! cases reuse already-grounded rows (`choa`→`tsua`, `goa`→`gua`,
//! `taigikhipuann`→`台語齒盤`) rather than introducing new dictionary rows.
//!
//! ## Run / record
//!
//! `cargo test -p composing --test golden_fetch_at_pos` asserts.
//! `UPDATE_GOLDEN=1 cargo test -p composing --test golden_fetch_at_pos`
//! re-records `tests/golden/fetch_at_pos.golden`. A non-empty diff on a
//! behavior-neutral slice means the slice is NOT behavior-neutral — stop,
//! do not `UPDATE_GOLDEN` to paper over it.

// 中文: S0 — golden FetchAtPos 快照測試,凍結 dispatch FetchAtPos 的完整 proto 候選向量,
// 中文:   作為 v3.5.9 重構每個 behavior-neutral slice 的驗收閘(空 golden diff = 通過)。
// 中文: 必裝真實 syllables.fst(parity.rs 傳 "" 會靜默漏 walker 路徑)。所有字典三元組
// 中文:   逐字取自既有 hermetic 測試,無捏造字典內容。UPDATE_GOLDEN=1 重錄。

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};

use composing::api::Engine;
use composing::dispatch;
use fst::SetBuilder;
use lexicon::{EngineHandle as LexiconHandle, LexiconPaths};
use phonetics::canonicalize_syllable;
use protos::engine::composing_request::Method;
use protos::engine::{
    AppConfig, ComposingRequest, CustomDictEntry, EnterContinuous, FetchAtPos, FrequencyEntry,
    Start,
};

const SEPARATOR: u8 = 0xFF;
const RANK_NEUTRAL_BITMASK: u16 = 1u16 << 11;
const TKDB_HEADER_SIZE: usize = 16;

/// Serializes install vs. assertion within THIS test binary. Each
/// `tests/*.rs` is its own process with its own `lexicon::EngineHandle`
/// singleton, so the cross-crate concern `parity.rs` documents does not
/// apply here (Codex post-impl NIT); the lock is still the established
/// pattern and keeps a future second `#[test]` in this binary from
/// swapping the singleton mid-assertion under cargo's in-binary
/// parallelism.
fn engine_install_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}

// --- hermetic fixture builders (re-impl of lexicon/tests/common + parity) --

/// One dictionary fixture row. Empty `hanzi` ⇒ TAILO (no hanji), mirroring
/// `span_local_fetch.rs`'s `mode_carrier` row. `rowid` is the 1-based
/// slice index, shared between `dictionary.bin` (offset-table order) and
/// `dictionary.fst` (`tl:<key> + 0xFF + rowid_le`).
struct Row {
    toneless_key: &'static str,
    hanzi: &'static str,
    tl: &'static str,
    syll: u8,
    freq: u32,
}

/// Per-process temp namespace (pid) so a concurrent invocation of this
/// same test binary cannot truncate/rewrite another's fixture files mid
/// install/read (Codex post-impl SHOULD; mirrors the `pid`-namespaced
/// `unique_temp_path` pattern in `span_local_fetch.rs`).
fn write_temp(name: &str, bytes: &[u8]) -> PathBuf {
    let pid = std::process::id();
    let path = std::env::temp_dir().join(format!("composing-golden-s0-{pid}-{name}"));
    std::fs::write(&path, bytes).expect("write temp fixture");
    path
}

/// TKDB v3 byte layout — verbatim logic from
/// `engine/lexicon/tests/common/mod.rs::build_tkdb_bin` (`build_tkdb_v3`
/// path: every row carries a `syllable_count` byte + a `kautian_subtag` u16).
fn build_tkdb_v3(rows: &[Row]) -> Vec<u8> {
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
        payload.push(row.syll); // v2 layout
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

/// `dictionary.fst` — entry per row keyed `b"tl:" + toneless_key + 0xFF +
/// rowid_le_u32`, inserted in ascending byte order (verbatim pattern from
/// `span_local_fetch.rs` / `user_freq_plumb.rs`).
fn build_dictionary_fst(rows: &[Row]) -> PathBuf {
    let mut entries: Vec<Vec<u8>> = Vec::with_capacity(rows.len());
    for (idx, row) in rows.iter().enumerate() {
        let rowid = (idx + 1) as u32;
        // `tl:` family — pre-B-2 path, unchanged.
        let mut e = Vec::with_capacity(row.toneless_key.len() + 4 + 5);
        e.extend_from_slice(b"tl:");
        e.extend_from_slice(row.toneless_key.as_bytes());
        e.push(SEPARATOR);
        e.extend_from_slice(&rowid.to_le_bytes());
        entries.push(e);
        // v3.5.9 B-2 — `poj:` family. Derive `poj_notone` at fixture
        // build time the way `dictionary/build/create_fst.py:124-127`
        // does in production (TL display → POJ display →
        // per-syllable `canonicalize_poj_syllable` → concat). Rows whose
        // TL display does not phonotactically gate have no POJ family
        // entry in the real `dictionary.fst` either, so the fixture
        // simply skips them.
        if let Some(poj_notone) = derive_poj_notone(row.tl) {
            let mut e2 = Vec::with_capacity(poj_notone.len() + 5 + 5);
            e2.extend_from_slice(b"poj:");
            e2.extend_from_slice(poj_notone.as_bytes());
            e2.push(SEPARATOR);
            e2.extend_from_slice(&rowid.to_le_bytes());
            entries.push(e2);
        }
        // v3.5.9 D / C-5 — `tps:` family. Mirrors `create_fst.py:124-138`:
        // emit `tps:<tps_notone>` per row (primary), plus
        // `tps:<tps_notone_var>` for the C-3a er↔or dual-emit. Derivation
        // uses `phonetics::tps_notone_from_tl` (same chain the runtime
        // continuous-input guard uses) so fixture ↔ production parity
        // holds for the cases the golden matrix exercises.
        // 中文: D / C-5 — tps: 家族;對齊 create_fst.py 的雙發 (主形 + 變體形)。
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

/// v3.5.9 B-2 — derive `poj_notone` from `record.tl` at fixture-build
/// time, mirroring the production `dictionary/build/create_fst.py:124-127`
/// pipeline (TL display → POJ display → per-syllable
/// `canonicalize_poj_syllable` → concat). Returns `None` when any
/// non-empty syllable fails phonotactic gating (the production pipeline
/// would have flagged that row as stale and omitted its POJ keys).
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

/// `syllables.fst` — both numeric and toneless canonical keys, exactly
/// the builder shape from `engine/composing/tests/build_keys_tl_hyphen.rs`.
/// `.expect()` (not silent skip) so a wrong grounding assumption fails
/// loudly here rather than degrading a matrix case (Codex pre-impl BLOCK).
fn build_syllables_fst(samples: &[&str]) -> PathBuf {
    // v3.5.9 B-1 / B-2: tagged-single-FST — emit both `tl:` and `poj:`
    // family keys so production lookups via `contains_in(mode, …)`
    // resolve under either mode. B-2 added the `poj:` family (built via
    // `canonicalize_poj_syllable` to preserve POJ ASCII shape).
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
        // v3.5.9 B-2 — derive POJ-form sample from the TL-form input
        // (`tsua7` → POJ `chua7` then through canonicalize_poj_syllable
        // for normalization). The production POJ inventory is sourced
        // from the `poj_num` column of `dictionary.csv` — the fixture's
        // TL samples were rooted in `tl_num`, so we convert each
        // sample's display form to POJ first via
        // `phonetics::api::tl_display_to_poj_display`.
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
        // v3.5.9 D / C-5 — `tps:` family. Mirrors
        // `create_syllables_fst.py`: derive per-syllable TPS from each
        // TL sample via `phonetics::tl_numeric_token_to_tps` (the
        // pub-widened `to_zhuyin` thunk) with `or_maps_to_er=true`
        // matching the Node bridge default. Emit BOTH numeric (with
        // Bopomofo tone marks) AND toneless forms — same shape the
        // production pipeline ships.
        // 中文: D / C-5 — TPS 家族;
        // 中文:   逐 sample 經 to_zhuyin 取得 with-tone Bopomofo,
        // 中文:   再以 is_tps_tone_mark 剝除取得 toneless,雙發 tps: 入 syllables.fst。
        let numeric = phonetics::to_tone_number(s);
        let tps_with_tone = phonetics::tl_numeric_token_to_tps(&numeric, false, true);
        // `to_zhuyin` emits a trailing space marker for tone-1 inputs and
        // joins multi-token output with `-`; we treat the sample as a
        // single syllable so strip both. The C-3a variant glyph (ㄛ) is
        // not emitted here — syllable inventory is mode-blind; row-level
        // variant lives in `dictionary.fst` only.
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

/// Empty `association.bin` — `TKWA` + version 1 + 0 keys/entries/ts
/// (verbatim from `parity.rs::synth_association_bin`).
fn empty_association_bin() -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"TKWA");
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // key_count
    out.extend_from_slice(&0u32.to_le_bytes()); // entry_count
    out.extend_from_slice(&0u32.to_le_bytes()); // build_ts
    out
}

// --- the single union install fixture ------------------------------------

/// Dictionary rows (grounding cited per row in the module docs). Order is
/// load-bearing: rowid = 1-based index, shared by `dictionary.bin` and the
/// `dictionary.fst` keys.
fn fixture_rows() -> Vec<Row> {
    vec![
        // span_local_fetch::tsua_surfaces_zhi_zhuah_zhu_across_two_spans
        Row {
            toneless_key: "tsua",
            hanzi: "紙",
            tl: "tsuá",
            syll: 1,
            freq: 100,
        },
        Row {
            toneless_key: "tsua",
            hanzi: "珠仔",
            tl: "tsu-á",
            syll: 2,
            freq: 80,
        },
        Row {
            toneless_key: "tsu",
            hanzi: "珠",
            tl: "tsu",
            syll: 1,
            freq: 90,
        },
        // span_local_fetch::taiuantaigi_* / user_freq_plumb cold-start
        Row {
            toneless_key: "tai",
            hanzi: "台",
            tl: "tâi",
            syll: 1,
            freq: 31281,
        },
        Row {
            toneless_key: "taiuan",
            hanzi: "台灣",
            tl: "tâi-uân",
            syll: 2,
            freq: 1379,
        },
        Row {
            toneless_key: "taiuantaigi",
            hanzi: "臺灣台語",
            tl: "tâi-uân-tâi-gí",
            syll: 4,
            freq: 12,
        },
        // span_local_fetch::taigikhipuann_surfaces_long_reach_4_syllable_word
        Row {
            toneless_key: "taigi",
            hanzi: "台語",
            tl: "tâi-gí",
            syll: 2,
            freq: 150,
        },
        // Quanzhou-dialect variant of `tâi-gí` (`tl_notone = taigir`),
        // grounded in production `dictionary/output/dictionary.csv:33059`
        // + `:75190`. Same hanji as `taigi`/`tâi-gí`, distinct roman.
        // Drives the Step 4b prefix-extension scan: input `taigi` (FST
        // key `tl:taigi`, 5 bytes) cannot reach this row via the
        // span-local lookup_exact + walker path; only Step 4b's
        // `lookup_prefix("tl:taigi")` surfaces it because the FST key
        // here (`tl:taigir`, 6 bytes) extends beyond any lattice edge
        // the buffer can produce.
        // 中文: tâi-gír — 台語的泉州腔變體;產線 dictionary.csv:33059/75190 對應。
        // 中文:   FST key `tl:taigir` 比 `tl:taigi` 多 1 byte,Step 4b
        // 中文:   `lookup_prefix("tl:taigi")` 才能撈到。
        Row {
            toneless_key: "taigir",
            hanzi: "台語",
            tl: "tâi-gír",
            syll: 2,
            freq: 25,
        },
        Row {
            toneless_key: "taigikhipuann",
            hanzi: "台語齒盤",
            tl: "tâi-gí-khí-puânn",
            syll: 4,
            freq: 5,
        },
        // span_local_fetch::numeric_tone_multi_syllable_strips_each_segment_to_fused_key
        Row {
            toneless_key: "taibak",
            hanzi: "代墨",
            tl: "tâi-ba̍k",
            syll: 2,
            freq: 50,
        },
        // span_local_fetch::mode_carrier_propagates_through_fetch (TAILO + MIXED)
        Row {
            toneless_key: "li",
            hanzi: "",
            tl: "lí",
            syll: 1,
            freq: 50,
        },
        Row {
            toneless_key: "iausi",
            hanzi: "iáu是",
            tl: "iáu-sī",
            syll: 2,
            freq: 30,
        },
        // span_local_fetch::partial_prefix_engine_path_surfaces_lookup_prefix_hits
        Row {
            toneless_key: "gua",
            hanzi: "我",
            tl: "guá",
            syll: 1,
            freq: 200,
        },
        // v3.5.9 D Fork 7b — TPS standalone-syllable regression guard.
        // 有/ū → `tps_notone=ㄨ` (the vowel ㄨ IS a complete TPS syllable
        // on its own). Pairs with the `u7` sample below so the inventory
        // accepts `ㄨ` as a span ending → span-local fetch hits this row.
        // Mirrors production `dictionary.csv:8` (有,ū,53685,ū,u7,u7,ㄨ˫,...).
        // 中文: D Fork 7b — 有/ū 是 ㄨ 自身為完整音節的 fixture 證據;
        // 中文:   配合下方 u7 sample 讓 inv 認可 ㄨ,確保 ㄨ 走 span-local。
        Row {
            toneless_key: "u",
            hanzi: "有",
            tl: "ū",
            syll: 1,
            freq: 53685,
        },
    ]
}

/// Inventory samples. Component syllables of every multi-syllable matrix
/// raw plus the proven-`VALID_SAMPLES` OOV tail (`lang`/`kang`/`tan`/
/// `lai`, from `engine/lexicon/tests/syllables_fst.rs`).
const SYLLABLE_SAMPLES: &[&str] = &[
    "tsua7", "tsu", "tai1", "tai5", "bak4", "uan5", "gi2", "khi2", "puann5", "li2", "iau2", "si7",
    "gua2", "lang5", "kang1", "tan5", "lai5",
    // v3.5.9 D Fork 7b — `u7` for 有/ū; auto-derives `tps:ㄨ` so the
    // syllable inventory accepts `ㄨ` as a complete span ending.
    "u7",
];

fn install_union_fixture() {
    let rows = fixture_rows();
    let dict_path = write_temp("dictionary.bin", &build_tkdb_v3(&rows));
    let fst_path = build_dictionary_fst(&rows);
    let assoc_path = write_temp("association.bin", &empty_association_bin());
    let syllables_path = build_syllables_fst(SYLLABLE_SAMPLES);
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

// --- the matrix ----------------------------------------------------------

struct Case {
    name: &'static str,
    raw: &'static str,
    input_mode: &'static str,
    freq: Vec<FrequencyEntry>,
    now_ms: i64,
    custom: Vec<CustomDictEntry>,
    // PR-9.6 — source-toggle bitmask threaded into `FetchAtPos`. `0` is
    // the proto3-absent sentinel → dispatch normalises it to `u32::MAX`
    // (all sources on), so cases left at the default reproduce the
    // pre-PR-9.6 all-on behaviour. A restrictive case sets a real mask.
    enabled_sources_bitmask: u32,
}

fn case(name: &'static str, raw: &'static str, input_mode: &'static str) -> Case {
    Case {
        name,
        raw,
        input_mode,
        freq: Vec::new(),
        now_ms: 0,
        custom: Vec::new(),
        enabled_sources_bitmask: 0,
    }
}

fn matrix() -> Vec<Case> {
    vec![
        case("tl_toneless_multi", "tsua", "tl"),
        case("tl_toneless_long_reach", "taigikhipuann", "tl"),
        case("tl_numeric_single", "tsua7", "tl"),
        case("tl_numeric_multi", "tai1bak4", "tl"),
        case("poj_diacritic", "tâi-uân", "tl"),
        // Negative guard: post-C-3b TPS is first-class. This raw's
        // toneless body `ㄉㄧㄠㄨㄢ` matches NO `tps:` prefix in the
        // fixture's `dictionary.fst` (closest is `tps:ㄉㄞㄨㄢ` from 台灣;
        // bytes diverge at position 4 — ㄧ vs ㄞ), so the partial-prefix
        // byte-range scan returns empty. With Fork 7b activated
        // (2026-05-27 dogfood enable), this case now exercises the
        // `prefix_index.n` no-hit branch on the TPS family — empty result
        // is intended.
        // 中文: 負面守門 — TPS first-class + Fork 7b 開放後,此 raw 的 toneless
        // 中文:   `ㄉㄧㄠㄨㄢ` 在 fixture dict.fst 的 tps: 家族中無前綴匹配,
        // 中文:   prefix_index.n("tps:ㄉㄧㄠㄨㄢ") 回空。
        case("tps_no_inventory_match", "ㄉㄧㄠˊㄨㄢˊ", "tl"),
        // v3.5.9 D Fork 7b activation (2026-05-27 dogfood): leading lone
        // Bopomofo initial `ㄉ` is not a complete syllable in the TPS
        // inventory, so the syllabifier emits no valid ending → empty
        // keys → partial-prefix fallthrough. `build_partial_prefix_key`
        // emits `tps:ㄉ` and `prefix_index.n("tps:ㄉ")` byte-range scans
        // every dictionary row whose `tps_notone` starts with `ㄉ`. The
        // fixture's TL rows derive `tps_notone_from_tl("tâi")` →
        // `ㄉㄞ` + `tâi-gí` → `ㄉㄞㆣㄧ` + `tâi-uân` → `ㄉㄞㄨㄢ` + ...,
        // so `ㄉ` surfaces 台 / 台語 / 台灣 / 代墨 / 台語齒盤 / 臺灣台語
        // through the partial-prefix path with `coverage_kind =
        // COVERAGE_KIND_PARTIAL_PREFIX` and `consumed_span = (0, 3)`
        // (`ㄉ` is 3 bytes UTF-8). Mirrors librime / khiin-rs /
        // McBopomofo leading-prefix behavior.
        // 中文: D Fork 7b 啟用 — 單一注音聲母 `ㄉ` 走 partial-prefix,
        // 中文:   `tps:ㄉ` byte-range 掃出 fixture 中所有 ㄉ 開頭的字
        // 中文:   (台/台語/台灣/代墨/...),對齊主流注音 IME 行為。
        case("tps_partial_prefix_leading_initial", "\u{3109}", "tl"),
        // Regression guard: `ㄨ` (3 bytes) IS a complete TPS syllable
        // in the fixture inventory (it appears as the second syllable
        // of `tps:ㄉㄞㄨㄢ` for 台灣, and every TL `u`-sample like `tsua7`
        // derives `tps:ㄨ` via `to_zhuyin` since `u` → `ㄨ` is in the
        // ZHUYIN_VOWELS table). So this case must go through SPAN-LOCAL
        // fetch (`keys = ["tps:ㄨ"]`, non-empty) NOT partial-prefix. The
        // golden distinguishes the two paths via `coverage_kind`
        // (FULL=0 for span-local vs PARTIAL_PREFIX=1 above).
        // 中文: 回歸守門 — `ㄨ` 自身在 fixture inventory 為完整音節
        // 中文:   (tps:ㄨ 由 to_zhuyin 從 u 衍生),走 span-local 不走 partial,
        // 中文:   coverage_kind 為 FULL(0)。
        case("tps_standalone_syllable", "\u{3128}", "tl"),
        Case {
            name: "custom_dict",
            raw: "taigi",
            input_mode: "tl",
            freq: Vec::new(),
            now_ms: 0,
            custom: vec![CustomDictEntry {
                roman: "tâi-gí".into(),
                hanji: Some("台語".into()),
            }],
            enabled_sources_bitmask: 0,
        },
        case("mixed", "iausi", "tl"),
        case("tailo_no_hanji", "li", "tl"),
        Case {
            name: "user_freq_boosted",
            raw: "tai",
            input_mode: "tl",
            freq: vec![FrequencyEntry {
                display_text_key: "台".into(),
                count: 10,
                last_used_ms: 1,
            }],
            now_ms: 1_000_000_000_000,
            custom: Vec::new(),
            enabled_sources_bitmask: 0,
        },
        case("all_oov_partial_prefix", "g", "tl"),
        // Step 4b prefix-extension (2026-05-29): input `taigi` MUST
        // surface BOTH the walker slot-0 `tâi-gí`/`台語` and the
        // prefix-extension `tâi-gír`/`台語` (FST key `tl:taigir`,
        // 6 bytes — reachable only via lookup_prefix on the whole
        // input). Mainstream IME parity (librime predictive=true).
        // Pre-fix this raw surfaced 台 + walker 台語 only; tâi-gír
        // was unreachable from a 5-byte buffer. Pairs with the new
        // `taigir`/`台語` fixture row.
        // 中文: Step 4b 前綴延伸 — `taigi` 必須同時出 `tâi-gí`(walker)
        // 中文:   與 `tâi-gír`(prefix-extension);後者 FST key
        // 中文:   `tl:taigir` 比 buffer 多 1 byte,僅 lookup_prefix 能撈到。
        case("tl_prefix_extension_taigi", "taigi", "tl"),
        case("trailing_hyphen", "tai-", "tl"),
        // Spec §1.5 names `HitTui`; no grounded hit/tui-class fixture
        // exists (would be invented dictionary content). `TaiUan` over
        // the grounded `tai`/`taiuan` rows (span_local_fetch::
        // taiuantaigi_*) exercises BOTH single-segment recase (台,
        // span 0–3) AND multi-segment recase (台灣, span 0–6 →
        // `Tâi-Uân`) plus the walker slot-0 prepend — closer to
        // `HitTui`'s relocation off-by-one intent than single-span
        // `Tsua` was. Codex pre-impl Q2 + USER 裁示 2026-05-21.
        case("case_sensitive", "TaiUan", "tl"),
        case("headline_ranking", "taiuantaigi", "tl"),
        // PR-9.6 — source-toggle filtering reaches the continuous path.
        // Every fixture row is tagged source bit 11 (`lkk`) via
        // `RANK_NEUTRAL_BITMASK`; a dev-only mask (bit 10 set, `lkk` OFF)
        // must drop the `lkk`-tagged FST hits, proving the bitmask threads
        // from `FetchAtPos.enabled_sources_bitmask` through dispatch →
        // `ContinuousFetchCtx` → `Filter::from_enabled_bitmask` →
        // `passes_filter`. Contrast with `headline_ranking` (same raw,
        // default bitmask `0` → `u32::MAX` all-on) which keeps them — the
        // golden diff between the two cases IS the filtering proof.
        Case {
            name: "source_filter_lkk_off",
            raw: "taiuantaigi",
            input_mode: "tl",
            freq: Vec::new(),
            now_ms: 0,
            custom: Vec::new(),
            enabled_sources_bitmask: (1 << 10), // dev only; lkk (bit 11) off
        },
        // v3.5.9 B-2 — POJ first-class: `choa` is the POJ ASCII form
        // (POJ `chóa` → toneless `choa`). The `poj:choa` key in
        // `dictionary.fst` resolves to 紙 only — pre-B-2 this folded
        // through `tl:tsua` and surfaced 紙 + 珠仔 + 珠 together; B-2
        // narrows the result to the POJ family slice as designed.
        case("poj_ascii_choa", "choa", "poj"),
        // v3.5.9 B-2 — POJ first-class: `goa` (POJ ASCII) resolves
        // through `poj:goa` to 我.
        case("poj_ascii_goa", "goa", "poj"),
        // v3.5.9 B-2 — POJ render of `puann → pôaⁿ`: input is the
        // POJ-form buffer `taigikhipoann` (POJ user types POJ form).
        // Pre-B-2 this case used the TL-form `taigikhipuann` and worked
        // by virtue of the POJ→TL fold. B-2 makes POJ first-class so
        // the input must be POJ ASCII; the dictionary still resolves
        // `poj:taigikhipoann` to 台語齒盤 and the rendered roman is
        // `tâi-gí-khí-pôaⁿ` (`nn`→`ⁿ` via `recase_tl_as_poj_display`).
        case("poj_render_nn", "taigikhipoann", "poj"),
        // v3.5.9 B-2 — POJ post-render dedupe: custom entry's stored
        // roman is now in POJ display form `tâi-gí-khí-pôaⁿ` (not the
        // TL form `tâi-gí-khí-puânn` used pre-B-2) so `custom_toneless_key`
        // under POJ mode produces `poj:taigikhipoann`, byte-identical
        // to the walker edge key — dedupe fires. Pre-B-2 the TL form
        // matched because POJ mode folded everything to TL; B-2 makes
        // POJ first-class so custom roman must store the form matching
        // the user's input mode.
        Case {
            name: "poj_post_render_dedupe",
            raw: "taigikhipoann",
            input_mode: "poj",
            freq: Vec::new(),
            now_ms: 0,
            custom: vec![CustomDictEntry {
                roman: "tâi-gí-khí-pôaⁿ".into(),
                hanji: Some("台語齒盤".into()),
            }],
            enabled_sources_bitmask: 0,
        },
        // ≥6-syllable long-OOV-vs-dict: `tai`/`taigi` dict-covered,
        // `lang`/`kang`/`tan`/`lai` proven-valid + dict-absent → OOV path
        // (RC0 class, project_continuous_abbrev_collision). Codex pre-impl
        // BLOCK: known-valid syllables, not silent-skip degradation.
        case("long_oov_vs_dict", "taigilangkangtanlai", "tl"),
        // v3.5.9 D / C-5 — TPS first-class (post-C-3b). `contains_tps`
        // in `dispatch::handle` upgrades mode to `InputMode::Tps`
        // regardless of the platform-sent `input_mode` string, so the
        // `tl` mode flag here exercises the production auto-detect
        // path. Inventory + dict.fst extended in `build_*_fst` above
        // emit the `tps:` family per row (mirroring
        // `create_fst.py` / `create_syllables_fst.py`).
        //
        // Post 2026-05-27 fix: `syllabifier::tps::valid_span_endings_lowered`
        // is inv-driven BFS (mirrors TL), so medial-led second syllables
        // (`ㄉㄞ|ㄨㄢ` where `ㄨ` is medial not initial) segment correctly.
        // The pre-fix structural "next initial seen" rule could not split
        // `ㄉㄞㄨㄢ` and `台灣` coverage was deferred to the tone-marked
        // path — that limitation is gone.
        // 中文: D / C-5 — TPS first-class;dispatch::handle 以 contains_tps 自動升級模式,
        // 中文:   input_mode="tl" 但 raw 含注音 → 走 TPS(對應生產自動偵測)。
        // 中文: 2026-05-27 修復後 inv-driven BFS 可切 medial-led 第二音節 (例 ㄉㄞ|ㄨㄢ),
        // 中文:   無聲調 台灣 覆蓋恢復;舊 next-initial-seen 限制退役。
        //
        // TPS toneless 2-syllable: `ㄉㄞㆣㄧ` = `tps_notone_from_tl("tâi-gí")`
        // → 台語 row hit.
        case("tps_notone_taigi", "ㄉㄞㆣㄧ", "tl"),
        // TPS toneless 4-syllable continuous: `ㄉㄞㆣㄧㄎㄧㄅㄨㆩ` =
        // `tps_notone_from_tl("tâi-gí-khí-puânn")` → 台語齒盤 row hit
        // (4-syllable walker slot-0) + sub-span dict hits (台 / 台語).
        case("tps_continuous_taigikhipuann", "ㄉㄞㆣㄧㄎㄧㄅㄨㆩ", "tl"),
        // Medial-led 2-syllable: `ㄉㄞㄨㄢ` = `tps_notone_from_tl("tâi-uân")`
        // → 台灣 row hit. `ㄨ` (U+3128) is a medial vowel, not a TPS
        // initial — the pre-fix structural scanner could not segment
        // this buffer at all (the inv probe of fused `tps:ㄉㄞㄨㄢ`
        // always missed because the inventory only carries single
        // syllables). Post-fix inv-driven BFS accepts the chain
        // `tps:ㄉㄞ` + `tps:ㄨㄢ`, the walker emits the full-buffer 台灣
        // key, and the span-local list adds the 1-syll 台 hit.
        case("tps_notone_taiuan", "ㄉㄞㄨㄢ", "tl"),
        // User-reported bug 2026-05-27: medial-led 4-syllable
        // continuous `ㄉㄞㄨㄢㄉㄞㆣㄧ` = `tps_notone_from_tl("tâi-uân-tâi-gí")`
        // → 台灣台語 row hit. Pre-fix returned ZERO candidates because
        // the first medial-led boundary (`ㄉㄞ|ㄨㄢ`) could not split,
        // killing the BFS at depth 1. Post-fix the chain
        // `tps:ㄉㄞ + tps:ㄨㄢ + tps:ㄉㄞ + tps:ㆣㄧ` reaches the full
        // buffer; walker slot-0 emits 台灣台語, span-local emits
        // 台灣 (2-syll, 6 bytes) + 台 (1-syll) + 台語 (interior — not
        // emitted as left-anchored key, surfaced via path step) hits.
        case("tps_notone_taiuantaigi", "ㄉㄞㄨㄢㄉㄞㆣㄧ", "tl"),
    ]
}

// --- driver + golden -----------------------------------------------------

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

/// Drive one case through `Start → EnterContinuous → FetchAtPos` on a
/// fresh `Engine` and format its candidate vector as a golden block.
fn run_case(c: &Case) -> String {
    let cfg = config(c.input_mode);
    let mut engine = Engine::new();
    dispatch::handle(
        &req(Method::Start(Start { text: c.raw.into() })),
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
            frequency_entries: c.freq.clone(),
            now_ms: c.now_ms,
            custom_entries: c.custom.clone(),
            enabled_sources_bitmask: c.enabled_sources_bitmask,
        })),
        &mut engine,
        &cfg,
    )
    .expect("FetchAtPos");

    let mut block = format!("## {} :: {}\n", c.name, c.raw);
    match resp.continuous {
        None => block.push_str("(continuous carrier absent)\n"),
        Some(cont) if cont.candidates.is_empty() => block.push_str("(no candidates)\n"),
        Some(cont) => {
            for cand in cont.candidates {
                // Signed-zero canonicalization keeps `-0.0` from churning
                // the golden vs `0.0` — applied to BOTH score columns so
                // the bits column agrees with the human one (spec NIT).
                let score = if cand.score == 0.0 { 0.0 } else { cand.score };
                let hanji = cand.hanji.as_deref().unwrap_or("⌀");
                // Two score columns. `{:.4}` is the spec §1.3 human-readable
                // behavioral-equivalence value (engine's own equality is
                // `abs() < 1e-4`); the trailing `{:#010x}` is the EXACT
                // `f32::to_bits()` so the golden truly freezes the full
                // proto wire vector — a sub-1e-4 score regression can no
                // longer pass with a byte-identical golden (Codex post-impl
                // BLOCK). S0's real consumers (A1/A2 relocation, the D1
                // single-build fold) are pure code-movement and keep these
                // bits identical, so this strengthens the gate without
                // introducing FP-reassociation false positives; spec §1.3's
                // anti-bit-exact rationale targeted bits-INSTEAD-OF the
                // behavioral value, not bits-ALONGSIDE it.
                block.push_str(&format!(
                    "{}|{}|{}|{}|{}|{}|{}|{}|{:.4}|{:#010x}\n",
                    cand.consumed_span_start,
                    cand.consumed_span_end,
                    cand.syllable_count,
                    cand.form,
                    cand.mode,
                    cand.display_text,
                    cand.roman,
                    hanji,
                    score,
                    score.to_bits(),
                ));
            }
        }
    }
    block
}

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/fetch_at_pos.golden")
}

#[test]
fn golden_fetch_at_pos() {
    let _lock = engine_install_lock();
    install_union_fixture();

    let mut actual = String::new();
    for c in matrix() {
        actual.push_str(&run_case(&c));
    }

    let path = golden_path();
    if std::env::var("UPDATE_GOLDEN").is_ok() {
        std::fs::create_dir_all(path.parent().unwrap()).expect("create golden dir");
        std::fs::write(&path, &actual).expect("write golden");
        eprintln!("golden recorded: {}", path.display());
        return;
    }

    let expected = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "golden missing ({e}); record with `UPDATE_GOLDEN=1 cargo test -p composing \
             --test golden_fetch_at_pos` then commit {}",
            path.display()
        )
    });

    if actual != expected {
        let drift = first_drift_section(&expected, &actual);
        panic!(
            "golden FetchAtPos diff — a behavior-neutral slice is NOT \
             behavior-neutral (do NOT UPDATE_GOLDEN to paper over it).\n\
             First drift around section: {drift}\n\
             Re-record intentionally with UPDATE_GOLDEN=1 only if the \
             behavior change is deliberate and reviewed."
        );
    }
}

/// Name the first `## section` whose body differs, so a regression points
/// at the offending matrix case instead of a wall of lines.
fn first_drift_section(expected: &str, actual: &str) -> String {
    let mut section = "(preamble)".to_string();
    let mut e_lines = expected.lines();
    let mut a_lines = actual.lines();
    loop {
        match (e_lines.next(), a_lines.next()) {
            (Some(e), Some(a)) => {
                if e.starts_with("## ") {
                    section = e.to_string();
                }
                if e != a {
                    return section;
                }
            }
            (None, None) => return "(no line-level drift — trailing bytes differ)".to_string(),
            _ => return format!("{section} (length differs)"),
        }
    }
}
