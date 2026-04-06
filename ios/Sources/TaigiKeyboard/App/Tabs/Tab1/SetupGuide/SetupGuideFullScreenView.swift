import KeyboardKit
import SwiftUI

/// Full-screen setup guide shown on first launch.
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
            onComplete: onComplete,
        )
        .environmentObject(languageManager)
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
