import SwiftUI

enum CandidateViewModels {
    enum UI {
        static let buttonSpacing: CGFloat = 14
        static let maxDisplayCount: Int = 100
        static let height: CGFloat = 50 // 增加高度改善視覺比例
        static let primaryFontSize: CGFloat = 21
        static let secondaryFontSize: CGFloat = 16 // 放大 subtitle 字體提高可讀性

        // 展開網格視圖配置
        static let expandedRowSpacing: CGFloat = 6 // 減少行間距 (從12減到10)
        static let expandedItemSpacing: CGFloat = 4
        static let expandedMinRowHeight: CGFloat = 35 // 減少最小行高 (從44減到42)
        static let expandedButtonVerticalPadding: CGFloat = 7 // 減少垂直內邊距 (從8減到7)
    }

    enum Spacing {
        static let small: CGFloat = 2
    }

    enum Colors {
        static let separatorColor: Color = .init(.separator)
        static let primaryTextColor: Color = .init(.label)
        static let secondaryTextColor: Color = .init(.secondaryLabel) // 加深顏色提高可讀性
    }
}
