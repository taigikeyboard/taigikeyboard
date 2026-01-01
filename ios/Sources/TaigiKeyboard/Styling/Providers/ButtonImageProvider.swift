import KeyboardKit
import SwiftUI

/// 按鈕圖片提供者
///
/// 回傳按鍵應顯示的圖示（SF Symbols）。回傳 nil 時由 ButtonTextProvider 處理。
class ButtonImageProvider {

    private let keyboardContext: KeyboardContext

    init(keyboardContext: KeyboardContext) {
        self.keyboardContext = keyboardContext
    }

    /// 取得按鍵對應的圖示
    func buttonImage(for action: KeyboardAction) -> Image? {
        switch action {
        case .nextKeyboard:
            return Image(systemName: "globe")
        case .settings:
            return Image(systemName: "gearshape.fill")
        case let .custom(name):
            switch name {
            case "translate":
                let iconName = keyboardContext.isTranslateSwapped
                    ? "character.square.fill"  // 啟用狀態：實心
                    : "character.square"       // 預設狀態：空心
                return Image(systemName: iconName)
            default:
                return nil
            }
        default:
            return nil
        }
    }
}
