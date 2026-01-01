import SwiftUI

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
