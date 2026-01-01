import Foundation
import KeyboardKit

/// 字元輸入處理
///
/// 處理字元、空白、退格、Return 鍵的輸入邏輯。
extension ActionHandler {

    // MARK: - 字元輸入

    /// 處理字元輸入（含組字邏輯）
    func handleCharacterInput(_ char: String) -> Bool {
        let currentCase = keyboardContext.keyboardCase
        let autoCap = keyboardContext.settings.isAutocapitalizationEnabled

        logger.debug("[AUTOCAP][INPUT] char='\(char, privacy: .public)' keyboardCase=\(String(describing: currentCase), privacy: .public) autoCap=\(autoCap, privacy: .public)")

        guard !char.isEmpty else {
            return false
        }

        // 使用 CaseTransformationService 統一處理大小寫轉換
        let processedChar = CaseTransformationService.transformForInput(
            char,
            keyboardCase: currentCase,
            isAutoCapitalizationEnabled: autoCap,
            inputMode: settings.inputMode
        )

        logger.debug("[AUTOCAP][INPUT] processedChar='\(processedChar, privacy: .public)'")

        // 英文模式：直接插入字元，不進入組字邏輯
        if settings.inputMode == .english {
            keyboardContext.textDocumentProxy.insertText(processedChar)
            // 處理單次 Shift 復位（Caps Lock 除外）
            if keyboardContext.keyboardCase == .uppercased {
                keyboardContext.keyboardCase = .lowercased
            }
            return true
        }

        // 以下為台語模式（POJ/TL）的組字邏輯

        // 檢查是否為標點符號（除了連字符號）
        if isPunctuationExceptHyphen(processedChar) {
            // 如果正在組字，先確認組字
            if composingManager.isComposing {
                composingManager.commitComposition()
            }
            // 直接插入標點符號
            keyboardContext.textDocumentProxy.insertText(processedChar)
            return true
        }

        // 原有的組字邏輯（只處理字母、數字和連字符號）
        if composingManager.isComposing {
            if processedChar == "-" {
                composingManager.appendHyphen()
            } else {
                composingManager.appendCharacter(processedChar)
            }
        } else {
            // 非組字模式：檢查是否正在顯示 NextWord 候選詞
            if processedChar == "-" && isShowingNextWord {
                // NextWord 模式下輸入 "-"：直接輸出，保留 NextWord 候選詞
                // 用戶可以繼續點選 NextWord，或輸入其他字開始組字
                keyboardContext.textDocumentProxy.insertText("-")
                logger.debug("[INPUT] '-' committed in NextWord mode, keeping suggestions")
            } else {
                // 開始新組字時清除 NextWord 狀態
                if isShowingNextWord {
                    isShowingNextWord = false
                    keyboardController?.state.autocompleteContext.reset()
                }
                composingManager.startComposing(with: processedChar)
            }
        }

        // 處理單次 Shift 復位（Caps Lock 除外）
        if keyboardContext.keyboardCase == .uppercased {
            keyboardContext.keyboardCase = .lowercased
        }

        return true
    }

    // MARK: - 空白鍵

    func handleSpaceAction() -> Bool {
        // 拖曳移動游標時不處理
        if let keyboardController = keyboardController {
            let dragOffset = keyboardController.services.spacebarDragGestureHandler.currentDragTextPositionOffset

            if dragOffset != 0 {
                return true
            }
        }

        // 英文模式：直接插入空白
        if settings.inputMode == .english {
            keyboardContext.textDocumentProxy.insertText(" ")
            return true
        }

        // 以下為台語模式（POJ/TL）的邏輯
        if self.composingManager.isComposing {
            // 在確認之前先取得組字文字
            let committedText = composingManager.composingText

            // 確認當前組字 + 插入空白（不選擇候選詞）
            self.composingManager.commitComposition()
            keyboardContext.textDocumentProxy.insertText(" ")

            // 更新 lastSelectedWord，讓後續輸入可以建立關聯
            // （空白本身不觸發 NextWord 預測，但記錄已輸出的文字）
            updateLastSelectedWord(committedText)
        } else {
            keyboardContext.textDocumentProxy.insertText(" ")
        }
        return true
    }

    // MARK: - 退格鍵

    func handleBackspaceAction() -> Bool {
        logger.debug("[AUTOCAP][BACKSPACE] BEFORE delete: \(String(describing: self.keyboardContext.keyboardCase), privacy: .public)")

        // 英文模式：直接刪除
        if settings.inputMode == .english {
            keyboardContext.textDocumentProxy.deleteBackward()
            logger.debug("[AUTOCAP][BACKSPACE] AFTER delete: \(String(describing: self.keyboardContext.keyboardCase), privacy: .public)")
            return true
        }

        // 以下為台語模式（POJ/TL）的邏輯
        if composingManager.isComposing {
            composingManager.deleteBackward()
        } else {
            keyboardContext.textDocumentProxy.deleteBackward()
            handleBackspaceForNextWord()
        }

        logger.debug("[AUTOCAP][BACKSPACE] AFTER delete: \(String(describing: self.keyboardContext.keyboardCase), privacy: .public)")
        return true
    }

    /// 退格後重新預測 NextWord（根據剩餘文字的最後一個字）
    private func handleBackspaceForNextWord() {
        let textBeforeCursor = keyboardContext.textDocumentProxy.documentContextBeforeInput ?? ""
        let trimmedText = textBeforeCursor.trimmingCharacters(in: .whitespacesAndNewlines)

        if trimmedText.isEmpty {
            // 文字已清空，清除 NextWord 候選詞並重置上下文
            resetNextWordContext()
            if isShowingNextWord {
                isShowingNextWord = false
                keyboardController?.state.autocompleteContext.reset()
            }
            return
        }

        // 取得最後一個字進行預測
        let lastChar = String(trimmedText.last!)

        // 更新上下文（但不記錄關聯，因為是退格操作）
        lastSelectedWord = lastChar
        lastSelectionTime = Int64(Date().timeIntervalSince1970 * 1000)

        // 觸發 NextWord 預測
        triggerNextWordPrediction(for: lastChar)
    }

    // MARK: - Flick 輸入

    /// 處理 Flick 鍵盤的文字輸入
    ///
    /// Flick 輸入的特點：
    /// - 輸入的文字已經帶有調號（如 á, à, â 等）
    /// - 不需要額外的大小寫轉換
    /// - 直接進入組字邏輯
    func handleFlickInput(_ text: String) {
        guard !text.isEmpty else { return }

        // 英文模式：直接插入
        if settings.inputMode == .english {
            keyboardContext.textDocumentProxy.insertText(text)
            return
        }

        // 檢查是否為標點符號
        if isPunctuationExceptHyphen(text) {
            if composingManager.isComposing {
                composingManager.commitComposition()
            }
            keyboardContext.textDocumentProxy.insertText(text)
            return
        }

        // 台語模式：進入組字邏輯
        if composingManager.isComposing {
            // 逐字符加入組字
            for char in text {
                composingManager.appendCharacter(String(char))
            }
        } else {
            // 清除 NextWord 狀態
            if isShowingNextWord {
                isShowingNextWord = false
                keyboardController?.state.autocompleteContext.reset()
            }
            // 開始新組字
            composingManager.startComposing(with: text)
        }
    }

    // MARK: - Return 鍵

    func handleReturnAction() -> Bool {
        // 英文模式：直接插入換行
        if settings.inputMode == .english {
            keyboardContext.textDocumentProxy.insertText("\n")
            return true
        }

        // 以下為台語模式（POJ/TL）的邏輯
        if composingManager.isComposing {
            let committedText = composingManager.composingText

            if composingManager.selectedCandidateIndex == 0 {
                // 選中組字文字，直接確認
                composingManager.commitComposition()
                handleEnterNextWordPrediction(committedText: committedText)
            } else {
                // 選中候選詞，確認該候選詞
                let suggestions = keyboardController?.state.autocompleteContext.suggestions ?? []
                _ = composingManager.confirmSelectedCandidate(availableSuggestions: suggestions)
            }

            // 羅馬字模式：自動加空白（字尾非連字符時）
            if settings.isAutoSpaceEnabled && !settings.isTranslateSwapped {
                if !committedText.hasSuffix("-") {
                    keyboardContext.textDocumentProxy.insertText(" ")
                }
            }
            return true
        } else {
            keyboardContext.textDocumentProxy.insertText("\n")
        }
        return true
    }
}
