// 中文: 首次啟動時的全螢幕鍵盤設定引導。鍵盤完成設定後自動 dismiss。

import KeyboardKit
import SwiftUI

/// Full-screen setup guide shown on first launch.
// 中文: 首次啟動的全螢幕 setup guide。
// 中文: onComplete 在設定完成或使用者主動關閉時呼叫;
// 中文: didBecomeActive 通知會 refresh 鍵盤狀態以支援使用者切去系統設定後返回的場景。
struct SetupGuideFullScreenView: View {
    @StateObject private var viewModel = SetupGuideViewModel()

    let onComplete: () -> Void

    init(onComplete: @escaping () -> Void) {
        self.onComplete = onComplete
    }

    var body: some View {
        SetupGuideView(
            viewModel: viewModel,
            isFullScreen: true,
            onComplete: onComplete,
        )
        .task {
            viewModel.refresh()
            // Auto-dismiss if setup already complete
            if viewModel.isSetupComplete {
                onComplete()
            }
        }
        .onReceive(NotificationCenter.default.publisher(for: UIApplication.didBecomeActiveNotification)) { _ in
            Task { @MainActor in
                viewModel.refresh()
                // Auto-dismiss if setup already complete
                if viewModel.isSetupComplete {
                    onComplete()
                }
            }
        }
    }
}
