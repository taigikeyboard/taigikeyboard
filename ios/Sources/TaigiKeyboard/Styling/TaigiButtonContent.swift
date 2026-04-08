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
    let textProvider: ButtonTextProvider
    let imageProvider: ButtonImageProvider
    let fontProvider: ButtonFontProvider
    let keyTextColor: Color
    let inputMode: InputMode

    /// Input mode label for the space bar
    private var spaceInputModeLabel: String? {
        guard action == .space else { return nil }
        switch inputMode {
        case .poj: return "POJ"
        case .tl: return "TL"
        case .english: return "EN"
        case .tps: return "TPS"
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
                if textProvider.isTPSHint(for: action) {
                    // TPS layout: hint(s) on top, main text below
                    let hints = hint.split(separator: " ").map(String.init)
                    VStack(spacing: -1) {
                        if hints.count >= 2 {
                            // Two callouts: top-left and top-right
                            HStack {
                                Text(hints[0])
                                    .font(.system(size: 12))
                                    .opacity(0.45)
                                Spacer()
                                Text(hints[1])
                                    .font(.system(size: 12))
                                    .opacity(0.45)
                            }
                            .padding(.horizontal, 3)
                        } else {
                            Text(hint)
                                .font(.system(size: 13))
                                .opacity(0.45)
                        }
                        Text(text)
                            .font(.system(size: 19))
                    }
                    .lineLimit(1)
                    .padding(.vertical, 2)
                } else {
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
                }
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
                .font(KeyboardFonts.globalFont(size: 12))
                .foregroundColor(keyTextColor)
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottomTrailing)
                .padding(.trailing, 4)
                .padding(.bottom, 2)
        } else {
            standardContent
        }
    }
}
