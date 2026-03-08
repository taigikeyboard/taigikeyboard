package com.siansiansu.taigikeyboard.localization

/**
 * Tab3 詞庫文字
 * 包含：詞庫管理頁面
 * 對應 iOS Tab3Texts.swift
 */
object Tab3Texts {

    // MARK: - Tab 標題

    val tabTitle = LocalizedText(hanji = "詞庫")

    // MARK: - 通用按鍵

    val cancel = LocalizedText(hanji = "取消")
    val clear = LocalizedText(hanji = "清除")
    val ok = LocalizedText(hanji = "好")
    val save = LocalizedText(hanji = "儉起來")

    // MARK: - 詞庫管理

    val dictionarySettings = LocalizedText(hanji = "詞庫管理")
    val customDictionary = LocalizedText(hanji = "自訂詞庫")
    val variantDictionary = LocalizedText(hanji = "異用字")

    // MARK: - 自訂詞庫

    val customDictEmpty = LocalizedText(hanji = "揤 + 符號加入自訂詞")
    val addEntry = LocalizedText(hanji = "增加詞")
    val editEntry = LocalizedText(hanji = "編輯詞")
    val romanLabel = LocalizedText(hanji = "拍字")
    val romanPlaceholder = LocalizedText(hanji = "範例：gâu-tsá")
    val hanziLabel = LocalizedText(hanji = "對應")
    val hanziPlaceholder = LocalizedText(hanji = "範例：𠢕早")
    val entriesCount = LocalizedText(hanji = "个")
    val deleteAll = LocalizedText(hanji = "刪除全部")
    val deleteAllMessage = LocalizedText(hanji = "敢確定欲刪除所有自訂詞？")

    // MARK: - 匯入匯出

    val importCSV = LocalizedText(hanji = "匯入 CSV")
    val exportCSV = LocalizedText(hanji = "匯出 CSV")
    val exportSuccess = LocalizedText(hanji = "CSV 順利匯出")
    val importResult = LocalizedText(hanji = "匯入 %d 个成功，%d 个跳過")
    val invalidCSVFormat = LocalizedText(hanji = "檔案格式無正確，請使用 CSV 格式")
    val importExportTitle = LocalizedText(hanji = "匯入匯出")
    val importExportHelpTitle = LocalizedText(hanji = "匯入匯出說明")
    val importExportHelp = LocalizedText(hanji = "第 1 欄囥輸入 ê 詞 (羅馬字), 第 2 欄囥對應 (漢字/日文字)，若第 1 行有標題，會自動跳過。")

    // MARK: - 清除資料

    val clearCache = LocalizedText(hanji = "挕掉捷用詞紀錄")
    val clearCacheMessage = LocalizedText(hanji = "這个動作會挕掉所有捷用詞 ê 記錄。敢欲繼續？")
    val clearCacheSuccess = LocalizedText(hanji = "已經挕掉")

    // MARK: - 詞庫名稱

    // 教育部臺灣台語常用詞辭典
    val moeDict = LocalizedText(hanji = "台語常用詞辭典")

    // 台語新詞辭庫
    val newwordDict = LocalizedText(hanji = "台語新詞辭庫")

    // 台語工藝詞庫
    val kunggeDict = LocalizedText(hanji = "台語工藝辭庫")

    // iTaigi 華台辭典
    val iTaigiDict = LocalizedText(hanji = "iTaigi愛台語")

    // 台日大辭典
    val taiwanJapanDict = LocalizedText(hanji = "台日大辭典")

    // 台華線頂對照典
    val taiHuaDict = LocalizedText(hanji = "台華線頂對照典")

    // 台灣植物名彙
    val taiwanPlantDict = LocalizedText(hanji = "台灣植物名彙")

    // 學科術語辭庫
    val sttiDict = LocalizedText(hanji = "學科術語辭庫")

    // 腔口補充資料
    val khpooDict = LocalizedText(hanji = "腔口補充資料")
}
