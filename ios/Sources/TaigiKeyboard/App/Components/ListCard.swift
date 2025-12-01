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

// MARK: - List Card View

struct ListCardView: View {
    let title: LocalizedText
    let isPressed: Bool
    let isLast: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 16) {
                LocalizedTextView(title)
                    .font(Font.Theme.headline)
                    .foregroundColor(Color.Theme.textPrimary)

                Spacer()

                Image(systemName: "chevron.right")
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundColor(Color.Theme.textSecondary)
                    .opacity(0.5)
                    .offset(x: isPressed ? 2 : 0)
                    .animation(ThemeAnimation.smooth, value: isPressed)
            }
            .padding(.horizontal, 24)
            .padding(.vertical, 20)
            .background(
                Rectangle()
                    .fill(isPressed ? Color.Theme.textSecondary.opacity(0.05) : Color.clear)
                    .animation(ThemeAnimation.quick, value: isPressed)
            )
            .overlay(
                // Separator line for non-last items
                VStack {
                    Spacer()
                    if !isLast {
                        Rectangle()
                            .fill(Color.Theme.textSecondary.opacity(0.1))
                            .frame(height: 0.5)
                            .padding(.leading, 24)
                    }
                }
            )
            .contentShape(Rectangle())
            .frame(minHeight: 48)
            .scaleEffect(isPressed ? 0.98 : 1.0)
            .animation(ThemeAnimation.smooth, value: isPressed)
        }
        .buttonStyle(PlainButtonStyle())
    }
}
