// MARK: - SettingsTab 設定文字

// 包含：鍵盤設定頁面

enum SettingsTexts {
    // MARK: - Tab 標題

    static let tabTitle = "齒盤設定"
    static let tabBarTitle = "設定"

    // MARK: - 通用按鍵

    static let reset = "恢復"

    // MARK: - 輸入模式

    static let inputMode = "輸入模式"
    static let pojMode = "白話字"
    static let tlMode = "台羅"
    static let englishMode = "英文"
    static let tpsMode = "方音符號"

    // MARK: - 拍字設定

    static let typingSectionTitle = "拍字設定"
    static let isOutputBothScripts = "括號標註"
    static let autoCapitalization = "自動大本字"
    static let autoSpace = "自動空白"

    // MARK: - 齒盤設定

    static let keyboardSectionTitle = "齒盤設定"
    static let toolbarAutoCollapse = "自動切換工具列"
    static let globeKey = "插入齒盤切換揤鈕"

    // MARK: - 回饋設定

    static let feedbackSectionTitle = "拍字反應"
    static let soundFeedback = "揤仔聲"
    static let vibrationFeedback = "震動反應"

    // MARK: - 白話字設定

    static let pojSettingsSectionTitle = "白話字"
    static let doubleTapOO = "連紲拍 oo → o͘"
    static let doubleTapNN = "連紲拍 nn → ⁿ"

    // MARK: - 方音符號設定

    static let tpsSettingsSectionTitle = "方音符號"
    static let isTpsOrMappedToER = "or 對應 ㄜ"

    // MARK: - 重設設定

    static let resetSettings = "恢復設定"
    static let resetSettingsMessage = "這个動作會恢復所有設定，敢欲繼續？"

    // MARK: - 裝置資訊

    static let diagnosticSectionTitle = "裝置資訊"
    static let diagnosticCopy = "Khó͘-phih 裝置資訊"
    static let diagnosticCopied = "已 khó͘-phih"
    static let diagnosticShare = "分享裝置資訊"
    static let diagnosticEmail = "Email 回報問題"

    // MARK: - Settings Overlay

    static let openApp = "去APP調整"

    // MARK: - 設定說明 (info descriptions for questionmark.circle)

    static let toolbarAutoCollapseInfo = "選字了後工具列會自動合起來，予齒盤面頂空間較大。"
    static let globeKeyInfo = "佇齒盤面頂加 1 粒地球揤鈕，揤著會使切換去其他齒盤。"
    static let isTpsOrMappedToERInfo = "台羅 or 毋是正式寫法，方音符號 ㄜ 正式干焦對應 er。共 or 嘛對應 ㄜ 是就音值來處理，毋過會予 er 佮 or 兩个音位攏對應到仝一个 ㄜ。\n\n開啟（預設）：or 對應 ㄜ，候選詞排佇 er 後壁。\n關閉：or 對應 ㄛ（恢復台羅 o），避免 er／or 相濫。教典 or 攏有 o 版本，袂影響拍字。"
}
