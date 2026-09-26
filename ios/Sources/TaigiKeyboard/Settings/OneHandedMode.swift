import Foundation

/// One-handed keyboard mode: the key area narrows to 80% and docks to one edge.
///
/// Mirrors Android `ime/core/settings/OneHandedMode.kt`; [rawValue] is the persisted
/// string, identical on both platforms.
enum OneHandedMode: String, CaseIterable {
    case off
    case left
    case right

    // CROSS-PLATFORM INVARIANT — mirrors android/app/src/main/java/com/siansiansu/taigikeyboard/ime/core/settings/OneHandedMode.kt
    // KEY_AREA_FRACTION; KeyboardKit docks the key rows at this share. Drift causes silent divergence.
    static let keyAreaFraction: CGFloat = 0.8

    /// The opposite side; `.off` stays `.off`.
    var flipped: OneHandedMode {
        switch self {
        case .off: .off
        case .left: .right
        case .right: .left
        }
    }
}

/// The action behind the toolbar keyboard button — the most recent pick from its
/// long-press callout (choosing Normal there leaves it unchanged).
///
/// Mirrors Android `KeyboardToolbarAction`; [rawValue] is the persisted string.
enum KeyboardToolbarAction: String, CaseIterable {
    case dismiss
    case left
    case right

    /// The one-handed mode this action docks to; `nil` for `.dismiss`.
    var mode: OneHandedMode? {
        switch self {
        case .dismiss: nil
        case .left: .left
        case .right: .right
        }
    }

    /// Mode after a tap on the toolbar button while in [current]; `nil` = dismiss the keyboard.
    /// A side action toggles between that side and `.off`.
    func tapResult(current: OneHandedMode) -> OneHandedMode? {
        guard let mode else { return nil }
        return current == mode ? .off : mode
    }

    /// The action that re-applies [mode]; `nil` for `.off` (Normal leaves the action unchanged).
    init?(mode: OneHandedMode) {
        switch mode {
        case .off: return nil
        case .left: self = .left
        case .right: self = .right
        }
    }
}
