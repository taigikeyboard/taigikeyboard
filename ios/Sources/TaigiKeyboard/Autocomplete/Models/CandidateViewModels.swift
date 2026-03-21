import SwiftUI

/// 候選詞視圖常數
///
/// 定義候選詞視圖的 UI 尺寸、間距、顏色等配置。
enum CandidateViewModels {
    enum UI {
        static let buttonSpacing: CGFloat = 14
        static let maxDisplayCount: Int = 100
        static let baseHeight: CGFloat = 50
        /// Extra height to prevent bottom clipping from content offset
        private static let bottomPadding: CGFloat = 6
        static var height: CGFloat {
            baseHeight * SharedSettings.shared.candidateTextSizeScale + bottomPadding
        }

        // Base font sizes before screen-size adaptation and user scale
        static let basePrimaryFontSize: CGFloat = 20
        static let baseSecondaryFontSize: CGFloat = 15

        /// 主標題字體大小（根據螢幕尺寸自適應 + user scale）
        static var primaryFontSize: CGFloat {
            let base: CGFloat = switch ScreenSizeClass.current {
            case .phoneCompact:  basePrimaryFontSize
            case .phoneRegular:  21
            case .phoneLarge:    basePrimaryFontSize
            case .pad:           23
            }
            return base * SharedSettings.shared.candidateTextSizeScale
        }

        /// 副標題字體大小（根據螢幕尺寸自適應 + user scale）
        static var secondaryFontSize: CGFloat {
            let base: CGFloat = switch ScreenSizeClass.current {
            case .phoneCompact:  baseSecondaryFontSize
            case .phoneRegular:  16
            case .phoneLarge:    baseSecondaryFontSize
            case .pad:           17
            }
            return base * SharedSettings.shared.candidateTextSizeScale
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

        static var primaryTextColor: Color {
            SharedSettings.shared.colorSettings.candidateTextColor?.color ?? Color(.label)
        }

        static var secondaryTextColor: Color {
            if let custom = SharedSettings.shared.colorSettings.candidateTextColor?.color {
                return custom.opacity(0.7)
            }
            return Color(.secondaryLabel)
        }
    }
}
