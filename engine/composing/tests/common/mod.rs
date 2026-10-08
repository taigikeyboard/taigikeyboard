//! Shared helpers for composing integration tests.
//!
//! Each test binary root (`tests/it.rs`, `tests/prod.rs`,
//! `tests/no_lexicon.rs`, see `Cargo.toml`) declares `mod common;` once and
//! its test-file modules reach the helpers as `crate::common` — the helpers
//! never leak into the production crate (same pattern as
//! `engine/lexicon/tests/common/mod.rs`).
//!
//! Only helper *definitions* live here — the composing-shaped fixture
//! builders (`dictionary.fst` / `syllables.fst` families over [`Row`]), the
//! `AppConfig` / `ComposingRequest` constructors, and the `Start →
//! FetchAtPos` driver. Crate-neutral pieces (install lock,
//! temp files, TKDB / TKWA serializers, production artifacts) come from the
//! `test-support` dev-dependency. Every test file keeps its own fixture rows,
//! syllable samples, and assertions.

#![allow(dead_code)] // Different test files use different subsets.

use std::path::{Path, PathBuf};

use composing::api::Engine;
use composing::{requests, Intent, UserRows};
use lexicon::{
    CustomEntry, EngineHandle as LexiconHandle, LearnedEntry, LexiconPaths, SyllableInventory,
};
use phonetics::{canonicalize_poj_syllable, canonicalize_syllable};
use protos::engine::composing_request::Method;
use protos::engine::effect::Kind;
use protos::engine::{
    AppConfig, ComposingRequest, ComposingResponse, DictionarySourceToggles, Effect, FetchAtPos,
    Start,
};
use ranking::FrequencyData;
use test_support::{
    build_tkdb, build_tkwa, fst_entry, walker_cost_from_fixture_frequency, write_fst_set, TkdbRow,
};

const RANK_NEUTRAL_BITMASK: u16 = 1u16 << 11;

/// One dictionary fixture row. Empty `hanji` ⇒ TAILO (no hanji). `rowid` is
/// the 1-based slice index, shared between `dictionary.bin` (offset-table
/// order) and the `dictionary.fst` keys (`<family>:<key> + 0xFF + rowid_le`).
/// `toneless_key` feeds only the `tl:` family; TPS-only fixtures leave it
/// empty because their families derive from `tl`.
pub struct Row {
    pub toneless_key: &'static str,
    pub hanji: &'static str,
    pub tl: &'static str,
    pub syll: u8,
    pub freq: u32,
}

/// TKDB v4 `dictionary.bin`: every row rank-neutral with no kautian
/// provenance (subtag 0) and the walker cost its frequency had before E1
/// ([`walker_cost_from_fixture_frequency`]).
pub fn build_tkdb_v4(rows: &[Row]) -> Vec<u8> {
    build_tkdb_v4_costed(rows, |row| walker_cost_from_fixture_frequency(row.freq))
}

/// [`build_tkdb_v4`] with each row's walker cost from `walker_cost`, for
/// fixtures where the corpus cost and the frequency disagree on purpose.
pub fn build_tkdb_v4_costed(rows: &[Row], walker_cost: impl Fn(&Row) -> u16) -> Vec<u8> {
    let tkdb_rows: Vec<TkdbRow<'_>> = rows
        .iter()
        .map(|row| TkdbRow {
            bitmask: RANK_NEUTRAL_BITMASK,
            frequency: row.freq,
            syllable_count: Some(row.syll),
            kautian_subtag: Some(0),
            walker_cost: Some(walker_cost(row)),
            hanji: row.hanji,
            tl: row.tl,
        })
        .collect();
    build_tkdb(b"TKDB", 4, &tkdb_rows)
}

/// `dictionary.fst` with the toneless `tl:` family, the `poj:` family
/// (derived per row via `derive_poj_notone`) and the `tps:` family plus its
/// er↔or variant, and for multi-syllable rows the `<family>-abbrev:` acronym
/// key, plus the `hanzi:` family — the same chain as `create_fst.py`. Only
/// the whole-buffer abbreviation lookup (§46) reads the acronym family; the
/// nailed-join compound oracle (`lexicon::compound_hanji_exists`) reads
/// `hanzi:`.
pub fn build_dictionary_fst(rows: &[Row]) -> PathBuf {
    let mut entries: Vec<Vec<u8>> = Vec::with_capacity(rows.len());
    for (idx, row) in rows.iter().enumerate() {
        let rowid = (idx + 1) as u32;
        entries.push(fst_entry(b"tl:", row.toneless_key, rowid));
        if !row.hanji.is_empty() {
            entries.push(fst_entry(b"hanzi:", row.hanji, rowid));
        }
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
        // `extract_abbrev` / `extract_tps_abbrev` are "" for one syllable;
        // the acronym face goes to the family's own `*-abbrev:` range (§46).
        for (family, abbrev) in [
            (&b"tl-abbrev:"[..], phonetics::derive_abbrev(row.tl)),
            (&b"poj-abbrev:"[..], phonetics::poj_abbrev_from_tl(row.tl)),
            (&b"tps-abbrev:"[..], phonetics::tps_abbrev_from_tl(row.tl)),
        ] {
            if !abbrev.is_empty() {
                entries.push(fst_entry(family, &abbrev, rowid));
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
/// by [`write_fst_set`]).
pub fn inventory_from_keys(keys: Vec<String>) -> SyllableInventory {
    let path = write_fst_set(
        "inventory.fst",
        keys.into_iter().map(String::into_bytes).collect(),
    );
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

/// Empty `association.bin` — `TKWA` + version 2 + 0 keys/entries/ts.
pub fn empty_association_bin() -> Vec<u8> {
    build_tkwa(2, &[])
}

/// Validate the four fixture paths and swap them into the process-global
/// `lexicon::EngineHandle` singleton.
pub fn install_lexicon(
    fst_path: &Path,
    dict_path: &Path,
    association_path: &Path,
    syllables_path: &Path,
) {
    let paths = LexiconPaths::validated(
        fst_path.to_str().unwrap(),
        dict_path.to_str().unwrap(),
        association_path.to_str().unwrap(),
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

/// [`config`] asking for the Hanji conversion of a TPS preedit, every
/// dictionary source on.
pub fn config_converting(input_mode: &str) -> AppConfig {
    AppConfig {
        hanji_conversion: Some(protos::engine::HanjiConversion { toggles: None }),
        ..config(input_mode)
    }
}

/// The words of the engine's Hanji conversion as `(raw span, display)`;
/// empty when it holds none.
pub fn converted_words(engine: &Engine) -> Vec<((usize, usize), String)> {
    match engine.snapshot_state().phase {
        composing::Phase::Continuous {
            conversion: Some(conversion),
            ..
        } => conversion
            .segments
            .into_iter()
            .map(|segment| (segment.raw_span, segment.display_text))
            .collect(),
        _ => Vec::new(),
    }
}

/// A fresh engine already in `Phase::Continuous` over `raw` (TL config).
pub fn engine_in_continuous(raw: &str) -> Engine {
    let mut e = Engine::new();
    e.apply(
        composing::Intent::Start {
            text: raw.to_string(),
        },
        &config_tl(),
    );
    e
}

/// Wall clock the user-frequency fixtures are anchored on.
pub const NOW_MS: i64 = 1_700_000_000_000;

/// One `user_frequency.db` row: `hanji` / `canonical_tl` picked `count`
/// times, last `age_ms` before [`NOW_MS`].
#[derive(Clone, Debug)]
pub struct Selected {
    pub hanji: String,
    pub canonical_tl: String,
    pub count: i32,
    pub last_used_ms: i64,
}

pub fn selected(hanji: &str, canonical_tl: &str, count: i32, age_ms: i64) -> Selected {
    Selected {
        hanji: hanji.into(),
        canonical_tl: canonical_tl.into(),
        count,
        last_used_ms: NOW_MS - age_ms,
    }
}

/// A `FetchAtPos` as the engine runs it: the settings a platform sends plus
/// the user rows the engine reads from its stores (`composing::UserRows`).
/// The default filter is every source, as for a `FetchAtPos` without toggles.
#[derive(Clone, Debug)]
pub struct Fetch {
    pub now_ms: i64,
    pub enabled_sources_bitmask: u32,
    pub literal_roman_candidate_disabled: bool,
    pub frequency: Vec<Selected>,
    pub custom: Vec<CustomEntry>,
    pub learned: Vec<LearnedEntry>,
    pub context: ranking::ContextRanks,
}

impl Default for Fetch {
    fn default() -> Self {
        Self {
            now_ms: 0,
            enabled_sources_bitmask: u32::MAX,
            literal_roman_candidate_disabled: false,
            frequency: Vec::new(),
            custom: Vec::new(),
            learned: Vec::new(),
            context: ranking::ContextRanks::default(),
        }
    }
}

/// The sources a fresh install enables on every platform (desktop
/// `DictionarySourceToggles::DEFAULT`; iOS / Android defaults agree).
pub fn default_sources_bitmask() -> u32 {
    lexicon::api::dictionary_filter_bitmask(&DictionarySourceToggles {
        kautian: true,
        taigitv: true,
        kungge: true,
        stti: true,
        khpoo: true,
        lkk: true,
        dev: true,
        ..Default::default()
    })
}

/// `rows` as the frequency map a ranking reads.
pub fn frequency_map(rows: impl IntoIterator<Item = Selected>) -> ranking::FrequencyMap {
    rows.into_iter()
        .map(|row| {
            let data = FrequencyData {
                count: row.count,
                last_used_ms: row.last_used_ms,
            };
            (row.hanji, row.canonical_tl, data)
        })
        .collect()
}

impl Fetch {
    pub fn intent(self) -> Intent {
        let frequency = frequency_map(self.frequency);
        Intent::FetchAtPos {
            now_ms: self.now_ms,
            enabled_sources_bitmask: self.enabled_sources_bitmask,
            literal_roman_candidate_disabled: self.literal_roman_candidate_disabled,
            user_rows: UserRows {
                frequency,
                custom: self.custom,
                learned: self.learned,
            },
            context: self.context,
        }
    }
}

pub fn config_tl() -> AppConfig {
    config("tl")
}

/// The document text a response commits (`CommitTextReplacingPreedit`).
pub fn commit_text(resp: &ComposingResponse) -> Option<String> {
    resp.effect.iter().find_map(|e| match e.kind.as_ref() {
        Some(Kind::CommitTextReplacingPreedit(c)) => Some(c.text.clone()),
        _ => None,
    })
}

/// The ordered effect kinds of a response, by proto message name.
pub fn effect_kinds(effects: &[Effect]) -> Vec<&'static str> {
    effects
        .iter()
        .map(|e| match e.kind.as_ref().expect("effect kind") {
            Kind::UpdatePreedit(_) => "UpdatePreedit",
            Kind::ClearPreeditWithoutCommit(_) => "ClearPreeditWithoutCommit",
            Kind::CommitTextReplacingPreedit(_) => "CommitTextReplacingPreedit",
            Kind::ClearCandidates(_) => "ClearCandidates",
            Kind::RefreshCandidates(_) => "RefreshCandidates",
            Kind::ResetCandidateContext(_) => "ResetCandidateContext",
            Kind::NextWordUpdateLastSelectedWord(_) => "NextWordUpdateLastSelectedWord",
            Kind::NextWordWordSelected(_) => "NextWordWordSelected",
            Kind::NextWordClearForNewComposing(_) => "NextWordClearForNewComposing",
        })
        .collect()
}

pub fn req(method: Method) -> ComposingRequest {
    ComposingRequest {
        method: Some(method),
    }
}

/// Drive `raw` through `Start → FetchAtPos` on a fresh
/// `Engine` and return the `FetchAtPos` response (its `continuous` carrier is
/// `None` when the engine never entered the continuous phase — callers that
/// care distinguish that from an empty candidate list).
pub fn fetch_at_pos_response(config: &AppConfig, raw: &str, fetch: Fetch) -> ComposingResponse {
    let mut engine = continuous_engine(config, raw);
    requests::apply(fetch.intent(), &mut engine, config)
}

/// [`fetch_at_pos_response`] for a wire `FetchAtPos`, decoded the way a
/// platform request is (`requests::handle`) — no user rows.
pub fn wire_fetch_at_pos_response(
    config: &AppConfig,
    raw: &str,
    fetch: FetchAtPos,
) -> ComposingResponse {
    let mut engine = continuous_engine(config, raw);
    requests::handle(&req(Method::FetchAtPos(fetch)), &mut engine, config).expect("FetchAtPos")
}

fn continuous_engine(config: &AppConfig, raw: &str) -> Engine {
    let mut engine = Engine::new();
    requests::handle(
        &req(Method::Start(Start { text: raw.into() })),
        &mut engine,
        config,
    )
    .expect("Start");
    engine
}

/// [`fetch_at_pos_response`] under `config(input_mode)` with
/// `custom_dictionary.db` entries attached, reduced to the candidate hanji
/// in display order (roman-only candidates dropped). An empty `custom`
/// leaves the lexicon whole-buffer custom merge unexercised.
pub fn fetch_hanji_with_custom(
    raw: &str,
    input_mode: &str,
    custom: Vec<CustomEntry>,
) -> Vec<String> {
    fetch_hanji(
        raw,
        input_mode,
        Fetch {
            custom,
            ..Default::default()
        },
    )
}

/// [`fetch_at_pos_response`] under `config(input_mode)` with an arbitrary
/// `FetchAtPos` payload, reduced to the candidate hanji in display order
/// (roman-only candidates — including the §34 literal at index 0 — dropped).
pub fn fetch_hanji(raw: &str, input_mode: &str, fetch: Fetch) -> Vec<String> {
    let resp = fetch_at_pos_response(&config(input_mode), raw, fetch);
    resp.continuous
        .map(|c| {
            c.candidates
                .into_iter()
                .filter_map(|cand| cand.hanji)
                .collect()
        })
        .unwrap_or_default()
}

/// One candidate as the platform sees it: `(hanji, roman, display_text,
/// canonical_tl)` — the rendered cell plus the identity sidechannels it
/// commits.
pub type Cell = (Option<String>, String, String, String);

/// [`fetch_at_pos_response`] reduced to [`Cell`]s in display order (the §34
/// literal included, at index 0).
pub fn fetch_cells(config: &AppConfig, raw: &str, fetch: Fetch) -> Vec<Cell> {
    fetch_at_pos_response(config, raw, fetch)
        .continuous
        .map(|c| {
            c.candidates
                .into_iter()
                .map(|cand| (cand.hanji, cand.roman, cand.display_text, cand.canonical_tl))
                .collect()
        })
        .unwrap_or_default()
}

/// The first cell carrying `hanji`; panics with the whole list when none does.
pub fn cell_with_hanji<'a>(cells: &'a [Cell], hanji: &str) -> &'a Cell {
    cells
        .iter()
        .find(|c| c.0.as_deref() == Some(hanji))
        .unwrap_or_else(|| panic!("no candidate with hanji {hanji}; got {cells:?}"))
}

/// Installs the production lexicon (`assets/dictionaries/`) once per test process;
/// `false` (callers soft-skip) when the artifacts are absent — run `make dict`.
pub fn production_lexicon_ready() -> bool {
    static READY: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *READY.get_or_init(|| {
        let Some(artifacts) = test_support::ProductionArtifacts::locate() else {
            return false;
        };
        let paths = lexicon::LexiconPaths::validated(
            &artifacts.dictionary_fst,
            &artifacts.dictionary_bin,
            &artifacts.association_bin,
            &artifacts.syllables_fst,
            0,
        )
        .expect("validate production LexiconPaths");
        lexicon::EngineHandle::install(paths).is_ok()
    })
}
