/// 預計功能類型
///
/// 定義頭頁顯示的預計新功能項目。
enum UpcomingType: Int, CaseIterable {
    case voiceInput = 0
    case dictionaryExport = 1
    case themeCustomization = 2

    var number: Int { rawValue + 1 }

    var title: LocalizedText {
        switch self {
        case .voiceInput: return Tab1Texts.upcoming1
        case .dictionaryExport: return Tab1Texts.upcoming2
        case .themeCustomization: return Tab1Texts.upcoming3
        }
    }

    var detailParagraphs: [LocalizedText] {
        switch self {
        case .voiceInput: return Tab1Texts.upcoming1Paragraphs
        case .dictionaryExport: return Tab1Texts.upcoming2Paragraphs
        case .themeCustomization: return Tab1Texts.upcoming3Paragraphs
        }
    }
}
