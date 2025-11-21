import Foundation
import SwiftUI
import KeyboardKit
import Combine
#if canImport(UIKit)
import UIKit
#endif

/// 引導流程 ViewModel
@MainActor
class OnboardingViewModel: ObservableObject {
    @Published var isKeyboardEnabled = false
    @Published var isFullAccessEnabled = false
    @Published var currentPage = 0
    @Published var shouldShowOnboarding = false
    @Published var shouldShowCompletePage = false

    private let keyboardBundleId: String
    private let statusContext: KeyboardStatusContext
    private var preferences = OnboardingPreferences()
    private var cancellables = Set<AnyCancellable>()

    init(keyboardStatus: KeyboardStatusContext? = nil) {
        if let bundleId = Bundle.main.bundleIdentifier {
            self.keyboardBundleId = "\(bundleId).TaigiKeyboardExtension"
        } else {
            self.keyboardBundleId = "com.siansiansu.TaigiKeyboard.TaigiKeyboardExtension"
        }

        if let keyboardStatus = keyboardStatus {
            self.statusContext = keyboardStatus
        } else {
            self.statusContext = KeyboardStatusContext(bundleId: keyboardBundleId)
        }

        statusContext.$isKeyboardEnabled
            .assign(to: &$isKeyboardEnabled)
    }

    var isSetupComplete: Bool {
        isKeyboardEnabled && isFullAccessEnabled
    }

    var shouldShowWelcome: Bool {
        !preferences.hasSeenWelcome
    }

    /// 重新檢查鍵盤啟用狀態
    func refresh() {
        #if os(iOS)
        Task { @MainActor in
            statusContext.refresh()
            isKeyboardEnabled = statusContext.isKeyboardEnabled
            isFullAccessEnabled = statusContext.isFullAccessEnabled
        }
        #endif
    }

    /// 系統設定頁面 URL
    var settingsURL: URL? {
        #if os(iOS)
        return URL(string: UIApplication.openSettingsURLString)
        #else
        return nil
        #endif
    }

    /// 標記歡迎頁已顯示過
    func markWelcomeAsSeen() {
        preferences.hasSeenWelcome = true
    }

    /// 根據鍵盤狀態決定是否顯示引導流程（必須在 @MainActor 上下文中調用）
    private func handleStatusChange() {
        let isComplete = statusContext.isKeyboardEnabled && statusContext.isFullAccessEnabled

        if !isComplete {
            shouldShowOnboarding = true
        } else {
            shouldShowOnboarding = false
        }
    }

    /// 重新檢查鍵盤狀態
    func refreshAndCheck() {
        Task { @MainActor in
            statusContext.refresh()
            handleStatusChange()
        }
    }

    /// 首次啟動時檢查是否需要顯示引導
    func checkOnFirstLaunch() {
        guard !preferences.hasSeenWelcome else { return }
        Task { @MainActor in
            statusContext.refresh()
            handleStatusChange()
        }
    }

    /// 從設定頁面觸發引導流程
    func showOnboardingForSettings() {
        Task { @MainActor in
            statusContext.refresh()
            let isComplete = statusContext.isKeyboardEnabled && statusContext.isFullAccessEnabled

            if isComplete {
                // 已完成，顯示 Complete Page
                shouldShowCompletePage = true
            } else {
                // 未完成，顯示 Onboarding
                shouldShowOnboarding = true
            }
        }
    }

    func dismissOnboarding() {
        Task { @MainActor in
            shouldShowOnboarding = false
        }
    }

    func dismissCompletePage() {
        Task { @MainActor in
            shouldShowCompletePage = false
        }
    }

}
