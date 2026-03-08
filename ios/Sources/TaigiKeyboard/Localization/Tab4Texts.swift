import Foundation

// MARK: - Tab4 設定文字
// 包含：鍵盤設定頁面

enum Tab4Texts {

    // MARK: - Tab 標題

    static let tabTitle = LocalizedText(hanji: "設定")

    // MARK: - 通用按鍵

    static let cancel = LocalizedText(hanji: "取消")
    static let reset = LocalizedText(hanji: "恢復")

    // MARK: - 輸入模式

    static let inputMode = LocalizedText(hanji: "輸入模式")
    static let pojMode = LocalizedText(hanji: "白話字")
    static let tlMode = LocalizedText(hanji: "台羅")
    static let englishMode = LocalizedText(hanji: "英文")

    // MARK: - 開關設定

    static let outputBothScripts = LocalizedText(hanji: "括號標註")
    static let autoCapitalization = LocalizedText(hanji: "自動大本字")
    static let autoSpace = LocalizedText(hanji: "自動空白")
    static let doubleTapOO = LocalizedText(hanji: "連紲拍 oo → o͘")
    static let doubleTapNN = LocalizedText(hanji: "連紲拍 nn → ⁿ")

    // MARK: - 重設設定

    static let resetSettings = LocalizedText(hanji: "恢復設定")
    static let resetSettingsMessage = LocalizedText(hanji: "這个動作會恢復所有設定，敢欲繼續？")
    static let resetSuccess = LocalizedText(hanji: "設定已恢復")

    // MARK: - 裝置資訊

    static let diagnosticSectionTitle = LocalizedText(hanji: "裝置資訊")
    static let diagnosticCopy = LocalizedText(hanji: "Khó͘-phih 裝置資訊")
    static let diagnosticCopied = LocalizedText(hanji: "已 khó͘-phih")
    static let diagnosticShare = LocalizedText(hanji: "分享裝置資訊")
    static let diagnosticEmail = LocalizedText(hanji: "Email 回報問題")

    // MARK: - Debug 模式

    #if DEBUG
    static let debugMode = LocalizedText(hanji: "Debug 模式")
    #endif
}
