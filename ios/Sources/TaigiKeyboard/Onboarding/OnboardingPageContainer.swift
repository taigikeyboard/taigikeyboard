import SwiftUI

/// 引導頁面容器
struct OnboardingPageContainer<BottomContent: View>: View {
    let iconName: String
    let iconColor: Color
    let title: LocalizedText
    let message: LocalizedText
    let bottomContent: (() -> BottomContent)

    @EnvironmentObject var languageManager: LanguageManager

    var body: some View {
        GeometryReader { geometry in
            ScrollView {
                VStack(spacing: 0) {
                    Spacer()

                    Image(systemName: iconName)
                        .font(.system(size: 64, weight: .semibold))
                        .foregroundColor(iconColor)
                        .padding(.bottom, 32)

                    VStack(spacing: 16) {
                        LocalizedTextView(title)
                            .font(Font.Theme.title)
                            .foregroundColor(Color.Theme.textPrimary)
                            .multilineTextAlignment(.center)

                        LocalizedTextView(message)
                            .font(Font.Theme.body)
                            .foregroundColor(Color.Theme.textSecondary)
                            .multilineTextAlignment(.center)
                            .lineSpacing(4)
                    }
                    .padding(.horizontal, 32)
                    .padding(.bottom, 32)

                    bottomContent()
                        .padding(.horizontal, 24)

                    Spacer()
                }
                .frame(maxWidth: .infinity)
                .frame(minHeight: geometry.size.height)
            }
        }
    }
}

extension OnboardingPageContainer where BottomContent == EmptyView {
    init(
        iconName: String,
        iconColor: Color,
        title: LocalizedText,
        message: LocalizedText
    ) {
        self.iconName = iconName
        self.iconColor = iconColor
        self.title = title
        self.message = message
        self.bottomContent = { EmptyView() }
    }
}

extension OnboardingPageContainer {
    init(
        iconName: String,
        iconColor: Color,
        title: LocalizedText,
        @ViewBuilder bottomContent: @escaping () -> BottomContent
    ) {
        self.iconName = iconName
        self.iconColor = iconColor
        self.title = title
        self.message = LocalizedText(hanji: "", poj: "", tl: "")
        self.bottomContent = bottomContent
    }
}
