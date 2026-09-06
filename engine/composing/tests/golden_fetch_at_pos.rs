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
//! ## Why this lives in `composing` with its own fixture builders
//!
//! `dispatch::handle(FetchAtPos)` resolves candidates through the
//! process-global `lexicon::EngineHandle` singleton, NOT injected fixtures.
//! The only pattern that drives real candidates through it is
//! `EngineHandle::install(LexiconPaths)` (mirrors
//! `engine/lexicon/tests/parity.rs`). This file must call BOTH
//! `composing::dispatch::handle` AND `lexicon::EngineHandle::install`, so
//! it lives in `composing` (which depends on `lexicon`). The hermetic
//! fixture builders live in `tests/common/mod.rs` because
//! `lexicon/tests/common` is a lexicon-test-private module not visible to
//! composing tests. `parity.rs` passes `""` for `syllables_fst` → inventory `None` →
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

use std::path::PathBuf;

use protos::engine::{CustomDictEntry, FetchAtPos, FrequencyEntry};

mod common;
use common::{
    build_syllables_fst, build_tkdb_v3, config, derive_poj_notone, empty_association_bin,
    engine_install_lock, fetch_at_pos_response, fst_entry, install_lexicon, write_fst_set,
    write_temp, Row,
};

// --- golden-only fixture builder (union of every key family) ---------------

/// `dictionary.fst` — union of every key family the golden matrix exercises,
/// each derived the way `dictionary/build/create_fst.py` does in production:
/// - `tl:<toneless_key>` (pre-B-2 path) plus the toned `tl:<tl_num>` family via
///   `phonetics::normalize_input` (the runtime numeric-tone normalizer), emitted
///   only when the result carries a tone digit;
/// - `poj:<poj_notone>` (v3.5.9 B-2) plus toned `poj:<poj_num>` — rows whose TL
///   display fails the POJ phonotactic gate have no POJ family, as in production;
/// - `tps:<tps_notone>` plus the C-3a er↔or variant (v3.5.9 D / C-5).
fn build_dictionary_fst(rows: &[Row]) -> PathBuf {
    let mut entries: Vec<Vec<u8>> = Vec::with_capacity(rows.len());
    for (idx, row) in rows.iter().enumerate() {
        let rowid = (idx + 1) as u32;
        entries.push(fst_entry(b"tl:", row.toneless_key, rowid));
        let tl_num = phonetics::normalize_input(row.tl);
        if tl_num.bytes().any(|b| b.is_ascii_digit()) {
            entries.push(fst_entry(b"tl:", &tl_num, rowid));
        }
        if let Some(poj_notone) = derive_poj_notone(row.tl) {
            entries.push(fst_entry(b"poj:", &poj_notone, rowid));
        }
        if let Some(poj_num) = derive_poj_num(row.tl) {
            if poj_num.bytes().any(|b| b.is_ascii_digit()) {
                entries.push(fst_entry(b"poj:", &poj_num, rowid));
            }
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

/// Explicit-tone fix — derive `poj_num` (toned POJ) from `record.tl`,
/// mirroring [`derive_poj_notone`] but keeping each syllable's tone digit
/// (`canonicalize_poj_syllable` returns `(toneless, tone)`; this concats
/// `toneless + tone`). Production `create_fst.py` emits `poj:<poj_num>`
/// alongside `poj:<poj_notone>`; this gives the fixture the same toned
/// POJ family so a toned POJ query byte-matches. Returns `None` on the
/// same phonotactic-gate failure as `derive_poj_notone`.
fn derive_poj_num(tl_display: &str) -> Option<String> {
    let poj_display = phonetics::api::tl_display_to_poj_display(tl_display);
    let mut out = String::new();
    for token in poj_display.split(['-', ' ']) {
        if token.is_empty() {
            continue;
        }
        let (toneless, tone) = phonetics::canonicalize_poj_syllable(token)?;
        out.push_str(&toneless);
        out.push_str(&tone);
    }
    (!out.is_empty()).then_some(out)
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
    install_lexicon(&fst_path, &dict_path, &assoc_path, &syllables_path);
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
        // Explicit-tone fix — numeric single-syllable input now FILTERS by
        // tone. `tsua2` keeps its digit (`tl:tsua2`) so only 紙 (tsuá) is
        // surfaced; the toneless-key sibling 珠仔 (tsu-á, `tl:tsu1a2`) is
        // excluded. Contrast `tl_toneless_multi` (`tsua` → 紙 + 珠仔). This
        // is the headline bug fix: pre-fix `tsua2` stripped to `tl:tsua`
        // and surfaced every tone. (Was `tsua7`, which has no fixture word
        // — pre-fix it folded to `tl:tsua` and wrongly returned 紙/珠仔.)
        case("tl_numeric_single", "tsua2", "tl"),
        // Explicit-tone fix — numeric multi-syllable input keeps the full
        // toned key. `tai5bak8` (= `tâi-ba̍k`/代墨) hits `tl:tai5bak8`; the
        // 1-syllable prefix span `tai5` also surfaces 台 (`tl:tai5`). Was
        // `tai1bak4` (an off-reading of 代墨 = tone5+tone8) which pre-fix
        // folded to `tl:taibak` and matched regardless of tone.
        case("tl_numeric_multi", "tai5bak8", "tl"),
        case("poj_diacritic", "tâi-uân", "tl"),
        // Negative guard: post-C-3b TPS is first-class. This raw's
        // toneless body `ㄉㄧㄠㄨㄢ` matches NO `tps:` prefix in the
        // fixture's `dictionary.fst` (closest is `tps:ㄉㄞㄨㄢ` from 台灣;
        // bytes diverge at position 4 — ㄧ vs ㄞ), so the partial-prefix
        // byte-range scan returns empty. With Fork 7b activated
        // (2026-05-27 dogfood enable), this case now exercises the
        // `prefix_index.n` no-hit branch on the TPS family — empty result
        // is intended.
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
        case("tps_partial_prefix_leading_initial", "\u{3109}", "tl"),
        // Regression guard: `ㄨ` (3 bytes) IS a complete TPS syllable
        // in the fixture inventory (it appears as the second syllable
        // of `tps:ㄉㄞㄨㄢ` for 台灣, and every TL `u`-sample like `tsua7`
        // derives `tps:ㄨ` via `to_zhuyin` since `u` → `ㄨ` is in the
        // ZHUYIN_VOWELS table). So this case must go through SPAN-LOCAL
        // fetch (`keys = ["tps:ㄨ"]`, non-empty) NOT partial-prefix. The
        // golden distinguishes the two paths via `coverage_kind`
        // (FULL=0 for span-local vs PARTIAL_PREFIX=1 above).
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
                canonical_tl: String::new(),
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
        case("tl_prefix_extension_taigi", "taigi", "tl"),
        case("trailing_hyphen", "tai-", "tl"),
        // Spec §1.5 names `HitTui`; no grounded hit/tui-class fixture
        // exists (would be invented dictionary content). `TaiUan` over
        // the grounded `tai`/`taiuan` rows (span_local_fetch::
        // taiuantaigi_*) exercises BOTH single-segment recase (台,
        // span 0–3) AND multi-segment recase (台灣, span 0–6 →
        // `Tâi-Uân`) plus the walker slot-0 prepend — closer to
        // `HitTui`'s relocation off-by-one intent than single-span
        // `Tsua` was. Codex pre-impl Q2 + USER ruling 2026-05-21.
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

/// Drive one case through `Start → EnterContinuous → FetchAtPos` on a
/// fresh `Engine` and format its candidate vector as a golden block.
fn run_case(c: &Case) -> String {
    let cfg = config(c.input_mode);
    let resp = fetch_at_pos_response(
        &cfg,
        c.raw,
        FetchAtPos {
            position: 0,
            frequency_entries: c.freq.clone(),
            now_ms: c.now_ms,
            custom_entries: c.custom.clone(),
            enabled_sources_bitmask: c.enabled_sources_bitmask,
            // §34/S22: all golden cases keep the literal-roman candidate ON
            // (the toggle OFF path is covered by a focused dispatch unit test).
            literal_roman_candidate_disabled: false,
        },
    );

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
