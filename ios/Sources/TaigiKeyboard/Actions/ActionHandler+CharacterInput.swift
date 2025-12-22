import Foundation
import KeyboardKit

// MARK: - Character Input

extension ActionHandler {
    /// 處理字元輸入
    /// - Parameter char: 輸入的字元
    /// - Returns: 是否已處理該動作
    func handleCharacterInput(_ char: String) -> Bool {
        logger.debug("[INPUT] char='\(char, privacy: .public)'")

        guard !char.isEmpty else {
            return false
        }

        // 決定實際使用的大小寫狀態
        let effectiveCase: Keyboard.KeyboardCase
        if !settings.isAutoCapitalizationEnabled {
            // 自動大寫關閉：允許手動 Shift 和 Caps Lock，但阻止自動大寫
            // .uppercased 允許（手動按 Shift）
            // .capsLocked 允許（手動雙擊 Shift）
            // .auto 強制為 .lowercased（阻止自動大寫）
            effectiveCase = keyboardContext.keyboardCase == .auto ? .lowercased : keyboardContext.keyboardCase
        } else {
            // 自動大寫開啟：使用當前狀態
            effectiveCase = keyboardContext.keyboardCase
        }

        let processedChar = transformCharacterForCase(char, keyboardCase: effectiveCase)

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

    /// 處理空白鍵動作
    /// - Returns: 是否已處理該動作
    func handleSpaceAction() -> Bool {
        // 檢查是否正在進行空白鍵拖拽移動游標
        if let keyboardController = keyboardController {
            let dragOffset = keyboardController.services.spaceDragGestureHandler.currentDragTextPositionOffset

            if dragOffset != 0 {
                return true
            }
        }

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

    /// 處理退格鍵動作
    /// - Returns: 是否已處理該動作
    func handleBackspaceAction() -> Bool {
        if composingManager.isComposing {
            // 組字中：正常退格
            composingManager.deleteBackward()
        } else {
            // 非組字模式：執行退格
            keyboardContext.textDocumentProxy.deleteBackward()

            // 退格後根據剩餘文字重新預測 NextWord
            handleBackspaceForNextWord()
        }
        return true
    }

    /// 處理退格鍵的 NextWord 預測
    ///
    /// 退格刪除文字後，根據剩餘文字的最後一個字重新預測下一詞。
    /// 若文字已清空，則清除 NextWord 候選詞。
    private func handleBackspaceForNextWord() {
        // 取得游標前的文字
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
        triggerNextWordPredictionForBackspace(lastChar: lastChar)
    }

    /// 退格後觸發 NextWord 預測
    private func triggerNextWordPredictionForBackspace(lastChar: String) {
        Task { @MainActor in
            let predictions = await NextWordService.shared.predict(word: lastChar)

            if predictions.isEmpty {
                isShowingNextWord = false
                keyboardController?.state.autocompleteContext.reset()
                return
            }

            // 將預測結果轉換為 Autocomplete.Suggestion
            let suggestions = predictions.compactMap { prediction -> Autocomplete.Suggestion? in
                // 羅馬字模式下過濾無羅馬字的候選詞
                if !settings.isTranslateSwapped && prediction.tl.isEmpty && prediction.poj.isEmpty {
                    return nil
                }

                // 統一格式：text = 羅馬字, subtitle = 漢字
                // 讓 CandidateView 根據 isTranslateSwapped 統一處理顯示交換
                // 這樣與一般候選詞格式一致，避免雙重交換問題
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

    /// 處理 Return 鍵動作
    /// - Returns: 是否已處理該動作
    func handleReturnAction() -> Bool {
        if composingManager.isComposing {
            // 在確認之前先取得組字文字（確認後會清空）
            let committedText = composingManager.composingText

            // 檢查當前選中的候選詞索引
            if composingManager.selectedCandidateIndex == 0 {
                // 選中第 0 個候選詞（組字文字），直接確認組字
                composingManager.commitComposition()

                // 羅馬字模式：Enter 確認組字後觸發 NextWord 預測
                handleEnterNextWordPrediction(committedText: committedText)
            } else {
                // 選中其他候選詞，確認選中的候選詞
                let suggestions = keyboardController?.state.autocompleteContext.suggestions ?? []
                _ = composingManager.confirmSelectedCandidate(availableSuggestions: suggestions)
                // 注意：選擇候選詞時 handleSuggestionSelection 已經處理 NextWord
            }

            // 羅馬字模式：確認候選詞後自動加空白（字尾非連字符時）
            if settings.isAutoSpaceEnabled && !settings.isTranslateSwapped {
                // 檢查字尾是否為連字符
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
