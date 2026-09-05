import Foundation

/// Value-type container for the three mutually-dismissable overlay panels
/// mounted on `TaigiKeyboardView` (layout / symbol / settings).
///
/// `CandidateExpandState` is tracked separately as a reference type because
/// it is shared with child views via `@EnvironmentObject`.
struct OverlayPanelState: Equatable {
    var isLayoutExpanded = false
    var isSymbolExpanded = false
    var isSettingsExpanded = false

    // Closes all three at once; typically called on keyboard-type switch or an outside tap.
    mutating func closeAll() {
        isLayoutExpanded = false
        isSymbolExpanded = false
        isSettingsExpanded = false
    }
}
