//! Structured JSON + Markdown report writer and latency percentile tracker for
//! long-horizon simulation runs.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::oracles::{ReachabilityStats, SurpriseFinding};
use super::shrinker::ShrunkReproducer;
use super::UserDataCounts;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LatencySummary {
    pub samples: usize,
    pub p50_us: u64,
    pub p95_us: u64,
    pub p99_us: u64,
    pub max_us: u64,
}

impl LatencySummary {
    pub fn from_samples(raw: &[u64]) -> Self {
        if raw.is_empty() {
            return Self::default();
        }
        let mut sorted = raw.to_vec();
        sorted.sort_unstable();
        let n = sorted.len();
        let pct = |num: usize, den: usize| sorted[((n - 1) * num) / den];
        Self {
            samples: n,
            p50_us: pct(50, 100),
            p95_us: pct(95, 100),
            p99_us: pct(99, 100),
            max_us: sorted[n - 1],
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PersonaStats {
    pub episodes: u64,
    pub sentences: u64,
    pub steps: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BugReproRecord {
    pub id: String,
    pub code: String,
    pub persona: String,
    pub episode_seed: u64,
    pub sentence_index: usize,
    pub sentence_tailo: String,
    pub message: String,
    pub reproducer: ShrunkReproducer,
}

#[derive(Clone, Debug, Serialize)]
pub struct MarathonReport {
    pub seed: u64,
    pub elapsed_secs: f64,
    pub target_secs: u64,
    pub corpus_source: String,
    pub corpus_sentences_available: usize,
    pub episodes_completed: u64,
    pub total_sentences_typed: u64,
    pub total_steps_executed: u64,
    pub throughput_steps_per_sec: f64,
    pub max_user_data_counts: UserDataCounts,
    pub early_episode_latency: LatencySummary,
    pub late_episode_latency: LatencySummary,
    pub per_persona: BTreeMap<String, PersonaStats>,
    pub reachability: ReachabilityStats,
    pub hard_failures: Vec<BugReproRecord>,
    pub surprises: Vec<SurpriseFinding>,
}

impl MarathonReport {
    pub fn write_to_dir(&self, out_dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(out_dir)?;
        let json_bytes = serde_json::to_vec_pretty(self).expect("serialize report json");
        std::fs::write(out_dir.join("report.json"), json_bytes)?;
        std::fs::write(out_dir.join("report.md"), self.to_markdown())?;

        for bug in &self.hard_failures {
            let scen_bytes =
                serde_json::to_vec_pretty(&bug.reproducer.e2e_scenario).expect("scenario json");
            std::fs::write(
                out_dir.join(format!("{}.scenario.json", bug.id)),
                scen_bytes,
            )?;
        }
        Ok(())
    }

    pub fn to_markdown(&self) -> String {
        let mut md = String::with_capacity(8192);
        let _ = writeln!(md, "# Long-Horizon User-Interaction Marathon Report\n");
        let _ = writeln!(md, "## Run Overview\n");
        let _ = writeln!(md, "| Metric | Value |");
        let _ = writeln!(md, "|---|---|");
        let _ = writeln!(md, "| Master seed | `0x{:016X}` |", self.seed);
        let _ = writeln!(
            md,
            "| Duration | {:.1} s (target {} s) |",
            self.elapsed_secs, self.target_secs
        );
        let _ = writeln!(
            md,
            "| Corpus | `{}` ({} sentences) |",
            self.corpus_source, self.corpus_sentences_available
        );
        let _ = writeln!(md, "| Episodes completed | {} |", self.episodes_completed);
        let _ = writeln!(md, "| Sentences typed | {} |", self.total_sentences_typed);
        let _ = writeln!(
            md,
            "| User-interaction steps | {} ({:.0} steps/s) |",
            self.total_steps_executed, self.throughput_steps_per_sec
        );
        let _ = writeln!(
            md,
            "| Metamorphic checks | {} |",
            self.reachability.metamorphic_checks
        );
        let _ = writeln!(
            md,
            "| Learning monotonicity checks | {} |",
            self.reachability.monotonicity_checks
        );
        let _ = writeln!(
            md,
            "| Peak SQLite rows (episode) | freq={}, assoc={}, phrases={}, custom={} |",
            self.max_user_data_counts.frequency_rows,
            self.max_user_data_counts.association_rows,
            self.max_user_data_counts.learned_phrase_rows,
            self.max_user_data_counts.custom_rows
        );
        let _ = writeln!(
            md,
            "| Hard invariant / metamorphic failures | **{}** |",
            self.hard_failures.len()
        );
        let _ = writeln!(
            md,
            "| Unique behavioral surprises | **{}** |\n",
            self.surprises.len()
        );

        let _ = writeln!(md, "## Latency Across Horizon (Early vs. Late Episode)\n");
        let _ = writeln!(
            md,
            "| Horizon Window | Samples | p50 (µs) | p95 (µs) | p99 (µs) | max (µs) |"
        );
        let _ = writeln!(md, "|---|---:|---:|---:|---:|---:|");
        let _ = writeln!(
            md,
            "| Early (sentences 1–30, cold user DB) | {} | {} | {} | {} | {} |",
            self.early_episode_latency.samples,
            self.early_episode_latency.p50_us,
            self.early_episode_latency.p95_us,
            self.early_episode_latency.p99_us,
            self.early_episode_latency.max_us
        );
        let _ = writeln!(
            md,
            "| Late (sentences 150+, populated user DB) | {} | {} | {} | {} | {} |\n",
            self.late_episode_latency.samples,
            self.late_episode_latency.p50_us,
            self.late_episode_latency.p95_us,
            self.late_episode_latency.p99_us,
            self.late_episode_latency.max_us
        );

        let _ = writeln!(md, "## Persona Coverage\n");
        let _ = writeln!(md, "| Persona | Episodes | Sentences | Steps |");
        let _ = writeln!(md, "|---|---:|---:|---:|");
        for (name, p) in &self.per_persona {
            let _ = writeln!(
                md,
                "| `{name}` | {} | {} | {} |",
                p.episodes, p.sentences, p.steps
            );
        }
        let _ = writeln!(md);

        if self.reachability.words_checked > 0 {
            let n = self.reachability.words_checked as f64;
            let pct = |v: u64| (v as f64 * 100.0) / n;
            let _ = writeln!(
                md,
                "## Corpus Lexicon Word Reachability ({} word queries)\n",
                self.reachability.words_checked
            );
            let _ = writeln!(
                md,
                "| Input Form | Rank #1 | Page 1 (Top 9) | Reachable (Any Page) | Missing |"
            );
            let _ = writeln!(md, "|---|---:|---:|---:|---:|");
            let _ = writeln!(
                md,
                "| Numbered TL (`digit_tl`) | {} ({:.1}%) | {} ({:.1}%) | {} ({:.1}%) | {} ({:.1}%) |",
                self.reachability.digit_tl_rank1,
                pct(self.reachability.digit_tl_rank1),
                self.reachability.digit_tl_top9,
                pct(self.reachability.digit_tl_top9),
                self.reachability.digit_tl_any,
                pct(self.reachability.digit_tl_any),
                self.reachability.digit_tl_missing,
                pct(self.reachability.digit_tl_missing),
            );
            let _ = writeln!(
                md,
                "| Continuous Toneless TL | {} ({:.1}%) | {} ({:.1}%) | {} ({:.1}%) | {} ({:.1}%) |\n",
                self.reachability.toneless_rank1,
                pct(self.reachability.toneless_rank1),
                self.reachability.toneless_top9,
                pct(self.reachability.toneless_top9),
                self.reachability.toneless_any,
                pct(self.reachability.toneless_any),
                self.reachability.toneless_missing,
                pct(self.reachability.toneless_missing),
            );
        }

        let _ = writeln!(
            md,
            "## Hard Invariant & Metamorphic Failures ({})\n",
            self.hard_failures.len()
        );
        if self.hard_failures.is_empty() {
            let _ = writeln!(
                md,
                "None — all per-keystroke invariants and metamorphic equivalences held.\n"
            );
        } else {
            for bug in &self.hard_failures {
                let _ = writeln!(md, "### `{}` — `{}` ({})", bug.id, bug.code, bug.persona);
                let _ = writeln!(md, "- **Episode seed**: `0x{:016X}`", bug.episode_seed);
                let _ = writeln!(
                    md,
                    "- **Sentence #{}**: `{}`",
                    bug.sentence_index, bug.sentence_tailo
                );
                let _ = writeln!(md, "- **Failure**: {}", bug.message);
                let _ = writeln!(
                    md,
                    "- **Shrunk**: {} → {} prior sentences, {} → {} steps",
                    bug.reproducer.original_prior_sentences,
                    bug.reproducer.shrunk_prior_sentences,
                    bug.reproducer.original_steps,
                    bug.reproducer.shrunk_steps
                );
                let _ = writeln!(
                    md,
                    "- **Minimal steps**: `{:?}`\n",
                    bug.reproducer.minimal_steps
                );
            }
        }

        let _ = writeln!(
            md,
            "## Behavioral Surprises & Edge Cases ({})\n",
            self.surprises.len()
        );
        if self.surprises.is_empty() {
            let _ = writeln!(md, "No soft anomalies recorded.\n");
        } else {
            let mut grouped: BTreeMap<&str, Vec<&SurpriseFinding>> = BTreeMap::new();
            for s in &self.surprises {
                grouped.entry(s.kind).or_default().push(s);
            }
            for (kind, items) in grouped {
                let _ = writeln!(md, "### `{kind}` ({} unique)\n", items.len());
                for item in items.iter().take(25) {
                    let _ = writeln!(
                        md,
                        "- **[#{}]** `{}` / `{}` — {}",
                        item.sentence_index, item.sentence_hanji, item.sentence_tailo, item.detail
                    );
                }
                if items.len() > 25 {
                    let _ = writeln!(md, "- *(+ {} more in `report.json`)*", items.len() - 25);
                }
                let _ = writeln!(md);
            }
        }

        md
    }
}
