//! Dev-only candidate-dump harness for continuous-input diagnosis.
//!
//! Drives the **production** dictionary artifacts
//! (`dictionary/output/{dictionary.fst,dictionary.bin,association.bin,syllables.fst}`)
//! through the real `Start → EnterContinuous → FetchAtPos` pipeline and
//! prints the full candidate list for one or more inputs. This is the
//! deterministic, offline replacement for "log what the keyboard showed":
//! the engine is a pure function of (artifacts, input), so any reported
//! candidate strip can be reproduced exactly by typing the same input here.
//! No on-device candidate logging (which `.claude/rules/security-rules.md`
//! forbids for full user input) is needed.
//!
//! Why this is NOT a cross-IME comparison: peer IMEs under `references/`
//! (khiin-rs, librime, McBopomofo) ship different dictionaries, word
//! identities, and romanization coverage, so their candidate output for a
//! given key is not apples-to-apples with ours. They are an *algorithm*
//! reference (`docs/references/mainstream-ime-comparison.md`), not a
//! candidate-output oracle.
//!
//! ## Usage
//!
//! `#[ignore]`d so it never runs in the normal suite. Run on demand:
//!
//! ```sh
//! # default sample inputs (tl mode)
//! cargo test -p composing --test candidate_dump -- --ignored --nocapture
//! # custom inputs (comma-separated) + mode (tl|poj|tps)
//! DUMP_INPUTS="tai5,tai5gi2,tsua" DUMP_MODE=tl \
//!   cargo test -p composing --test candidate_dump -- --ignored --nocapture
//! ```
//!
//! Requires the production artifacts to exist (run `make dict && make build`
//! first if `dictionary/output/` is stale or absent — see the stale-binary
//! gate in `CLAUDE.md`). Prints `consumed_span`, `syllable_count`, `roman`,
//! and `hanji` per candidate.

// 中文: 開發用候選詞傾印工具 — 以 production 字典 artifacts 跑真實 FetchAtPos,
// 中文:   印出任意輸入的完整候選清單。引擎是 (artifacts, input) 的純函數,
// 中文:   故任何回報的候選列皆可在此精確重現,無需在裝置端記錄使用者輸入
// 中文:   (security-rules 禁止記錄完整使用者輸入)。預設 #[ignore],按需 --ignored 執行。

use std::path::PathBuf;

use composing::api::Engine;
use composing::dispatch;
use lexicon::{EngineHandle as LexiconHandle, LexiconPaths};
use protos::engine::composing_request::Method;
use protos::engine::{AppConfig, ComposingRequest, EnterContinuous, FetchAtPos, Start};

const DEFAULT_INPUTS: &str = "tai5,tai5gi2,tai,tsua,ka";

fn production_artifact(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../dictionary/output")
        .join(name)
        .to_str()
        .expect("artifact path is valid UTF-8")
        .to_string()
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

#[test]
#[ignore = "dev diagnosis harness — run with --ignored against production artifacts"]
fn dump_continuous_candidates() {
    let fst = production_artifact("dictionary.fst");
    if !std::path::Path::new(&fst).exists() {
        eprintln!(
            "candidate_dump: production artifacts absent at {fst} — run `make dict && make build` first; skipping."
        );
        return;
    }
    let paths = LexiconPaths::validated(
        &fst,
        &production_artifact("dictionary.bin"),
        &production_artifact("association.bin"),
        &production_artifact("syllables.fst"),
        0,
    )
    .expect("validate production LexiconPaths");
    LexiconHandle::install(paths).expect("install production lexicon");

    let mode = std::env::var("DUMP_MODE").unwrap_or_else(|_| "tl".to_string());
    let inputs = std::env::var("DUMP_INPUTS").unwrap_or_else(|_| DEFAULT_INPUTS.to_string());
    let cfg = config(&mode);

    for raw in inputs.split(',').map(str::trim).filter(|s| !s.is_empty()) {
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
            })),
            &mut engine,
            &cfg,
        )
        .expect("FetchAtPos");

        let candidates = resp.continuous.map(|c| c.candidates).unwrap_or_default();
        println!(
            "\n==== mode={mode} raw={raw:?}  ({} candidates) ====",
            candidates.len()
        );
        for (i, cand) in candidates.iter().enumerate() {
            println!(
                "  [{i:>3}] span=({},{}) syll={} roman={:<16} hanji={}",
                cand.consumed_span_start,
                cand.consumed_span_end,
                cand.syllable_count,
                cand.roman,
                cand.hanji.as_deref().unwrap_or(""),
            );
        }
    }
}
