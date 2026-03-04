import KeyboardKit
import SwiftUI

/// Custom button content view
///
/// Integrates ButtonImageProvider, ButtonTextProvider, and ButtonFontProvider
/// to determine display content with priority: image > text > standard content.
///
/// - Note: `keyboardContext` must use `@ObservedObject` to respond to state changes like `isComposingText`
struct TaigiButtonContent<StandardContent: View>: View {
    let action: KeyboardAction
    @ObservedObject var keyboardContext: KeyboardContext
    let standardContent: StandardContent
    let keyFontSizeScale: CGFloat

    private var textProvider: ButtonTextProvider {
        ButtonTextProvider(keyboardContext: keyboardContext)
    }

    private var imageProvider: ButtonImageProvider {
        ButtonImageProvider(keyboardContext: keyboardContext)
    }

    private var fontProvider: ButtonFontProvider {
        ButtonFontProvider(keyboardContext: keyboardContext)
    }

    /// Resolved key text color: custom color from settings, or system default
    private var keyTextColor: Color {
        SharedSettings.shared.colorSettings.keyTextColor?.color ?? Color(.label)
    }

    /// Input mode label for the space bar (romanization layouts only)
    private var spaceInputModeLabel: String? {
        guard action == .space,
              SharedSettings.shared.keyboardLayoutType != .tps else { return nil }
        switch SharedSettings.shared.inputMode {
        case .poj: return "POJ"
        case .tl: return "TL"
        case .english: return "EN"
        }
    }

    var body: some View {
        // Priority 1: Custom image
        if let image = imageProvider.buttonImage(for: action) {
            image
        }
        // Priority 2: Custom text with custom font
        else if let text = textProvider.buttonText(for: action) {
            if let hint = textProvider.buttonHintText(for: action) {
                VStack(spacing: -4) {
                    Text(hint)
                        .font(.system(size: textProvider.isTextHint(for: action) ? 10 : 20))
                        .foregroundColor(keyTextColor.opacity(0.5))
                        .frame(height: 10)
                        .offset(y: textProvider.isTextHint(for: action) ? 2 : textProvider.hintOffsetY(for: action))
                    Text(text)
                        .font(fontProvider.buttonKeyboardFont(for: action).font)
                }
                .lineLimit(1)
            } else {
                Text(text)
                    .font(fontProvider.buttonKeyboardFont(for: action).font)
                    .lineLimit(1)
                    .minimumScaleFactor(0.7)
            }
        }
        // Fallback: Standard content (with optional space bar input mode hint)
        // Render mode label alone without standardContent to avoid
        // KeyboardKit's "space" text overlapping on iPad.
        else if let modeLabel = spaceInputModeLabel {
            Text(modeLabel)
                .font(KeyboardModels.Fonts.globalFont(size: 12))
                .foregroundColor(keyTextColor)
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottomTrailing)
                .padding(.trailing, 4)
                .padding(.bottom, 2)
        }
        else {
            standardContent
        }
    }
}
