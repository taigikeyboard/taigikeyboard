import SwiftUI

/// 設定頁面容器視圖
struct SettingsPageView<Content: View>: View {
    let onDismiss: () -> Void
    let content: Content

    init(onDismiss: @escaping () -> Void, @ViewBuilder content: () -> Content) {
        self.onDismiss = onDismiss
        self.content = content()
    }

    var body: some View {
        NavigationView {
            GeometryReader { geometry in
                ZStack {
                    // Retro flat background (warm beige)
                    Color.Theme.surfacePrimary
                        .ignoresSafeArea()

                    VStack(spacing: 0) {
                        ScrollView {
                            VStack(spacing: 24) {
                                content
                            }
                            .padding(.horizontal, 20)
                            .padding(.top, 20)
                            .padding(.bottom, max(20, geometry.safeAreaInsets.bottom))
                        }
                        .scrollIndicators(.hidden)
                    }
                }
            }
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    SettingsCloseButton(action: onDismiss)
                }
            }
        }
        .navigationViewStyle(.stack)
    }
}

/// 設定頁關閉按鈕
struct SettingsCloseButton: View {
    let action: () -> Void
    @State private var isPressed = false

    var body: some View {
        Button(action: action) {
            Text(LanguageManager.shared.text(AppTexts.done))
                .font(Font.Theme.body)
                .foregroundColor(Color.Theme.accent)
                .padding(.horizontal, 16)
                .padding(.vertical, 8)
                .background(
                    RoundedRectangle(cornerRadius: 12)
                        .fill(isPressed ? Color.Theme.accent.opacity(0.1) : Color.clear)
                        .animation(ThemeAnimation.quick, value: isPressed)
                )
        }
        .buttonStyle(PlainButtonStyle())
        .pressable($isPressed, scaleEffect: 0.96, minimumDistance: 50)
        .animation(ThemeAnimation.bouncy, value: isPressed)
    }
}
