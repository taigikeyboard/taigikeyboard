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
//! Every dictionary triple `(toneless_key, hanzi, tl)` is copied verbatim
//! from a named existing hermetic test:
//!
//! - `span_local_fetch.rs` — tsua/taigikhipuann/taiuantaigi/taibak/
//!   mode-carrier/partial-prefix rows.
//! - `user_freq_plumb.rs` — the `台` user-freq row.
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

/// TKDB v2 byte layout — verbatim logic from
/// `engine/lexicon/tests/common/mod.rs::build_tkdb_bin` (`build_tkdb_v2`
/// path: every row carries a `syllable_count` byte).
fn build_tkdb_v2(rows: &[Row]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"TKDB");
    out.extend_from_slice(&2u32.to_le_bytes()); // version
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
        let mut e = Vec::with_capacity(row.toneless_key.len() + 4 + 5);
        e.extend_from_slice(b"tl:");
        e.extend_from_slice(row.toneless_key.as_bytes());
        e.push(SEPARATOR);
        e.extend_from_slice(&rowid.to_le_bytes());
        entries.push(e);
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

/// `syllables.fst` — both numeric and toneless canonical keys, exactly
/// the builder shape from `engine/composing/tests/build_keys_tl_hyphen.rs`.
/// `.expect()` (not silent skip) so a wrong grounding assumption fails
/// loudly here rather than degrading a matrix case (Codex pre-impl BLOCK).
fn build_syllables_fst(samples: &[&str]) -> PathBuf {
    // v3.5.9 B-1: tagged-single-FST — emit keys with `tl:` prefix so
    // production lookups via `contains_in(InputMode::Tl, …)` hit.
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
    ]
}

/// Inventory samples. Component syllables of every multi-syllable matrix
/// raw plus the proven-`VALID_SAMPLES` OOV tail (`lang`/`kang`/`tan`/
/// `lai`, from `engine/lexicon/tests/syllables_fst.rs`).
const SYLLABLE_SAMPLES: &[&str] = &[
    "tsua7", "tsu", "tai1", "tai5", "bak4", "uan5", "gi2", "khi2", "puann5", "li2", "iau2", "si7",
    "gua2", "lang5", "kang1", "tan5", "lai5",
];

fn install_union_fixture() {
    let rows = fixture_rows();
    let dict_path = write_temp("dictionary.bin", &build_tkdb_v2(&rows));
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
}

fn case(name: &'static str, raw: &'static str, input_mode: &'static str) -> Case {
    Case {
        name,
        raw,
        input_mode,
        freq: Vec::new(),
        now_ms: 0,
        custom: Vec::new(),
    }
}

fn matrix() -> Vec<Case> {
    vec![
        case("tl_toneless_multi", "tsua", "tl"),
        case("tl_toneless_long_reach", "taigikhipuann", "tl"),
        case("tl_numeric_single", "tsua7", "tl"),
        case("tl_numeric_multi", "tai1bak4", "tl"),
        case("poj_diacritic", "tâi-uân", "tl"),
        // Negative guard: TPS has no partial-prefix and no tiau/uan dict
        // rows → empty list. Catches an accidental TPS partial-prefix
        // regression (Codex pre-impl Q3: keep empty, do not invent rows).
        case("tps_walker_excluded", "ㄉㄧㄠˊㄨㄢˊ", "tl"),
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
        },
        case("all_oov_partial_prefix", "g", "tl"),
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
        // POJ-ASCII fold: `choa`(ch→ts,oa→ua)→`tsua` (grounded:
        // syllables_fst.rs VALID_SAMPLE `choa7→tsua` + canonicalize_poj
        // ch→ts test).
        case("poj_ascii_choa", "choa", "poj"),
        // POJ-ASCII fold: `goa`(oa→ua)→`gua`→我 (grounded:
        // canonicalize_poj_shadow_poj_ascii_chhia_oa_oe_eng_ek_fold).
        case("poj_ascii_goa", "goa", "poj"),
        // POJ render: `台語齒盤`'s roman `tâi-gí-khí-puânn` → `nn`→`ⁿ`
        // (grounded: render_roman_for_mode_poj_rewrites_oo_and_nn).
        case("poj_render_nn", "taigikhipuann", "poj"),
        // POJ post-render dedupe: a custom entry mirroring the grounded
        // `台語齒盤` dict row collides after the POJ render → first-wins
        // (grounded: dedupe_rendered_continuous_drops_post_render_collision).
        Case {
            name: "poj_post_render_dedupe",
            raw: "taigikhipuann",
            input_mode: "poj",
            freq: Vec::new(),
            now_ms: 0,
            custom: vec![CustomDictEntry {
                roman: "tâi-gí-khí-puânn".into(),
                hanji: Some("台語齒盤".into()),
            }],
        },
        // ≥6-syllable long-OOV-vs-dict: `tai`/`taigi` dict-covered,
        // `lang`/`kang`/`tan`/`lai` proven-valid + dict-absent → OOV path
        // (RC0 class, project_continuous_abbrev_collision). Codex pre-impl
        // BLOCK: known-valid syllables, not silent-skip degradation.
        case("long_oov_vs_dict", "taigilangkangtanlai", "tl"),
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
