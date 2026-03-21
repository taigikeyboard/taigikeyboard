import SwiftUI
import KeyboardKit

/// 台語鍵盤 App 進入點
@main
struct TaigiKeyboardApp: App {
    @StateObject private var keyboardStatus = KeyboardStatusContext(
        bundleId: (Bundle.main.bundleIdentifier ?? "com.siansiansu.TaigiKeyboard") + ".TaigiKeyboardExtension"
    )

    init() {
        // 設定 KeyboardKit 使用 App Group 持久化設定
        // 必須在任何 @AppStorage 存取之前呼叫
        KeyboardSettings.setupStore(forAppGroup: SharedSettings.appGroupId)

        // Navigation bar title font (UIKit appearance, not affected by SwiftUI .environment)
        let navAppearance = UINavigationBarAppearance()
        navAppearance.configureWithDefaultBackground()
        let largeFont = UIFont(name: KeyboardModels.Fonts.openHuninnFontName, size: 34)
            ?? .systemFont(ofSize: 34)
        let inlineFont = UIFont(name: KeyboardModels.Fonts.openHuninnFontName, size: 17)
            ?? .systemFont(ofSize: 17)
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
                .withFontEnvironment()
        }
    }
}

/// App 根視圖
///
/// 管理 Deep Link 和設定引導流程。
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
                    }
                )
            }
            .task {
                viewModel.checkKeyboardStatus()
            }
    }

    /// 處理 Deep Link
    private func handleDeepLink(_ url: URL) {
        guard url.scheme == "taigikeyboard" else { return }

        // taigikeyboard://settings → 切換到設定 Tab
        // 目前 ContentView 使用 @State 管理 selectedTab
        // Deep Link 支援需要透過其他方式實現（如 @AppStorage 或 NotificationCenter）
        // 暫時保留此處理邏輯，後續可擴展
        if url.host == "settings" {
            // 可透過 NotificationCenter 通知 ContentView 切換 Tab
            NotificationCenter.default.post(
                name: .switchToSettingsTab,
                object: nil
            )
        }
    }
}

// MARK: - 通知名稱

extension Notification.Name {
    /// 切換到設定 Tab
    static let switchToSettingsTab = Notification.Name("switchToSettingsTab")
}
