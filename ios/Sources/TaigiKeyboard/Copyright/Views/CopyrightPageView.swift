import SwiftUI

/// 版權頁面內容視圖
struct CopyrightPageView: View {
    let page: CopyrightPage
    let isActive: Bool
    let geometry: GeometryProxy
    @Binding var isPressed: [Bool]
    let openURL: OpenURLAction

    var body: some View {
        ScrollView(.vertical, showsIndicators: false) {
            VStack(spacing: 32) {
                // Header Card
                VStack(spacing: 24) {
                    VStack(spacing: 12) {
                        LocalizedTextView(page.title)
                            .themeFontTitle()
                            .foregroundColor(Color.Theme.textPrimary)
                            .multilineTextAlignment(.center)

                        LocalizedTextView(page.description)
                            .themeFontBody()
                            .foregroundColor(Color.Theme.textSecondary)
                            .multilineTextAlignment(.center)
                            .lineSpacing(2)
                            .opacity(0.9)
                    }
                }
                .frame(maxWidth: .infinity)
                .padding(.vertical, 24)
                .padding(.horizontal, 24)
                .themedCard()

                // License Info Card
                VStack(alignment: .leading, spacing: 20) {
                    LocalizedTextView(page.licenseDescription)
                        .themeFontHeadline()
                        .foregroundColor(Color.Theme.textPrimary)

                    Rectangle()
                        .fill(Color.Theme.textSecondary.opacity(0.1))
                        .frame(height: 1)

                    // Action Buttons
                    VStack(spacing: 0) {
                        ForEach(page.buttons.indices, id: \.self) { buttonIndex in
                            let button = page.buttons[buttonIndex]
                            let pressedIndex = page.id * 2 + buttonIndex

                            actionButton(
                                iconColor: page.accentColor,
                                text: button.text,
                                isPressed: isPressed.indices.contains(pressedIndex) ? isPressed[pressedIndex] : false,
                                action: {
                                    if let url = URL(string: button.url) {
                                        openURL(url)
                                    }
                                }
                            )
                            .simultaneousGesture(
                                DragGesture(minimumDistance: 30)
                                    .onChanged { _ in
                                        if isPressed.indices.contains(pressedIndex) {
                                            withAnimation(ThemeAnimation.press) {
                                                isPressed[pressedIndex] = true
                                            }
                                        }
                                    }
                                    .onEnded { _ in
                                        if isPressed.indices.contains(pressedIndex) {
                                            withAnimation(ThemeAnimation.release) {
                                                isPressed[pressedIndex] = false
                                            }
                                        }
                                    }
                            )

                            if buttonIndex < page.buttons.count - 1 {
                                Rectangle()
                                    .fill(Color.Theme.textSecondary.opacity(0.1))
                                    .frame(height: 0.5)
                            }
                        }
                    }
                }
                .padding(.horizontal, 20)
                .padding(.vertical, 20)
                .themedCard()
            }
            .padding(.horizontal, 20)
            .frame(maxWidth: min(geometry.size.width, 500))
            .frame(maxWidth: .infinity)
            .padding(.vertical, 20)
        }
    }

    private func actionButton(
        iconColor: Color,
        text: LocalizedText,
        isPressed: Bool,
        action: @escaping () -> Void
    ) -> some View {
        Button(action: action) {
            HStack(spacing: 16) {
                LocalizedTextView(text)
                    .themeFontBody()
                    .foregroundColor(Color.Theme.textPrimary)

                Spacer()

                Image(systemName: "arrow.up.right")
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundColor(iconColor)
                    .offset(x: isPressed ? 3 : 0, y: isPressed ? -1 : 0)
                    .animation(ThemeAnimation.smooth, value: isPressed)
            }
            .padding(.vertical, 18)
            .contentShape(Rectangle())
            .background(
                Rectangle()
                    .fill(isPressed ? iconColor.opacity(0.06) : Color.clear)
                    .animation(ThemeAnimation.quick, value: isPressed)
            )
            .scaleEffect(isPressed ? 0.97 : 1.0)
            .animation(ThemeAnimation.smooth, value: isPressed)
        }
        .buttonStyle(PlainButtonStyle())
    }
}
