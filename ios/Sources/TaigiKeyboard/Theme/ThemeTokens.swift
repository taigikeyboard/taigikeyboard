import SwiftUI
import UIKit

/// SPY×FAMILY 復古主題色彩系統（支援 Light/Dark Mode）
extension Color {
    enum Theme {
        // MARK: - 背景色

        /// 主要背景色（Light: 米色, Dark: 深灰）
        static var surfacePrimary: Color {
            Color(UIColor { trait in
                trait.userInterfaceStyle == .dark
                    ? UIColor(red: 0.11, green: 0.11, blue: 0.12, alpha: 1)  // #1c1c1e
                    : UIColor(red: 0.961, green: 0.941, blue: 0.910, alpha: 1)  // #f5f0e8
            })
        }

        /// 次要背景色 / 卡片背景（Light: 淺白, Dark: 卡片深灰）
        static var surfaceSecondary: Color {
            Color(UIColor { trait in
                trait.userInterfaceStyle == .dark
                    ? UIColor(red: 0.17, green: 0.17, blue: 0.18, alpha: 1)  // #2c2c2e
                    : UIColor(red: 0.984, green: 0.976, blue: 0.961, alpha: 1)  // #fbf9f5
            })
        }

        // MARK: - 強調色

        /// 主要強調色（青灰色，Dark Mode 稍亮）
        static var accent: Color {
            Color(UIColor { trait in
                trait.userInterfaceStyle == .dark
                    ? UIColor(red: 0.545, green: 0.651, blue: 0.592, alpha: 1)  // #8ba697
                    : UIColor(red: 0.431, green: 0.541, blue: 0.490, alpha: 1)  // #6e8a7d
            })
        }

        /// 次要強調色（深紅色，Dark Mode 稍亮）
        static var accentSecondary: Color {
            Color(UIColor { trait in
                trait.userInterfaceStyle == .dark
                    ? UIColor(red: 0.85, green: 0.35, blue: 0.35, alpha: 1)  // #d95959
                    : UIColor(red: 0.380, green: 0.039, blue: 0.063, alpha: 1)  // #610a10
            })
        }

        /// 第三強調色（粉紅色）
        static var accentTertiary: Color {
            Color(UIColor { trait in
                trait.userInterfaceStyle == .dark
                    ? UIColor(red: 0.98, green: 0.75, blue: 0.73, alpha: 1)  // #fabeба
                    : UIColor(red: 0.980, green: 0.702, blue: 0.678, alpha: 1)  // #fab3ad
            })
        }

        // MARK: - 文字色

        /// 主要文字色（Light: 深棕灰, Dark: 白色）
        static var textPrimary: Color {
            Color(UIColor { trait in
                trait.userInterfaceStyle == .dark
                    ? UIColor(red: 1.0, green: 1.0, blue: 1.0, alpha: 1)  // #ffffff
                    : UIColor(red: 0.173, green: 0.157, blue: 0.153, alpha: 1)  // #2c2827
            })
        }

        /// 次要文字色（Light: 深青灰, Dark: 系統灰）
        static var textSecondary: Color {
            Color(UIColor { trait in
                trait.userInterfaceStyle == .dark
                    ? UIColor(red: 0.556, green: 0.556, blue: 0.576, alpha: 1)  // #8e8e93
                    : UIColor(red: 0.341, green: 0.404, blue: 0.361, alpha: 1)  // #57675c
            })
        }

        // MARK: - 邊框色

        /// 卡片邊框色
        static var cardStroke: Color {
            Color(UIColor { trait in
                trait.userInterfaceStyle == .dark
                    ? UIColor(red: 1.0, green: 1.0, blue: 1.0, alpha: 0.15)  // 白色 15%
                    : UIColor(red: 0.341, green: 0.404, blue: 0.361, alpha: 0.2)  // #57675c 20%
            })
        }
    }
}

/// 主題字型系統（靜態版本，用於不需要動態更新的場景）
extension Font {
    enum Theme {
        static var heroTitle: Font { FontManager.shared.font(size: 34) }
        static var title: Font { FontManager.shared.font(size: 24) }
        static var headline: Font { FontManager.shared.font(size: 18) }
        static var body: Font { FontManager.shared.font(size: 17) }
        static var caption: Font { FontManager.shared.font(size: 14) }
        static var footnote: Font { FontManager.shared.font(size: 12) }
    }
}

/// 動態主題字體 Modifier（會響應字體設定變化）
struct ThemeFontModifier: ViewModifier {
    @ObservedObject private var fontManager = FontManager.shared
    let size: CGFloat

    func body(content: Content) -> some View {
        content.font(fontManager.font(size: size))
    }
}

extension View {
    /// 套用動態主題字體（會響應字體設定變化）
    func themeFont(size: CGFloat) -> some View {
        modifier(ThemeFontModifier(size: size))
    }

    /// 預設主題字體大小
    func themeFontHeroTitle() -> some View { themeFont(size: 34) }
    func themeFontTitle() -> some View { themeFont(size: 24) }
    func themeFontHeadline() -> some View { themeFont(size: 18) }
    func themeFontBody() -> some View { themeFont(size: 17) }
    func themeFontCaption() -> some View { themeFont(size: 14) }
    func themeFontFootnote() -> some View { themeFont(size: 12) }
}

/// 主題動畫系統
struct ThemeAnimation {
    static let quick = Animation.spring(response: 0.3, dampingFraction: 0.8)
    static let smooth = Animation.spring(response: 0.5, dampingFraction: 0.8)
    static let bouncy = Animation.spring(response: 0.4, dampingFraction: 0.6)
    static let press = Animation.spring(response: 0.2, dampingFraction: 0.7)
    static let release = Animation.spring(response: 0.4, dampingFraction: 0.8)
}

/// 邊框樣式定義
struct BorderStyle {
    let color: Color
    let width: CGFloat
}

/// 主題邊框系統
struct ThemeBorder {
    static let standard = BorderStyle(
        color: Color.Theme.cardStroke,
        width: 1
    )
}
