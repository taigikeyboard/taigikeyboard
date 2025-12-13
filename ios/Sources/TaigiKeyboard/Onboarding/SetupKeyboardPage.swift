import SwiftUI
import KeyboardKit

/// 設定鍵盤頁面
struct SetupKeyboardPage: View {
    @ObservedObject var viewModel: OnboardingViewModel
    @EnvironmentObject var languageManager: LanguageManager
    @Environment(\.openURL) private var openURL

    var body: some View {
        OnboardingPageContainer(
            iconName: "keyboard.badge.ellipsis",
            iconColor: Color.Theme.accent,
            title: AppTexts.onboardingAddKeyboardTitle,
            bottomContent: {
                VStack(spacing: 24) {
                    SetupStepsView()

                    LocalizedTextView(AppTexts.setupInfoMessage)
                        .themeFontCaption()
                        .foregroundColor(Color.Theme.textSecondary)
                        .multilineTextAlignment(.center)
                        .lineSpacing(4)
                        .padding(.horizontal, 8)

                    OnboardingActionButton(
                        text: AppTexts.onboardingGoToSettings,
                        action: {
                            if let url = viewModel.settingsURL {
                                openURL(url)
                            }
                        }
                    )
                }
            }
        )
    }
}
