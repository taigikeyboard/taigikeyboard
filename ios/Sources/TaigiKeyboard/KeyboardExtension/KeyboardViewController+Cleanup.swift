// Cleanup extension for the keyboard extension — called when the keyboard is dismissed or
// rebuilt, to ensure the next activation starts from a clean state.

import Foundation
import KeyboardKit
import UIKit

// MARK: - Cleanup Operations

extension KeyboardViewController {
    // Idempotent entry point — clears input state, service references, and the emoji delegate.
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
