import SwiftUI
import KeyboardKit

/// 設定引導全螢幕視圖
///
/// 首次啟動時顯示的鍵盤設定引導。
struct SetupGuideFullScreenView: View {
    @StateObject private var viewModel = SetupGuideViewModel()
    @StateObject private var languageManager = LanguageManager.shared

    let onComplete: () -> Void

    init(onComplete: @escaping () -> Void) {
        self.onComplete = onComplete
    }

    var body: some View {
        SetupGuideView(
            viewModel: viewModel,
            isFullScreen: true,
            onComplete: onComplete
        )
        .environmentObject(languageManager)
        .task {
            viewModel.refresh()
            // 若已完成設定，直接關閉
            if viewModel.isSetupComplete {
                onComplete()
            }
        }
        .onReceive(NotificationCenter.default.publisher(for: UIApplication.didBecomeActiveNotification)) { _ in
            Task { @MainActor in
                viewModel.refresh()
                // 若已完成設定，直接關閉
                if viewModel.isSetupComplete {
                    onComplete()
                }
            }
        }
    }
}
