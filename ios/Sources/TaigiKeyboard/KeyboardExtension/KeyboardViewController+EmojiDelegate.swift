// 中文: KeyboardViewController 對 EmojiService 的 delegate 實作 — emoji 選取、切回字母鍵盤、退格、關閉鍵盤。

import KeyboardKit
import UIKit

// MARK: - EmojiServiceDelegate

extension KeyboardViewController: EmojiServiceDelegate {
    /// Route emoji insertion through `ComposingManager` so an active Taigi
    /// preedit is committed atomically together with the emoji, rather than
    /// leaking a silent `finishComposingText`-equivalent. See
    /// `composing-state-boundary.md` §11.6 (Android mirror: `MediaInputManager`).
    // 中文: 點選 emoji 時走 ComposingManager.commitPreeditThenInsertExternal,
    // 中文: 讓正在組字的 preedit 與 emoji 一起 atomic 送出,避免靜默清掉 preedit。
    func emojiDidSelect(_ emoji: String) {
        actionHandler?.beginInputEvent()
        if let manager = actionHandler?.composingManager {
            manager.commitPreeditThenInsertExternal(emoji)
        } else {
            textDocumentProxy.insertText(emoji)
        }
    }

    // 中文: emoji 鍵盤要求切回字母鍵盤時呼叫。
    func emojiKeyboardShouldSwitchToAlphabetic() {
        state.keyboardContext.keyboardType = .alphabetic
    }

    // 中文: emoji 鍵盤要求關閉整個鍵盤時呼叫。
    func emojiKeyboardShouldDismiss() {
        dismissKeyboard()
    }

    /// Reuse the standard backspace path so composing/idle branches match
    /// the regular keyboard: composing → delete one preedit grapheme,
    /// idle → document backspace + NextWord re-predict.
    // 中文: emoji 鍵盤的退格走標準退格路徑,讓組字 / 閒置兩種分支與一般鍵盤一致。
    func emojiKeyboardShouldDeleteBackward() {
        if let handler = actionHandler {
            handler.beginInputEvent()
            _ = handler.handleBackspaceAction()
        } else {
            textDocumentProxy.deleteBackward()
        }
    }
}
