package com.siansiansu.taigikeyboard.localization

// Theme tab text constants (corresponds to iOS ThemeTexts.swift). Covers the tab
// title, the global font row label (Settings tab), the custom-theme shelf, and the
// theme editor (section headers, color-row labels, slider labels, name dialog, menu,
// cap alert). Built-in family titles live in BuiltInThemes.families (catalog display
// data), not here.
object ThemeTexts {
    const val tabTitle = "主題"

    // Global keyboard font row label, shown in the Settings tab (font is a global
    // setting, not a per-theme field). Mirrors iOS ThemeTexts.customFont.
    const val customFont = "字型設定"

    // Custom-theme shelf
    const val customThemesSection = "自訂主題"
    const val createNewTheme = "新主題…"

    // Editor section headers
    const val keyboardSection = "齒盤介面"
    const val colorKeySection = "揤鈕介面"
    const val candidateSection = "候選詞介面"

    // Editor color-row labels
    const val colorKeyboardBackground = "齒盤色水"
    const val colorKeyText = "揤鈕文字"
    const val colorNormalKeyFill = "一般揤鈕色水"
    const val colorSpecialKeyFill = "特殊揤鈕色水"
    const val colorCandidateText = "候選詞文字"
    const val colorCandidateBackground = "候選詞背景"

    // Editor slider labels
    const val keyHeight = "齒盤懸度"
    const val keyFontSize = "揤鈕字大細"
    const val candidateTextSize = "候選詞大細"
    const val keyCornerRadius = "揤鈕圓角"
    const val keyBorderWidth = "揤鈕邊粗幼"
    const val keyShadow = "揤鈕陰影"

    // Editor titles + actions
    const val editorTitleNew = "新主題"
    const val editorTitleEdit = "編輯主題"
    const val themeNameHeader = "主題名稱"
    const val themeNamePlaceholder = "輸入主題名稱"
    const val editorSave = "儲存"
    const val editorCancel = "取消"
    const val editorResetAll = "恢復預設設定"

    // Name left blank at save time falls back to this (identity is the UUID; names
    // may repeat and are editable later).
    const val defaultThemeName = "新主題"

    // Custom-theme card action menu
    const val themeMenu = "主題選項"
    const val themeMenuApply = "套用"
    const val themeMenuEdit = "編輯"
    const val themeMenuDelete = "刪除"

    // Cap-reached alert (UserThemeStore.MAX_USER_THEMES = 5)
    const val capReachedTitle = "已達主題數量上限"
    const val capReachedMessage = "自訂主題上限是 5 个,請先刪除一个才會使閣新增。"
    const val capReachedOK = "好"

    // Color picker dialog (ColorPickerDialog, opened from the theme editor): tab labels + RGB slider labels.
    const val colorPickerGrid = "格仔"
    const val colorPickerSpectrum = "光譜"
    const val colorPickerSliders = "滑桿"
    const val colorRed = "紅色"
    const val colorGreen = "綠色"
    const val colorBlue = "藍色"
}
