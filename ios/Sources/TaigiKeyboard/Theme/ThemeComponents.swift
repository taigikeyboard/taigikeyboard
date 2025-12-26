import SwiftUI

/// 復古扁平卡片樣式
struct ThemeCard<Content: View>: View {
    let content: Content
    let borderStyle: BorderStyle
    let cornerRadius: CGFloat

    init(
        borderStyle: BorderStyle = ThemeBorder.standard,
        cornerRadius: CGFloat = 10,
        @ViewBuilder content: () -> Content
    ) {
        self.content = content()
        self.borderStyle = borderStyle
        self.cornerRadius = cornerRadius
    }

    var body: some View {
        content
            .background(
                RoundedRectangle(cornerRadius: cornerRadius)
                    .fill(Color.Theme.surfaceSecondary)
                    .overlay(
                        RoundedRectangle(cornerRadius: cornerRadius)
                            .stroke(borderStyle.color, lineWidth: borderStyle.width)
                    )
            )
    }
}

/// 復古 X 關閉按鈕（扁平）
struct CloseButton: View {
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            ZStack {
                Circle()
                    .fill(Color.Theme.surfaceSecondary)
                    .overlay(
                        Circle()
                            .stroke(Color.Theme.cardStroke, lineWidth: 1)
                    )
                    .frame(width: 40, height: 40)

                Image(systemName: "xmark")
                    .font(.system(size: 18, weight: .semibold, design: .rounded))
                    .foregroundColor(Color.Theme.textPrimary)
            }
        }
        .taigiButtonStyle()
        .accessibilityLabel("關閉")
        .accessibilityHint("關閉目前視窗")
    }
}

/// Setup Guide 背景（含漸層裝飾圓圈，與 Android activity_setup_guide.xml 對應）
struct SetupGuideBackground: View {
    var body: some View {
        ZStack {
            Color.Theme.surfacePrimary
                .ignoresSafeArea()

            Circle()
                .fill(Color.Theme.accent.opacity(0.05))
                .frame(width: 300, height: 300)
                .blur(radius: 100)
                .offset(x: 100, y: -200)

            Circle()
                .fill(Color.Theme.accentSecondary.opacity(0.03))
                .frame(width: 250, height: 250)
                .blur(radius: 80)
                .offset(x: -120, y: 300)
        }
    }
}
