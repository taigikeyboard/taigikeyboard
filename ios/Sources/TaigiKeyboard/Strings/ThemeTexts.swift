// 中文: ThemeTab(主題)所有 UI 文字常數。包含 tab 標題 + 外觀設定頁的字型 /
// 中文: 尺寸 slider / 配色 row / Section header / 重置按鈕文字(PR-1 自 LayoutTexts 移入)。
// 中文: 主題選擇器 Shelf 與自訂主題編輯器文字將於後續 PR(PR-2b / PR-3)補上。

// MARK: - ThemeTab 主題文字

// 中文: 主題頁文字 namespace。MARK 子區塊對應「外觀設定」頁的 Form Section 切分。
enum ThemeTexts {
    // MARK: - Tab 標題

    static let tabTitle = "主題"
    static let tabBarTitle = "主題"

    // MARK: - 字體設定

    static let customFont = "字型設定"

    // MARK: - 外觀設定

    static let appearanceSettings = "外觀設定"
    static let keyHeight = "齒盤懸度"
    static let keyFontSize = "揤鈕字大細"
    static let candidateTextSize = "候選詞大細"
    static let keyCornerRadius = "揤鈕圓角"
    static let keyBorderWidth = "揤鈕邊粗幼"
    static let appearanceResetAll = "恢復預設外觀"

    // MARK: - 色水設定

    static let colorKeyboardBackground = "齒盤色水"
    static let colorKeyText = "揤鈕文字"
    static let colorNormalKeyFill = "一般揤鈕色水"
    static let colorSpecialKeyFill = "特殊揤鈕色水"
    static let colorCandidateText = "候選詞文字"
    static let colorCandidateBackground = "候選詞背景"
    static let colorKeySection = "揤鈕介面"

    // MARK: - 區域分組 Header

    static let keyboardSection = "齒盤介面"
    static let candidateSection = "候選詞介面"
}
