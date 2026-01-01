import Foundation
import KeyboardKit

/// 自定義動作處理
///
/// 處理台語鍵盤特有的自定義按鍵動作。
extension ActionHandler {

    func handleCustomAction(_ name: String) {
        switch name {
        case "translate":
            handleTranslateToggle()
        default:
            break
        }
    }

    /// 切換羅馬字／漢字顯示模式
    func handleTranslateToggle() {
        keyboardContext.toggleTranslateSwapped()
        if feedbackContext.settings.isHapticFeedbackEnabled {
            triggerHapticFeedback(.lightImpact)
        }
    }

    /// 開啟主應用程式設定頁面
    func openMainAppSettings() {
        guard let url = URL(string: "taigikeyboard://"),
              let controller = keyboardController else { return }
        controller.openUrl(url)
    }
}
