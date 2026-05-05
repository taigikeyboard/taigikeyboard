package com.siansiansu.taigikeyboard.localization

/**
 * 設定文字
 * 包含：鍵盤設定頁面
 * 對應 iOS SettingsTexts.swift
 */
object SettingsTexts {
    // MARK: - Tab 標題

    const val tabTitle = "齒盤設定"

    // MARK: - 通用按鍵

    const val reset = "恢復"

    /** Confirm key labels per input mode (used by KeyView) */
    fun confirmKeyLabel(
        inputMode: String,
        isTranslateSwapped: Boolean,
    ): String =
        when {
            inputMode == "tps" || isTranslateSwapped -> "選"
            inputMode == "poj" -> "soán"
            else -> "suán"
        }

    // MARK: - 輸入模式

    const val inputMode = "輸入模式"
    const val pojMode = "白話字"
    const val tlMode = "台羅"
    const val englishMode = "英文"
    const val tpsMode = "方音符號"

    // MARK: - 拍字設定

    const val typingSectionTitle = "拍字設定"
    const val outputBothScripts = "括號標註"
    const val autoCapitalization = "自動大本字"
    const val autoSpace = "自動空白"

    // MARK: - 齒盤設定

    const val keyboardSectionTitle = "齒盤設定"
    const val toolbarAutoCollapse = "自動隱藏工具列"
    const val globeKey = "插入齒盤切換揤鈕"

    // MARK: - 回饋設定

    const val feedbackSectionTitle = "拍字反應"
    const val soundFeedback = "揤仔聲"
    const val vibrationFeedback = "震動反應"

    // MARK: - 白話字設定

    const val pojSettingsSectionTitle = "白話字"
    const val doubleTapOO = "連紲拍 oo → o͘"
    const val doubleTapNN = "連紲拍 nn → ⁿ"

    // MARK: - 方音符號設定

    const val tpsSettingsSectionTitle = "方音符號"
    const val tpsOrMapsToER = "or 對應 ㄜ"

    // MARK: - 重設設定

    const val resetSettings = "恢復設定"
    const val resetSettingsMessage = "這个動作會恢復所有設定，敢欲繼續？"
    const val resetSuccess = "設定已恢復"
    const val resetFailed = "恢復設定失敗，請重試"
    const val noEmailApp = "揣無 Email App"

    // MARK: - 設定說明 (info descriptions for help icons)

    const val toolbarAutoCollapseInfo = "選字了後工具列會自動合起來，予齒盤面頂空間較大。"
    const val globeKeyInfo = "佇齒盤面頂加 1 粒地球揤鈕，揤著會使切換去其他齒盤。"
    val tpsOrMapsToERInfo =
        "台羅 or 毋是正式寫法，方音符號 ㄜ 正式干焦對應 er。共 or 嘛對應 ㄜ 是就音值來處理，毋過會予 er 佮 or 兩个音位攏對應到仝一个 ㄜ。\n\n開啟（預設）：or 對應 ㄜ，候選詞排佇 er 後壁。\n關閉：or 對應 ㄛ（恢復台羅 o），避免 er／or 相濫。教典 or 攏有 o 版本，袂影響拍字。"

    // MARK: - 裝置資訊

    const val diagnosticSectionTitle = "裝置資訊"
    const val diagnosticCopy = "Khó͘-phih 裝置資訊"
    const val diagnosticCopied = "已 khó͘-phih"
    const val diagnosticShare = "分享裝置資訊"
    const val diagnosticEmail = "Email 回報問題"

    // MARK: - Settings Overlay

    const val openApp = "去APP調整"
}
