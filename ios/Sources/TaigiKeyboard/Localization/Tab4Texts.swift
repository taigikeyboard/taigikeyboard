import Foundation

// MARK: - Tab4 設定文字

// 包含：鍵盤設定頁面

enum Tab4Texts {
    // MARK: - Tab 標題

    static let tabTitle = LocalizedText(hanji: "齒盤設定")
    static let tabBarTitle = LocalizedText(hanji: "設定")

    // MARK: - 通用按鍵

    static let cancel = LocalizedText(hanji: "取消")
    static let reset = LocalizedText(hanji: "恢復")

    // MARK: - 輸入模式

    static let inputMode = LocalizedText(hanji: "輸入模式")
    static let pojMode = LocalizedText(hanji: "白話字")
    static let tlMode = LocalizedText(hanji: "台羅")
    static let englishMode = LocalizedText(hanji: "英文")
    static let tpsMode = LocalizedText(hanji: "方音符號")

    // MARK: - 拍字設定

    static let typingSectionTitle = LocalizedText(hanji: "拍字設定")
    static let outputBothScripts = LocalizedText(hanji: "括號標註")
    static let autoCapitalization = LocalizedText(hanji: "自動大本字")
    static let autoSpace = LocalizedText(hanji: "自動空白")

    // MARK: - 齒盤設定

    static let keyboardSectionTitle = LocalizedText(hanji: "齒盤設定")
    static let toolbarAutoCollapse = LocalizedText(hanji: "自動切換工具列")
    static let globeKey = LocalizedText(hanji: "插入齒盤切換揤鈕")

    // MARK: - 回饋設定

    static let feedbackSectionTitle = LocalizedText(hanji: "拍字反應")
    static let soundFeedback = LocalizedText(hanji: "揤仔聲")
    static let vibrationFeedback = LocalizedText(hanji: "震動反應")

    // MARK: - 白話字設定

    static let pojSettingsSectionTitle = LocalizedText(hanji: "白話字")
    static let doubleTapOO = LocalizedText(hanji: "連紲拍 oo → o͘")
    static let doubleTapNN = LocalizedText(hanji: "連紲拍 nn → ⁿ")

    // MARK: - 方音符號設定

    static let tpsSettingsSectionTitle = LocalizedText(hanji: "方音符號")
    static let tpsOrMapsToER = LocalizedText(hanji: "or 對應 ㄜ")

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

    // MARK: - Settings Overlay

    static let openApp = LocalizedText(hanji: "去APP調整")

    // MARK: - 設定說明 (info descriptions for questionmark.circle)

    static let toolbarAutoCollapseInfo = LocalizedText(hanji: "選字了後工具列會自動合起來，予齒盤面頂空間較大。")
    static let globeKeyInfo = LocalizedText(hanji: "佇齒盤面頂加 1 粒地球揤鈕，揤著會使切換去其他齒盤。")
    static let tpsOrMapsToERInfo = LocalizedText(hanji: "台羅 or 毋是正式寫法，方音符號 ㄜ 正式干焦對應 er。共 or 嘛對應 ㄜ 是就音值來處理，毋過會予 er 佮 or 兩个音位攏對應到仝一个 ㄜ。\n\n開啟（預設）：or 對應 ㄜ，候選詞排佇 er 後壁。\n關閉：or 對應 ㄛ（恢復台羅 o），避免 er／or 相濫。教典 or 攏有 o 版本，袂影響拍字。")

    // MARK: - Icons (shared between Tab4 and keyboard overlay)

    static let outputBothScriptsIcon = "character.book.closed"
    static let autoCapitalizationIcon = "textformat.size"
    static let autoSpaceIcon = "space"
    static let toolbarIcon = "menubar.rectangle"
    static let globeKeyIcon = "globe"
    static let soundFeedbackIcon = "speaker.wave.2"
    static let vibrationFeedbackIcon = "iphone.radiowaves.left.and.right"
}
