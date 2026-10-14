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
            // Composing: nil lets ButtonTextProvider show the confirmation text
            guard !keyboardContext.isComposingText else { return nil }
            let returnKeyType = keyboardContext.textDocumentProxy.returnKeyType
            return Image(latinSystemName: Self.returnKeySymbolName(for: returnKeyType))
        case .settings:
            return Image(latinSystemName: "gearshape.fill")
        case let .custom(name):
            switch name {
            case "translate":
                // Lit when the key has been taken OFF the shipped state — the
                // layouts type half-width marks and a commit leads with the
                // romanization (USER 2026-09-18, after hanji-first became the
                // default: a key that lights up on a fresh install reads as a
                // mode the user never chose). Outlined = hanji + full-width,
                // what the key flips in every mode it is shown in.
                let iconName = keyboardContext.isFullWidthPunctuation
                    ? "character.square" // Default: outlined
                    : "character.square.fill" // Off the default: filled
                return Image(latinSystemName: iconName)
            default:
                return nil
            }
        default:
            return nil
        }
    }

    /// SF Symbol for the idle Return key, by the host field's return key type.
    // CROSS-PLATFORM INVARIANT — aligns the return-key icon groups with Android
    // `KeyContent.kt` `resolveEnterVisual` (done / arrow / search / send / newline).
    static func returnKeySymbolName(for returnKeyType: UIReturnKeyType?) -> String {
        switch returnKeyType {
        case .done:
            "checkmark"
        case .go, .next, .join, .route, .continue, .emergencyCall:
            "arrow.right"
        case .search, .google, .yahoo:
            "magnifyingglass"
        case .send:
            "paperplane"
        case nil, .default:
            "arrow.turn.down.left"
        @unknown default:
            "arrow.turn.down.left"
        }
    }
}
