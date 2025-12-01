import Foundation

/// Helper for determining confirm key text based on settings
struct ConfirmKeyTextHelper {

    /// Get the appropriate text for the confirm/return key
    /// - Returns: The text to display on the confirm key
    static func getConfirmKeyText() -> String {
        let settings = SharedSettings.shared

        // showHanjiMode 固定為 true，只檢查翻譯交換設定
        if settings.isTranslateSwapped {
            return AppTexts.confirmKey.hanji
        }

        // 其他情況都根據輸入模式顯示羅馬字
        return settings.inputMode == .poj ? AppTexts.confirmKey.poj : AppTexts.confirmKey.tl
    }
}
