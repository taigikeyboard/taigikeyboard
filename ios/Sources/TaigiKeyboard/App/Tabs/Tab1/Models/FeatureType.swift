/// 新功能類型
///
/// 定義頭頁顯示的新功能項目。
enum FeatureType: Int, CaseIterable {
    case nextWord = 0
    case variant = 1
    case customFont = 2
    case userDict = 3
    case caseSwitch = 4

    var title: LocalizedText {
        switch self {
        case .nextWord: return Tab1Texts.featureNextWord
        case .variant: return Tab1Texts.featureVariant
        case .customFont: return Tab1Texts.featureCustomFont
        case .userDict: return Tab1Texts.featureUserDict
        case .caseSwitch: return Tab1Texts.featureCaseSwitch
        }
    }

    var icon: String {
        switch self {
        case .nextWord: return "lightbulb.max"
        case .variant: return "arrow.left.arrow.right"
        case .customFont: return "book.fill"
        case .userDict: return "externaldrive.badge.person.crop"
        case .caseSwitch: return "textformat.size"
        }
    }

    var detailParagraphs: [LocalizedText] {
        switch self {
        case .nextWord: return Tab1Texts.featureNextWordParagraphs
        case .variant: return Tab1Texts.featureVariantParagraphs
        case .customFont: return Tab1Texts.featureCustomFontParagraphs
        case .userDict: return Tab1Texts.featureUserDictParagraphs
        case .caseSwitch: return Tab1Texts.featureCaseSwitchParagraphs
        }
    }
}
