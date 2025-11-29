import Foundation
import KeyboardKit

// MARK: - Character Input

extension ActionHandler {
    /// 處理字元輸入
    /// - Parameter char: 輸入的字元
    /// - Returns: 是否已處理該動作
    func handleCharacterInput(_ char: String) -> Bool {
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
            composingManager.startComposing(with: processedChar)
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
            // 確認當前組字 + 插入空白（不選擇候選詞）
            self.composingManager.commitComposition()
            keyboardContext.textDocumentProxy.insertText(" ")
        } else {
            keyboardContext.textDocumentProxy.insertText(" ")
        }
        return true
    }

    /// 處理退格鍵動作
    /// - Returns: 是否已處理該動作
    func handleBackspaceAction() -> Bool {
        if composingManager.isComposing {
            composingManager.deleteBackward()
        } else {
            keyboardContext.textDocumentProxy.deleteBackward()
        }
        return true
    }

    /// 處理 Return 鍵動作
    /// - Returns: 是否已處理該動作
    func handleReturnAction() -> Bool {
        if composingManager.isComposing {
            // 檢查當前選中的候選詞索引
            if composingManager.selectedCandidateIndex == 0 {
                // 選中第 0 個候選詞（組字文字），直接確認組字
                composingManager.commitComposition()
            } else {
                // 選中其他候選詞，確認選中的候選詞
                let suggestions = keyboardController?.state.autocompleteContext.suggestions ?? []
                _ = composingManager.confirmSelectedCandidate(availableSuggestions: suggestions)
            }

            // 羅馬字模式：確認候選詞後自動加空白
            if settings.isAutoSpaceEnabled && !settings.isTranslateSwapped {
                keyboardContext.textDocumentProxy.insertText(" ")
            }

            return true
        } else {
            keyboardContext.textDocumentProxy.insertText("\n")
        }
        return true
    }
}
