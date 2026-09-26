import Foundation

/// Value-type container for the mutually-dismissable overlay panels
/// mounted on `TaigiKeyboardView` (layout / symbol / settings / one-handed callout).
///
/// `CandidateExpandState` is tracked separately as a reference type because
/// it is shared with child views via `@EnvironmentObject`.
struct OverlayPanelState: Equatable {
    var isLayoutExpanded = false
    var isSymbolExpanded = false
    var isSettingsExpanded = false
    var isOneHandedMenuExpanded = false

    // Closes all at once; typically called on keyboard-type switch or an outside tap.
    mutating func closeAll() {
        isLayoutExpanded = false
        isSymbolExpanded = false
        isSettingsExpanded = false
        isOneHandedMenuExpanded = false
    }
}
