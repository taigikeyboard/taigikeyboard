package com.siansiansu.taigikeyboard.localization

/**
 * Tab3 詞庫文字
 * 包含：詞庫管理頁面
 * 對應 iOS Tab3Texts.swift
 */
object Tab3Texts {
    // MARK: - Tab 標題

    const val tabTitle = "詞庫管理"

    // MARK: - 通用按鍵

    const val clear = "清除"
    const val delete = "刪除"
    const val save = "儉起來"

    // MARK: - 詞庫管理

    const val dictionarySettings = "詞庫管理"
    const val customDictionary = "自訂詞庫"
    const val customDictEnabled = "啟用自訂詞庫"
    const val customDictEnabledInfo = "拍開了後，家己加入 ê 詞會出現佇候選詞內底。關起來了後，自訂詞就袂閣出現。"
    const val dataManagement = "個人資料"
    const val variantDictionary = "異用字"
    const val khiin = "在來字"

    // MARK: - 自訂詞庫

    const val customDictDescription = "自訂詞庫使用 CSV 純文字檔案，第 1 欄囥欲拍 ê 羅馬字，第 2 欄囥漢字，毋免囥標題。"
    const val customDictEmpty = "揤 + 符號加入自訂詞"
    const val addEntry = "增加詞"
    const val editEntry = "編輯詞"
    const val romanLabel = "拍字"
    const val romanPlaceholder = "見本：gâu-tsá"
    const val hanziLabel = "對應"
    const val hanziPlaceholder = "見本：𠢕早"
    const val entriesCount = "項"
    const val deleteAll = "刪除全部自訂詞"
    const val deleteAllMessage = "敢確定欲刪除所有自訂詞？"
    const val customDictPrivacyWarning = "請毋通佇自訂詞庫囥敏感 ê 個人資料，親像身分證字號、口座密碼、信用卡號碼，請注意家己 ê 資訊安全。"

    // MARK: - 匯入匯出

    const val importCSV = "匯入詞庫"
    const val exportCSV = "匯出詞庫"
    const val exportSuccess = "CSV 順利匯出"
    const val importResult = "匯入 %d 項成功，%d 項重複"
    const val invalidCSVFormat = "檔案格式無正確，請使用 CSV 格式"
    const val fileTooLarge = "檔案傷大（上限 5 MB）"
    const val tooManyEntries = "詞傷濟（上限 30,000 項）"
    const val importExportTitle = "匯出匯入"
    const val importExportHelpTitle = "匯出匯入說明"
    const val importExportHelp = "第 1 欄囥欲拍 ê 羅馬字，第 2 欄囥漢字、日本字，毋免囥標題"

    // MARK: - 詞頻匯入匯出

    const val frequencyExportCSV = "匯出詞頻紀錄"
    const val frequencyImportCSV = "匯入詞頻紀錄"
    const val frequencyDescription = "詞頻紀錄使用 CSV 純文字檔案，第 1 欄囥詞，第 2 欄囥次數，毋免囥標題。"
    const val frequencyImportResult = "匯入 %d 項成功，%d 項重複"

    // MARK: - 詞關聯匯入匯出

    const val associationExportCSV = "匯出詞關聯紀錄"
    const val associationImportCSV = "匯入詞關聯紀錄"
    const val associationDescription = "詞關聯紀錄使用 CSV 純文字檔案，共 5 欄：頭前詞、頭前拍字、後壁詞、後壁拍字、次數，毋免囥標題。"
    const val associationImportResult = "匯入 %d 項成功，%d 項重複"

    // MARK: - 詞庫區塊標題

    const val moeSectionTitle = "教育部用字"
    const val otherSectionTitle = "其他辭典"
    const val supplementSectionTitle = "補充資料"

    // MARK: - 詞庫名稱 (Tab3-only)

    const val lkkDict = "漢羅合用建議用字"

    // MARK: - 詞庫搜尋

    const val searchPlaceholder = "拍字揣詞"
    const val noResults = "揣無結果"
    const val customDictionarySource = "自訂詞庫"

    // MARK: - 辭典查詢選項

    const val lookupChhoe = "ChhoeTaigi 辭典"
    const val lookupMoe = "教育部辭典"

    // MARK: - 常用詞管理

    const val frequentWordsManagement = "常用詞管理"
    const val frequencyTab = "詞頻"
    const val frequencyManagement = "詞頻紀錄"
    const val frequencyRecordingEnabled = "開啟詞頻紀錄"
    const val frequencyRecordingEnabledInfo = "拍開了後，齒盤會記錄你揀過 ê 詞幾擺，予候選詞排序做參考，定定揀 ê 詞就會排較頭前，按呢候選詞就會愈來愈準。"
    const val associationTab = "詞關聯"
    const val associationManagement = "詞關聯紀錄"
    const val associationRecordingEnabled = "開啟詞關聯紀錄"
    const val associationRecordingEnabledInfo = "拍開了後，齒盤會記錄頭前、後壁 ê 關聯詞，予連紲建議愈來愈準。"
    const val totalEntries = "共 %d / %d 筆"
    const val frequencyPrivacyWarning = "詞頻紀錄對台語研究來講是真有價值 ê 資料。若欲提供予人研究訓練模型，請先刪除敏感 ê 私人資料。紀錄功能嘛會使關起來，毋過按呢候選詞 ê 排序就會較無準。"
    const val clearAllFrequency = "刪除所有詞頻紀錄"
    const val associationPrivacyWarning = "詞關聯紀錄對台語研究來講是真有價值 ê 資料。若欲提供予人做研究，請先刪除敏感 ê 內容。紀錄功能嘛會使關起來，毋過後一詞預測會較無準。"
    const val clearAllAssociation = "刪除所有詞關聯紀錄"
    const val clearFrequencyMessage = "確定欲刪除所有詞頻紀錄？"
    const val clearAssociationMessage = "確定欲刪除所有詞關聯紀錄？"
    const val noData = "無資料"
    const val filterHint = "頂面上濟顯示 100 个詞，若揣無詞，請用下跤 ê「拍字揣詞」功能搜揣。"

    // MARK: - 備份還原

    const val backupRestore = "備份復原"
    const val exportBackup = "一擺全出"
    const val importBackup = "一擺全入"
    const val exportBackupSuccess = "備份順利匯出"
    const val importBackupResult = "匯入 %d 項自訂詞、%d 項詞頻、%d 項詞關聯"
    const val backupHelpTitle = "備份還原說明"
    const val backupHelp = "匯出所有使用者資料（自訂詞庫、詞頻、詞關聯），適用換機。"
    const val backupPrivacyWarning = "備份檔案內底包含你所有 ê 使用紀錄，佇 iOS 佮 Android 攏通用，準講換機仔嘛毋免煩惱。"
}
