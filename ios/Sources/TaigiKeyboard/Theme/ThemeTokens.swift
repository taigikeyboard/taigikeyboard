import SwiftUI

/// SPY×FAMILY 復古主題色彩系統
extension Color {
    enum Theme {
        static let surfacePrimary = Color(red: 0.961, green: 0.941, blue: 0.910) // #f5f0e8 溫暖米色背景
        static let surfaceSecondary = Color(red: 0.984, green: 0.976, blue: 0.961) // #FBF9F5 卡片背景（溫暖白色）

        static let accent = Color(red: 0.431, green: 0.541, blue: 0.490) // #6e8a7d Loid's Teal Gray
        static let accentSecondary = Color(red: 0.380, green: 0.039, blue: 0.063) // #610a10 Yor's Deep Red
        static let accentTertiary = Color(red: 0.980, green: 0.702, blue: 0.678) // #fab3ad Anya's Warm Pink

        static let textPrimary = Color(red: 0.173, green: 0.157, blue: 0.153) // #2c2827 深棕灰
        static let textSecondary = Color(red: 0.341, green: 0.404, blue: 0.361) // #57675c 深青灰

        static let cardStroke = Color(red: 0.341, green: 0.404, blue: 0.361).opacity(0.2) // #57675c @ 20% opacity
    }
}

/// 主題字型系統（靜態版本，用於不需要動態更新的場景）
extension Font {
    enum Theme {
        static var heroTitle: Font { FontManager.shared.font(size: 34) }
        static var title: Font { FontManager.shared.font(size: 24) }
        static var headline: Font { FontManager.shared.font(size: 18) }
        static var body: Font { FontManager.shared.font(size: 16) }
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
    func themeFontBody() -> some View { themeFont(size: 16) }
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
