import Foundation
import KeyboardKit

// MARK: - Cleanup Operations

extension KeyboardViewController {
    func performCleanup() {
        guard !isCleanedUp else {
            return
        }

        isCleanedUp = true

        cleanupInputState()
        cleanupServices()

        if let emojiService = emojiServiceStorage {
            emojiService.delegate = nil
            emojiServiceStorage = nil
        }
    }

    func cleanupServices() {
        actionHandler = nil
    }

    /// Ensure clean state for next keyboard activation
    func cleanupInputState() {
        if let handler = actionHandler {
            handler.composingManager.reset()
        }
        clearMarkedText()
        state.autocompleteContext.reset()
    }
}
