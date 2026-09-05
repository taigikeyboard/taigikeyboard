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

    // Setup is complete only when both preconditions hold.
    var isSetupComplete: Bool {
        isKeyboardEnabled && isFullAccessEnabled
    }

    /// Re-check keyboard activation status.
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
    func checkKeyboardStatus() {
        Task { @MainActor in
            statusContext.refresh()
            isKeyboardEnabled = statusContext.isKeyboardEnabled
            isFullAccessEnabled = statusContext.isFullAccessEnabled

            let isComplete = isKeyboardEnabled && isFullAccessEnabled
            shouldShowSetupGuide = !isComplete
        }
    }

    // User-initiated dismissal; no UI currently wires to this, kept for API completeness.
    func dismissSetupGuide() {
        Task { @MainActor in
            shouldShowSetupGuide = false
        }
    }
}
