//! Dev-only candidate-dump harness for continuous-input diagnosis.
//!
//! Drives the **production** dictionary artifacts
//! (`dictionaries/{dictionary.fst,dictionary.bin,association.bin,syllables.fst}`)
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
//! # custom inputs (comma-separated) + mode (tl|poj|tps) + simulated
//! # user_frequency rows `display:tl:count[:age_ms]`, custom_dictionary
//! # rows `roman[:hanji]` and learned-phrase rows `hanji:canonical_tl`
//! DUMP_FREQ="更新:king-sin:1:7200000" \
//! DUMP_CUSTOM="tâi-gí:台語" DUMP_LEARNED="記起來:kì--khí-lâi" \
//! DUMP_INPUTS="tai5,tai5gi2,tsua" DUMP_MODE=tl \
//!   cargo test -p composing --test candidate_dump -- --ignored --nocapture
//! ```
//!
//! Requires the production artifacts to exist (run `make dict` first if
//! `dictionaries/` is stale — see the stale-binary gate in `CLAUDE.md`).
//! Prints `consumed_span`, `syllable_count`, `roman`,
//! and `hanji` per candidate.

use std::path::PathBuf;

use lexicon::{EngineHandle as LexiconHandle, LexiconPaths};

mod common;
use common::{config, fetch_at_pos_response, Fetch, Selected};
use lexicon::{CustomEntry, LearnedEntry};

const DEFAULT_INPUTS: &str = "tai5,tai5gi2,tai,tsua,ka";

fn production_artifact(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../dictionaries")
        .join(name)
        .to_str()
        .expect("artifact path is valid UTF-8")
        .to_string()
}

/// `;`-separated `a[:b]` pairs from an env var; `b` is `""` when absent.
fn env_pairs(var: &str) -> Vec<(String, String)> {
    std::env::var(var)
        .unwrap_or_default()
        .split(';')
        .filter(|s| !s.is_empty())
        .map(|e| {
            let (a, b) = e.split_once(':').unwrap_or((e, ""));
            (a.to_string(), b.to_string())
        })
        .collect()
}

#[test]
#[ignore = "dev diagnosis harness — run with --ignored against production artifacts"]
fn dump_continuous_candidates() {
    let fst = production_artifact("dictionary.fst");
    if !std::path::Path::new(&fst).exists() {
        eprintln!(
            "candidate_dump: production artifacts absent at {fst} — run `make dict` first; skipping."
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
    // DUMP_BITMASK lets a run mimic the device's source-toggle filter
    // (e.g. dev on + itaigi off). Default 0 → dispatch normalizes to
    // u32::MAX (all sources), matching the prior all-sources behavior.
    let bitmask: u32 = std::env::var("DUMP_BITMASK")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let cfg = config(&mode);

    // DUMP_FREQ="更新:king-sin:10;羽:ú:10:7200000" — simulated
    // user_frequency rows `display:tl:count[:age_ms]`, default age 1 s.
    let now_ms: i64 = 1_800_000_000_000;
    let freq: Vec<Selected> = std::env::var("DUMP_FREQ")
        .unwrap_or_default()
        .split(';')
        .filter(|s| !s.is_empty())
        .map(|e| {
            let p: Vec<&str> = e.split(':').collect();
            Selected {
                hanji: p[0].to_string(),
                canonical_tl: p[1].to_string(),
                count: p[2].parse().unwrap(),
                last_used_ms: p
                    .get(3)
                    .map(|a| now_ms - a.parse::<i64>().unwrap())
                    .unwrap_or(now_ms - 1000),
            }
        })
        .collect();

    // DUMP_CUSTOM="kì-khí-lâi:記起來;tâi-gí" — simulated
    // `custom_dictionary.db` rows `roman[:hanji]`.
    let custom: Vec<CustomEntry> = env_pairs("DUMP_CUSTOM")
        .into_iter()
        .map(|(roman, hanji)| CustomEntry {
            roman,
            hanji: (!hanji.is_empty()).then_some(hanji),
        })
        .collect();

    // DUMP_LEARNED="記起來:kì--khí-lâi;…" — simulated learned-phrase rows
    // `hanji:canonical_tl` (§50), as the engine reads them for an exact
    // whole-buffer key match.
    let learned: Vec<LearnedEntry> = env_pairs("DUMP_LEARNED")
        .into_iter()
        .map(|(hanji, canonical_tl)| LearnedEntry {
            hanji,
            canonical_tl,
        })
        .collect();

    for raw in inputs.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let resp = fetch_at_pos_response(
            &cfg,
            raw,
            Fetch {
                enabled_sources_bitmask: bitmask,
                frequency: freq.clone(),
                now_ms,
                custom: custom.clone(),
                learned: learned.clone(),
                ..Default::default()
            },
        );

        let candidates = resp.continuous.map(|c| c.candidates).unwrap_or_default();
        println!(
            "\n==== mode={mode} raw={raw:?}  ({} candidates) ====",
            candidates.len()
        );
        for (i, cand) in candidates.iter().enumerate() {
            println!(
                "  [{i:>3}] span=({},{}) syll={} score={:<12} roman={:<16} hanji={}",
                cand.consumed_span_start,
                cand.consumed_span_end,
                cand.syllable_count,
                cand.score,
                cand.roman,
                cand.hanji.as_deref().unwrap_or(""),
            );
        }
    }
}
