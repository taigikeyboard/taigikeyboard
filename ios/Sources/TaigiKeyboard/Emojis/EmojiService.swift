// Wraps the third-party ISEmojiView, translating its delegate events into EmojiServiceDelegate.
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
        // Single source of truth: shared taigi-emojis dist/emoji.json. TaigiEmojiData
        // asserts on load failure rather than falling back to a plist (no redundant fallback).
        keyboardSettings.customEmojis = TaigiEmojiData.loadISEmojiCategories()

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

    /// A themed keyboard (custom surface) shows its surface through the emoji keyboard: the root
    /// `ThemeBackgroundSurface` paints behind it, so ISEmojiView's own background goes clear.
    /// The adaptive default keeps ISEmojiView's `.secondarySystemBackground`.
    func updateUIView(_ view: EmojiView, context: Context) {
        view.backgroundColor = context.environment.candidateTheme.surface == nil ? .secondarySystemBackground : .clear
    }
}
