/// Dictionary metadata for info buttons in DictionaryTab.
/// Carries the i18n key for the source's description; the call site resolves it
/// via the active display language (`lang.string(info.descriptionKey)`).
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
