package com.siansiansu.taigikeyboard.localization

/**
 * Tab4 設定文字
 * 包含：鍵盤設定頁面
 * 對應 iOS Tab4Texts.swift
 */
object Tab4Texts {

    // MARK: - Tab 標題

    val tabTitle = LocalizedText(hanji = "設定")

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

    // MARK: - 開關設定

    val outputBothScripts = LocalizedText(hanji = "括號標註")
    val autoCapitalization = LocalizedText(hanji = "自動大本字")
    val autoSpace = LocalizedText(hanji = "自動空白")

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

    // MARK: - Debug 模式

    val debugMode = LocalizedText(hanji = "Debug 模式")
}
