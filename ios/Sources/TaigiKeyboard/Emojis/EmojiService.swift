// 表情符號鍵盤的封裝 — 包住第三方 ISEmojiView,把它的 delegate 事件轉成
// EmojiServiceDelegate(emoji 選取 / 切回字母 / 收鍵盤 / 倒退鍵)。

import ISEmojiView
import KeyboardKit
import SwiftUI
import UIKit

// 表情符號鍵盤事件的 delegate protocol。
protocol EmojiServiceDelegate: AnyObject {
    func emojiDidSelect(_ emoji: String)
    func emojiKeyboardShouldSwitchToAlphabetic()
    func emojiKeyboardShouldDismiss()
    func emojiKeyboardShouldDeleteBackward()
}

// 表情符號鍵盤服務 — 封裝 ISEmojiView 的設定與 delegate 轉接。
final class EmojiService: NSObject {
    weak var delegate: EmojiServiceDelegate?
    private let emojiView: EmojiView

    override init() {
        let keyboardSettings = KeyboardSettings(bottomType: .categories)
        keyboardSettings.countOfRecentsEmojis = 30
        keyboardSettings.needToShowAbcButton = true
        keyboardSettings.isShowPopPreview = true
        keyboardSettings.needToShowDeleteButton = true
        keyboardSettings.updateRecentEmojiImmediately = true
        // 表情符號資料來源 = 共用 taigi-emojis dist/emoji.json(單一來源)。
        // 載入失敗在 TaigiEmojiData 直接 assert 炸出錯點,不做 plist fallback(USER:不要冗餘 fallback)。
        keyboardSettings.customEmojis = TaigiEmojiData.loadISEmojiCategories()

        emojiView = EmojiView(keyboardSettings: keyboardSettings)
        super.init()
        emojiView.delegate = self
    }

    // 把 ISEmojiView 包成 SwiftUI 可用的 AnyView。
    var emojiKeyboardView: AnyView {
        AnyView(EmojiViewRepresentable(emojiView: emojiView))
    }
}

extension EmojiService: EmojiViewDelegate {
    func emojiViewDidSelectEmoji(_ emoji: String, emojiView _: EmojiView) {
        delegate?.emojiDidSelect(emoji)
    }

    func emojiViewDidPressChangeKeyboardButton(_: EmojiView) {
        delegate?.emojiKeyboardShouldSwitchToAlphabetic()
    }

    func emojiViewDidPressDeleteBackwardButton(_: EmojiView) {
        delegate?.emojiKeyboardShouldDeleteBackward()
    }

    func emojiViewDidPressDismissKeyboardButton(_: EmojiView) {
        delegate?.emojiKeyboardShouldDismiss()
    }
}

private struct EmojiViewRepresentable: UIViewRepresentable {
    let emojiView: EmojiView

    func makeUIView(context _: Context) -> EmojiView {
        emojiView
    }

    func updateUIView(_: EmojiView, context _: Context) {}
}
