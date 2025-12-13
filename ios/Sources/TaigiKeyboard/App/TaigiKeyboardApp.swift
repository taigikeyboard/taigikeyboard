import SwiftUI
import KeyboardKit

@main
struct TaigiKeyboardApp: App {
    @StateObject private var keyboardStatus = KeyboardStatusContext(
        bundleId: (Bundle.main.bundleIdentifier ?? "com.siansiansu.TaigiKeyboard") + ".TaigiKeyboardExtension"
    )

    var body: some Scene {
        WindowGroup {
            AppRootView(keyboardStatus: keyboardStatus)
                .withLanguageEnvironment()
                .withFontEnvironment()
        }
    }
}

struct AppRootView: View {
    @ObservedObject var keyboardStatus: KeyboardStatusContext
    @StateObject private var viewModel: OnboardingViewModel
    @State private var shouldShowSettings = false

    init(keyboardStatus: KeyboardStatusContext) {
        self.keyboardStatus = keyboardStatus
        _viewModel = StateObject(wrappedValue: OnboardingViewModel(keyboardStatus: keyboardStatus))
    }

    var body: some View {
        ContentView(
            initialShowSettings: $shouldShowSettings,
            viewModel: viewModel
        )
        .onOpenURL { url in
            if url.scheme == "taigikeyboard", url.host == "settings" {
                shouldShowSettings = true
            }
        }
        .sheet(isPresented: $viewModel.shouldShowOnboarding) {
            OnboardingView(
                showWelcome: viewModel.shouldShowWelcome,
                startFromComplete: false,
                onComplete: {
                    viewModel.markWelcomeAsSeen()
                    viewModel.dismissOnboarding()
                }
            )
        }
        .sheet(isPresented: $viewModel.shouldShowCompletePage) {
            CompletedPageStandalone(
                onComplete: {
                    viewModel.dismissCompletePage()
                }
            )
        }
        .task {
            viewModel.checkOnFirstLaunch()
        }
    }
}
