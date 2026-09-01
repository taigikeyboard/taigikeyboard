//! Cross-mode candidate parity — the SAME Taiwanese word, typed in TL / POJ /
//! TPS, must surface a CONSISTENT visible-Hanji candidate set: TL and POJ
//! identical, and neither's Hanji dropped by TPS. This is a *no-missing*
//! guard on the visible Hanji, NOT a full word-identity (`(hanzi, tl)`) or
//! rowid-equality check — TPS is allowed to ADD candidates (see the invariant
//! below), only never to drop one.
//!
//! `dictionary.csv` carries each word's three complete toned-syllable inputs
//! (`tl_num` / `poj_num` / `tps_num`, e.g. 我 → `gua2` / `goa2` / `ㆣㄨㄚˋ`),
//! so the three modes resolve to overlapping FST rowids — if the conversion +
//! key-building + fetch + display path is correct, TL/POJ produce the same
//! Hanji and TPS contains them all.
//! This complements the conversion-layer parity tests
//! (`lexicon/tests/{tps_notone_parity,tps_abbrev_parity}.rs`): those pin the
//! `tl → tps_*` derivation; this drives the full `FetchAtPos` pipeline and
//! catches fetch/ranking/display-layer divergence (the class behind the
//! recent TPS single-initial bug, which the conversion tests could not see).
//!
//! Invariant: `TL == POJ` (both ASCII — must be identical) AND `TL ⊆ TPS`.
//! TPS is a documented SUPERSET, not an equal set, because the `tps:` index
//! carries always-on dialect tolerance (er↔or `_var` families) and cannot
//! distinguish a few tone classes (tone-1 has no mark; entering tone-4 shares
//! the stop-coda glyph with tone-8) — so `tps_num` legitimately matches extra
//! readings. The dangerous direction is TPS MISSING a TL/POJ candidate
//! (`TL ⊄ TPS`); that is what this test guards against, plus any `TL ≠ POJ`.
//!
//! Normalizations for KNOWN intentional divergences:
//! - TL/POJ surface a literal-roman candidate (hanji absent); TPS does not
//!   (S22 excludes TPS). Disabled via `literal_roman_candidate_disabled` AND
//!   `hanji.is_some()` filter, so it never enters the set.
//! - Candidate ORDER differs between modes (ranking) → compare SETs.
//! - Only COMPLETE single syllables are compared (`consumed_span == (0, len)`
//!   + `syllable_count == 1`), excluding Step-4b multi-syllable extensions —
//!   those are a different (ranking-capped) path, not this invariant.
//! - Sampling excludes tone-1 words (TPS tone-1 has no mark → toneless →
//!   superset) and `tps_notone_var` (er↔or dialect) words to shrink the
//!   TPS-superset noise, though the assertion tolerates supersets anyway.

// 中文: 跨模式候選 parity — 同一台語詞用 TL/POJ/TPS 拍,Hanji 候選集須一致。
// 中文:   詞庫每列已有三模式完整含調拼法 (tl_num/poj_num/tps_num);三者解到同 rowid,
// 中文:   故轉換+建鍵+fetch+顯示正確時 Hanji 集相等。補 conversion-layer parity 沒覆蓋的
// 中文:   fetch/ranking/display 分歧 (近期 TPS 單聲母 bug 的類別)。normalize 已知刻意差異:
// 中文:   羅馬字[0] (TL/POJ 有 TPS 無) 關掉+丟 hanji=None;比 set 不比 order;只比完整單音節;
// 中文:   抽樣排除第1聲 (TPS 無調號→超集) 與 er↔or var。

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::OnceLock;

use composing::api::Engine;
use composing::dispatch;
use lexicon::{EngineHandle as LexiconHandle, LexiconPaths};
use protos::engine::composing_request::Method;
use protos::engine::{AppConfig, ComposingRequest, EnterContinuous, FetchAtPos, Start};

// ---------------------------------------------------------------------------
// Production artifact + lexicon install (once per test process).
// ---------------------------------------------------------------------------

fn production_artifact(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../dictionary/output")
        .join(name)
}

/// Install the production lexicon exactly once. Returns `false` (and the
/// callers soft-skip) when artifacts are absent — mirrors `candidate_dump.rs`;
/// the repo runs tests locally with artifacts present (no GitHub CI).
fn lexicon_ready() -> bool {
    static READY: OnceLock<bool> = OnceLock::new();
    *READY.get_or_init(|| {
        let fst = production_artifact("dictionary.fst");
        if !fst.exists() {
            eprintln!(
                "cross_mode_parity: production artifacts absent at {} — run `make dict && make build` first; skipping.",
                fst.display()
            );
            return false;
        }
        let to_str = |p: PathBuf| p.to_str().expect("artifact path UTF-8").to_string();
        let paths = LexiconPaths::validated(
            &to_str(production_artifact("dictionary.fst")),
            &to_str(production_artifact("dictionary.bin")),
            &to_str(production_artifact("association.bin")),
            &to_str(production_artifact("syllables.fst")),
            0,
        )
        .expect("validate production LexiconPaths");
        LexiconHandle::install(paths).is_ok()
    })
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

/// Drive `Start → EnterContinuous → FetchAtPos` for one input in one mode and
/// return the set of Hanji from COMPLETE single-syllable candidates only
/// (`consumed_span == (0, len)` + `syllable_count == 1`), dropping the
/// literal-roman candidate (`hanji == None`, also disabled at the request).
fn complete_syllable_hanji_set(input: &str, mode: &str) -> BTreeSet<String> {
    let cfg = config(mode);
    let mut engine = Engine::new();
    dispatch::handle(
        &req(Method::Start(Start { text: input.into() })),
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
            enabled_sources_bitmask: u32::MAX,
            literal_roman_candidate_disabled: true,
        })),
        &mut engine,
        &cfg,
    )
    .expect("FetchAtPos");

    let raw_len = input.len() as u32;
    resp.continuous
        .map(|c| c.candidates)
        .unwrap_or_default()
        .into_iter()
        .filter(|c| {
            c.consumed_span_start == 0 && c.consumed_span_end == raw_len && c.syllable_count == 1
        })
        .filter_map(|c| c.hanji)
        .collect()
}

// ---------------------------------------------------------------------------
// dictionary.csv — sampled parity cases.
// ---------------------------------------------------------------------------

struct Case {
    hanzi: String,
    tl_num: String,
    poj_num: String,
    tps_num: String,
    frequency: u32,
}

/// Read `dictionary.csv` and return the filtered + sampled parity cases.
/// Filter: single-syllable, tone ≠ 1, no er↔or variant, all three inputs
/// present. Sample: top-`TOP_BY_FREQ` by frequency + every-`STRIDE`-th of the
/// remaining filtered rows (deterministic — no RNG).
fn sampled_cases() -> Vec<Case> {
    const TOP_BY_FREQ: usize = 200;
    const STRIDE: usize = 60;

    let path = production_artifact("dictionary.csv");
    let text = std::fs::read_to_string(&path).expect("read dictionary.csv");
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().expect("CSV header").split(',').collect();
    let col = |name: &str| {
        header
            .iter()
            .position(|c| *c == name)
            .unwrap_or_else(|| panic!("`{name}` column present in CSV header"))
    };
    let (i_hanzi, i_tl, i_freq, i_tl_num, i_poj_num, i_tps_num, i_tps_var) = (
        col("hanzi"),
        col("tl"),
        col("frequency"),
        col("tl_num"),
        col("poj_num"),
        col("tps_num"),
        col("tps_notone_var"),
    );
    let max_idx = [
        i_hanzi, i_tl, i_freq, i_tl_num, i_poj_num, i_tps_num, i_tps_var,
    ]
    .into_iter()
    .max()
    .unwrap();

    let mut filtered: Vec<Case> = Vec::new();
    for line in lines {
        let f: Vec<&str> = line.split(',').collect();
        if f.len() <= max_idx {
            continue;
        }
        let tl = f[i_tl];
        let tl_num = f[i_tl_num];
        let poj_num = f[i_poj_num];
        let tps_num = f[i_tps_num];
        // single-syllable only
        if tl.contains('-') || tl.contains(' ') {
            continue;
        }
        // all three mode inputs present
        if tl_num.is_empty() || poj_num.is_empty() || tps_num.is_empty() {
            continue;
        }
        // tone 1 has no TPS mark → tps_num toneless → would match all tones
        if tl_num.ends_with('1') {
            continue;
        }
        // er↔or dialect variant → TPS emits extra `_var` candidates
        if !f[i_tps_var].is_empty() {
            continue;
        }
        filtered.push(Case {
            hanzi: f[i_hanzi].to_string(),
            tl_num: tl_num.to_string(),
            poj_num: poj_num.to_string(),
            tps_num: tps_num.to_string(),
            frequency: f[i_freq].parse().unwrap_or(0),
        });
    }

    // top-by-freq + stride over the rest (both deterministic). The `tl_num`
    // tie-break keeps the sample stable across dictionary rebuilds when a
    // hanzi has several equal-frequency readings.
    filtered.sort_by(|a, b| {
        b.frequency
            .cmp(&a.frequency)
            .then(a.hanzi.cmp(&b.hanzi))
            .then(a.tl_num.cmp(&b.tl_num))
    });
    let mut out: Vec<Case> = Vec::new();
    let mut seen: BTreeSet<(String, String)> = BTreeSet::new();
    for (idx, case) in filtered.into_iter().enumerate() {
        let take = idx < TOP_BY_FREQ || idx % STRIDE == 0;
        if take && seen.insert((case.hanzi.clone(), case.tl_num.clone())) {
            out.push(case);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// A — complete-syllable cross-mode Hanji-set parity (CI).
// ---------------------------------------------------------------------------

#[test]
fn cross_mode_candidate_hanji_parity() {
    if !lexicon_ready() {
        return;
    }
    let cases = sampled_cases();
    assert!(
        cases.len() > 50,
        "expected a non-trivial sample, got {} cases",
        cases.len()
    );

    let only = |a: &BTreeSet<String>, b: &BTreeSet<String>| -> Vec<String> {
        a.difference(b).cloned().collect()
    };
    let mut violations: Vec<String> = Vec::new();
    for case in &cases {
        let tl = complete_syllable_hanji_set(&case.tl_num, "tl");
        let poj = complete_syllable_hanji_set(&case.poj_num, "poj");
        let tps = complete_syllable_hanji_set(&case.tps_num, "tps");
        // TL == POJ (both ASCII) AND TL ⊆ TPS (TPS may add dialect/tone
        // supersets, but must never DROP a TL/POJ candidate).
        if tl == poj && tl.is_subset(&tps) {
            continue;
        }
        violations.push(format!(
            "  {} tl_num={:?} poj_num={:?} tps_num={:?}\n     poj\\tl={:?} tl\\poj={:?}  MISSING-from-tps (tl\\tps)={:?}",
            case.hanzi,
            case.tl_num,
            case.poj_num,
            case.tps_num,
            only(&poj, &tl),
            only(&tl, &poj),
            only(&tl, &tps),
        ));
    }

    if !violations.is_empty() {
        let shown = violations.len().min(20);
        panic!(
            "cross-mode candidate parity FAILED on {} of {} sampled words \
             (TL≠POJ or a TL candidate missing from TPS; showing first {}):\n{}",
            violations.len(),
            cases.len(),
            shown,
            violations[..shown].join("\n")
        );
    }
}

// ---------------------------------------------------------------------------
// B — dev dump: print three-mode Hanji sets + MATCH/MISMATCH for given words.
// Generates verified device-dogfood sheets + investigates reported diverges.
//   PARITY_WORDS=我,水,樹 cargo test -p composing --test cross_mode_parity \
//     -- --ignored --nocapture dump_cross_mode_parity
// ---------------------------------------------------------------------------

#[test]
#[ignore = "dev — set PARITY_WORDS=漢,字 and run with --ignored --nocapture"]
fn dump_cross_mode_parity() {
    if !lexicon_ready() {
        return;
    }
    let words = std::env::var("PARITY_WORDS").unwrap_or_else(|_| "我,水,樹".to_string());
    // Index the highest-frequency single-syllable row per requested hanzi.
    let cases = sampled_cases();
    for want in words.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let Some(case) = cases.iter().find(|c| c.hanzi == want) else {
            println!(
                "\n==== {want} — not in sampled set (multi-syllable / tone-1 / variant?) ===="
            );
            continue;
        };
        let tl = complete_syllable_hanji_set(&case.tl_num, "tl");
        let poj = complete_syllable_hanji_set(&case.poj_num, "poj");
        let tps = complete_syllable_hanji_set(&case.tps_num, "tps");
        let verdict = if tl == poj && poj == tps {
            "MATCH"
        } else {
            "MISMATCH"
        };
        println!(
            "\n==== {want}  TL={} POJ={} TPS={}  [{verdict}] ====",
            case.tl_num, case.poj_num, case.tps_num
        );
        println!("  TL : {tl:?}");
        println!("  POJ: {poj:?}");
        println!("  TPS: {tps:?}");
    }
}
