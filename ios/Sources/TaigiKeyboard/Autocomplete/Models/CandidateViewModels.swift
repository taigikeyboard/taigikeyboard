import SwiftUI

/// 候選詞視圖常數
///
/// 視圖無關的固定幾何參數（不受使用者外觀設定影響）。
/// 受 `candidateTextSizeScale` / `colorSettings` 影響的動態值改由
/// `CandidateTheme`（透過 SwiftUI environment）提供。
enum CandidateViewModels {
    enum UI {
        static let buttonSpacing: CGFloat = 14
        static let maxDisplayCount: Int = 200

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
    }
}
