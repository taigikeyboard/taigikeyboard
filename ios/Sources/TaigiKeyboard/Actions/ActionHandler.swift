import Foundation
import KeyboardKit

/// 處理台語鍵盤所有按鍵動作的處理器
/// 繼承自 KeyboardKit 的標準動作處理器，加入台語特定的處理邏輯
public class ActionHandler: KeyboardAction.StandardActionHandler {
    /// 共用設定管理器
    let settings = SharedSettings.shared

    /// 組字管理器，處理台語羅馬字輸入
    public let composingManager = ComposingManager()

    /// 追蹤空白鍵拖曳狀態，用於區分拖曳和點擊
    private var isSpaceDragInProgress = false

    /// 鍵盤視圖控制器的弱引用，避免循環引用
    weak var keyboardViewController: KeyboardInputViewController?

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
            keyboardController?.performAutocomplete()
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
}
