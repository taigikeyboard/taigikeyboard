package com.siansiansu.taigikeyboard.localization

/**
 * Tab4 設定文字
 * 包含：鍵盤設定頁面
 * 對應 iOS Tab4Texts.swift
 */
object Tab4Texts {

    // MARK: - Tab 標題

    val tabTitle = LocalizedText(hanji = "齒盤設定")

    // MARK: - 通用按鍵

    val cancel = LocalizedText(hanji = "取消")
    val reset = LocalizedText(hanji = "恢復")
    val confirmKey = LocalizedText(hanji = "選", poj = "soán", tl = "suán")

    // MARK: - 輸入模式

    val inputMode = LocalizedText(hanji = "輸入模式")
    val pojMode = LocalizedText(hanji = "白話字")
    val tlMode = LocalizedText(hanji = "台羅")
    val englishMode = LocalizedText(hanji = "英文")
    val tpsMode = LocalizedText(hanji = "方音符號")

    // MARK: - 拍字設定

    val typingSectionTitle = LocalizedText(hanji = "拍字設定")
    val outputBothScripts = LocalizedText(hanji = "括號標註")
    val autoCapitalization = LocalizedText(hanji = "自動大本字")
    val autoSpace = LocalizedText(hanji = "自動空白")
    // MARK: - 齒盤設定

    val keyboardSectionTitle = LocalizedText(hanji = "齒盤設定")
    val toolbarAutoCollapse = LocalizedText(hanji = "自動切換工具列")
    val globeKey = LocalizedText(hanji = "插入齒盤切換揤鈕")

    // MARK: - 回饋設定

    val feedbackSectionTitle = LocalizedText(hanji = "拍字反應")
    val soundFeedback = LocalizedText(hanji = "揤仔聲")
    val vibrationFeedback = LocalizedText(hanji = "震動反應")

    // MARK: - 白話字設定

    val pojSettingsSectionTitle = LocalizedText(hanji = "白話字")
    val doubleTapOO = LocalizedText(hanji = "連紲拍 oo → o͘")
    val doubleTapNN = LocalizedText(hanji = "連紲拍 nn → ⁿ")

    // MARK: - 方音符號設定

    val tpsSettingsSectionTitle = LocalizedText(hanji = "方音符號")
    val tpsOrMapsToER = LocalizedText(hanji = "or 對應 ㄜ")

    // MARK: - 重設設定

    val resetSettings = LocalizedText(hanji = "恢復設定")
    val resetSettingsMessage = LocalizedText(hanji = "這个動作會恢復所有設定，敢欲繼續？")
    val resetSuccess = LocalizedText(hanji = "設定已恢復")

    // MARK: - 設定說明 (info descriptions for help icons)

    val toolbarAutoCollapseInfo = LocalizedText(hanji = "選字了後工具列會自動合起來，予齒盤面頂空間較大。")
    val globeKeyInfo = LocalizedText(hanji = "佇齒盤面頂加 1 粒地球揤鈕，揤著會使切換去其他齒盤。")
    val tpsOrMapsToERInfo = LocalizedText(hanji = "台羅 or 毋是正式寫法，方音符號 ㄜ 正式干焦對應 er。共 or 嘛對應 ㄜ 是就音值來處理，毋過會予 er 佮 or 兩个音位攏對應到仝一个 ㄜ。\n\n開啟（預設）：or 對應 ㄜ，候選詞排佇 er 後壁。\n關閉：or 對應 ㄛ（恢復台羅 o），避免 er／or 相濫。教典 or 攏有 o 版本，袂影響拍字。")

    // MARK: - Settings Overlay

    val openApp = LocalizedText(hanji = "去APP調整")
}
