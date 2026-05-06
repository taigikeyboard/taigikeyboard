// 中文: 鍵盤每顆按鍵的客製化 SwiftUI content view。
// 中文: 用 ButtonImageProvider → ButtonTextProvider → ButtonFontProvider 三段渲染鏈,
// 中文: 並把 TPS callout、調符 hint、空白鍵的模式標籤等特殊呈現都收在這裡。

import KeyboardKit
import SwiftUI

// MARK: - Layout Constants

/// Layout constants for `TaigiButtonContent` sub-views.
/// File-scoped because Swift does not allow static stored properties in nested types of generic structs.
// 中文: 鍵面 sub-view 的版面常數。Swift 不允許 generic struct 的 nested type 持有 static stored property,
// 中文: 所以放在 file-scope private enum。
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
// 中文: 鍵盤按鍵的客製化 content view — 串起 image / text / font 三組 provider。
// 中文: keyboardContext 必須用 @ObservedObject,才能對 isComposingText 等狀態變更作出反應。
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
    // 中文: 空白鍵右下角顯示的輸入模式標籤(POJ / TL / EN / TPS)。
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
    // 中文: 渲染優先順序 — 圖示 → 文字 → 預設 standardContent。每個 provider 回 nil 即放行給下一棒。
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
    // 中文: TPS 佈局 — hint 在上、主鍵字在下。兩個 hint 左右排開,單一 hint 置中。
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
    // 中文: 標準 hint 佈局 — 鍵面字上方放調符或文字 hint。
    // 中文: 文字 hint(標點)用較小字級;獨立調符用較大字級。
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
    // 中文: 空白鍵的模式標籤(TL / POJ / EN / TPS);單獨渲染,避免在 iPad 上與 KeyboardKit 的 "space" 文字重疊。
    private func spaceModeLabelContent(label: String) -> some View {
        Text(label)
            .font(KeyboardFonts.globalFont(size: ButtonContentLayout.modeLabelFontSize))
            .foregroundColor(keyTextColor)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottomTrailing)
            .padding(.trailing, ButtonContentLayout.modeLabelTrailingPadding)
            .padding(.bottom, ButtonContentLayout.modeLabelBottomPadding)
    }
}
