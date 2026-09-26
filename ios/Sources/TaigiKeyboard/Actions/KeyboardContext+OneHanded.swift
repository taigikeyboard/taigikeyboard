import KeyboardKit

extension OneHandedMode {
    /// KeyboardKit dock edge that narrows the key rows to 80%; `nil` keeps full width.
    var dockEdge: Keyboard.DockEdge? {
        switch self {
        case .off: nil
        case .left: .leading
        case .right: .trailing
        }
    }
}

/// `KeyboardContext` extension for one-handed mode, backed by `SharedSettings`. Writers
/// re-render through `notifyDisplayChange()`, the same path the 文/A toggle uses.
/// Mirrors Android `ToolbarManager` one-handed handlers.
extension KeyboardContext {
    private var oneHandedMode: OneHandedMode {
        SharedSettings.shared.oneHandedMode
    }

    var keyboardToolbarAction: KeyboardToolbarAction {
        SharedSettings.shared.keyboardToolbarAction
    }

    /// Callout / side-panel pick of a mode. A side also becomes the toolbar button's action;
    /// `.off` leaves the action unchanged.
    /// Writes only changed values: every App Group write also fires `didChangeNotification` → `syncSettings()`.
    func selectOneHandedMode(_ mode: OneHandedMode) {
        if oneHandedMode != mode {
            SharedSettings.shared.oneHandedMode = mode
        }
        if let action = KeyboardToolbarAction(mode: mode), keyboardToolbarAction != action {
            SharedSettings.shared.keyboardToolbarAction = action
        }
        notifyDisplayChange()
    }

    /// Callout pick of Dismiss: it becomes the toolbar button's action. The caller dismisses.
    func selectDismissToolbarAction() {
        guard keyboardToolbarAction != .dismiss else { return }
        SharedSettings.shared.keyboardToolbarAction = .dismiss
        notifyDisplayChange()
    }

    /// Toolbar button tap. Returns `true` when the caller should dismiss the keyboard.
    func performKeyboardToolbarAction() -> Bool {
        guard let mode = keyboardToolbarAction.tapResult(current: oneHandedMode) else { return true }
        SharedSettings.shared.oneHandedMode = mode
        notifyDisplayChange()
        return false
    }
}
