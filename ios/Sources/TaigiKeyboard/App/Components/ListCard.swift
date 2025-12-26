import SwiftUI

// MARK: - Pressable Card Gesture

struct PressableCardGesture: ViewModifier {
    let isPressed: Bool
    let onPressChanged: (Bool) -> Void

    func body(content: Content) -> some View {
        content
            .simultaneousGesture(
                DragGesture(minimumDistance: 30)
                    .onChanged { _ in
                        withAnimation(ThemeAnimation.press) {
                            onPressChanged(true)
                        }
                    }
                    .onEnded { _ in
                        withAnimation(ThemeAnimation.release) {
                            onPressChanged(false)
                        }
                    }
            )
    }
}

extension View {
    func pressableCardGesture(isPressed: Bool, onPressChanged: @escaping (Bool) -> Void) -> some View {
        modifier(PressableCardGesture(isPressed: isPressed, onPressChanged: onPressChanged))
    }
}

