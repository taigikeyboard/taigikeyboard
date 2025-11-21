import Foundation

/// Helper for determining confirm key text based on settings
struct ConfirmKeyTextHelper {

    /// Get the appropriate text for the confirm/return key
    /// - Returns: The text to display on the confirm key
    static func getConfirmKeyText() -> String {
        let settings = SharedSettings.shared

        // 只有在顯示漢字模式且翻譯交換時才顯示漢字
        if settings.showHanjiMode && settings.isTranslateSwapped {
            return AppTexts.confirmKey.hanji
        }

        // 其他情況都根據輸入模式顯示羅馬字
        return settings.inputMode == .poj ? AppTexts.confirmKey.poj : AppTexts.confirmKey.tl
    }
}
