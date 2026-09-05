// Setup guide ViewModel。檢查鍵盤啟用狀態並驅動 setup guide 顯示邏輯。

import Combine
import Foundation
import KeyboardKit
import SwiftUI
#if canImport(UIKit)
    import UIKit
#endif

/// Setup guide view model.
///
/// Checks keyboard activation status and controls setup guide flow.
// Setup guide 的 ObservableObject ViewModel。@MainActor 確保所有狀態更新走主執行緒。
@MainActor
class SetupGuideViewModel: ObservableObject {
    // 鍵盤是否已在系統設定加入。
    @Published var isKeyboardEnabled = false
    // 是否已啟用「允許完整存取」(Full Access)。
    @Published var isFullAccessEnabled = false
    // 是否要顯示 setup guide 全螢幕引導。
    @Published var shouldShowSetupGuide = false

    private let keyboardBundleId: String
    private let statusContext: KeyboardStatusContext
    private var cancellables = Set<AnyCancellable>()

    init(keyboardStatus: KeyboardStatusContext? = nil) {
        if let bundleId = Bundle.main.bundleIdentifier {
            keyboardBundleId = "\(bundleId).TaigiKeyboardExtension"
        } else {
            keyboardBundleId = "com.siansiansu.TaigiKeyboard.TaigiKeyboardExtension"
        }

        if let keyboardStatus {
            statusContext = keyboardStatus
        } else {
            statusContext = KeyboardStatusContext(bundleId: keyboardBundleId)
        }

        statusContext.$isKeyboardEnabled
            .assign(to: &$isKeyboardEnabled)
    }

    // 兩個前置條件都成立才視為設定完成。
    var isSetupComplete: Bool {
        isKeyboardEnabled && isFullAccessEnabled
    }

    /// Re-check keyboard activation status.
    // 重新檢查鍵盤啟用狀態(從系統設定回到 App 時會呼叫)。
    func refresh() {
        #if os(iOS)
            Task { @MainActor in
                statusContext.refresh()
                isKeyboardEnabled = statusContext.isKeyboardEnabled
                isFullAccessEnabled = statusContext.isFullAccessEnabled
            }
        #endif
    }

    /// Check keyboard status; show setup guide if not complete.
    // 啟動時呼叫 — 檢查鍵盤狀態,未完成時觸發 shouldShowSetupGuide=true。
    func checkKeyboardStatus() {
        Task { @MainActor in
            statusContext.refresh()
            isKeyboardEnabled = statusContext.isKeyboardEnabled
            isFullAccessEnabled = statusContext.isFullAccessEnabled

            let isComplete = isKeyboardEnabled && isFullAccessEnabled
            shouldShowSetupGuide = !isComplete
        }
    }

    // 使用者主動關閉 setup guide(目前 UI 未實作該入口,保留 API)。
    func dismissSetupGuide() {
        Task { @MainActor in
            shouldShowSetupGuide = false
        }
    }
}
