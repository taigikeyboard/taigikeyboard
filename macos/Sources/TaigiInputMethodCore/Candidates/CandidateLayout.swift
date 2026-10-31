// Which shape the candidate window takes: a row, a column, or an unfolding grid.

import Foundation

/// The candidate window layouts a user can choose between — MacishType's
/// horizontal / vertical / expandable trio, plus a vertical expandable that
/// unfolds a column into the same grid (USER 2026-10-31). Twin of
/// `taigi-desktop-core` `settings/choices.rs` `CandidateLayout`.
///
/// `String` raw values so the choice persists through `UserDefaults` and
/// `@AppStorage`; a stored value from a build that has since removed a case
/// reads back as the default.
enum CandidateLayout: String, CaseIterable, Sendable {
    case horizontal
    case vertical
    case expandable
    case verticalExpandable

    /// How this layout's cells hold their two scripts. A row of candidates
    /// fits far fewer of them when every cell is as wide as both scripts side
    /// by side, so the row-shaped layouts stack; the column-shaped ones keep
    /// them inline, where their rows align every annotation on one x.
    var cellArrangement: CandidateCellArrangement {
        switch self {
        case .horizontal, .expandable: .stacked
        case .vertical, .verticalExpandable: .inline
        }
    }
}
