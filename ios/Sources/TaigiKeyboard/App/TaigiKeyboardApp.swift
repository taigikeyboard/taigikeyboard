import KeyboardKit
import SwiftUI

/// Taigi Keyboard app entry point.
@main
struct TaigiKeyboardApp: App {
    @StateObject private var keyboardStatus = KeyboardStatusContext(
        bundleId: (Bundle.main.bundleIdentifier ?? "com.siansiansu.TaigiKeyboard") + ".TaigiKeyboardExtension",
    )

    init() {
        // Configure KeyboardKit to persist settings via App Group.
        // Must be called before any @AppStorage access.
        KeyboardSettings.setupStore(forAppGroup: SharedSettings.appGroupId)

        // Navigation bar title font (UIKit appearance, not affected by SwiftUI .environment)
        let navAppearance = UINavigationBarAppearance()
        navAppearance.configureWithDefaultBackground()
        let largeFont = UIFont(name: KeyboardFonts.openHuninnFontName, size: AppStyle.navBarLargeTitleSize)
            ?? .systemFont(ofSize: AppStyle.navBarLargeTitleSize)
        let inlineFont = UIFont(name: KeyboardFonts.openHuninnFontName, size: AppStyle.navBarInlineTitleSize)
            ?? .systemFont(ofSize: AppStyle.navBarInlineTitleSize)
        navAppearance.largeTitleTextAttributes = [.font: largeFont]
        navAppearance.titleTextAttributes = [.font: inlineFont]
        UINavigationBar.appearance().standardAppearance = navAppearance
        UINavigationBar.appearance().scrollEdgeAppearance = navAppearance

        // Seed custom dictionary default entries on first install only
        Task {
            try? await CustomDictionaryService.shared.seedDefaultEntryIfEmpty()
        }
    }

    var body: some Scene {
        WindowGroup {
            AppRootView(keyboardStatus: keyboardStatus)
                .withLanguageEnvironment()
        }
    }
}

/// App root view.
///
/// Manages deep links and setup guide flow.
struct AppRootView: View {
    @ObservedObject var keyboardStatus: KeyboardStatusContext
    @StateObject private var viewModel: SetupGuideViewModel

    init(keyboardStatus: KeyboardStatusContext) {
        self.keyboardStatus = keyboardStatus
        _viewModel = StateObject(wrappedValue: SetupGuideViewModel(keyboardStatus: keyboardStatus))
    }

    var body: some View {
        ContentView(viewModel: viewModel)
            .onOpenURL { url in
                handleDeepLink(url)
            }
            .fullScreenCover(isPresented: $viewModel.shouldShowSetupGuide) {
                SetupGuideFullScreenView(
                    onComplete: {
                        viewModel.dismissSetupGuide()
                    },
                )
            }
            .task {
                viewModel.checkKeyboardStatus()
            }
    }

    /// Handle deep link (e.g. taigikeyboard://settings).
    private func handleDeepLink(_ url: URL) {
        guard url.scheme == "taigikeyboard" else { return }

        if url.host == "settings" {
            NotificationCenter.default.post(
                name: .switchToSettingsTab,
                object: nil,
            )
        }
    }
}

// MARK: - Notification Names

extension Notification.Name {
    /// Switch to Settings tab via deep link.
    static let switchToSettingsTab = Notification.Name("switchToSettingsTab")
}
