import KeyboardKit
import Foundation

/// Provider for button text customization
class ButtonTextProvider {

    private let keyboardContext: KeyboardContext

    init(keyboardContext: KeyboardContext) {
        self.keyboardContext = keyboardContext
    }

    /// Get custom button text for the given action
    /// - Parameter action: The keyboard action
    /// - Returns: Custom text if applicable, nil to use default
    func buttonText(for action: KeyboardAction) -> String? {
        switch action {
        case let .character(char):
            let currentCase = keyboardContext.keyboardCase
            switch currentCase {
            case .uppercased, .capsLocked:
                return char.uppercased()
            case .lowercased, .auto:
                return char.lowercased()
            }
        case .keyboardType(.numeric):
            return "123"
        case .keyboardType(.alphabetic):
            return "ABC"
        case .keyboardType(.symbolic):
            return "#+="
        case .keyboardType(.emojis):
             return "😀"
        case .space:
            return nil
        case .primary(.return):
            if keyboardContext.isComposingText {
                let text = ConfirmKeyTextHelper.getConfirmKeyText()
                return text
            }
            return "↵"
        case .backspace:
            return "⌫"
        case .shift:
            return "⇧"
        case .capsLock:
            return "⇪"
        case .primary(.done):
            return "↵"
        case .primary(.go):
            return "↵"
        case .primary(.search):
            return "↵"
        case .primary(.send):
            return "↵"
        case .primary(.next):
            return "↵"
        case .primary(.continue):
            return "↵"
        case .primary:
            // Fallback for any other primary key types
            return "↵"
        case .settings:
            return nil
        case let .custom(name):
            switch name {
            case "translate":
                return nil
            default:
                return name
            }
        default:
            return nil
        }
    }
}
