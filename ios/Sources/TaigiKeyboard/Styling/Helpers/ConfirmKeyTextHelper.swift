import Foundation

/// 確認鍵文字輔助工具
///
/// 組字模式時，Enter 按鍵顯示的確認文字。
/// 根據 isTranslateSwapped 和 inputMode 顯示不同文字：
/// - isTranslateSwapped = true → "選"
/// - isTranslateSwapped = false, inputMode = .poj → "soán"
/// - isTranslateSwapped = false, inputMode = .tl → "suán"
struct ConfirmKeyTextHelper {

    static func getConfirmKeyText() -> String {
        let settings = SharedSettings.shared

        // TPS layout: always show "選" (bopomofo users don't read romanization)
        if settings.inputMode == .tps {
            return "選"
        }

        if settings.isTranslateSwapped {
            return "選"
        }

        switch settings.inputMode {
        case .poj:
            return "soán"
        case .tl, .english, .tps:
            return "suán"
        }
    }
}
