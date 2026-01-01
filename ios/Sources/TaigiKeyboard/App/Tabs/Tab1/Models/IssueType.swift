/// 已知問題類型
///
/// 定義頭頁顯示的處理中問題項目。
enum IssueType: Int, CaseIterable {
    case keyboardClipping = 0
    case missingWords = 1
    case toneConversion = 2

    var number: Int { rawValue + 1 }

    var title: LocalizedText {
        switch self {
        case .keyboardClipping: return Tab1Texts.issue1
        case .missingWords: return Tab1Texts.issue2
        case .toneConversion: return Tab1Texts.issue3
        }
    }

    var detailParagraphs: [LocalizedText] {
        switch self {
        case .keyboardClipping: return Tab1Texts.issue1Paragraphs
        case .missingWords: return Tab1Texts.issue2Paragraphs
        case .toneConversion: return Tab1Texts.issue3Paragraphs
        }
    }
}
