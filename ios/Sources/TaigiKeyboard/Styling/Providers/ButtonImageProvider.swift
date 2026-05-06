// 中文: 鍵面圖示 provider — 渲染鏈第一棒。
// 中文: 只負責 globe / return / settings / translate 等圖示鍵,其餘 nil 交給 ButtonTextProvider 接手。

import KeyboardKit
import SwiftUI

/// Button image provider — first in the render chain.
///
/// Returns the SF Symbol image for a key. Returns nil to defer to `ButtonTextProvider`.
/// Only handles keys with icon-based rendering: globe, return, settings, translate toggle.
///
/// Created by: `TaigiKeyboardView.RenderProviders`
/// Queried by: `TaigiButtonContent.body` (first priority check)
/// Depends on: `KeyboardContext` (composing state, translate toggle state)
// 中文: 鍵面圖示 provider — 渲染鏈第一棒,nil 即放行給文字 provider。
final class ButtonImageProvider {
    private let keyboardContext: KeyboardContext

    init(keyboardContext: KeyboardContext) {
        self.keyboardContext = keyboardContext
    }

    /// Returns an SF Symbol image, or nil to defer to text rendering.
    // 中文: 回傳 SF Symbol;nil 表示讓 ButtonTextProvider 接手。
    // 中文: return 鍵在組字中時不顯示圖示,讓文字 provider 顯示「選 / soán / suán」確認字。
    func buttonImage(for action: KeyboardAction) -> Image? {
        switch action {
        case .nextKeyboard:
            return Image(latinSystemName: "globe")
        case .primary(.return):
            // Show newline icon when not composing; nil lets ButtonTextProvider show confirmation text
            return keyboardContext.isComposingText ? nil : Image(latinSystemName: "arrow.turn.down.left")
        case .settings:
            return Image(latinSystemName: "gearshape.fill")
        case let .custom(name):
            switch name {
            case "translate":
                let iconName = keyboardContext.isTranslateSwapped
                    ? "character.square.fill" // Active: filled
                    : "character.square" // Default: outlined
                return Image(latinSystemName: iconName)
            default:
                return nil
            }
        default:
            return nil
        }
    }
}
