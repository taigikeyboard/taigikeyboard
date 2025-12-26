import Foundation
import SwiftUI
import KeyboardKit
import Combine
#if canImport(UIKit)
import UIKit
#endif

/// Setup Guide ViewModel（與 Android SetupGuideActivity 對應）
@MainActor
class SetupGuideViewModel: ObservableObject {
    @Published var isKeyboardEnabled = false
    @Published var isFullAccessEnabled = false
    @Published var shouldShowSetupGuide = false

    private let keyboardBundleId: String
    private let statusContext: KeyboardStatusContext
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

    /// 檢查鍵盤設定狀態，未完成則顯示 Setup Guide
    func checkKeyboardStatus() {
        Task { @MainActor in
            statusContext.refresh()
            isKeyboardEnabled = statusContext.isKeyboardEnabled
            isFullAccessEnabled = statusContext.isFullAccessEnabled

            let isComplete = isKeyboardEnabled && isFullAccessEnabled
            shouldShowSetupGuide = !isComplete
        }
    }

    func dismissSetupGuide() {
        Task { @MainActor in
            shouldShowSetupGuide = false
        }
    }
}
