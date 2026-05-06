// 中文: DictionaryTab(詞庫管理)所有 UI 文字常數。包含詞庫切換、CSV 匯入/匯出、
// 中文: 詞頻、詞關聯、備份還原、隱私警語等所有 row 顯示字串。

// MARK: - DictionaryTab 詞庫文字

// 包含：詞庫管理頁面

// 中文: 詞庫管理頁文字 namespace。每個 MARK 子區塊對應 UI 上的一個 Section/子頁。
enum DictionaryTexts {
    // MARK: - Tab 標題

    static let tabTitle = "詞庫管理"
    static let tabBarTitle = "詞庫"

    // MARK: - 通用按鍵

    static let clear = "清除"

    // MARK: - 詞庫管理

    static let customDictionary = "自訂詞庫"
    static let isCustomDictEnabled = "啟用自訂詞庫"
    static let isCustomDictEnabledInfo = "拍開了後，家己加入 ê 詞會出現佇候選詞內底。關起來了後，自訂詞就袂閣出現。"
    static let dataManagement = "個人資料"
    static let variantDictionary = "異用字"
    static let khiin = "在來字"

    // MARK: - 自訂詞庫

    static let customDictDescription = "自訂詞庫使用 CSV 純文字檔案，第 1 欄囥欲拍 ê 羅馬字，第 2 欄囥漢字，毋免囥標題。"
    static let customDictEmpty = "揤 + 符號加入自訂詞"
    static let addEntry = "增加詞"
    static let editEntry = "編輯詞"
    static let save = "儉起來"
    static let ok = "好"
    static let romanLabel = "拍字"
    static let romanPlaceholder = "見本：gâu-tsá"
    static let hanziLabel = "對應"
    static let hanziPlaceholder = "見本：𠢕早"
    static let deleteAll = "刪除全部自訂詞"
    static let deleteAllMessage = "敢確定欲刪除所有自訂詞？"
    static let customDictPrivacyWarning = "請毋通佇自訂詞庫囥敏感 ê 個人資料，親像身分證字號、口座密碼、信用卡號碼，請注意家己 ê 資訊安全。"

    // MARK: - 匯入匯出

    static let importCSV = "匯入詞庫"
    static let exportCSV = "匯出詞庫"
    static let exportSuccess = "CSV 順利匯出"
    static let importResultFormat = "匯入 %d 項成功，%d 項重複"
    static let invalidCSVFormat = "檔案格式無正確，請使用 CSV 格式"
    static let fileTooLarge = "檔案傷大（上限 5 MB）"
    static let tooManyEntries = "詞傷濟（上限 30,000 項）"
    static let importExportTitle = "匯出匯入"

    // MARK: - 詞頻匯入匯出

    static let frequencyExportCSV = "匯出詞頻紀錄"
    static let frequencyImportCSV = "匯入詞頻紀錄"
    static let frequencyDescription = "詞頻紀錄使用 CSV 純文字檔案，第 1 欄囥詞，第 2 欄囥次數，毋免囥標題。"

    // MARK: - 詞關聯匯入匯出

    static let associationExportCSV = "匯出詞關聯紀錄"
    static let associationImportCSV = "匯入詞關聯紀錄"
    static let associationDescription = "詞關聯紀錄使用 CSV 純文字檔案，共 5 欄：頭前詞、頭前拍字、後壁詞、後壁拍字、次數，毋免囥標題。"

    // MARK: - 詞庫區塊標題

    static let moeSectionTitle = "教育部用字"
    static let otherSectionTitle = "其他辭典"
    static let supplementSectionTitle = "補充資料"

    // MARK: - 詞庫名稱

    static let lkkDict = "漢羅合用建議用字"

    // MARK: - 詞庫搜尋

    static let searchPlaceholder = "拍字揣詞"
    static let noResults = "揣無結果"

    // MARK: - 辭典查詢選項

    static let lookupChhoe = "ChhoeTaigi 辭典"
    static let lookupMoe = "教育部辭典"

    // MARK: - 常用詞管理

    static let frequencyManagement = "詞頻紀錄"
    static let isFrequencyRecordingEnabled = "開啟詞頻紀錄"
    static let isFrequencyRecordingEnabledInfo = "拍開了後，齒盤會記錄你揀過 ê 詞幾擺，予候選詞排序做參考，定定揀 ê 詞就會排較頭前，按呢候選詞就會愈來愈準。"
    static let associationManagement = "詞關聯紀錄"
    static let isAssociationRecordingEnabled = "開啟詞關聯紀錄"
    static let isAssociationRecordingEnabledInfo = "拍開了後，齒盤會記錄頭前、後壁 ê 關聯詞，予連紲建議愈來愈準。"
    static let frequencyPrivacyWarning = "詞頻紀錄對台語研究來講是真有價值 ê 資料。若欲提供予人研究訓練模型，請先刪除敏感 ê 私人資料。紀錄功能嘛會使關起來，毋過按呢候選詞 ê 排序就會較無準。"
    static let clearAllFrequency = "刪除所有詞頻紀錄"
    static let associationPrivacyWarning = "詞關聯紀錄對台語研究來講是真有價值 ê 資料。若欲提供予人做研究，請先刪除敏感 ê 內容。紀錄功能嘛會使關起來，毋過後一詞預測會較無準。"
    static let clearAllAssociation = "刪除所有詞關聯紀錄"
    static let clearFrequencyMessage = "確定欲刪除所有詞頻紀錄？"
    static let clearAssociationMessage = "確定欲刪除所有詞關聯紀錄？"
    static let noData = "無資料"
    static let filterHint = "頂面上濟顯示 100 个詞，若揣無詞，請用下跤 ê「拍字揣詞」功能搜揣。"

    // MARK: - 備份還原

    static let backupRestore = "備份復原"
    static let exportBackup = "一擺全出"
    static let importBackup = "一擺全入"
    static let exportBackupSuccess = "備份順利匯出"
    static let importBackupResult = "匯入 %d 項自訂詞、%d 項詞頻、%d 項詞關聯"
    static let backupPrivacyWarning = "備份檔案內底包含你所有 ê 使用紀錄，佇 iOS 佮 Android 攏通用，準講換機仔嘛毋免煩惱。"
}
