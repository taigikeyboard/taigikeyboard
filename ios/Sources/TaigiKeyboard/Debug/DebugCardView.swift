import SwiftUI

/// Debug Zone 入口卡片（僅 DEBUG 模式顯示）
struct DebugCardView: View {
    @Binding var showDebug: Bool
    @State private var isPressed = false

    var body: some View {
        VStack(spacing: 0) {
            Button(action: {
                withAnimation(ThemeAnimation.smooth) {
                    showDebug = true
                }
            }) {
                HStack {
                    Image(systemName: "ladybug")
                        .font(.system(size: 20))
                        .foregroundColor(Color.Theme.accent)
                        .frame(width: 32)

                    Text("Debug Zone")
                        .themeFontBody()
                        .foregroundColor(Color.Theme.textPrimary)

                    Spacer()

                    Image(systemName: "chevron.right")
                        .font(.system(size: 14, weight: .semibold))
                        .foregroundColor(Color.Theme.textSecondary)
                }
                .padding(.vertical, 16)
                .padding(.horizontal, 20)
                .background(isPressed ? Color.Theme.surfaceSecondary : Color.clear)
            }
            .buttonStyle(.plain)
            .simultaneousGesture(
                DragGesture(minimumDistance: 0)
                    .onChanged { _ in isPressed = true }
                    .onEnded { _ in isPressed = false }
            )
        }
    }
}

#if DEBUG
struct DebugCardView_Previews: PreviewProvider {
    static var previews: some View {
        DebugCardView(showDebug: .constant(false))
            .themedCard()
            .padding()
            .background(Color.Theme.surfacePrimary)
    }
}
#endif
