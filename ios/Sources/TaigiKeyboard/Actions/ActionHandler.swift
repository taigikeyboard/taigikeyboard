// 中文: 鍵盤手勢入口 — 把 KeyboardKit gesture 派送到各 action 處理器。
// 中文: 主檔負責 dispatch + KeyboardKit 覆寫;細部邏輯在 ActionHandler+*.swift 各擴充檔。

import Foundation
import KeyboardKit

/// Taigi keyboard action handler — dispatches keyboard gestures to per-action handlers.
///
/// **Action flow** (gesture → output):
/// 1. `handle(_:on:)` — KeyboardKit entry point, filters gesture type
/// 2. `handleTaigiSpecificAction` — dispatches by action type
/// 3. Per-action handlers (in extension files):
///    - `handleCharacterInput` → composing / direct output  (KeyActions)
///    - `handleSpaceAction` → commit composing / insert space  (KeyActions)
///    - `handleReturnAction` → commit raw or selected candidate  (KeyActions)
///    - `handleBackspaceAction` → delete / re-predict NextWord  (KeyActions)
///    - `handleSuggestionSelection` → commit + frequency + NextWord  (Suggestions)
/// 4. `nextWordController.process()` — record association → update state → predict  (NextWordController)
// 中文: 台語鍵盤的 ActionHandler。手勢入口走 handle(_:on:),收 release / repeatPress 後派送。
// 中文: 跨檔協作:KeyActions / Suggestions / CustomActions / Utilities 各掌一塊,主檔只做 dispatch。
public class ActionHandler: StandardKeyboardActionHandler {
    // MARK: - Properties

    let logger = DebugLogger(category: "ActionHandler")

    let settings = SharedSettings.shared
    public let composingManager = ComposingManager()
    let nextWordController = NextWordController()

    // MARK: - Action Dispatch

    /// - Returns: true if handled (skip KeyboardKit default)
    // 中文: 把 KeyboardAction 派送到對應的 handler。回 true 代表已處理,跳過 KeyboardKit 預設行為。
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

    // 中文: KeyboardKit 手勢覆寫入口。先處理 spacebar 拖曳手勢結束 → 過濾 release / repeatPress → 派送給 Taigi handler,
    // 中文: 未處理者最後落到 super 的預設行為。
    override public func handle(_ gesture: Keyboard.Gesture, on action: KeyboardAction) {
        if action == .space, isSpacebarDragGestureEnding(gesture) {
            // A spacebar long-press moves the cursor instead of typing, so this gesture
            // must not emit a space — hand the teardown to KeyboardKit.
            // KeyboardKit 10.9 rebuilt `SpacebarDragGestureHandler` as closure-driven and
            // removed the public `currentDragTextPositionOffset` this branch once reset
            // (the issue-#545 workaround for a leaked drag offset that permanently killed
            // the spacebar). Whether KK still keeps equivalent private state is unknowable
            // (binary-only); drag-end behavior must be re-verified on device after any KK
            // upgrade.
            // 中文: KeyboardKit 10.9 重寫拖曳手勢,#545 歸零 workaround 所依賴的 public offset
            // 中文: 已移除、workaround 被迫退役;KK 內部是否仍有等價私有狀態不可知,拖曳後
            // 中文: 空白鍵行為仍需實機 dogfood 驗證。
            super.handle(gesture, on: action)
            return
        }

        guard gesture == .release else {
            if action == .backspace {
                // Allow backspace repeat-press gesture
                if gesture == .repeatPress {
                    TraceContext.with(TraceId.next()) {
                        logger.debug("[INPUT] fn=handle gesture=repeatPress action=\(String(describing: action))")
                        _ = handleBackspaceAction()
                    }
                }
                return
            }

            super.handle(gesture, on: action)
            return
        }

        var handled = false
        TraceContext.with(TraceId.next()) {
            logger.debug("[INPUT] fn=handle gesture=release action=\(String(describing: action))")
            handled = handleTaigiSpecificAction(action)
            if handled, !shouldSkipAutocomplete(for: action) {
                keyboardController?.performAutocomplete()
            }
        }
        if handled {
            return
        }
        super.handle(gesture, on: action)
    }

    /// Whether `gesture` ends an active spacebar drag gesture — the long-press sequence
    /// that moves the input cursor, whether or not the finger actually moved.
    ///
    /// `keyboardContext.isSpacebarDragGestureActive` is KeyboardKit's own drag state.
    /// Observed on KK ≤ 10.4: `.longPress` sets it, and `.release` / `.end` clear it
    /// *inside* `super.handle`, so it must be read before dispatching to `super`. KK is
    /// binary-only since 10.9 — the timing is an observation to re-verify on device, not
    /// a documented contract. `.end` is checked as well because a cancelled gesture
    /// delivers `.end` without a preceding `.release`.
    // 中文: 判定此手勢是否結束一個進行中的 spacebar 拖曳手勢(長按移游標,不論手指有無真的移動)。
    // 中文: 用 KeyboardKit 自己的 drag 狀態當唯一來源;「super 會把它清掉」是 ≤10.4 的觀察,
    // 中文: 10.9 起閉源無法查證,升級後靠實機驗證。
    private func isSpacebarDragGestureEnding(_ gesture: Keyboard.Gesture) -> Bool {
        switch gesture {
        case .release, .end:
            return keyboardContext.isSpacebarDragGestureActive
        default:
            return false
        }
    }

    /// Align with Android: skip autocomplete to preserve NextWord suggestions
    /// when not composing and pressing space or "-" during NextWord
    // 中文: 在 NextWord 顯示中按空白或 "-" 時跳過 autocomplete,避免清掉 NextWord 候選。與 Android 對齊。
    private func shouldSkipAutocomplete(for action: KeyboardAction) -> Bool {
        guard !composingManager.isComposing else { return false }
        if action == .space { return true }
        if case .character("-") = action, nextWordController.isShowing { return true }
        return false
    }

    // 中文: 候選詞點選的 KeyboardKit 入口。English 模式走 KK 預設,Taigi 模式走自家路徑。
    override public func handle(_ suggestion: AutocompleteSuggestion) {
        // English mode: use KeyboardKit default (auto-deletes typed chars then inserts)
        if settings.inputMode == .english {
            super.handle(suggestion)
            return
        }
        // Taigi mode: custom handling
        TraceContext.with(TraceId.next()) {
            handleSuggestionSelection(suggestion)
        }
    }

    override public func handle(_ action: KeyboardAction) {
        handle(.release, on: action)
    }

}

// MARK: - AutocompleteContextUpdater

// 中文: 把 NextWord 引擎結果寫進 KeyboardKit autocomplete context 的單一通道。
// 中文: 這是引擎端 prediction 唯一接觸 KeyboardKit 型別的地方。
extension ActionHandler: AutocompleteContextUpdater {
    /// Engine-side predictions arrive here and are mapped to KeyboardKit
    /// `AutocompleteSuggestion` values. This is the only place the
    /// engine's `RustEngineBridge.NextWordEnginePrediction` touches
    /// KeyboardKit types.
    // 中文: 把引擎回傳的 NextWord 預測映射為 KeyboardKit 的 AutocompleteSuggestion。
    func setNextWordPredictions(_ predictions: [RustEngineBridge.NextWordEnginePrediction]) {
        let suggestions = predictions.map { prediction in
            AutocompleteSuggestion(
                text: prediction.text,
                title: prediction.text,
                subtitle: prediction.subtitle,
                additionalInfo: [
                    "isNextWord": "true",
                    "hanzi": prediction.hanzi,
                    // Raw TL sidechannel — NOT the commit string. Consumed only
                    // by the association-recording fork in
                    // `handleSuggestionSelection` (engine's `pojToTL` needs raw
                    // roman, not the mode-shaped display `text`).
                    "tl": prediction.tl,
                    // R5 pair-key (#7): canonical-TL reading for the
                    // user-frequency `(displayText, canonicalTl)` write.
                    // `prediction.tl` is the engine-side canonical TL (only
                    // `text`/`subtitle` are mode-shaped), matching the
                    // Continuous read key. Without it the freq write would
                    // land in the legacy `tl == ""` bucket and 重/tāng could
                    // inherit a count learned from 重/tîng.
                    "canonicalTl": prediction.tl,
                    "displayText": prediction.hanzi,
                ],
            )
        }
        keyboardController?.state.autocompleteContext.suggestionsFromService = suggestions
    }

    // 中文: 清空 NextWord 顯示 — 走 KeyboardKit autocomplete reset。
    func resetNextWordSuggestions() {
        keyboardController?.state.autocompleteContext.reset()
    }
}
