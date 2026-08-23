// How a candidate cell arranges its two scripts.

import Foundation

/// Where a cell puts the candidate's second script.
///
/// The vertical layout reads its rows as a column of pairs, so it keeps the
/// two scripts on one line and aligns every annotation on the same x. The
/// horizontal and expandable layouts read as a row of candidates, where a cell
/// as wide as both scripts together fits far fewer of them — those stack.
enum CandidateCellArrangement: Sendable {
    /// Annotation beside the candidate, sharing its baseline.
    case inline
    /// Annotation under the candidate, both centred.
    case stacked
}
