import KeyboardKit
import UIKit

// MARK: - EmojiServiceDelegate

extension KeyboardViewController: EmojiServiceDelegate {
    /// Route emoji insertion through `ComposingManager` so an active Taigi
    /// preedit is committed atomically together with the emoji, rather than
    /// leaking a silent `finishComposingText`-equivalent. See
    /// `composing-state-boundary.md` §11.6 (Android mirror: `MediaInputManager`).
    func emojiDidSelect(_ emoji: String) {
        if let manager = actionHandler?.composingManager {
            manager.commitPreeditThenInsertExternal(emoji)
        } else {
            textDocumentProxy.insertText(emoji)
        }
    }

    func emojiKeyboardShouldSwitchToAlphabetic() {
        state.keyboardContext.keyboardType = .alphabetic
    }

    func emojiKeyboardShouldDismiss() {
        dismissKeyboard()
    }

    /// Reuse the standard backspace path so composing/idle branches match
    /// the regular keyboard: composing → delete one preedit grapheme,
    /// idle → document backspace + NextWord re-predict.
    func emojiKeyboardShouldDeleteBackward() {
        if let handler = actionHandler {
            _ = handler.handleBackspaceAction()
        } else {
            textDocumentProxy.deleteBackward()
        }
    }
}
