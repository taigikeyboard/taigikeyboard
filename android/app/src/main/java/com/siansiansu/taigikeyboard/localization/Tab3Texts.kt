package com.siansiansu.taigikeyboard.localization

/**
 * Tab3 詞庫文字
 * 包含：詞庫管理頁面
 * 對應 iOS Tab3Texts.swift
 */
object Tab3Texts {
    // MARK: - Tab 標題

    val tabTitle = LocalizedText(hanji = "詞庫管理")

    // MARK: - 通用按鍵

    val clear = LocalizedText(hanji = "清除")
    val delete = LocalizedText(hanji = "刪除")
    val ok = LocalizedText(hanji = "好")
    val save = LocalizedText(hanji = "儉起來")

    // MARK: - 詞庫管理

    val dictionarySettings = LocalizedText(hanji = "詞庫管理")
    val customDictionary = LocalizedText(hanji = "自訂詞庫")
    val customDictEnabled = LocalizedText(hanji = "啟用自訂詞庫")
    val customDictEnabledInfo = LocalizedText(hanji = "拍開了後，家己加入 ê 詞會出現佇候選詞內底。關起來了後，自訂詞就袂閣出現。")
    val dataManagement = LocalizedText(hanji = "個人資料")
    val variantDictionary = LocalizedText(hanji = "異用字")
    val khiin = LocalizedText(hanji = "在來字")

    // MARK: - 自訂詞庫

    val customDictDescription = LocalizedText(hanji = "自訂詞庫使用 CSV 純文字檔案，第 1 欄囥欲拍 ê 羅馬字，第 2 欄囥漢字，毋免囥標題。")
    val customDictEmpty = LocalizedText(hanji = "揤 + 符號加入自訂詞")
    val addEntry = LocalizedText(hanji = "增加詞")
    val editEntry = LocalizedText(hanji = "編輯詞")
    val romanLabel = LocalizedText(hanji = "拍字")
    val romanPlaceholder = LocalizedText(hanji = "見本：gâu-tsá")
    val hanziLabel = LocalizedText(hanji = "對應")
    val hanziPlaceholder = LocalizedText(hanji = "見本：𠢕早")
    val entriesCount = LocalizedText(hanji = "項")
    val deleteAll = LocalizedText(hanji = "刪除全部自訂詞")
    val deleteAllMessage = LocalizedText(hanji = "敢確定欲刪除所有自訂詞？")
    val customDictPrivacyWarning = LocalizedText(hanji = "請毋通佇自訂詞庫囥敏感 ê 個人資料，親像身分證字號、口座密碼、信用卡號碼，請注意家己 ê 資訊安全。")

    // MARK: - 匯入匯出

    val importCSV = LocalizedText(hanji = "匯入詞庫")
    val exportCSV = LocalizedText(hanji = "匯出詞庫")
    val exportSuccess = LocalizedText(hanji = "CSV 順利匯出")
    val importResult = LocalizedText(hanji = "匯入 %d 項成功，%d 項重複")
    val invalidCSVFormat = LocalizedText(hanji = "檔案格式無正確，請使用 CSV 格式")
    val fileTooLarge = LocalizedText(hanji = "檔案傷大（上限 5 MB）")
    val tooManyEntries = LocalizedText(hanji = "詞傷濟（上限 30,000 項）")
    val importExportTitle = LocalizedText(hanji = "匯出匯入")
    val importExportHelpTitle = LocalizedText(hanji = "匯出匯入說明")
    val importExportHelp = LocalizedText(hanji = "第 1 欄囥欲拍 ê 羅馬字，第 2 欄囥漢字、日本字，毋免囥標題")

    // MARK: - 詞頻匯入匯出

    val frequencyExportCSV = LocalizedText(hanji = "匯出詞頻紀錄")
    val frequencyImportCSV = LocalizedText(hanji = "匯入詞頻紀錄")
    val frequencyDescription = LocalizedText(hanji = "詞頻紀錄使用 CSV 純文字檔案，第 1 欄囥詞，第 2 欄囥次數，毋免囥標題。")
    val frequencyImportResult = LocalizedText(hanji = "匯入 %d 項成功，%d 項重複")

    // MARK: - 詞關聯匯入匯出

    val associationExportCSV = LocalizedText(hanji = "匯出詞關聯紀錄")
    val associationImportCSV = LocalizedText(hanji = "匯入詞關聯紀錄")
    val associationDescription = LocalizedText(hanji = "詞關聯紀錄使用 CSV 純文字檔案，共 5 欄：頭前詞、頭前拍字、後壁詞、後壁拍字、次數，毋免囥標題。")
    val associationImportResult = LocalizedText(hanji = "匯入 %d 項成功，%d 項重複")

    // MARK: - 詞庫區塊標題

    val moeSectionTitle = LocalizedText(hanji = "教育部用字")
    val otherSectionTitle = LocalizedText(hanji = "其他辭典")
    val supplementSectionTitle = LocalizedText(hanji = "補充資料")

    // MARK: - 詞庫名稱 (Tab3-only)

    val lkkDict = LocalizedText(hanji = "LKK漢羅合用建議用字")

    // MARK: - 詞庫搜尋

    val searchPlaceholder = LocalizedText(hanji = "拍字揣詞")
    val noResults = LocalizedText(hanji = "揣無結果")
    val customDictionarySource = LocalizedText(hanji = "自訂詞庫")

    // MARK: - 辭典查詢選項

    val lookupChhoe = LocalizedText(hanji = "ChhoeTaigi 辭典")
    val lookupMoe = LocalizedText(hanji = "教育部辭典")

    // MARK: - 常用詞管理

    val frequentWordsManagement = LocalizedText(hanji = "常用詞管理")
    val frequencyTab = LocalizedText(hanji = "詞頻")
    val frequencyManagement = LocalizedText(hanji = "詞頻紀錄")
    val frequencyRecordingEnabled = LocalizedText(hanji = "開啟詞頻紀錄")
    val frequencyRecordingEnabledInfo = LocalizedText(hanji = "拍開了後，齒盤會記錄你揀過 ê 詞幾擺，予候選詞排序做參考，定定揀 ê 詞就會排較頭前，按呢候選詞就會愈來愈準。")
    val associationTab = LocalizedText(hanji = "詞關聯")
    val associationManagement = LocalizedText(hanji = "詞關聯紀錄")
    val associationRecordingEnabled = LocalizedText(hanji = "開啟詞關聯紀錄")
    val associationRecordingEnabledInfo = LocalizedText(hanji = "拍開了後，齒盤會記錄頭前、後壁 ê 關聯詞，予連紲建議愈來愈準。")
    val totalEntries = LocalizedText(hanji = "共 %d / %d 筆")
    val frequencyPrivacyWarning = LocalizedText(hanji = "詞頻紀錄對台語研究來講是真有價值 ê 資料。若欲提供予人研究訓練模型，請先刪除敏感 ê 私人資料。紀錄功能嘛會使關起來，毋過按呢候選詞 ê 排序就會較無準。")
    val clearAllFrequency = LocalizedText(hanji = "刪除所有詞頻紀錄")
    val associationPrivacyWarning = LocalizedText(hanji = "詞關聯紀錄對台語研究來講是真有價值 ê 資料。若欲提供予人做研究，請先刪除敏感 ê 內容。紀錄功能嘛會使關起來，毋過後一詞預測會較無準。")
    val clearAllAssociation = LocalizedText(hanji = "刪除所有詞關聯紀錄")
    val clearFrequencyMessage = LocalizedText(hanji = "確定欲刪除所有詞頻紀錄？")
    val clearAssociationMessage = LocalizedText(hanji = "確定欲刪除所有詞關聯紀錄？")
    val noData = LocalizedText(hanji = "無資料")
    val filterHint = LocalizedText(hanji = "頂面上濟顯示 100 个詞，若揣無詞，請用下跤 ê「拍字揣詞」功能搜揣。")

    // MARK: - 備份還原

    val backupRestore = LocalizedText(hanji = "備份復原")
    val exportBackup = LocalizedText(hanji = "一擺全出")
    val importBackup = LocalizedText(hanji = "一擺全入")
    val exportBackupSuccess = LocalizedText(hanji = "備份順利匯出")
    val importBackupResult = LocalizedText(hanji = "匯入 %d 項自訂詞、%d 項詞頻、%d 項詞關聯")
    val backupHelpTitle = LocalizedText(hanji = "備份還原說明")
    val backupHelp = LocalizedText(hanji = "匯出所有使用者資料（自訂詞庫、詞頻、詞關聯），適用換機。")
    val backupPrivacyWarning = LocalizedText(hanji = "備份檔案內底包含你所有 ê 使用紀錄，佇 iOS 佮 Android 攏通用，準講換機仔嘛毋免煩惱。")
}
