import SwiftUI

extension View {
    /// 套用主題卡片樣式
    func themedCard(
        borderStyle: BorderStyle = ThemeBorder.standard,
        cornerRadius: CGFloat = 10
    ) -> some View {
        ThemeCard(borderStyle: borderStyle, cornerRadius: cornerRadius) {
            self
        }
    }

    /// 套用台語鍵盤按鈕樣式
    func taigiButtonStyle(accentColor: Color = Color.Theme.accent) -> some View {
        self.buttonStyle(TaigiButtonStyle(accentColor: accentColor))
    }

    /// 套用按壓回饋效果
    func pressable(_ isPressed: Binding<Bool>, scaleEffect: CGFloat = 0.95, minimumDistance: CGFloat = 30) -> some View {
        modifier(PressableModifier(isPressed: isPressed, scaleEffect: scaleEffect, minimumDistance: minimumDistance))
    }
}
