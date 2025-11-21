import SwiftUI

/// 完成設定頁面
struct CompletedPage: View {
    @ObservedObject var viewModel: OnboardingViewModel
    @EnvironmentObject var languageManager: LanguageManager
    let onComplete: () -> Void
    @State private var showContent = false

    var body: some View {
        CompletedPageContent(onComplete: onComplete, showContent: $showContent)
            .onAppear {
                withAnimation(.easeOut(duration: 0.6).delay(0.2)) {
                    showContent = true
                }
            }
    }
}

/// 獨立顯示的完成頁面
struct CompletedPageStandalone: View {
    @StateObject private var languageManager = LanguageManager.shared
    let onComplete: () -> Void
    @State private var showContent = false

    var body: some View {
        ZStack {
            OnboardingBackground()

            CompletedPageContent(onComplete: onComplete, showContent: $showContent)
        }
        .environmentObject(languageManager)
        .onAppear {
            withAnimation(.easeOut(duration: 0.6).delay(0.2)) {
                showContent = true
            }
        }
    }
}

/// 完成頁面共用內容
struct CompletedPageContent: View {
    @EnvironmentObject var languageManager: LanguageManager
    let onComplete: () -> Void
    @Binding var showContent: Bool

    var body: some View {
        OnboardingPageContainer(
            iconName: "checkmark.circle.fill",
            iconColor: Color.green,
            title: AppTexts.onboardingCompletedTitle,
            bottomContent: {
                VStack(spacing: 24) {
                    LocalizedTextView(AppTexts.onboardingCompletedMessage)
                        .font(Font.Theme.body)
                        .foregroundColor(Color.Theme.textSecondary)
                        .multilineTextAlignment(.center)
                        .lineSpacing(4)
                        .padding(.horizontal, 8)
                        .opacity(showContent ? 1 : 0)
                        .offset(y: showContent ? 0 : 20)

                    OnboardingActionButton(
                        text: AppTexts.onboardingGetStarted,
                        action: onComplete
                    )
                    .opacity(showContent ? 1 : 0)
                    .offset(y: showContent ? 0 : 20)
                }
            }
        )
    }
}
