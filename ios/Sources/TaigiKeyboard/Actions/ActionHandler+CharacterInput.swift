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

        logger.debug("[AUTOCAP][INPUT] char='\(char)' keyboardCase=\(String(describing: currentCase)) autoCap=\(autoCap)")

        guard !char.isEmpty else {
            return false
        }

        // 使用 CaseTransformer 統一處理大小寫轉換
        let processedChar = CaseTransformer.transformForInput(
            char,
            keyboardCase: currentCase,
            isAutoCapitalizationEnabled: autoCap,
            inputMode: settings.inputMode,
        )

        // TPS layout: context-aware character adjustments
        let finalChar: String
        if settings.inputMode == .tps {
            var adjusted = TPSConverter.adjustTPSInitialKey(processedChar, afterRawInput: composingManager.rawInput)
            adjusted = TPSConverter.adjustTPSNasalizedVowelKey(adjusted, afterRawInput: composingManager.rawInput)
            // Syllabic nasal auto-correct: ㄇ+tone → ㆬ, ㄫ+tone → ㆭ
            if let nasalReplacement = TPSConverter.syllabicNasalReplacement(forIncoming: adjusted, lastRawChar: composingManager.rawInput.last) {
                composingManager.replaceLastCharacter(with: nasalReplacement)
            }
            // Palatalization auto-correct: ㄗ/ㄘ/ㄙ/ㆡ + ㄧ/ㆪ → ㄐ/ㄑ/ㄒ/ㆢ
            if let replacement = TPSConverter.palatalizationReplacement(forIncoming: adjusted, lastRawChar: composingManager.rawInput.last) {
                composingManager.replaceLastCharacter(with: replacement)
            }
            finalChar = adjusted
        } else {
            finalChar = processedChar
        }

        logger.debug("[AUTOCAP][INPUT] processedChar='\(finalChar)'")

        // 英文模式：直接插入字元，不進入組字邏輯
        if settings.inputMode == .english {
            keyboardContext.textDocumentProxy.insertText(finalChar)
            // 處理單次 Shift 復位（Caps Lock 除外）
            if keyboardContext.keyboardCase == .uppercased {
                keyboardContext.keyboardCase = .lowercased
            }
            return true
        }

        // 以下為台語模式（POJ/TL/TPS）的組字邏輯

        // Standalone digit: commit directly without entering composing mode.
        // Digits only enter composing as tone markers appended to existing romanization.
        if !composingManager.isComposing, finalChar.first?.isNumber == true {
            if isShowingNextWord {
                isShowingNextWord = false
                keyboardController?.state.autocompleteContext.reset()
            }
            keyboardContext.textDocumentProxy.insertText(finalChar)
            return true
        }

        // 組字字元（字母、TPS 符號、連字符號、˙）→ 進入組字
        if isComposingCharacter(finalChar) {
            if composingManager.isComposing {
                if finalChar == "-" {
                    composingManager.appendHyphen()
                } else {
                    composingManager.appendCharacter(finalChar)
                }
            } else {
                // 非組字模式：檢查是否正在顯示 NextWord 候選詞
                if finalChar == "-", isShowingNextWord {
                    // NextWord 模式下輸入 "-"：直接輸出，保留 NextWord 候選詞
                    keyboardContext.textDocumentProxy.insertText("-")
                    logger.debug("[INPUT] '-' committed in NextWord mode, keeping suggestions")
                } else {
                    if isShowingNextWord {
                        isShowingNextWord = false
                        keyboardController?.state.autocompleteContext.reset()
                    }
                    composingManager.startComposing(with: finalChar)
                }
            }
        } else if composingManager.isComposing, finalChar.first?.isNumber == true {
            // 組字中輸入數字 → 作為聲調標記追加
            composingManager.appendCharacter(finalChar)
        } else {
            // 非組字字元（標點、符號、箭頭等）→ 確認組字後直接輸出
            if composingManager.isComposing {
                composingManager.commitComposition()
            }
            keyboardContext.textDocumentProxy.insertText(finalChar)
            return true
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
        if let keyboardController {
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

        // TPS mode: space as tone 1/4 syllable boundary marker.
        // If the current syllable has no explicit tone mark, space adds a syllable
        // boundary and stays in composing mode (like Microsoft Zhuyin's space for tone 1).
        // If the syllable already has a tone mark or ends with space, fall through to commit.
        if settings.inputMode == .tps, composingManager.isComposing {
            if let lastChar = composingManager.rawInput.last,
               !TPSConverter.isTPSToneMark(lastChar), lastChar != " "
            {
                composingManager.appendCharacter(" ")
                return true
            }
        }

        // 以下為台語模式（POJ/TL）的邏輯
        if composingManager.isComposing {
            // 在確認之前先取得組字文字
            let committedText = composingManager.composingText

            // 確認當前組字 + 插入空白（不選擇候選詞）
            composingManager.commitComposition()
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
        let caseBefore = String(describing: keyboardContext.keyboardCase)
        logger.debug("[AUTOCAP][BACKSPACE] BEFORE delete: \(caseBefore)")

        // 英文模式：直接刪除
        if settings.inputMode == .english {
            keyboardContext.textDocumentProxy.deleteBackward()
            let caseAfter = String(describing: keyboardContext.keyboardCase)
            logger.debug("[AUTOCAP][BACKSPACE] AFTER delete: \(caseAfter)")
            return true
        }

        // 以下為台語模式（POJ/TL）的邏輯
        if composingManager.isComposing {
            composingManager.deleteBackward()
        } else {
            keyboardContext.textDocumentProxy.deleteBackward()
            handleBackspaceForNextWord()
        }

        let caseAfter = String(describing: keyboardContext.keyboardCase)
        logger.debug("[AUTOCAP][BACKSPACE] AFTER delete: \(caseAfter)")
        return true
    }

    /// 退格後重新預測 NextWord（根據剩餘文字的最後一個字）
    private func handleBackspaceForNextWord() {
        let textBeforeCursor = keyboardContext.textDocumentProxy.documentContextBeforeInput ?? ""
        let trimmedText = textBeforeCursor.trimmingCharacters(in: .whitespacesAndNewlines)

        if trimmedText.isEmpty {
            // 文字已清空，清除 NextWord 候選詞並重置上下文
            let wasShowingNextWord = isShowingNextWord
            resetNextWordContext()
            if wasShowingNextWord {
                keyboardController?.state.autocompleteContext.reset()
            }
            return
        }

        // 取得最後一個字進行預測
        let lastChar = String(trimmedText.last!)

        // 更新上下文（但不記錄關聯，因為是退格操作）
        lastSelectedWord = lastChar
        lastSelectedRoman = nil
        lastSelectionTime = Int64(Date().timeIntervalSince1970 * 1000)

        // 觸發 NextWord 預測
        triggerNextWordPrediction(for: lastChar)
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
            // Capture rawInput before commit clears it
            let capturedRawInput = composingManager.rawInput

            if composingManager.selectedCandidateIndex == 0 {
                // Enter at index 0: commit raw input (literal keystrokes)
                // This allows English words to pass through without tone conversion
                // (Google Pinyin convention: Enter = raw Latin text, Space = converted text)
                composingManager.commitRawInput()
                handleEnterNextWordPrediction(committedText: capturedRawInput, rawInput: capturedRawInput)
            } else {
                // 選中候選詞，確認該候選詞
                let suggestions = keyboardController?.state.autocompleteContext.suggestions ?? []
                _ = composingManager.confirmSelectedCandidate(availableSuggestions: suggestions)
            }

            // 羅馬字模式：自動加空白（字尾非連字符時）
            if settings.isAutoSpaceEnabled, !settings.isTranslateSwapped {
                if !capturedRawInput.hasSuffix("-") {
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
