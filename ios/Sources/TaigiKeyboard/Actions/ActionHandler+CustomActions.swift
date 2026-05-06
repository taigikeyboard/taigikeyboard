// ActionHandler extension: custom button actions (translate toggle, open main app).
// 中文: ActionHandler 的自訂按鈕擴充 — 翻譯切換、開主 App 等。

import Foundation
import KeyboardKit

extension ActionHandler {
    // 中文: 依按鈕名稱派送到對應動作處理器。
    func handleCustomAction(_ name: String) {
        switch name {
        case "translate":
            handleTranslateToggle()
        default:
            break
        }
    }

    /// Toggle romanization / Hanji (漢字) display mode
    // 中文: 切換羅馬字 / 漢字顯示模式 — 對應翻譯按鈕。
    func handleTranslateToggle() {
        keyboardContext.toggleTranslateSwapped()
        if feedbackContext.settings.isHapticFeedbackEnabled {
            triggerHapticFeedback(.lightImpact)
        }
    }

    /// Open main app settings page
    // 中文: 透過 deep link 開啟主 App 設定頁。
    func openMainAppSettings() {
        guard let url = URL(string: "taigikeyboard://"),
              let controller = keyboardController else { return }
        controller.openUrl(url)
    }
}
