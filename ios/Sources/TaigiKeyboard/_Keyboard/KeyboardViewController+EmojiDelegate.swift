import KeyboardKit

// MARK: - EmojiServiceDelegate

extension KeyboardViewController: EmojiServiceDelegate {
    func emojiDidSelect(_ emoji: String) {
        textDocumentProxy.insertText(emoji)
    }

    func emojiKeyboardShouldSwitchToAlphabetic() {
        state.keyboardContext.keyboardType = .alphabetic
    }

    func emojiKeyboardShouldDismiss() {
        dismissKeyboard()
    }

    func emojiKeyboardShouldDeleteBackward() {
        textDocumentProxy.deleteBackward()
    }
}
