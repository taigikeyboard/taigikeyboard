import Foundation
import KeyboardKit
import OSLog

/// 台語鍵盤動作處理器
///
/// 繼承自 KeyboardKit 的 `StandardActionHandler`，處理所有按鍵動作。
/// 主要功能包含：
/// - 字元輸入與組字管理
/// - 候選詞選擇
/// - NextWord 下一詞預測
///
/// - Note: 相關 extension 定義於 `ActionHandler+*.swift`
public class ActionHandler: KeyboardAction.StandardActionHandler {

    // MARK: - 屬性

    let logger = Logger(
        subsystem: "com.siansiansu.taigikeyboard",
        category: "ActionHandler"
    )

    let settings = SharedSettings.shared
    public let composingManager = ComposingManager()
    weak var keyboardViewController: KeyboardInputViewController?

    /// 空白鍵拖曳狀態（用於區分拖曳移動游標與點擊輸入空白）
    private var isSpaceDragInProgress = false

    // MARK: - NextWord 狀態

    /// 前一個選中的詞（用於記錄詞彙關聯）
    var lastSelectedWord: String?
    var lastSelectionTime: Int64 = 0
    var isShowingNextWord: Bool = false
    private var contextTimeoutTimer: Timer?

    private enum NextWordConstants {
        /// 連續選詞間隔上限（超過則不記錄關聯）
        static let associationTimeoutMs: Int64 = 10_000
        /// 上下文超時（超過則清除 NextWord 狀態）
        static let contextTimeoutMs: Int64 = 30_000
        static let contextTimeoutSeconds: TimeInterval = 30.0
        /// 句末標點（遇到時重置 NextWord 上下文）
        static let sentenceEndPunctuation = Set<Character>(["。", "！", "？", ".", "!", "?"])
    }

    // MARK: - 動作分發

    /// 處理台語鍵盤特定動作
    /// - Returns: 是否已處理（true 表示不需繼續傳遞給 KeyboardKit）
    private func handleTaigiSpecificAction(_ action: KeyboardAction) -> Bool {
        switch action {
        case .settings:
            openMainAppSettings()
            return false

        case let .character(char):
            return handleCharacterInput(char)

        case .space:
            return handleSpaceAction()

        case .primary(.return):
            return handleReturnAction()

        case .backspace:
            return handleBackspaceAction()

        case let .custom(name):
            handleCustomAction(name)
            return false

        default:
            return false
        }
    }

    // MARK: - KeyboardKit Override

    override public func handle(_ gesture: Keyboard.Gesture, on action: KeyboardAction) {
        // 空白鍵拖曳狀態追蹤
        if action == .space {
            switch gesture {
            case .longPress:
                isSpaceDragInProgress = true
            case .release:
                if isSpaceDragInProgress {
                    isSpaceDragInProgress = false
                    super.handle(gesture, on: action) // 確保 KeyboardKit 處理拖曳結束
                    return
                }
                isSpaceDragInProgress = false
            case .end:
                // 重置拖曳狀態
                isSpaceDragInProgress = false
            default:
                break
            }
        }

        guard gesture == .release else {
            if action == .backspace {
                // 允許退格鍵的重複按壓手勢
                if gesture == .repeatPress {
                    _ = handleBackspaceAction()
                }
                return
            }

            // DEBUG: 追蹤 keyboardType 切換時的 keyboardCase 變化
            if case .keyboardType = action {
                let beforeCase = keyboardContext.keyboardCase
                logger.debug("[CASE][handle] BEFORE super.handle(\(String(describing: gesture), privacy: .public), \(String(describing: action), privacy: .public)): keyboardCase=\(String(describing: beforeCase), privacy: .public)")
                super.handle(gesture, on: action)
                let afterCase = keyboardContext.keyboardCase
                logger.debug("[CASE][handle] AFTER super.handle: keyboardCase=\(String(describing: afterCase), privacy: .public)")
                if beforeCase != afterCase {
                    logger.debug("[CASE][handle] ⚠️ keyboardCase CHANGED from \(String(describing: beforeCase), privacy: .public) to \(String(describing: afterCase), privacy: .public)")
                }
                return
            }

            super.handle(gesture, on: action)
            return
        }

        let handled = handleTaigiSpecificAction(action)
        if handled {

            // 對齊 Android 行為：以下情況不觸發 autocomplete（保留 NextWord 候選詞）
            // 1. 非組字模式按空白鍵
            // 2. 非組字模式輸入 "-" 且正在顯示 NextWord
            let skipAutocomplete: Bool = {
                if !composingManager.isComposing {
                    if action == .space {
                        return true
                    }
                    if case .character("-") = action, isShowingNextWord {
                        return true
                    }
                }
                return false
            }()

            if !skipAutocomplete {
                keyboardController?.performAutocomplete()
            }
            return
        }
        super.handle(gesture, on: action)
    }

    override public func handle(_ suggestion: Autocomplete.Suggestion) {
        // 英文模式：使用 KeyboardKit 預設處理（會自動刪除已輸入的字元再插入）
        if settings.inputMode == .english {
            super.handle(suggestion)
            return
        }
        // 台語模式：使用自定義處理
        handleSuggestionSelection(suggestion)
    }

    override public func handle(_ action: KeyboardAction) {
        handle(.release, on: action)
    }

    /// 覆寫 KeyboardKit 的 keyboardCase 自動調整
    ///
    /// - Shift 動作：始終讓 super 處理（保留 doubleTap → Caps Lock 功能）
    /// - 其他動作：只在自動大寫開啟時調用 super，避免 KeyboardKit 自動將 keyboardCase 改為大寫
    override public func tryChangeKeyboardCase(
        after gesture: Keyboard.Gesture,
        on action: KeyboardAction
    ) {
        let beforeCase = keyboardContext.keyboardCase
        let isAutoCap = keyboardContext.settings.isAutocapitalizationEnabled

        logger.debug("[CASE][tryChange] gesture=\(String(describing: gesture), privacy: .public) action=\(String(describing: action), privacy: .public) before=\(String(describing: beforeCase), privacy: .public) isAutoCap=\(isAutoCap, privacy: .public)")

        // Shift 動作：始終讓 super 處理（包括 doubleTap → Caps Lock）
        if case .shift = action {
            super.tryChangeKeyboardCase(after: gesture, on: action)
            logger.debug("[CASE][tryChange] after shift: \(String(describing: self.keyboardContext.keyboardCase), privacy: .public)")
            return
        }

        // 其他動作：只在自動大寫開啟時調用 super
        if isAutoCap {
            super.tryChangeKeyboardCase(after: gesture, on: action)
            logger.debug("[CASE][tryChange] after autoCap: \(String(describing: self.keyboardContext.keyboardCase), privacy: .public)")
        } else {
            logger.debug("[CASE][tryChange] skipped (autoCap=false)")
        }
    }

    // MARK: - NextWord 狀態管理

    func resetNextWordContext() {
        lastSelectedWord = nil
        lastSelectionTime = 0
        isShowingNextWord = false
        stopContextTimeoutTimer()
    }

    func startContextTimeoutTimer() {
        stopContextTimeoutTimer()
        contextTimeoutTimer = Timer.scheduledTimer(
            withTimeInterval: NextWordConstants.contextTimeoutSeconds,
            repeats: false
        ) { [weak self] _ in
            self?.handleContextTimeout()
        }
    }

    func stopContextTimeoutTimer() {
        contextTimeoutTimer?.invalidate()
        contextTimeoutTimer = nil
    }

    private func handleContextTimeout() {
        logger.debug("[NEXTWORD] Context timeout - resetting")
        resetNextWordContext()
        if isShowingNextWord {
            DispatchQueue.main.async { [weak self] in
                self?.keyboardController?.state.autocompleteContext.reset()
            }
        }
    }

    /// 檢查是否應記錄詞彙關聯（間隔需小於 10 秒）
    func shouldRecordAssociation() -> Bool {
        guard lastSelectedWord != nil else { return false }
        let now = Int64(Date().timeIntervalSince1970 * 1000)
        return (now - lastSelectionTime) < NextWordConstants.associationTimeoutMs
    }

    /// 檢查是否為雜訊（標點、空白、純數字不觸發 NextWord）
    func isNoiseText(_ text: String) -> Bool {
        guard let firstChar = text.first else { return true }
        let punctuation = "。！？.!?，,、；;：:「」『』\"\"\u{2018}\u{2019}（）()【】[]{}—–-～~…·"
        if punctuation.contains(firstChar) { return true }
        if firstChar.isWhitespace { return true }
        if text.allSatisfy({ $0.isNumber }) { return true }
        return false
    }

    func isSentenceEndPunctuation(_ text: String) -> Bool {
        guard let firstChar = text.first else { return false }
        return NextWordConstants.sentenceEndPunctuation.contains(firstChar)
    }

    func updateNextWordState(selectedWord: String) {
        lastSelectedWord = selectedWord
        lastSelectionTime = Int64(Date().timeIntervalSince1970 * 1000)
        startContextTimeoutTimer()
    }

    /// 更新前一詞記錄（不觸發預測）
    ///
    /// 用於空白鍵確認組字時，記錄已輸出的文字讓後續輸入可建立關聯。
    func updateLastSelectedWord(_ word: String) {
        guard !word.isEmpty, !isNoiseText(word) else { return }

        lastSelectedWord = word
        lastSelectionTime = Int64(Date().timeIntervalSince1970 * 1000)
        recordCompoundWordAssociationsForSpace(word: word)
        startContextTimeoutTimer()
    }

    /// 記錄複合詞內部關聯（如 tshit-niû → tshit, niû）
    private func recordCompoundWordAssociationsForSpace(word: String) {
        let parts = word.split(separator: "-").map(String.init).filter { !$0.isEmpty }

        guard parts.count > 1 else { return }

        let useTl = (settings.inputMode == .tl)

        Task {
            for i in 0..<(parts.count - 1) {
                let prevPart = parts[i]
                let nextPart = parts[i + 1]

                // 空白確認時沒有羅馬字資訊，只記錄漢字關聯
                let nextTl = useTl ? nextPart : ""
                let nextPoj = useTl ? "" : nextPart

                await NextWordService.shared.recordAssociation(
                    prev: prevPart,
                    nextHanzi: nextPart,
                    nextTl: nextTl,
                    nextPoj: nextPoj
                )
            }
        }
    }

    // MARK: - NextWord 預測

    /// 根據指定詞彙觸發下一詞預測
    func triggerNextWordPrediction(for word: String) {
        Task { @MainActor in
            let predictions = await NextWordService.shared.predict(word: word)

            if predictions.isEmpty {
                isShowingNextWord = false
                keyboardController?.state.autocompleteContext.reset()
                return
            }

            // 轉換為候選詞格式（羅馬字模式下過濾無羅馬字的項目）
            let suggestions = predictions.compactMap { prediction -> Autocomplete.Suggestion? in
                if !settings.isTranslateSwapped && prediction.tl.isEmpty && prediction.poj.isEmpty {
                    return nil
                }

                let roman = prediction.tl.isEmpty ? prediction.poj : prediction.tl
                let text = roman.isEmpty ? prediction.hanzi : roman
                let subtitle: String? = roman.isEmpty ? nil : prediction.hanzi

                return Autocomplete.Suggestion(
                    text: text,
                    title: text,
                    subtitle: subtitle,
                    additionalInfo: [
                        "isNextWord": "true",
                        "hanzi": prediction.hanzi,
                        "tl": prediction.tl,
                        "poj": prediction.poj,
                        "displayText": prediction.hanzi
                    ]
                )
            }

            if let controller = keyboardViewController {
                if suggestions.isEmpty {
                    isShowingNextWord = false
                    controller.state.autocompleteContext.reset()
                } else {
                    controller.state.autocompleteContext.suggestionsFromService = suggestions
                    isShowingNextWord = true
                    startContextTimeoutTimer()
                }
            }
        }
    }
}
