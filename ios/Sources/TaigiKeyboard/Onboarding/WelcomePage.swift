import SwiftUI

/// 歡迎頁面
struct WelcomePage: View {
    @EnvironmentObject var languageManager: LanguageManager
    let onStartSetup: () -> Void

    var body: some View {
        OnboardingPageContainer(
            iconName: "keyboard.fill",
            iconColor: Color.Theme.accent,
            title: AppTexts.onboardingWelcomeTitle,
            bottomContent: {
                VStack(spacing: 24) {
                    LocalizedTextView(AppTexts.onboardingWelcomeMessage)
                        .themeFontBody()
                        .foregroundColor(Color.Theme.textSecondary)
                        .multilineTextAlignment(.center)
                        .lineSpacing(4)
                        .padding(.horizontal, 8)

                    OnboardingActionButton(
                        text: AppTexts.onboardingStartSetup,
                        action: onStartSetup
                    )
                }
            }
        )
    }
}
