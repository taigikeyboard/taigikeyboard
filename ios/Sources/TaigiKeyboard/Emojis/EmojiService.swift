import ISEmojiView
import KeyboardKit
import SwiftUI
import UIKit

protocol EmojiServiceDelegate: AnyObject {
    func emojiDidSelect(_ emoji: String)
    func emojiKeyboardShouldSwitchToAlphabetic()
    func emojiKeyboardShouldDismiss()
    func emojiKeyboardShouldDeleteBackward()
}

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

        emojiView = EmojiView(keyboardSettings: keyboardSettings)
        super.init()
        emojiView.delegate = self
    }

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
