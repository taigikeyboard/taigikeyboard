import KeyboardKit
import SwiftUI

/// Provider for button image customization
class ButtonImageProvider {

    private let keyboardContext: KeyboardContext

    init(keyboardContext: KeyboardContext) {
        self.keyboardContext = keyboardContext
    }

    /// Get custom button image for the given action
    /// - Parameter action: The keyboard action
    /// - Returns: Custom image if applicable, nil to use default
    func buttonImage(for action: KeyboardAction) -> Image? {
        switch action {
        case .nextKeyboard:
            return Image(systemName: "globe")
        case .shift:
            // 使用 KeyboardKit 的標準圖示邏輯，根據 keyboardCase 狀態自動選擇
            return nil
        case .primary(.return):
            // 組字模式時回傳 nil，讓自訂的 buttonText 能顯示
            if keyboardContext.isComposingText {
                return nil
            }
            // 非組字模式保持預設圖示
            return nil
        case .settings:
            return Image(systemName: "gearshape.fill")
        case let .custom(name):
            switch name {
            case "translate":
                let iconName = keyboardContext.isTranslateSwapped
                    ? "character.bubble.fill"  // 啟用狀態：實心
                    : "character.bubble"       // 預設狀態：空心
                return Image(systemName: iconName)
            default:
                return nil
            }
        default:
            return nil
        }
    }
}
