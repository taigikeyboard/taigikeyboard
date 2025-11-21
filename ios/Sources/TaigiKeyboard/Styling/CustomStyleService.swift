import KeyboardKit
import SwiftUI

/// Custom style service that delegates to specialized providers
class CustomStyleService: KeyboardStyle.StandardStyleService {

    private let textProvider: ButtonTextProvider
    private let imageProvider: ButtonImageProvider
    private let fontProvider: ButtonFontProvider

    override init(keyboardContext: KeyboardContext) {
        self.textProvider = ButtonTextProvider(keyboardContext: keyboardContext)
        self.imageProvider = ButtonImageProvider(keyboardContext: keyboardContext)
        self.fontProvider = ButtonFontProvider(keyboardContext: keyboardContext)
        super.init(keyboardContext: keyboardContext)
    }

    override func buttonText(for action: KeyboardAction) -> String? {
        textProvider.buttonText(for: action) ?? super.buttonText(for: action)
    }

    override func buttonImage(for action: KeyboardAction) -> Image? {
        // 特殊處理：組字時的 Return 鍵不顯示圖示，讓文字能夠顯示
        if action == .primary(.return) && keyboardContext.isComposingText {
            return nil
        }
        return imageProvider.buttonImage(for: action) ?? super.buttonImage(for: action)
    }

    override func buttonKeyboardFont(for action: KeyboardAction) -> KeyboardFont {
        fontProvider.buttonKeyboardFont(for: action)
    }

    override func buttonContentInsets(for action: KeyboardAction) -> EdgeInsets {
        let spacing = KeyboardModels.UI.Keyboard.horizontalSpacing / 4
        let verticalSpacing = KeyboardModels.UI.Keyboard.verticalSpacing / 4

        switch action {
        case .space:
            return EdgeInsets(top: verticalSpacing, leading: spacing * 2, bottom: verticalSpacing, trailing: spacing * 2)
        case .character:
            return EdgeInsets(top: verticalSpacing, leading: spacing, bottom: verticalSpacing, trailing: spacing)
        case .backspace, .shift, .keyboardType, .primary:
            return EdgeInsets(top: verticalSpacing, leading: spacing, bottom: verticalSpacing, trailing: spacing)
        default:
            return EdgeInsets(top: verticalSpacing, leading: spacing, bottom: verticalSpacing, trailing: spacing)
        }
    }
}
