import Foundation
import KeyboardKit
import OSLog

/// 處理台語鍵盤所有按鍵動作的處理器
/// 繼承自 KeyboardKit 的標準動作處理器，加入台語特定的處理邏輯
public class ActionHandler: KeyboardAction.StandardActionHandler {
    /// 日誌記錄器
    let logger = Logger(
        subsystem: "com.siansiansu.taigikeyboard",
        category: "ActionHandler"
    )

    /// 共用設定管理器
    let settings = SharedSettings.shared

    /// 組字管理器，處理台語羅馬字輸入
    public let composingManager = ComposingManager()

    /// 追蹤空白鍵拖曳狀態，用於區分拖曳和點擊
    private var isSpaceDragInProgress = false

    /// 鍵盤視圖控制器的弱引用，避免循環引用
    weak var keyboardViewController: KeyboardInputViewController?

    // MARK: - NextWord 狀態追蹤

    /// 前一個選中的詞（用於 NextWord 關聯記錄）
    var lastSelectedWord: String?

    /// 上次選擇時間（毫秒）
    var lastSelectionTime: Int64 = 0

    /// 是否正在顯示 NextWord 候選詞
    var isShowingNextWord: Bool = false

    /// NextWord 上下文超時 Timer
    private var contextTimeoutTimer: Timer?

    /// NextWord 超時設定
    private enum NextWordConstants {
        static let associationTimeoutMs: Int64 = 10_000  // 連續選詞間隔 10 秒
        static let contextTimeoutMs: Int64 = 30_000      // 上下文超時 30 秒
        static let contextTimeoutSeconds: TimeInterval = 30.0
        static let sentenceEndPunctuation = Set<Character>(["。", "！", "？", ".", "!", "?"])
    }

    /// 處理台語鍵盤特定的動作
    /// - Parameter action: 鍵盤動作
    /// - Returns: 是否已處理該動作（true 表示已處理，不需要繼續傳遞）
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

    /// 判斷是否能處理指定的手勢
    /// - Returns: 永遠回傳 true，表示可以處理所有手勢
    override public func canHandle(_: Keyboard.Gesture, on _: KeyboardAction)
        -> Bool
    { true }

    /// 處理鍵盤手勢
    /// - Parameters:
    ///   - gesture: 手勢類型（點擊、長按、拖曳等）
    ///   - action: 對應的鍵盤動作
    override public func handle(_ gesture: Keyboard.Gesture, on action: KeyboardAction) {
        // 追蹤空白鍵拖曳狀態
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

    /// 處理候選詞選擇
    /// - Parameter suggestion: 使用者選擇的候選詞
    override public func handle(_ suggestion: Autocomplete.Suggestion) {
        handleSuggestionSelection(suggestion)
    }

    /// 處理鍵盤動作的簡化介面
    /// 預設使用 release 手勢
    override public func handle(_ action: KeyboardAction) { handle(.release, on: action) }

    // MARK: - NextWord Methods

    /// 重置 NextWord 上下文
    func resetNextWordContext() {
        lastSelectedWord = nil
        lastSelectionTime = 0
        isShowingNextWord = false
        stopContextTimeoutTimer()
    }

    /// 啟動上下文超時 Timer
    func startContextTimeoutTimer() {
        stopContextTimeoutTimer()

        contextTimeoutTimer = Timer.scheduledTimer(
            withTimeInterval: NextWordConstants.contextTimeoutSeconds,
            repeats: false
        ) { [weak self] _ in
            self?.handleContextTimeout()
        }
    }

    /// 停止上下文超時 Timer
    func stopContextTimeoutTimer() {
        contextTimeoutTimer?.invalidate()
        contextTimeoutTimer = nil
    }

    /// 處理上下文超時
    private func handleContextTimeout() {
        logger.debug("[NEXTWORD] Context timeout - resetting")
        resetNextWordContext()

        // 如果正在顯示 NextWord，清除候選詞
        if isShowingNextWord {
            DispatchQueue.main.async { [weak self] in
                self?.keyboardController?.state.autocompleteContext.reset()
            }
        }
    }

    /// 檢查是否應該記錄關聯（間隔 < 10 秒）
    func shouldRecordAssociation() -> Bool {
        guard lastSelectedWord != nil else { return false }
        let now = Int64(Date().timeIntervalSince1970 * 1000)
        return (now - lastSelectionTime) < NextWordConstants.associationTimeoutMs
    }

    /// 檢查是否為雜訊字元（標點符號、空白、數字）
    func isNoiseText(_ text: String) -> Bool {
        guard let firstChar = text.first else { return true }

        // 標點符號
        let punctuation = "。！？.!?，,、；;：:「」『』\"\"\u{2018}\u{2019}（）()【】[]{}—–-～~…·"
        if punctuation.contains(firstChar) { return true }

        // 空白
        if firstChar.isWhitespace { return true }

        // 純數字
        if text.allSatisfy({ $0.isNumber }) { return true }

        return false
    }

    /// 檢查是否為句末標點
    func isSentenceEndPunctuation(_ text: String) -> Bool {
        guard let firstChar = text.first else { return false }
        return NextWordConstants.sentenceEndPunctuation.contains(firstChar)
    }

    /// 更新 NextWord 狀態
    func updateNextWordState(selectedWord: String) {
        lastSelectedWord = selectedWord
        lastSelectionTime = Int64(Date().timeIntervalSince1970 * 1000)

        // 啟動上下文超時 Timer
        startContextTimeoutTimer()
    }

    /// 更新 lastSelectedWord（不觸發 NextWord 預測）
    ///
    /// 用於空白鍵確認組字時，記錄已輸出的文字，
    /// 讓後續輸入可以建立關聯
    ///
    /// - Parameter word: 已輸出的文字
    func updateLastSelectedWord(_ word: String) {
        guard !word.isEmpty else { return }

        // 雜訊不更新 lastSelectedWord
        guard !isNoiseText(word) else { return }

        lastSelectedWord = word
        lastSelectionTime = Int64(Date().timeIntervalSince1970 * 1000)

        // 記錄複合詞內部的關聯（如 tshit-niû → tshit → niû）
        recordCompoundWordAssociationsForSpace(word: word)

        // 啟動上下文超時 Timer
        startContextTimeoutTimer()
    }

    /// 空白確認時記錄複合詞內部關聯
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
}
