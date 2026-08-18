// Which shape the candidate window takes: a row, a column, or (later) a grid.

import Foundation

/// The candidate window layouts a user can choose between — MacishType's
/// horizontal / vertical / expandable trio, of which the expandable grid is
/// still to be ported (`docs/architecture/macos-candidate-window-port.md` PR3).
///
/// `String` raw values so the choice persists through `UserDefaults` and
/// `@AppStorage`; a stored value from a build that has since removed a case
/// reads back as the default.
enum CandidateLayout: String, CaseIterable, Sendable {
    case horizontal
    case vertical
}
