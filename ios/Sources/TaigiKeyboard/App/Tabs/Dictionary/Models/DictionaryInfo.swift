// DictionaryTab 詞典 info button 用的 metadata — 帶可切換顯示語言的說明文字 key。
// 每個 static 對應一个用 info 按鈕的詞典(iTaigi / 台日大辭典 …);說明文字經 i18n 解析。

/// Dictionary metadata for info buttons in DictionaryTab.
/// Carries the i18n key for the source's description; the call site resolves it
/// via the active display language (`lang.string(info.descriptionKey)`).
// 詞典資訊 model — 存說明文字的 StringKey,顯示時才依目前顯示語言解析。
struct DictionaryInfo {
    let descriptionKey: StringKey
}

extension DictionaryInfo {
    static let iTaigi = DictionaryInfo(descriptionKey: .dictionaryITaigiDescription)
    static let taiwanJapan = DictionaryInfo(descriptionKey: .dictionaryTaiwanJapanDescription)
    static let taiHua = DictionaryInfo(descriptionKey: .dictionaryTaiHuaDescription)
    static let taiwanPlant = DictionaryInfo(descriptionKey: .dictionaryTaiwanPlantDescription)
    static let variant = DictionaryInfo(descriptionKey: .dictionaryVariantDescription)
    static let khpoo = DictionaryInfo(descriptionKey: .dictionaryKhpooDescription)
    static let khiin = DictionaryInfo(descriptionKey: .dictionaryKhiinDescription)
}
