import SwiftUI
import KeyboardKit

/// 引導流程主視圖
struct OnboardingView: View {
    @StateObject private var viewModel = OnboardingViewModel()
    @StateObject private var languageManager = LanguageManager.shared
    @State private var currentPage: Int
    @State private var preferences = OnboardingPreferences()

    let showWelcome: Bool
    let startFromComplete: Bool
    let onComplete: () -> Void

    init(showWelcome: Bool = true, startFromComplete: Bool = false, onComplete: @escaping () -> Void) {
        self.showWelcome = showWelcome
        self.startFromComplete = startFromComplete
        self.onComplete = onComplete

        if startFromComplete {
            _currentPage = State(initialValue: showWelcome ? 2 : 1)
        } else {
            _currentPage = State(initialValue: 0)
        }
    }

    @ViewBuilder
    private var currentPageView: some View {
        switch (showWelcome, currentPage) {
        case (true, 0):
            WelcomePage(onStartSetup: {
                Task { @MainActor in
                    preferences.hasSeenWelcome = true
                    withAnimation {
                        currentPage = 1
                    }
                }
            })
        case (true, 1), (false, 0):
            SetupKeyboardPage(viewModel: viewModel)
        case (true, 2), (false, 1):
            CompletedPage(viewModel: viewModel, onComplete: onComplete)
        default:
            EmptyView()
        }
    }

    var body: some View {
        ZStack {
            OnboardingBackground()

            VStack(spacing: 0) {
                HStack {
                    Spacer()
                    if currentPage < 2 {
                        Button(action: onComplete) {
                            LocalizedTextView(AppTexts.onboardingSkip)
                                .font(Font.Theme.body)
                                .foregroundColor(Color.Theme.textSecondary)
                                .padding()
                        }
                    }
                }
                .padding(.horizontal)

                currentPageView
                .onChange(of: currentPage) { oldValue, newValue in
                    Task { @MainActor in
                        if oldValue == 2 && newValue == 1 {
                            currentPage = 2
                        }
                        if newValue == 2 && !viewModel.isSetupComplete {
                            currentPage = oldValue
                        }
                    }
                }
            }
        }
        .environmentObject(languageManager)
        .task {
            viewModel.refresh()
        }
        .onReceive(NotificationCenter.default.publisher(for: UIApplication.didBecomeActiveNotification)) { _ in
            Task { @MainActor in
                viewModel.refresh()

                let isOnSetupPage = showWelcome ? (currentPage == 1) : (currentPage == 0)
                guard isOnSetupPage else { return }

                if viewModel.isSetupComplete {
                    onComplete()
                }
            }
        }
    }
}
