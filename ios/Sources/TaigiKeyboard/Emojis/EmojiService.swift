// 中文: 表情符號鍵盤的封裝 — 包住第三方 ISEmojiView,把它的 delegate 事件轉成
// 中文: EmojiServiceDelegate(emoji 選取 / 切回字母 / 收鍵盤 / 倒退鍵)。

import ISEmojiView
import KeyboardKit
import SwiftUI
import UIKit

// 中文: 表情符號鍵盤事件的 delegate protocol。
protocol EmojiServiceDelegate: AnyObject {
    func emojiDidSelect(_ emoji: String)
    func emojiKeyboardShouldSwitchToAlphabetic()
    func emojiKeyboardShouldDismiss()
    func emojiKeyboardShouldDeleteBackward()
}

// 中文: 表情符號鍵盤服務 — 封裝 ISEmojiView 的設定與 delegate 轉接。
final class EmojiService: NSObject {
    weak var delegate: EmojiServiceDelegate?
    private let emojiView: EmojiView
    private let logger = DebugLogger(category: "Emoji")

    override init() {
        let keyboardSettings = KeyboardSettings(bottomType: .categories)
        keyboardSettings.countOfRecentsEmojis = 30
        keyboardSettings.needToShowAbcButton = true
        keyboardSettings.isShowPopPreview = true
        keyboardSettings.needToShowDeleteButton = true
        keyboardSettings.updateRecentEmojiImmediately = true
        // 中文: 表情符號資料來源 = 共用 taigi-emojis dist/emoji.json(取代 ISEmojiView 內建 plist)。
        // 中文: nil = 資源缺失時退回 ISEmojiView 內建 plist(防呆,資源已隨擴充打包)。
        let taigiEmojiCategories = TaigiEmojiData.loadISEmojiCategories()
        keyboardSettings.customEmojis = taigiEmojiCategories

        emojiView = EmojiView(keyboardSettings: keyboardSettings)
        super.init()
        emojiView.delegate = self
        // 中文: 載入失敗才會走 ISEmojiView plist fallback — 出 log 讓 dogfood 抓得到「資源沒打包進擴充」。
        if taigiEmojiCategories == nil {
            logger.debug("[EMOJI] emoji.json missing/unreadable — fell back to ISEmojiView plist; emoji source NOT swapped")
        }
    }

    // 中文: 把 ISEmojiView 包成 SwiftUI 可用的 AnyView。
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
