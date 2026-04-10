import Foundation

// MARK: - Tab3 詞庫文字

// 包含：詞庫管理頁面

enum Tab3Texts {
    // MARK: - Tab 標題

    static let tabTitle = LocalizedText(hanji: "詞庫管理")
    static let tabBarTitle = LocalizedText(hanji: "詞庫")

    // MARK: - 通用按鍵

    static let cancel = LocalizedText(hanji: "取消")
    static let clear = LocalizedText(hanji: "清除")

    // MARK: - 詞庫管理

    static let dictionarySettings = LocalizedText(hanji: "詞庫管理")
    static let customDictionary = LocalizedText(hanji: "自訂詞庫")
    static let isCustomDictEnabled = LocalizedText(hanji: "啟用自訂詞庫")
    static let isCustomDictEnabledInfo = LocalizedText(hanji: "拍開了後，家己加入 ê 詞會出現佇候選詞內底。關起來了後，自訂詞就袂閣出現。")
    static let dataManagement = LocalizedText(hanji: "個人資料")
    static let variantDictionary = LocalizedText(hanji: "異用字")
    static let khiin = LocalizedText(hanji: "在來字")

    // MARK: - 自訂詞庫

    static let customDictDescription = LocalizedText(hanji: "自訂詞庫使用 CSV 純文字檔案，第 1 欄囥欲拍 ê 羅馬字，第 2 欄囥漢字，毋免囥標題。")
    static let customDictEmpty = LocalizedText(hanji: "揤 + 符號加入自訂詞")
    static let addEntry = LocalizedText(hanji: "增加詞")
    static let editEntry = LocalizedText(hanji: "編輯詞")
    static let save = LocalizedText(hanji: "儉起來")
    static let ok = LocalizedText(hanji: "好")
    static let romanLabel = LocalizedText(hanji: "拍字")
    static let romanPlaceholder = LocalizedText(hanji: "見本：gâu-tsá")
    static let hanziLabel = LocalizedText(hanji: "對應")
    static let hanziPlaceholder = LocalizedText(hanji: "見本：𠢕早")
    static let entriesCount = LocalizedText(hanji: "項")
    static let deleteAll = LocalizedText(hanji: "刪除全部自訂詞")
    static let deleteAllMessage = LocalizedText(hanji: "敢確定欲刪除所有自訂詞？")
    static let customDictPrivacyWarning = LocalizedText(hanji: "請毋通佇自訂詞庫囥敏感 ê 個人資料，親像身分證字號、口座密碼、信用卡號碼，請注意家己 ê 資訊安全。")

    // MARK: - 匯入匯出

    static let importCSV = LocalizedText(hanji: "匯入詞庫")
    static let exportCSV = LocalizedText(hanji: "匯出詞庫")
    static let exportSuccess = LocalizedText(hanji: "CSV 順利匯出")
    static let importResult = LocalizedText(hanji: "匯入 %d 項成功，%d 項重複")
    static let invalidCSVFormat = LocalizedText(hanji: "檔案格式無正確，請使用 CSV 格式")
    static let fileTooLarge = LocalizedText(hanji: "檔案傷大（上限 5 MB）")
    static let tooManyEntries = LocalizedText(hanji: "詞傷濟（上限 30,000 項）")
    static let importExportTitle = LocalizedText(hanji: "匯出匯入")
    static let importExportHelpTitle = LocalizedText(hanji: "匯出匯入說明")
    static let importExportHelp = LocalizedText(hanji: "第 1 欄囥欲拍 ê 羅馬字，第 2 欄囥漢字、日本字，毋免囥標題")

    // MARK: - 詞頻匯入匯出

    static let frequencyExportCSV = LocalizedText(hanji: "匯出詞頻紀錄")
    static let frequencyImportCSV = LocalizedText(hanji: "匯入詞頻紀錄")
    static let frequencyDescription = LocalizedText(hanji: "詞頻紀錄使用 CSV 純文字檔案，第 1 欄囥詞，第 2 欄囥次數，毋免囥標題。")
    static let frequencyImportResult = LocalizedText(hanji: "匯入 %d 項成功，%d 項重複")

    // MARK: - 詞關聯匯入匯出

    static let associationExportCSV = LocalizedText(hanji: "匯出詞關聯紀錄")
    static let associationImportCSV = LocalizedText(hanji: "匯入詞關聯紀錄")
    static let associationDescription = LocalizedText(hanji: "詞關聯紀錄使用 CSV 純文字檔案，共 5 欄：頭前詞、頭前拍字、後壁詞、後壁拍字、次數，毋免囥標題。")
    static let associationImportResult = LocalizedText(hanji: "匯入 %d 項成功，%d 項重複")

    // MARK: - 詞庫資訊按鈕

    static let viewWebsite = LocalizedText(hanji: "官方網站")

    // MARK: - 詞庫區塊標題

    static let moeSectionTitle = LocalizedText(hanji: "教育部用字")
    static let otherSectionTitle = LocalizedText(hanji: "其他辭典")
    static let supplementSectionTitle = LocalizedText(hanji: "補充資料")

    // MARK: - 詞庫名稱

    /// 教育部臺灣台語常用詞辭典
    static let moeDict = LocalizedText(hanji: "教育部臺灣台語常用詞辭典")

    /// 台語新詞辭庫
    static let newwordDict = LocalizedText(hanji: "公視台語台台語新詞辭庫")

    /// 台語工藝詞庫
    static let kunggeDict = LocalizedText(hanji: "工藝中心臺灣台語工藝詞庫")

    /// iTaigi 華台辭典
    static let iTaigiDict = LocalizedText(hanji: "iTaigi愛台語")

    /// 臺日大辭典
    static let taiwanJapanDict = LocalizedText(hanji: "臺日大辭典台語譯本")

    /// 台華線頂對照典
    static let taiHuaDict = LocalizedText(hanji: "台華線頂對照典")

    /// 台灣植物名彙
    static let taiwanPlantDict = LocalizedText(hanji: "台灣植物名彙")

    /// 學科術語辭典
    static let sttiDict = LocalizedText(hanji: "教育部學科術語臺灣台語對譯")

    /// 腔口差
    static let khpooDict = LocalizedText(hanji: "腔口差")

    /// LKK漢羅合用建議用字
    static let lkkDict = LocalizedText(hanji: "LKK漢羅合用建議用字")

    // MARK: - 詞庫搜尋

    static let searchPlaceholder = LocalizedText(hanji: "拍字揣詞")
    static let noResults = LocalizedText(hanji: "揣無結果")
    static let customDictionarySource = LocalizedText(hanji: "自訂詞庫")

    // MARK: - 辭典查詢選項

    static let lookupChhoe = LocalizedText(hanji: "ChhoeTaigi 辭典")
    static let lookupMoe = LocalizedText(hanji: "教育部辭典")

    // MARK: - 常用詞管理

    static let frequentWordsManagement = LocalizedText(hanji: "常用詞管理")
    static let frequencyTab = LocalizedText(hanji: "詞頻")
    static let frequencyManagement = LocalizedText(hanji: "詞頻紀錄")
    static let isFrequencyRecordingEnabled = LocalizedText(hanji: "開啟詞頻紀錄")
    static let isFrequencyRecordingEnabledInfo = LocalizedText(hanji: "拍開了後，齒盤會記錄你揀過 ê 詞幾擺，予候選詞排序做參考，定定揀 ê 詞就會排較頭前，按呢候選詞就會愈來愈準。")
    static let associationTab = LocalizedText(hanji: "詞關聯")
    static let associationManagement = LocalizedText(hanji: "詞關聯紀錄")
    static let isAssociationRecordingEnabled = LocalizedText(hanji: "開啟詞關聯紀錄")
    static let isAssociationRecordingEnabledInfo = LocalizedText(hanji: "拍開了後，齒盤會記錄頭前、後壁 ê 關聯詞，予連紲建議愈來愈準。")
    static let totalEntries = LocalizedText(hanji: "共 %d / %d 筆")
    static let frequencyPrivacyWarning = LocalizedText(hanji: "詞頻紀錄對台語研究來講是真有價值 ê 資料。若欲提供予人研究訓練模型，請先刪除敏感 ê 私人資料。紀錄功能嘛會使關起來，毋過按呢候選詞 ê 排序就會較無準。")
    static let clearAllFrequency = LocalizedText(hanji: "刪除所有詞頻紀錄")
    static let associationPrivacyWarning = LocalizedText(hanji: "詞關聯紀錄對台語研究來講是真有價值 ê 資料。若欲提供予人做研究，請先刪除敏感 ê 內容。紀錄功能嘛會使關起來，毋過後一詞預測會較無準。")
    static let clearAllAssociation = LocalizedText(hanji: "刪除所有詞關聯紀錄")
    static let clearFrequencyMessage = LocalizedText(hanji: "確定欲刪除所有詞頻紀錄？")
    static let clearAssociationMessage = LocalizedText(hanji: "確定欲刪除所有詞關聯紀錄？")
    static let noData = LocalizedText(hanji: "無資料")
    static let filterHint = LocalizedText(hanji: "頂面上濟顯示 100 个詞，若揣無詞，請用下跤 ê「拍字揣詞」功能搜揣。")

    // MARK: - 備份還原

    static let backupRestore = LocalizedText(hanji: "備份復原")
    static let exportBackup = LocalizedText(hanji: "一擺全出")
    static let importBackup = LocalizedText(hanji: "一擺全入")
    static let exportBackupSuccess = LocalizedText(hanji: "備份順利匯出")
    static let importBackupResult = LocalizedText(hanji: "匯入 %d 項自訂詞、%d 項詞頻、%d 項詞關聯")
    static let backupHelpTitle = LocalizedText(hanji: "備份還原說明")
    static let backupHelp = LocalizedText(hanji: "匯出所有使用者資料（自訂詞庫、詞頻、詞關聯），適用換機。")
    static let backupPrivacyWarning = LocalizedText(hanji: "備份檔案內底包含你所有 ê 使用紀錄，佇 iOS 佮 Android 攏通用，準講換機仔嘛毋免煩惱。")
}
