import SwiftUI

/// 操作說明關閉按鈕
struct UserGuideCloseButton: View {
    let action: () -> Void
    @State private var isPressed = false

    var body: some View {
        Button(action: action) {
            ZStack {
                Circle()
                    .fill(Color.Theme.surfaceSecondary)
                    .overlay(
                        Circle()
                            .stroke(Color.Theme.cardStroke, lineWidth: 1)
                    )
                    .frame(width: 36, height: 36)

                Image(systemName: "xmark")
                    .font(.system(size: 16, weight: .semibold, design: .rounded))
                    .foregroundColor(Color.Theme.textPrimary)
            }
        }
        .pressable($isPressed)
        .animation(ThemeAnimation.quick, value: isPressed)
        .accessibilityLabel("關閉")
        .accessibilityHint("關閉操作說明視窗")
    }
}
