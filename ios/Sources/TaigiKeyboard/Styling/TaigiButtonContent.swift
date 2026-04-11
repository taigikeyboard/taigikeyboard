import KeyboardKit
import SwiftUI

// MARK: - Layout Constants

/// Layout constants for `TaigiButtonContent` sub-views.
/// File-scoped because Swift does not allow static stored properties in nested types of generic structs.
private enum ButtonContentLayout {
    // TPS key layout
    static let tpsHintFontSize: CGFloat = 12
    static let tpsSingleHintFontSize: CGFloat = 13
    static let tpsMainFontSize: CGFloat = 19
    static let tpsHintOpacity: Double = 0.45
    static let tpsVStackSpacing: CGFloat = -1
    static let tpsHintHorizontalPadding: CGFloat = 3
    static let tpsVerticalPadding: CGFloat = 2

    // Standard hint layout (tone diacritics above key label)
    static let textHintFontSize: CGFloat = 10
    static let diacriticHintFontSize: CGFloat = 20
    static let hintOpacity: Double = 0.5
    static let hintVStackSpacing: CGFloat = -4
    static let hintFrameHeight: CGFloat = 10
    static let textHintOffsetY: CGFloat = 2

    /// Text-only key
    static let minimumScaleFactor: CGFloat = 0.7

    // Space bar mode label
    static let modeLabelFontSize: CGFloat = 12
    static let modeLabelTrailingPadding: CGFloat = 4
    static let modeLabelBottomPadding: CGFloat = 2
}

/// Custom button content view for each keyboard key.
///
/// Integrates ButtonImageProvider, ButtonTextProvider, and ButtonFontProvider
/// to determine display content with priority: image > text > standard content.
/// When a provider returns nil, the next provider in the chain is tried.
///
/// Created by: `TaigiKeyboardView.coreKeyboard` (KeyboardView buttonContent closure)
/// Depends on: `ButtonImageProvider`, `ButtonTextProvider`, `ButtonFontProvider`, `KeyboardFonts`
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

    // MARK: - Body

    /// Render priority: image (ButtonImageProvider) > text (ButtonTextProvider) > standard (KeyboardKit default).
    /// Each provider returns nil to defer to the next in chain.
    var body: some View {
        if let image = imageProvider.buttonImage(for: action) {
            image
        } else if let text = textProvider.buttonText(for: action) {
            if let hint = textProvider.buttonHintText(for: action) {
                if textProvider.isTPSHint(for: action) {
                    tpsHintContent(text: text, hint: hint)
                } else {
                    standardHintContent(text: text, hint: hint)
                }
            } else {
                textOnlyContent(text: text)
            }
        } else if let modeLabel = spaceInputModeLabel {
            spaceModeLabelContent(label: modeLabel)
        } else {
            standardContent
        }
    }

    // MARK: - Sub-views

    /// TPS layout: callout hint(s) on top, main text below.
    /// Two hints are spread left/right; a single hint is centered.
    private func tpsHintContent(text: String, hint: String) -> some View {
        let hints = hint.split(separator: " ").map(String.init)
        return VStack(spacing: ButtonContentLayout.tpsVStackSpacing) {
            if hints.count >= 2 {
                HStack {
                    Text(hints[0])
                        .font(.system(size: ButtonContentLayout.tpsHintFontSize))
                        .opacity(ButtonContentLayout.tpsHintOpacity)
                    Spacer()
                    Text(hints[1])
                        .font(.system(size: ButtonContentLayout.tpsHintFontSize))
                        .opacity(ButtonContentLayout.tpsHintOpacity)
                }
                .padding(.horizontal, ButtonContentLayout.tpsHintHorizontalPadding)
            } else {
                Text(hint)
                    .font(.system(size: ButtonContentLayout.tpsSingleHintFontSize))
                    .opacity(ButtonContentLayout.tpsHintOpacity)
            }
            Text(text)
                .font(.system(size: ButtonContentLayout.tpsMainFontSize))
        }
        .lineLimit(1)
        .padding(.vertical, ButtonContentLayout.tpsVerticalPadding)
    }

    /// Standard hint layout: tone diacritic or text hint above the key label.
    /// Text hints (punctuation) use a smaller font; standalone diacritics (tone marks) use a larger font.
    private func standardHintContent(text: String, hint: String) -> some View {
        let isText = textProvider.isTextHint(for: action)
        let hintFontSize = isText ? ButtonContentLayout.textHintFontSize : ButtonContentLayout.diacriticHintFontSize
        let offsetY = isText ? ButtonContentLayout.textHintOffsetY : textProvider.hintOffsetY(for: action)

        return VStack(spacing: ButtonContentLayout.hintVStackSpacing) {
            Text(hint)
                .font(.system(size: hintFontSize))
                .foregroundColor(keyTextColor.opacity(ButtonContentLayout.hintOpacity))
                .frame(height: ButtonContentLayout.hintFrameHeight)
                .offset(y: offsetY)
            Text(text)
                .font(fontProvider.buttonKeyboardFont(for: action).font)
        }
        .lineLimit(1)
    }

    /// Text-only key without hint. Shrinks to fit if needed.
    private func textOnlyContent(text: String) -> some View {
        Text(text)
            .font(fontProvider.buttonKeyboardFont(for: action).font)
            .lineLimit(1)
            .minimumScaleFactor(ButtonContentLayout.minimumScaleFactor)
    }

    /// Space bar input mode label (e.g. "TL", "POJ").
    /// Rendered alone without standardContent to avoid KeyboardKit's "space" text overlapping on iPad.
    private func spaceModeLabelContent(label: String) -> some View {
        Text(label)
            .font(KeyboardFonts.globalFont(size: ButtonContentLayout.modeLabelFontSize))
            .foregroundColor(keyTextColor)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottomTrailing)
            .padding(.trailing, ButtonContentLayout.modeLabelTrailingPadding)
            .padding(.bottom, ButtonContentLayout.modeLabelBottomPadding)
    }
}
