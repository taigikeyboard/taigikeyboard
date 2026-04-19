import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// Engine-side NextWord prediction row at the service boundary.
///
/// Replaces the previous platform-bound `NextWordService.Prediction` so
/// pure engine code (`NextWordEngine`) can consume prediction results
/// without depending on the service type.
///
/// **Design divergence from `nextword-engine-boundary.md` §2.4**: the
/// original sketch proposed `count: Int` + `source: Source (.user | .dict)`
/// so shared-core ports could re-run `NextWordScorer` themselves. Current
/// scoring happens platform-side inside `NextWordService` and already
/// *merges* dict + user entries with summed scores — splitting that back
/// into `count`/`source` would be lossy (a merged row has no single
/// source). Phase I keeps `score: Double` as an opaque ordering weight;
/// elevating scoring into shared-core is a Phase IV-B follow-up tracked
/// on the shared-core roadmap.
struct RawNextWordPrediction: Equatable {
    let hanzi: String
    let tl: String
    let score: Double
}
