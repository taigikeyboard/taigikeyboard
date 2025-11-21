import KeyboardKit
import SwiftUI

/// Provider for button layout and style customization
class ButtonLayoutProvider {

    private let keyboardContext: KeyboardContext
    private let standardService: KeyboardStyle.StandardStyleService

    init(keyboardContext: KeyboardContext) {
        self.keyboardContext = keyboardContext
        self.standardService = KeyboardStyle.StandardStyleService(keyboardContext: keyboardContext)
    }

    /// Get button content insets - use KeyboardKit default
    func buttonContentInsets(for action: KeyboardAction) -> EdgeInsets {
        return standardService.buttonContentInsets(for: action)
    }

    /// Get button style - use KeyboardKit default
    func buttonStyle(for action: KeyboardAction, isPressed: Bool) -> Keyboard.ButtonStyle {
        return standardService.buttonStyle(for: action, isPressed: isPressed)
    }
}
