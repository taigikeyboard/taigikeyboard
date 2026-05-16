// ActionHandler extension: per-key action handlers (character, space, backspace, return).
// Each handler returns true if handled (skips KeyboardKit default).
// 中文: ActionHandler 的核心鍵動作擴充 — 字元 / 空白 / 退格 / Return。
// 中文: 每個 handler 回 true 代表已處理,呼叫端會跳過 KeyboardKit 預設行為。

import Foundation
import KeyboardKit

extension ActionHandler {
    // MARK: - Character Input

    /// Returns true if handled (skip KeyboardKit default)
    // 中文: 字元鍵入口。會做大小寫轉換、TPS 鍵層調整,再依模式進入組字或直接送字。
    func handleCharacterInput(_ char: String) -> Bool {
        let currentCase = keyboardContext.keyboardCase
        let autoCap = keyboardContext.settings.isAutocapitalizationEnabled

        logger.debug("[AUTOCAP][INPUT] char='\(char)' keyboardCase=\(String(describing: currentCase)) autoCap=\(autoCap)")

        guard !char.isEmpty else {
            return false
        }

        // Unified case transformation. Adapter: KK's Keyboard.KeyboardCase →
        // RustEngineBridge.CaseTransformLetterCase. `autoCap` is read for
        // logging but not passed to `transformInputCase` — the legacy
        // `CaseTransformer.transformForInput` ignored the flag too;
        // auto-cap is consumed by `capitalizeCandidate` instead.
        let processedChar = RustEngineBridge.transformInputCase(
            char,
            letterCase: currentCase.asLetterCase,
            mode: settings.inputMode,
        )

        // TPS key-level adjustments via pure pipeline (returns adjusted char +
        // optional retroactive replacement for the last raw-input char).
        let adjustment = CharacterInputPipeline.adjust(
            processedChar,
            inputMode: settings.inputMode,
            rawInput: composingManager.rawInput,
        )
        if let replacement = adjustment.replaceLast {
            composingManager.replaceLastCharacter(with: replacement)
        }
        let finalChar = adjustment.char

        logger.debug("[AUTOCAP][INPUT] processedChar='\(finalChar)'")

        // English mode: insert directly, skip composing
        if settings.inputMode == .english {
            keyboardContext.textDocumentProxy.insertText(finalChar)
            // Reset single-shift (preserve Caps Lock)
            if keyboardContext.keyboardCase == .uppercased {
                keyboardContext.keyboardCase = .lowercased
            }
            return true
        }

        // Taigi mode (POJ/TL/TPS) composing logic

        // Standalone digit: commit directly without entering composing mode.
        // Digits only enter composing as tone markers appended to existing romanization.
        if !composingManager.isComposing, finalChar.first?.isNumber == true {
            if nextWordController.isShowing {
                nextWordController.clearDisplay()
            }
            keyboardContext.textDocumentProxy.insertText(finalChar)
            return true
        }

        // Composing characters (letters, TPS symbols, hyphen, ˙) → enter composing
        if isComposingCharacter(finalChar) {
            if composingManager.isComposing {
                if finalChar == "-" {
                    composingManager.appendHyphen()
                } else {
                    composingManager.appendCharacter(finalChar)
                }
            } else {
                // Not composing: check NextWord state
                if finalChar == "-", nextWordController.isShowing {
                    // "-" during NextWord: output directly, keep NextWord suggestions
                    keyboardContext.textDocumentProxy.insertText("-")
                    logger.debug("[INPUT] '-' committed in NextWord mode, keeping suggestions")
                } else {
                    if nextWordController.isShowing {
                        nextWordController.clearDisplay()
                    }
                    composingManager.startComposing(with: finalChar)
                }
            }
        } else if composingManager.isComposing, finalChar.first?.isNumber == true {
            // Digit while composing → tone marker
            composingManager.appendCharacter(finalChar)
        } else {
            // Non-composing char (punctuation, symbols, etc.) → commit then output
            if composingManager.isComposing {
                composingManager.commitComposition()
            }
            // Model B §10.3: the engine commit (commitComposition→CommitRaw)
            // already fires the terminal NextWordWordSelected (records the
            // association). Punctuation SUPPRESSES the next-word *display*
            // (converges with Space; resolves the iOS/Android Model-B
            // divergence). Unconditional on this path — mirrors Android's
            // unconditional `clearCandidates()` on the punctuation path so a
            // stale strip is also cleared when NextWord was showing and we
            // were NOT composing (Codex pre-impl P1). clearDisplay() bumps the
            // NextWord generation so the in-flight prediction query is dropped
            // stale; it is a cheap no-op when nothing is showing.
            // 中文: Model B — 標點抑制下詞顯示(與 Space 一致);無條件呼叫對齊
            // 中文: Android,連非組字時的殘留 strip 也一併清掉。
            nextWordController.clearDisplay()
            keyboardContext.textDocumentProxy.insertText(finalChar)
            return true
        }

        // Reset single-shift (preserve Caps Lock)
        if keyboardContext.keyboardCase == .uppercased {
            keyboardContext.keyboardCase = .lowercased
        }

        return true
    }

    // MARK: - Space

    // 中文: 空白鍵 — drag 中略過;English 直接插入;TPS 模式視為音節邊界 / 調 1 標記;
    // 中文: Taigi 模式組字中時送出當前 derived,並交給 NextWord 記錄關聯。
    func handleSpaceAction() -> Bool {
        // Ignore during cursor-drag
        if let keyboardController {
            let dragOffset = keyboardController.services.spacebarDragGestureHandler.currentDragTextPositionOffset

            if dragOffset != 0 {
                return true
            }
        }

        // English mode: insert space directly
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
               !RustEngineBridge.isTPSToneMark(lastChar), lastChar != " "
            {
                composingManager.appendCharacter(" ")
                return true
            }
        }

        // Taigi mode (POJ/TL)
        if composingManager.isComposing {
            // Model B §10.3: the engine commit (commitComposition→CommitRaw)
            // fires the terminal NextWordWordSelected → records the
            // association (the SOLE association source; the old manual
            // `process(triggerPrediction:false)` double-recorded it). Space
            // SUPPRESSES the next-word *display*: clearDisplay() AFTER the
            // commit bumps the NextWord generation so the engine's in-flight
            // prediction query is dropped stale.
            // 中文: Model B — 引擎 commit 已記關聯(唯一來源);Space 用 clearDisplay()
            // 中文: 在 commit 後抑制下詞顯示(舊手動 process 會雙記關聯)。
            composingManager.commitComposition()
            keyboardContext.textDocumentProxy.insertText(" ")
            nextWordController.clearDisplay()
        } else {
            keyboardContext.textDocumentProxy.insertText(" ")
        }
        return true
    }

    // MARK: - Backspace

    // 中文: 退格鍵 — English 直接 deleteBackward;Taigi 組字中走引擎退格,
    // 中文: 否則對輸入框退格並依剩餘上下文重新預測 NextWord。
    func handleBackspaceAction() -> Bool {
        let caseBefore = String(describing: keyboardContext.keyboardCase)
        logger.debug("[AUTOCAP][BACKSPACE] BEFORE delete: \(caseBefore)")

        // English mode: delete directly
        if settings.inputMode == .english {
            keyboardContext.textDocumentProxy.deleteBackward()
            let caseAfter = String(describing: keyboardContext.keyboardCase)
            logger.debug("[AUTOCAP][BACKSPACE] AFTER delete: \(caseAfter)")
            return true
        }

        // Taigi mode
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

    /// Re-predict NextWord after backspace based on last remaining character.
    /// Extracts context from text proxy, then delegates to NextWordController.
    // 中文: 退格後依剩下的最後一個字元重新預測 NextWord(非候選詞選取,不記錄關聯)。
    private func handleBackspaceForNextWord() {
        let textBeforeCursor = keyboardContext.textDocumentProxy.documentContextBeforeInput ?? ""
        let trimmedText = textBeforeCursor.trimmingCharacters(in: .whitespacesAndNewlines)

        if trimmedText.isEmpty {
            nextWordController.resetAndClearUI()
            return
        }

        // Re-predict based on last remaining character (not a word selection — no association recording)
        nextWordController.rePredictAfterBackspace(lastChar: String(trimmedText.last!))
    }

    // MARK: - Return

    // 中文: Return 鍵 — English 直接插入換行;Taigi 組字中時 index 0 送 raw、其它送選中候選,
    // 中文: 並依 isAutoSpaceEnabled 決定是否自動補空白。
    func handleReturnAction() -> Bool {
        // English mode: insert newline directly
        if settings.inputMode == .english {
            keyboardContext.textDocumentProxy.insertText("\n")
            return true
        }

        // Taigi mode
        if composingManager.isComposing {
            // Capture rawInput before commit clears it
            let capturedRawInput = composingManager.rawInput

            if composingManager.selectedCandidateIndex == 0 {
                // Enter at index 0: commit raw input (literal keystrokes)
                // This allows English words to pass through without tone conversion
                // (Google Pinyin convention: Enter = raw Latin text, Space = converted text)
                //
                // Model B §10.3: `commitRawInput()`→CommitRaw → the engine's
                // Continuous final-commit emits the terminal
                // NextWordWordSelected(trigger:true), routed to
                // `nextWordController.process` → records the association +
                // predicts. Enter KEEPS the engine prediction (nothing clears
                // after; only auto-space inserts a literal). The old manual
                // `process(requireRomanMode:true)` was redundant — in swapped
                // mode it no-op'd (so the engine effect already drove
                // behavior since Phase 9 Item 3), in non-swapped mode it
                // double-recorded the association. Removing it is
                // behavior-preserving and makes the engine effect the SOLE
                // source (§10.3).
                // 中文: Model B — commitRawInput 由引擎發終端 NextWord;Enter 保留引擎
                // 中文: 預測;移除冗餘手動 process(swapped 時本就 no-op,否則雙記關聯)。
                composingManager.commitRawInput()
            } else {
                // Non-zero index: confirm selected candidate.
                // Strip KK type at the boundary; ComposingManager is engine-pure.
                let texts = (keyboardController?.state.autocompleteContext.suggestions ?? []).map(\.text)
                _ = composingManager.confirmSelectedCandidate(availableTexts: texts)
            }

            // Romanization mode: auto-space (unless trailing hyphen)
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
