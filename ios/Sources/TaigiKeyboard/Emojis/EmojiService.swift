import KeyboardKit
import OSLog
import SwiftUI
import UIKit

protocol EmojiServiceDelegate: AnyObject {
    func emojiDidSelect(_ emoji: String)
    func emojiKeyboardShouldSwitchToAlphabetic()
    func emojiKeyboardShouldDismiss()
    func emojiKeyboardShouldDeleteBackward()
}

class EmojiService: NSObject {

    weak var delegate: EmojiServiceDelegate?
    private var emojiView: EmojiView?
    private let logger = Logger(
        subsystem: LexiconConstants.Logging.subsystem,
        category: "EmojiService",
    )

    override init() {
        super.init()
        setupEmojiView()
    }

    private func setupEmojiView() {
        setupEmojiView(with: .default)
    }

    private func setupEmojiView(with configuration: Configuration) {
        let keyboardSettings = KeyboardSettings(
            bottomType: configuration.bottomType,
        )
        keyboardSettings.countOfRecentsEmojis =
            configuration.countOfRecentsEmojis
        keyboardSettings.needToShowAbcButton = configuration.needToShowAbcButton
        keyboardSettings.isShowPopPreview = configuration.isShowPopPreview
        keyboardSettings.needToShowDeleteButton =
            configuration.needToShowDeleteButton
        keyboardSettings.updateRecentEmojiImmediately =
            configuration.updateRecentEmojiImmediately

        emojiView = EmojiView(keyboardSettings: keyboardSettings)
        emojiView?.delegate = self
    }

    func getEmojiKeyboardView() -> AnyView {
        guard let emojiView else {
            logger.warning("[WARN] EmojiView not available")
            return AnyView(EmptyView())
        }
        return AnyView(EmojiViewRepresentable(emojiView: emojiView))
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

struct EmojiViewRepresentable: UIViewRepresentable {
    let emojiView: EmojiView

    func makeUIView(context _: Context) -> EmojiView {
        emojiView
    }

    func updateUIView(_: EmojiView, context _: Context) {
        // Update logic if needed
    }

    typealias UIViewType = EmojiView
}

extension EmojiService {
    struct Configuration {
        let bottomType: BottomType
        let countOfRecentsEmojis: Int
        let needToShowAbcButton: Bool
        let isShowPopPreview: Bool
        let needToShowDeleteButton: Bool
        let updateRecentEmojiImmediately: Bool

        static let `default` = Configuration(
            bottomType: .categories,
            countOfRecentsEmojis: KeyboardModels.UI.Emoji.defaultRecentCount,
            needToShowAbcButton: true,
            isShowPopPreview: true,
            needToShowDeleteButton: true,
            updateRecentEmojiImmediately: true,
        )

    }
}
