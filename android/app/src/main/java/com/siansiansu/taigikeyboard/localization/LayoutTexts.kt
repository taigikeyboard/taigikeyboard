package com.siansiansu.taigikeyboard.localization

// Layout-selection and font-name text constants (corresponds to iOS LayoutTexts.swift).
// Appearance-editing labels (key sizes, colors, section headers) moved to ThemeTexts
// when the standalone appearance page was removed (theme editor owns them now). The
// "字型設定" row label lives in ThemeTexts.customFont; the font NAMES stay here.
object LayoutTexts {
    const val tabTitle = "佈局"

    const val romanizationKeyboard = "羅馬字齒盤"
    const val taigiPhonetic = "方音符號"
    const val standardLayout = "Lohankha"
    const val phahTaigiLayout = "PhahTaigi"
    const val tpsLayout = "方音符號齒佈"
    const val moe1Layout = "教育部輸入法齒佈1"
    const val moe2Layout = "教育部輸入法齒佈2"
    const val comingSoon = "連鞭上市"

    const val fontSystemDefault = "系統"
    const val fontOpenHuninn = "粉圓"
    const val fontIansui = "芫荽"
    const val fontGenYoMin = "源樣明體"
    const val fontGenYoGothic = "源樣烏體"

    // Color picker dialog labels (used by ColorPickerDialog → theme editor)
    const val colorPickerGrid = "格仔"
    const val colorPickerSpectrum = "光譜"
    const val colorPickerSliders = "滑桿"
    const val colorRed = "紅色"
    const val colorGreen = "綠色"
    const val colorBlue = "藍色"
}
