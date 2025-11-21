import SwiftUI

/// 互動式按鈕樣式（扁平無陰影）
struct TaigiButtonStyle: ButtonStyle {
    let accentColor: Color

    init(accentColor: Color = Color.Theme.accent) {
        self.accentColor = accentColor
    }

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .scaleEffect(configuration.isPressed ? 0.96 : 1.0)
            .opacity(configuration.isPressed ? 0.9 : 1.0)
            .animation(
                configuration.isPressed ? ThemeAnimation.press : ThemeAnimation.release,
                value: configuration.isPressed
            )
    }
}

/// 提供按壓時的視覺回饋效果
struct PressableModifier: ViewModifier {
    @Binding var isPressed: Bool
    let scaleEffect: CGFloat
    let minimumDistance: CGFloat

    init(isPressed: Binding<Bool>, scaleEffect: CGFloat = 0.95, minimumDistance: CGFloat = 30) {
        _isPressed = isPressed
        self.scaleEffect = scaleEffect
        self.minimumDistance = minimumDistance
    }

    func body(content: Content) -> some View {
        content
            .scaleEffect(isPressed ? scaleEffect : 1.0)
            .simultaneousGesture(
                DragGesture(minimumDistance: minimumDistance)
                    .onChanged { _ in
                        withAnimation(ThemeAnimation.press) {
                            isPressed = true
                        }
                    }
                    .onEnded { _ in
                        withAnimation(ThemeAnimation.release) {
                            isPressed = false
                        }
                    }
            )
    }
}
