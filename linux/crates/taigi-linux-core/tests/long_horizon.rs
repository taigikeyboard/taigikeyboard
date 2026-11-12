//! Long-horizon user-interaction simulator and metamorphic / corpus-guided
//! oracle suite over `taigi_desktop_core::session` + real dictionaries + real
//! SQLite `userdata` stores.
//!
//! Quick smoke suite (runs in normal `cargo test`):
//!   `cargo test -p taigi-linux-core --test long_horizon`
//!
//! 30-minute marathon run (release-optimized):
//!   `TAIGI_MARATHON_SECS=1800 cargo test --release -p taigi-linux-core --test long_horizon long_horizon_marathon -- --ignored --nocapture`
//!
//! Failure reports include lossless `<id>.repro.json` artifacts. Replay one in
//! a fresh filtered process (the saved SQLite state is loaded before opening):
//!   `TAIGI_REPRO_PATH=<report-dir>/<id>.repro.json cargo test --release -p taigi-linux-core --test long_horizon long_horizon_replay -- --ignored --nocapture`
//! These artifacts retain full settings, capabilities, priming and exact actions.
//! Metamorphic replays also retain both input sequences and all four user-data
//! stores. Portable e2e scenarios are explicitly unavailable: that schema cannot
//! express these failure predicates and every simulator interaction faithfully.

mod long_horizon_support;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use long_horizon_support::corpus::{load_corpus, parse_example_sentences_js};
use long_horizon_support::host::{HostSimulator, SimStep, SpecialKey};
use long_horizon_support::oracles::{
    capture_probe_snapshot, check_committed_output_surprises, check_learning_monotonicity,
    check_metamorphic_case, diff_probe_snapshots, evaluate_corpus_reachability,
    run_metamorphic_suite, standard_tl_settings, ReachabilityStats, SurpriseFinding,
};
use long_horizon_support::personas::{build_episode_settings, compile_sentence_steps, PersonaKind};
use long_horizon_support::report::{BugReproRecord, LatencySummary, MarathonReport, PersonaStats};
use long_horizon_support::rng::Rng;
use long_horizon_support::shrinker::{
    shrink_episode_failure, shrink_metamorphic_failure, ReproSetup,
};
use long_horizon_support::{
    acquire_process_env, acquire_process_env_with_snapshot, capture_user_data_snapshot,
    reset_all_persistent_state, sync_user_data_workers, LockedProcessEnv, UserDataCounts,
};

const MONOTONICITY_READINGS: &[&str] = &[
    "ho2", "tsit8", "lang5", "ki5", "tua7", "si7", "kong2", "khui1", "lai5", "tng2", "sann1",
    "khuann3",
];

#[test]
fn corpus_loader_and_js_trailing_comma_parser_work() {
    let env = acquire_process_env();
    let _ = (env.user_data_dir, env.dictionaries_dir);

    let sample_js = r#"
    const exampleSentences = [
      [
        {
          "hanji": "食飽未？今仔日天氣真好。",
          "tailo": "Tsia̍h-pá--buē? Kin-á-ji̍t thinn-khì tsin hó.",
          "dic_url": "synthetic://0",
        },
      ],
    ];
    "#;
    let parsed = parse_example_sentences_js(sample_js).expect("parses JS with trailing commas");
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].hanji, "食飽未？今仔日天氣真好。");

    let corpus = load_corpus(Some(24));
    assert!(!corpus.sentences.is_empty());
    let first = &corpus.sentences[0];
    assert!(!first.segments.is_empty());
}

#[test]
fn all_six_personas_preserve_host_and_session_invariants() {
    let env = acquire_process_env();
    reset_all_persistent_state();
    let corpus = load_corpus(Some(30));
    let mut rng = Rng::new(0x5441_4947_2026_0007);

    for persona in PersonaKind::ALL {
        reset_all_persistent_state();
        let settings = build_episode_settings(persona, &mut rng);
        let mut sim = HostSimulator::new(
            env.user_data_dir,
            env.dictionaries_dir,
            &settings,
            rng.chance(3, 4),
        );
        for i in 0..6 {
            let sentence = &corpus.sentences[i % corpus.sentences.len()];
            let steps = compile_sentence_steps(persona, sentence, &settings, &mut rng);
            for step in &steps {
                if let Err(v) = sim.step(step) {
                    panic!(
                        "persona {} tripped {} on sentence #{}: {} (step={:?})",
                        persona.name(),
                        v.code,
                        sentence.index,
                        v.message,
                        v.step
                    );
                }
            }
        }
    }
    reset_all_persistent_state();
}

#[test]
fn metamorphic_confluences_and_shrinker_work() {
    let env = acquire_process_env();
    reset_all_persistent_state();
    let corpus = load_corpus(Some(16));
    let mut stats = ReachabilityStats::default();

    for sentence in corpus.sentences.iter().take(6) {
        if let Err(fail) = run_metamorphic_suite(
            env.user_data_dir,
            env.dictionaries_dir,
            sentence,
            &mut stats,
        ) {
            panic!("metamorphic failure: {fail:?}");
        }
    }
    assert!(stats.metamorphic_checks >= 4);

    // Verify two-stage shrinker reduces noisy batches + steps down to the exact 2-step fault.
    let settings = taigi_desktop_core::settings::SettingsDocument::default();
    let prior = vec![
        vec![
            SimStep::KeyChar {
                ch: 'a',
                shift: false,
            },
            SimStep::Special(SpecialKey::Enter),
        ],
        vec![
            SimStep::KeyChar {
                ch: 'i',
                shift: false,
            },
            SimStep::Special(SpecialKey::Enter),
        ],
        vec![
            SimStep::KeyChar {
                ch: 'u',
                shift: false,
            },
            SimStep::Special(SpecialKey::Enter),
        ],
    ];
    let failing = vec![
        SimStep::KeyChar {
            ch: 'k',
            shift: false,
        },
        SimStep::KeyChar {
            ch: 'x',
            shift: false,
        },
        SimStep::KeyChar {
            ch: 'a',
            shift: false,
        },
        SimStep::Special(SpecialKey::Backspace),
        SimStep::KeyChar {
            ch: '!',
            shift: true,
        },
        SimStep::Special(SpecialKey::Enter),
    ];
    let shrunk = shrink_episode_failure(
        &ReproSetup::new(&settings, true),
        &prior,
        &failing,
        |_p, s| {
            let has_x = s.contains(&SimStep::KeyChar {
                ch: 'x',
                shift: false,
            });
            let has_bang = s.contains(&SimStep::KeyChar {
                ch: '!',
                shift: true,
            });
            has_x && has_bang
        },
    );
    assert_eq!(shrunk.shrunk_prior_sentences, 0);
    assert_eq!(shrunk.shrunk_steps, 2);
}

#[test]
#[ignore = "long-horizon marathon; invoke explicitly with --ignored --nocapture"]
fn long_horizon_marathon() {
    let env = acquire_process_env();
    reset_all_persistent_state();

    let target_secs: u64 = std::env::var("TAIGI_MARATHON_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1800);
    let master_seed: u64 = std::env::var("TAIGI_MARATHON_SEED")
        .ok()
        .and_then(|s| {
            let trimmed = s.trim();
            if let Some(hex) = trimmed.strip_prefix("0x") {
                u64::from_str_radix(hex, 16).ok()
            } else {
                trimmed.parse().ok()
            }
        })
        .unwrap_or(0x5441_4947_495F_3236);
    let out_dir: PathBuf = std::env::var("TAIGI_MARATHON_OUT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            long_horizon_support::corpus::repo_root().join("linux/target/long-horizon-latest")
        });

    println!(
        "[long-horizon] loading corpus (target_secs={target_secs}, seed=0x{master_seed:016X}, out={})...",
        out_dir.display()
    );
    let load_t0 = Instant::now();
    let corpus = load_corpus(None);
    println!(
        "[long-horizon] loaded {} sentences (real_moe_corpus={}) from {} in {:.2}s",
        corpus.sentences.len(),
        corpus.is_real_corpus,
        corpus.source_label,
        load_t0.elapsed().as_secs_f64()
    );

    let mut rng = Rng::new(master_seed);
    let start_instant = Instant::now();
    let deadline = start_instant + Duration::from_secs(target_secs);
    let mut last_checkpoint = Instant::now();

    let mut episodes_completed = 0u64;
    let mut total_sentences_typed = 0u64;
    let mut total_steps_executed = 0u64;
    let mut sweep_cursor: usize = std::env::var("TAIGI_SWEEP_OFFSET")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let mut max_counts = UserDataCounts::default();
    let mut early_latencies: Vec<u64> = Vec::with_capacity(60_000);
    let mut late_latencies: Vec<u64> = Vec::with_capacity(60_000);
    let mut per_persona: BTreeMap<String, PersonaStats> = BTreeMap::new();
    let mut reachability = ReachabilityStats::default();
    let mut hard_failures: Vec<BugReproRecord> = Vec::new();
    let mut seen_bug_signatures: BTreeSet<String> = BTreeSet::new();
    let mut surprises: Vec<SurpriseFinding> = Vec::new();
    let mut seen_surprise_keys: BTreeSet<String> = BTreeSet::new();

    // Baseline Golden Probe snapshot on a completely clean user-data store.
    reset_all_persistent_state();
    let baseline_probes = capture_probe_snapshot(env.user_data_dir, env.dictionaries_dir);

    while Instant::now() < deadline {
        let episode_idx = episodes_completed as usize;
        let persona = PersonaKind::ALL[episode_idx % PersonaKind::ALL.len()];
        let episode_seed = rng.next_u64();
        let mut ep_rng = Rng::new(episode_seed);

        reset_all_persistent_state();
        let settings = build_episode_settings(persona, &mut ep_rng);
        let initial_surrounding = ep_rng.chance(4, 5);
        let mut sim = HostSimulator::new(
            env.user_data_dir,
            env.dictionaries_dir,
            &settings,
            initial_surrounding,
        );

        let episode_horizon = ep_rng.range_inclusive(180, 260);
        let mut prior_batches: Vec<Vec<SimStep>> = Vec::with_capacity(episode_horizon);

        for sent_offset in 0..episode_horizon {
            if Instant::now() >= deadline {
                break;
            }

            // Interleave sequential full-corpus sweep with random sentence combinations.
            let sentence = if sent_offset % 2 == 0 {
                let s = &corpus.sentences[sweep_cursor % corpus.sentences.len()];
                sweep_cursor = sweep_cursor.wrapping_add(1);
                s
            } else {
                ep_rng.choose(&corpus.sentences)
            };

            sim.clear_committed_buffer();
            let lat_before = sim.key_latencies_us.len();
            let steps = compile_sentence_steps(persona, sentence, &settings, &mut ep_rng);

            let mut tripped = None;
            let mut executed = 0usize;
            for step in &steps {
                executed += 1;
                if let Err(v) = sim.step(step) {
                    tripped = Some(v);
                    break;
                }
            }

            let new_lats = &sim.key_latencies_us[lat_before..];
            if sent_offset < 30 && early_latencies.len() < 250_000 {
                early_latencies.extend_from_slice(new_lats);
            } else if sent_offset >= 150 && late_latencies.len() < 250_000 {
                late_latencies.extend_from_slice(new_lats);
            }

            total_sentences_typed += 1;
            total_steps_executed += executed as u64;
            let p_entry = per_persona.entry(persona.name().to_owned()).or_default();
            p_entry.sentences += 1;
            p_entry.steps += executed as u64;

            if let Some(violation) = tripped {
                let sig = violation.code.to_owned();
                if seen_bug_signatures.insert(sig) && Instant::now() < deadline {
                    let bug_id = format!(
                        "repro-{:02}-{}",
                        hard_failures.len() + 1,
                        violation.code.to_lowercase()
                    );
                    let target_code = violation.code;
                    let setup = ReproSetup::new(&settings, initial_surrounding);
                    let repro = shrink_episode_failure(
                        &setup,
                        &prior_batches,
                        &steps[..executed],
                        |prior, steps| {
                            reproduces_host_failure(&env, &setup, prior, steps, target_code)
                        },
                    );
                    println!(
                        "[long-horizon] BUG FOUND: {} ({}) shrunk {}->{} prior, {}->{} steps",
                        violation.code,
                        persona.name(),
                        repro.original_prior_sentences,
                        repro.shrunk_prior_sentences,
                        repro.original_steps,
                        repro.shrunk_steps
                    );
                    hard_failures.push(BugReproRecord {
                        id: bug_id,
                        code: violation.code.to_owned(),
                        persona: persona.name().to_owned(),
                        episode_seed,
                        sentence_index: sentence.index,
                        sentence_tailo: sentence.tailo.clone(),
                        message: violation.message,
                        reproducer: repro,
                    });
                }
                // Recover episode simulator after failure.
                reset_all_persistent_state();
                sim = HostSimulator::new(
                    env.user_data_dir,
                    env.dictionaries_dir,
                    &settings,
                    initial_surrounding,
                );
                prior_batches.clear();
                continue;
            }

            check_committed_output_surprises(
                &sim.buffer.committed,
                persona.name(),
                sentence,
                &mut seen_surprise_keys,
                &mut surprises,
            );
            prior_batches.push(steps);

            // Periodic deep metamorphic checks over live accumulated user data.
            if sent_offset % 14 == 7 {
                let oracle_snapshot = capture_user_data_snapshot(env.user_data_dir);
                if let Err(oracle_fail) = run_metamorphic_suite(
                    env.user_data_dir,
                    env.dictionaries_dir,
                    sentence,
                    &mut reachability,
                ) {
                    let sig = format!("oracle:{}", oracle_fail.oracle);
                    if seen_bug_signatures.insert(sig) {
                        let bug_id = format!(
                            "repro-{:02}-{}",
                            hard_failures.len() + 1,
                            oracle_fail.oracle.to_lowercase()
                        );
                        let setup = ReproSetup {
                            settings: standard_tl_settings(),
                            can_delete_surrounding: true,
                            user_data_snapshot: Some(oracle_snapshot),
                        };
                        let shrunk =
                            shrink_metamorphic_failure(&setup, &oracle_fail.case, |case| {
                                check_metamorphic_case(
                                    env.user_data_dir,
                                    env.dictionaries_dir,
                                    &setup.settings,
                                    case,
                                )
                                .is_err_and(|failure| failure.oracle == oracle_fail.oracle)
                            });
                        println!(
                            "[long-horizon] METAMORPHIC FAILURE: {} on sentence #{}",
                            oracle_fail.oracle, sentence.index
                        );
                        hard_failures.push(BugReproRecord {
                            id: bug_id,
                            code: oracle_fail.oracle.to_owned(),
                            persona: persona.name().to_owned(),
                            episode_seed,
                            sentence_index: oracle_fail.sentence_index,
                            sentence_tailo: oracle_fail.sentence_tailo,
                            message: oracle_fail.detail,
                            reproducer: shrunk,
                        });
                    }
                }
            }

            // Deterministic corpus word reachability sweep.
            if sent_offset % 6 == 0 {
                evaluate_corpus_reachability(
                    env.user_data_dir,
                    env.dictionaries_dir,
                    sentence,
                    &mut reachability,
                    &mut seen_surprise_keys,
                    &mut surprises,
                );
            }

            // Explicit learning monotonicity check.
            if sent_offset == 90 {
                let reading = MONOTONICITY_READINGS[episode_idx % MONOTONICITY_READINGS.len()];
                check_learning_monotonicity(
                    env.user_data_dir,
                    env.dictionaries_dir,
                    reading,
                    sentence.index,
                    &mut reachability,
                    &mut seen_surprise_keys,
                    &mut surprises,
                );
            }

            // Periodic 60s checkpoint report flush.
            if last_checkpoint.elapsed() >= Duration::from_secs(60) {
                let elapsed = start_instant.elapsed().as_secs_f64();
                let ckpt = MarathonReport {
                    seed: master_seed,
                    elapsed_secs: elapsed,
                    target_secs,
                    corpus_source: corpus.source_label.clone(),
                    corpus_sentences_available: corpus.sentences.len(),
                    episodes_completed,
                    total_sentences_typed,
                    total_steps_executed,
                    throughput_steps_per_sec: total_steps_executed as f64 / elapsed.max(0.001),
                    max_user_data_counts: max_counts,
                    early_episode_latency: LatencySummary::from_samples(&early_latencies),
                    late_episode_latency: LatencySummary::from_samples(&late_latencies),
                    per_persona: per_persona.clone(),
                    reachability: reachability.clone(),
                    hard_failures: hard_failures.clone(),
                    surprises: surprises.clone(),
                };
                let _ = ckpt.write_to_dir(&out_dir);
                println!(
                    "[long-horizon] checkpoint {:.0}s/{target_secs}s | episodes={} sentences={} steps={} ({:.0}/s) | bugs={} surprises={}",
                    elapsed,
                    episodes_completed,
                    total_sentences_typed,
                    total_steps_executed,
                    ckpt.throughput_steps_per_sec,
                    hard_failures.len(),
                    surprises.len()
                );
                last_checkpoint = Instant::now();
            }
        }

        let counts = sync_user_data_workers();
        max_counts.frequency_rows = max_counts.frequency_rows.max(counts.frequency_rows);
        max_counts.association_rows = max_counts.association_rows.max(counts.association_rows);
        max_counts.learned_phrase_rows = max_counts
            .learned_phrase_rows
            .max(counts.learned_phrase_rows);
        max_counts.custom_rows = max_counts.custom_rows.max(counts.custom_rows);

        let after_probes = capture_probe_snapshot(env.user_data_dir, env.dictionaries_dir);
        diff_probe_snapshots(
            &baseline_probes,
            &after_probes,
            episode_idx,
            &mut seen_surprise_keys,
            &mut surprises,
        );

        episodes_completed += 1;
        per_persona
            .entry(persona.name().to_owned())
            .or_default()
            .episodes += 1;
    }

    reset_all_persistent_state();
    let elapsed = start_instant.elapsed().as_secs_f64();
    let final_report = MarathonReport {
        seed: master_seed,
        elapsed_secs: elapsed,
        target_secs,
        corpus_source: corpus.source_label,
        corpus_sentences_available: corpus.sentences.len(),
        episodes_completed,
        total_sentences_typed,
        total_steps_executed,
        throughput_steps_per_sec: total_steps_executed as f64 / elapsed.max(0.001),
        max_user_data_counts: max_counts,
        early_episode_latency: LatencySummary::from_samples(&early_latencies),
        late_episode_latency: LatencySummary::from_samples(&late_latencies),
        per_persona,
        reachability,
        hard_failures,
        surprises,
    };
    final_report
        .write_to_dir(&out_dir)
        .expect("write final marathon report");
    println!(
        "[long-horizon] DONE in {:.1}s | episodes={} sentences={} steps={} ({:.0}/s) | bugs={} surprises={} | report={}",
        elapsed,
        final_report.episodes_completed,
        final_report.total_sentences_typed,
        final_report.total_steps_executed,
        final_report.throughput_steps_per_sec,
        final_report.hard_failures.len(),
        final_report.surprises.len(),
        out_dir.join("report.md").display()
    );
}

/// Replays the exact setup, sentence boundaries and actions the host shrinker
/// tests. Keeping this shared prevents the exported native replay from drifting.
fn reproduces_host_failure(
    env: &LockedProcessEnv<'_>,
    setup: &ReproSetup,
    prior_batches: &[Vec<SimStep>],
    steps: &[SimStep],
    code: &str,
) -> bool {
    reset_all_persistent_state();
    let mut sim = HostSimulator::new(
        env.user_data_dir,
        env.dictionaries_dir,
        &setup.settings,
        setup.can_delete_surrounding,
    );
    for batch in prior_batches {
        sim.clear_committed_buffer();
        for step in batch {
            if sim.step(step).is_err() {
                return false;
            }
        }
    }
    sim.clear_committed_buffer();
    for step in steps {
        if let Err(violation) = sim.step(step) {
            return violation.code == code;
        }
    }
    false
}

/// TAIGI_REPRO_PATH=<report-dir>/<id>.repro.json cargo test -p taigi-linux-core
/// --test long_horizon long_horizon_replay -- --ignored --nocapture
#[test]
#[ignore = "native reproducer replay; requires TAIGI_REPRO_PATH and a fresh filtered test process"]
fn long_horizon_replay() {
    let path =
        std::env::var("TAIGI_REPRO_PATH").expect("set TAIGI_REPRO_PATH to a .repro.json file");
    let record: BugReproRecord =
        serde_json::from_slice(&std::fs::read(path).expect("read reproducer"))
            .expect("decode native reproducer");
    let repro = record.reproducer;
    let env = acquire_process_env_with_snapshot(repro.setup.user_data_snapshot.as_ref());
    let reproduced = if let Some(case) = &repro.metamorphic_case {
        check_metamorphic_case(
            env.user_data_dir,
            env.dictionaries_dir,
            &repro.setup.settings,
            case,
        )
        .is_err_and(|failure| failure.oracle == record.code)
    } else {
        reproduces_host_failure(
            &env,
            &repro.setup,
            &repro.priming_batches,
            &repro.minimal_steps,
            &record.code,
        )
    };
    assert!(
        reproduced,
        "native replay did not reproduce {}",
        record.code
    );
    println!("[long-horizon] reproduced {} ({})", record.id, record.code);
}
