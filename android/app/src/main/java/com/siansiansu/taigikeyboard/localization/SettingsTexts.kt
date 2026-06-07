package com.siansiansu.taigikeyboard.localization

/**
 * 設定文字
 * 包含：鍵盤設定頁面
 * 對應 iOS SettingsTexts.swift
 */
object SettingsTexts {
    // MARK: - Tab 標題

    const val tabTitle = "設定"

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
    const val literalRomanCandidate = "顯示羅馬字"
    const val literalRomanCandidateInfo = "候選詞列第一个位囥羅馬字，會當用手點抑是揤 Enter 送出，若關，干焦會當揤 Enter 送出，袂當用手點，但是候選詞列空間較大。"
    const val autoCapitalization = "自動大本字"
    const val autoSpace = "自動空白"

    // MARK: - 齒盤設定

    const val keyboardSectionTitle = "齒盤設定"
    const val toolbarAutoCollapse = "自動隱藏工具列"
    const val globeKey = "齒盤切換揤鈕"

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
        "台羅 or 毋是正式寫法，方音符號 ㄜ 正式干焦對應 er。本設定只控制候選詞按怎顯示;字典揣詞已經共 er 佮 or 攏對應做仝一个音位，無論本設定開抑無開攏揣會著。\n\n開啟（預設）：or 顯示做 ㄜ。\n關閉：or 顯示做 ㄛ（恢復台羅 o）。"

    // MARK: - 裝置資訊

    const val diagnosticSectionTitle = "裝置資訊"
    const val diagnosticCopy = "Khó͘-phih 裝置資訊"
    const val diagnosticCopied = "已 khó͘-phih"
    const val diagnosticShare = "分享裝置資訊"
    const val diagnosticEmail = "Email 回報問題"

    // MARK: - Settings Overlay

    const val openApp = "去APP調整"
}
