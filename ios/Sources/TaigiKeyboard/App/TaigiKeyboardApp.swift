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

// MARK: - Deep Link 通知

extension Notification.Name {
    static let switchToSettingsTab = Notification.Name("switchToSettingsTab")
}
