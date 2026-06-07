// 中文: ThemeTab(主題)所有 UI 文字常數。包含 tab 標題 + 主題選擇器 Shelf(PR-2b)+
// 中文: 外觀設定頁的字型 / 尺寸 slider / 配色 row / Section header / 重置按鈕文字(PR-1 自 LayoutTexts 移入)。
// 中文: 自訂主題編輯器(+ 號)文字將於 PR-3 補上。

// MARK: - ThemeTab 主題文字

// 中文: 主題頁文字 namespace。MARK 子區塊對應「主題選擇器」與「外觀設定」頁的切分。
enum ThemeTexts {
    // MARK: - Tab 標題

    static let tabTitle = "主題"
    static let tabBarTitle = "主題"

    // MARK: - 主題選擇器 Shelf(PR-2b)

    static let builtInThemesSection = "內建主題"
    static let defaultThemeName = "預設"
    static let customAppearance = "自訂外觀設定"

    // MARK: - 主題選擇器版面 placeholder

    // 中文: 版面草稿用字串(暫借 KeyboardKit 原文)。實際主題逐一加回時 USER 會改成自訂版本。
    static let customThemesSection = "Custom Themes"
    static let createNewTheme = "Create New…"

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

    // MARK: - 主題編輯器(PR-B)

    static let editorTitleNew = "新主題"
    static let editorTitleEdit = "編輯主題"
    static let themeNameHeader = "主題名稱"
    static let themeNamePlaceholder = "輸入主題名稱"
    static let keyShadow = "揤鈕陰影"
    static let editorSave = "儲存"
    static let editorCancel = "取消"

    // MARK: - Custom Themes shelf 動作選單

    static let themeMenuApply = "套用"
    static let themeMenuEdit = "編輯"
    static let themeMenuDelete = "刪除"

    // 中文: 自訂主題達上限(UserThemeStore.maxUserThemes = 5)的提示。
    static let capReachedTitle = "已達主題數量上限"
    static let capReachedMessage = "自訂主題上限是 5 个,請先刪除一个才會使閣新增。"
    static let capReachedOK = "好"
}
