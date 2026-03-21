import Foundation

// MARK: - Tab3 詞庫文字
// 包含：詞庫管理頁面

enum Tab3Texts {

    // MARK: - Tab 標題

    static let tabTitle = LocalizedText(hanji: "詞庫")

    // MARK: - 通用按鍵

    static let cancel = LocalizedText(hanji: "取消")
    static let clear = LocalizedText(hanji: "清除")

    // MARK: - 詞庫管理

    static let dictionarySettings = LocalizedText(hanji: "詞庫管理")
    static let customDictionary = LocalizedText(hanji: "自訂詞庫")
    static let variantDictionary = LocalizedText(hanji: "異用字")
    static let khiin = LocalizedText(hanji: "在來字")

    // MARK: - 自訂詞庫

    static let customDictEmpty = LocalizedText(hanji: "揤 + 符號加入自訂詞")
    static let addEntry = LocalizedText(hanji: "增加詞")
    static let editEntry = LocalizedText(hanji: "編輯詞")
    static let save = LocalizedText(hanji: "儉起來")
    static let ok = LocalizedText(hanji: "好")
    static let romanLabel = LocalizedText(hanji: "拍字")
    static let romanPlaceholder = LocalizedText(hanji: "見本：gâu-tsá")
    static let hanziLabel = LocalizedText(hanji: "對應")
    static let hanziPlaceholder = LocalizedText(hanji: "見本：𠢕早")
    static let entriesCount = LocalizedText(hanji: "个")
    static let deleteAll = LocalizedText(hanji: "刪除全部")
    static let deleteAllMessage = LocalizedText(hanji: "敢確定欲刪除所有自訂詞？")

    // MARK: - 匯入匯出

    static let importCSV = LocalizedText(hanji: "匯入 CSV")
    static let exportCSV = LocalizedText(hanji: "匯出 CSV")
    static let exportSuccess = LocalizedText(hanji: "CSV 順利匯出")
    static let importResult = LocalizedText(hanji: "匯入 %d 个成功，%d 个跳過")
    static let invalidCSVFormat = LocalizedText(hanji: "檔案格式無正確，請使用 CSV 格式")
    static let importExportTitle = LocalizedText(hanji: "匯入匯出")
    static let importExportHelpTitle = LocalizedText(hanji: "匯入匯出說明")
    static let importExportHelp = LocalizedText(hanji: "第 1 欄囥輸入 ê 詞 (羅馬字), 第 2 欄囥對應 (漢字/日文字)，若第 1 行有標題，會自動跳過。")

    // MARK: - 清除資料

    static let clearCache = LocalizedText(hanji: "挕掉捷用詞紀錄")
    static let clearCacheMessage = LocalizedText(hanji: "這个動作會挕掉所有捷用詞 ê 記錄。敢欲繼續？")
    static let clearCacheSuccess = LocalizedText(hanji: "已經挕掉")

    // MARK: - 詞庫資訊按鈕

    static let viewWebsite = LocalizedText(hanji: "官方網站")

    // MARK: - 詞庫區塊標題

    static let moeSectionTitle = LocalizedText(hanji: "教育部用字")
    static let otherSectionTitle = LocalizedText(hanji: "其他辭典")
    static let supplementSectionTitle = LocalizedText(hanji: "補充資料")

    // MARK: - 詞庫名稱

    // 教育部臺灣台語常用詞辭典
    static let moeDict = LocalizedText(hanji: "教育部臺灣台語常用詞辭典")

    // 台語新詞辭庫
    static let newwordDict = LocalizedText(hanji: "公視台語台台語新詞辭庫")

    // 台語工藝詞庫
    static let kunggeDict = LocalizedText(hanji: "工藝中心臺灣台語工藝詞庫")

    // iTaigi 華台辭典
    static let iTaigiDict = LocalizedText(hanji: "iTaigi愛台語")

    // 臺日大辭典
    static let taiwanJapanDict = LocalizedText(hanji: "臺日大辭典台語譯本")

    // 台華線頂對照典
    static let taiHuaDict = LocalizedText(hanji: "台華線頂對照典")

    // 台灣植物名彙
    static let taiwanPlantDict = LocalizedText(hanji: "台灣植物名彙")

    // 學科術語辭典
    static let sttiDict = LocalizedText(hanji: "教育部學科術語臺灣台語對譯")

    // 腔口差
    static let khpooDict = LocalizedText(hanji: "腔口差")

    // LKK漢羅合用建議用字
    static let lkkDict = LocalizedText(hanji: "LKK漢羅合用建議用字")

    // MARK: - 詞庫搜尋

    static let searchPlaceholder = LocalizedText(hanji: "拍字揣詞")
    static let noResults = LocalizedText(hanji: "揣無結果")
    static let customDictionarySource = LocalizedText(hanji: "自訂詞庫")

    // MARK: - 辭典查詢選項

    static let lookupChhoe = LocalizedText(hanji: "ChhoeTaigi 辭典")
    static let lookupMoe = LocalizedText(hanji: "教育部辭典")

}
