/// FAQ 類型
///
/// 定義頭頁顯示的常見問題項目。
enum FAQType: Int, CaseIterable {
    case installIssue = 0
    case feedback = 1
    case toneHandling = 2

    var question: LocalizedText {
        switch self {
        case .installIssue: return Tab1Texts.faq1Question
        case .feedback: return Tab1Texts.faq2Question
        case .toneHandling: return Tab1Texts.faq3Question
        }
    }

    var icon: String {
        switch self {
        case .installIssue: return "keyboard.badge.ellipsis"
        case .feedback: return "envelope"
        case .toneHandling: return "textformat.123"
        }
    }

    var answerParagraphs: [LocalizedText] {
        switch self {
        case .installIssue: return Tab1Texts.faq1Paragraphs
        case .feedback: return Tab1Texts.faq2Paragraphs
        case .toneHandling: return Tab1Texts.faq3Paragraphs
        }
    }
}
