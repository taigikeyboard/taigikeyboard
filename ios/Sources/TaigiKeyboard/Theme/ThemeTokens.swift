import SwiftUI

/// SPY×FAMILY 復古主題色彩系統
extension Color {
    enum Theme {
        static let surfacePrimary = Color(red: 0.961, green: 0.941, blue: 0.910) // #f5f0e8 溫暖米色背景
        static let surfaceSecondary = Color(red: 0.984, green: 0.976, blue: 0.961) // #FBF9F5 卡片背景（溫暖白色）

        static let accent = Color(red: 0.553, green: 0.663, blue: 0.608) // #8da99b Loid's Teal Gray
        static let accentSecondary = Color(red: 0.380, green: 0.039, blue: 0.063) // #610a10 Yor's Deep Red
        static let accentTertiary = Color(red: 0.980, green: 0.702, blue: 0.678) // #fab3ad Anya's Warm Pink

        static let textPrimary = Color(red: 0.173, green: 0.157, blue: 0.153) // #2c2827 深棕灰
        static let textSecondary = Color(red: 0.341, green: 0.404, blue: 0.361) // #57675c 深青灰

        static let cardStroke = Color(red: 0.341, green: 0.404, blue: 0.361).opacity(0.2) // #57675c @ 20% opacity
    }
}

/// 主題字型系統
extension Font {
    enum Theme {
        static let heroTitle = Font.system(size: 34, weight: .bold, design: .rounded)
        static let title = Font.system(size: 24, weight: .semibold, design: .rounded)
        static let headline = Font.system(size: 18, weight: .semibold, design: .default)
        static let body = Font.system(size: 16, weight: .regular, design: .default)
        static let caption = Font.system(size: 14, weight: .medium, design: .default)
        static let footnote = Font.system(size: 12, weight: .regular, design: .default)
    }
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
