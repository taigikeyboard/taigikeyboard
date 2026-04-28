//! `process_candidates` op orchestrator — engine-side implementation of
//! the full ranking pipeline.
//!
//! Pipeline (mirrors `LexiconService.search` / `composeRanked` on both
//! platforms):
//!
//! 1. `dedup::remove_duplicates` on the merged custom + system raw list.
//! 2. Build the user-frequency map from `FrequencyEntry` repeated field.
//! 3. `sort::sort_by_score` — score + descending sort.
//! 4. If `tps_dedup_enabled`, run `dedup::remove_display_duplicates` on
//!    the sorted list (TPS visual dedup must run AFTER sort so the
//!    highest-ranked entry per hanji survives).
//! 5. Optionally include per-candidate ScoreBreakdown if
//!    `include_breakdown` was set (paired with `ranked` by index).
//!
//! Engine never logs the breakdown — that's a platform concern (iOS uses
//! `LoggerFactory`, Android uses `LoggerBackend`); engine returns the
//! data and lets the platform format / emit.

use std::collections::HashMap;

use protos::engine::{
    FrequencyEntry, ProcessCandidatesRequest, ProcessCandidatesResponse, ScoreBreakdown,
};

use crate::dedup;
use crate::score::FrequencyData;
use crate::sort::{self, FrequencyMap};

/// Run the merged-candidate ranking pipeline against `req` and return the
/// proto response. Stateless and panic-free for well-formed input.
pub fn process_candidates(req: ProcessCandidatesRequest) -> ProcessCandidatesResponse {
    let freq_map = build_frequency_map(&req.freq);

    // Phase 1 — engine dedup (key = "<roman>|<hanji>").
    let merged_unique = dedup::remove_duplicates(req.raw);

    // Phase 2 — score + descending sort. `include_breakdown` is plumbed
    // straight into `sort_by_score` so the release path skips both the
    // per-candidate `ScoreBreakdown` allocation AND the parallel
    // `Vec<ScoreBreakdown>` build-up. When `false`, the returned
    // `breakdowns` is `Vec::new()`.
    let (sorted_words, sorted_breakdowns) = sort::sort_by_score(
        merged_unique,
        &req.normalized_input,
        &freq_map,
        req.now_ms,
        req.include_breakdown,
    );

    // Phase 3 — TPS-gated display dedup. iOS / Android only call this
    // when input mode is TPS; the gate decision stays platform-side and
    // arrives as `tps_dedup_enabled`.
    let (ranked, breakdown) = if req.tps_dedup_enabled {
        post_sort_display_dedup(sorted_words, sorted_breakdowns)
    } else {
        (sorted_words, sorted_breakdowns)
    };

    ProcessCandidatesResponse { ranked, breakdown }
}

/// Build the per-display-text frequency lookup. The proto's
/// `FrequencyEntry.display_text_key` matches `TaigiWord.displayText`
/// (`hanji` if non-empty else `roman`); that pairing lives at the
/// platform → engine boundary.
fn build_frequency_map(entries: &[FrequencyEntry]) -> FrequencyMap {
    let mut map = HashMap::with_capacity(entries.len());
    for entry in entries {
        map.insert(
            entry.display_text_key.clone(),
            FrequencyData {
                count: entry.count as i32,
                last_used_ms: entry.last_used_ms,
            },
        );
    }
    map
}

/// Apply display-dedup to the sorted word list, keeping any parallel
/// breakdown vector aligned.
///
/// `sorted_breakdowns` may be empty (release path with
/// `include_breakdown=false`); when populated it must be 1:1 with
/// `sorted_words`. The empty path delegates to
/// [`dedup::remove_display_duplicates`] so the canonical dedup logic
/// stays in one place. The populated path inlines a single-pass dedup
/// to drop the breakdown at the same index as its dropped word.
fn post_sort_display_dedup(
    sorted_words: Vec<protos::engine::TaigiWord>,
    sorted_breakdowns: Vec<ScoreBreakdown>,
) -> (Vec<protos::engine::TaigiWord>, Vec<ScoreBreakdown>) {
    if sorted_breakdowns.is_empty() {
        return (
            dedup::remove_display_duplicates(sorted_words),
            Vec::new(),
        );
    }

    debug_assert_eq!(
        sorted_breakdowns.len(),
        sorted_words.len(),
        "breakdowns must be 1:1 with sorted_words when populated"
    );

    let mut seen = std::collections::HashSet::<String>::with_capacity(sorted_words.len());
    let mut kept_words = Vec::with_capacity(sorted_words.len());
    let mut kept_breakdowns = Vec::with_capacity(sorted_words.len());

    for (word, breakdown) in sorted_words.into_iter().zip(sorted_breakdowns) {
        let keep = match word.hanji.as_deref() {
            None | Some("") => true,
            Some(hanji) => seen.insert(hanji.to_owned()),
        };
        if keep {
            kept_words.push(word);
            kept_breakdowns.push(breakdown);
        }
    }

    (kept_words, kept_breakdowns)
}

#[cfg(test)]
mod tests {
    use super::*;
    use protos::engine::TaigiWord;

    fn word(id: i64, roman: &str, hanji: Option<&str>, length_score: Option<i32>) -> TaigiWord {
        TaigiWord {
            id,
            roman: roman.to_owned(),
            hanji: hanji.map(str::to_owned),
            length_score,
            source_bitmask: None,
        }
    }

    #[test]
    fn pipeline_dedups_then_ranks_then_returns_breakdown_when_requested() {
        let req = ProcessCandidatesRequest {
            raw: vec![
                word(1, "gua", Some("我"), Some(50)),
                word(2, "gua", Some("我"), Some(50)), // duplicate of #1
                word(3, "gua", Some("瓜"), Some(50)),
            ],
            normalized_input: "gua".to_owned(),
            tps_dedup_enabled: false,
            freq: vec![FrequencyEntry {
                display_text_key: "我".to_owned(),
                count: 10,
                last_used_ms: 0,
            }],
            now_ms: 1_000_000_000,
            include_breakdown: true,
        };
        let resp = process_candidates(req);
        assert_eq!(resp.ranked.len(), 2, "duplicate dropped");
        assert_eq!(resp.ranked[0].id, 1, "user-freq leader");
        assert_eq!(resp.breakdown.len(), 2);
    }

    #[test]
    fn pipeline_omits_breakdown_when_not_requested() {
        let req = ProcessCandidatesRequest {
            raw: vec![word(1, "gua", Some("我"), Some(50))],
            normalized_input: "gua".to_owned(),
            tps_dedup_enabled: false,
            freq: Vec::new(),
            now_ms: 0,
            include_breakdown: false,
        };
        let resp = process_candidates(req);
        assert_eq!(resp.ranked.len(), 1);
        assert!(resp.breakdown.is_empty());
    }

    #[test]
    fn pipeline_runs_display_dedup_when_tps_enabled() {
        // Same hanji, different roman — TPS mode collapses to highest-ranked.
        let req = ProcessCandidatesRequest {
            raw: vec![
                word(1, "phuānn-tshiú", Some("伴手"), Some(100)),
                word(2, "phuǎnn-tshiú", Some("伴手"), Some(10)),
            ],
            normalized_input: "phuanntshiu".to_owned(),
            tps_dedup_enabled: true,
            freq: Vec::new(),
            now_ms: 0,
            include_breakdown: false,
        };
        let resp = process_candidates(req);
        assert_eq!(resp.ranked.len(), 1, "display dedup collapsed entries");
        assert_eq!(resp.ranked[0].id, 1, "highest-ranked survives");
    }

    #[test]
    fn pipeline_skips_display_dedup_when_tps_disabled() {
        let req = ProcessCandidatesRequest {
            raw: vec![
                word(1, "phuānn-tshiú", Some("伴手"), Some(100)),
                word(2, "phuǎnn-tshiú", Some("伴手"), Some(10)),
            ],
            normalized_input: "phuanntshiu".to_owned(),
            tps_dedup_enabled: false,
            freq: Vec::new(),
            now_ms: 0,
            include_breakdown: false,
        };
        let resp = process_candidates(req);
        assert_eq!(resp.ranked.len(), 2, "display dedup skipped in non-TPS mode");
    }

    #[test]
    fn pipeline_keeps_breakdown_aligned_with_ranked_after_display_dedup() {
        // Equal-length romans so closeness_bonus is identical across all
        // three candidates (= 500), making length_score the deciding
        // signal: 100 > 50 > 10 → b leads, c next, a last.
        let req = ProcessCandidatesRequest {
            raw: vec![
                word(1, "a", Some("同"), Some(10)),
                word(2, "b", Some("同"), Some(100)),
                word(3, "c", Some("獨"), Some(50)),
            ],
            normalized_input: "x".to_owned(),
            tps_dedup_enabled: true,
            freq: Vec::new(),
            now_ms: 0,
            include_breakdown: true,
        };
        let resp = process_candidates(req);
        assert_eq!(resp.ranked.len(), 2);
        assert_eq!(resp.breakdown.len(), 2);
        // b wins on length_score, c kept (different hanji), a dropped
        // because b already claimed "同".
        assert_eq!(resp.ranked[0].id, 2);
        assert_eq!(resp.ranked[1].id, 3);
    }

    #[test]
    fn pipeline_handles_empty_raw_list() {
        let req = ProcessCandidatesRequest {
            raw: Vec::new(),
            normalized_input: "x".to_owned(),
            tps_dedup_enabled: false,
            freq: Vec::new(),
            now_ms: 0,
            include_breakdown: true,
        };
        let resp = process_candidates(req);
        assert!(resp.ranked.is_empty());
        assert!(resp.breakdown.is_empty());
    }
}
