import SwiftUI

/// 候選詞視圖常數
///
/// 定義候選詞視圖的 UI 尺寸、間距、顏色等配置。
enum CandidateViewModels {
    enum UI {
        static let buttonSpacing: CGFloat = 14
        static let maxDisplayCount: Int = 100
        static let height: CGFloat = 50 // 增加高度改善視覺比例

        /// 主標題字體大小（根據螢幕尺寸自適應）
        static var primaryFontSize: CGFloat {
            switch ScreenSizeClass.current {
            case .phoneCompact:  return 20
            case .phoneRegular:  return 21
            case .phoneLarge:    return 20  // 大螢幕稍小，視覺比例更協調
            case .pad:           return 23
            }
        }

        /// 副標題字體大小（根據螢幕尺寸自適應）
        static var secondaryFontSize: CGFloat {
            switch ScreenSizeClass.current {
            case .phoneCompact:  return 15
            case .phoneRegular:  return 16
            case .phoneLarge:    return 15
            case .pad:           return 17
            }
        }

        // MARK: - TPS 專用字體大小（方音符號較大，需縮小）

        /// TPS 縮放比例（方音符號視覺上較大，縮小 15%）
        private static let tpsScale: CGFloat = 0.85

        /// TPS 主標題字體大小
        static var tpsPrimaryFontSize: CGFloat {
            primaryFontSize * tpsScale
        }

        /// TPS 副標題字體大小
        static var tpsSecondaryFontSize: CGFloat {
            secondaryFontSize * tpsScale
        }

        /// 長詞主標題字體大小（比一般候選詞稍小）
        static var longCellPrimaryFontSize: CGFloat {
            return primaryFontSize - 1
        }

        /// 長詞副標題字體大小（比一般候選詞稍小）
        static var longCellSecondaryFontSize: CGFloat {
            return secondaryFontSize - 2
        }

        /// TPS 長詞主標題字體大小
        static var tpsLongCellPrimaryFontSize: CGFloat {
            longCellPrimaryFontSize * tpsScale
        }

        /// TPS 長詞副標題字體大小
        static var tpsLongCellSecondaryFontSize: CGFloat {
            longCellSecondaryFontSize * tpsScale
        }

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
