//! Dev-only candidate-dump harness for continuous-input diagnosis.
//!
//! Drives the **production** dictionary artifacts
//! (`assets/dictionaries/{dictionary.fst,dictionary.bin,association.bin,syllables.fst}`)
//! through the real `Start → FetchAtPos` pipeline and
//! prints the full candidate list for one or more inputs. This is the
//! deterministic, offline replacement for "log what the keyboard showed":
//! the engine is a pure function of (artifacts, input), so any reported
//! candidate strip can be reproduced exactly by typing the same input here.
//! No on-device candidate logging (which `docs/contributing/security-rules.md`
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
//! cargo test -p composing --test prod candidate_dump -- --ignored --nocapture
//! # custom inputs (comma-separated) + mode (tl|poj|tps) + simulated
//! # user_frequency rows `display:tl:count[:age_ms]`, custom_dictionary
//! # rows `roman[:hanji]` and learned-phrase rows `hanji:canonical_tl`
//! DUMP_FREQ="更新:king-sin:1:7200000" \
//! DUMP_CUSTOM="tâi-gí:台語" DUMP_LEARNED="記起來:kì--khí-lâi" \
//! DUMP_INPUTS="tai5,tai5gi2,tsua" DUMP_MODE=tl \
//!   cargo test -p composing --test prod candidate_dump -- --ignored --nocapture
//! # many inputs, one per line, verbatim (keeps a TPS trailing space)
//! DUMP_INPUTS_FILE=/tmp/inputs.txt DUMP_MODE=tps \
//!   cargo test -p composing --test prod candidate_dump -- --ignored --nocapture
//! ```
//!
//! Requires the production artifacts to exist (run `make dict` first if
//! `assets/dictionaries/` is stale — see the stale-artifact gate in `AGENTS.md`).
//! Prints `consumed_span`, `syllable_count`, `roman`,
//! and `hanji` per candidate.

use lexicon::{EngineHandle as LexiconHandle, LexiconPaths};
use protos::engine::DictionarySourceToggles;

use crate::common::{config, config_with_sources, fetch_at_pos_response, Fetch, Selected};
use lexicon::{CustomEntry, LearnedEntry};

const DEFAULT_INPUTS: &str = "tai5,tai5gi2,tai,tsua,ka";

fn production_artifact(name: &str) -> String {
    test_support::production_artifact(name)
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
    // DUMP_INPUTS_FILE: one input per line, kept verbatim (a trailing TPS
    // space is a tone-closing key, which the comma list would trim away).
    let inputs: Vec<String> = match std::env::var("DUMP_INPUTS_FILE") {
        Ok(path) => std::fs::read_to_string(&path)
            .expect("read DUMP_INPUTS_FILE")
            .lines()
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect(),
        Err(_) => std::env::var("DUMP_INPUTS")
            .unwrap_or_else(|_| DEFAULT_INPUTS.to_string())
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect(),
    };
    // DUMP_BITMASK lets a run mimic the device's source-toggle filter
    // (e.g. dev on + itaigi off) — the source / variant bits 0-12, sent as
    // the config's switches. Default: every source.
    let cfg = match std::env::var("DUMP_BITMASK")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
    {
        Some(bitmask) => config_with_sources(&mode, sources_from_bitmask(bitmask)),
        None => config(&mode),
    };

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

    for raw in inputs.iter().map(String::as_str) {
        let resp = fetch_at_pos_response(
            &cfg,
            raw,
            Fetch {
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
                "  [{i:>3}] end={} syll={} roman={:<16} hanji={}",
                cand.consumed_span_end,
                cand.syllable_count,
                cand.roman,
                cand.hanji.as_deref().unwrap_or(""),
            );
        }
    }
}

/// The switches whose `dictionary_filter_bitmask` is `bitmask`'s source /
/// variant bits 0-12 (`lexicon::dictionary_filters` layout); the kautian
/// subcollection region is not modelled.
fn sources_from_bitmask(bitmask: u32) -> DictionarySourceToggles {
    let on = |bit: u32| bitmask & (1 << bit) != 0;
    DictionarySourceToggles {
        kautian: on(0),
        taigitv: on(1),
        itaigi: on(2),
        sitbut: on(3),
        taihoa: on(4),
        taijit: on(5),
        kungge: on(6),
        stti: on(7),
        khpoo: on(8),
        khiin: on(9),
        dev: on(10),
        lkk: on(11),
        variant: on(12),
        kautian_subcollections: None,
    }
}
