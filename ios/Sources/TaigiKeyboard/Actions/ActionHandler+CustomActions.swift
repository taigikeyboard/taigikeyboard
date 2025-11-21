import Foundation
import KeyboardKit

// MARK: - Custom Actions

extension ActionHandler {
    /// 處理自定義動作
    /// - Parameter name: 自定義動作名稱
    func handleCustomAction(_ name: String) {
        switch name {
        case "translate":
            handleTranslateToggle()
        default:
            break
        }
    }

    /// 處理翻譯切換功能
    /// 切換台語羅馬字與漢字的顯示模式
    func handleTranslateToggle() {
        if keyboardViewController?.responds(to: NSSelectorFromString("toggleTranslateSwap")) == true {
            keyboardViewController?.perform(NSSelectorFromString("toggleTranslateSwap"))
        }

        if feedbackContext.settings.isHapticFeedbackEnabled {
            triggerHapticFeedback(.lightImpact)
        }
    }

    /// 開啟主應用程式的設定頁面
    /// 使用 URL Scheme 跳轉到設定
    func openMainAppSettings() {
        guard let url = URL(string: "taigikeyboard://settings") else {
            return
        }

        guard let controller = keyboardController else {
            return
        }

        controller.openUrl(url)
    }
}
