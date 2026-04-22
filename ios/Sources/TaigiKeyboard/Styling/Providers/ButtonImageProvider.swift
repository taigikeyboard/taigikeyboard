import KeyboardKit
import SwiftUI

/// Button image provider — first in the render chain.
///
/// Returns the SF Symbol image for a key. Returns nil to defer to `ButtonTextProvider`.
/// Only handles keys with icon-based rendering: globe, return, settings, translate toggle.
///
/// Created by: `TaigiKeyboardView.RenderProviders`
/// Queried by: `TaigiButtonContent.body` (first priority check)
/// Depends on: `KeyboardContext` (composing state, translate toggle state)
final class ButtonImageProvider {
    private let keyboardContext: KeyboardContext

    init(keyboardContext: KeyboardContext) {
        self.keyboardContext = keyboardContext
    }

    /// Returns an SF Symbol image, or nil to defer to text rendering.
    func buttonImage(for action: KeyboardAction) -> Image? {
        switch action {
        case .nextKeyboard:
            return Image(latinSystemName: "globe")
        case .primary(.return):
            // Show newline icon when not composing; nil lets ButtonTextProvider show confirmation text
            return keyboardContext.isComposingText ? nil : Image(latinSystemName: "arrow.turn.down.left")
        case .settings:
            return Image(latinSystemName: "gearshape.fill")
        case let .custom(name):
            switch name {
            case "translate":
                let iconName = keyboardContext.isTranslateSwapped
                    ? "character.square.fill" // Active: filled
                    : "character.square" // Default: outlined
                return Image(latinSystemName: iconName)
            default:
                return nil
            }
        default:
            return nil
        }
    }
}
