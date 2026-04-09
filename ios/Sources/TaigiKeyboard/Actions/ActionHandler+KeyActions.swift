// ActionHandler extension: per-key action handlers (character, space, backspace, return).
// Each handler returns true if handled (skips KeyboardKit default).

import Foundation
import KeyboardKit

extension ActionHandler {
    // MARK: - Character Input

    /// Returns true if handled (skip KeyboardKit default)
    func handleCharacterInput(_ char: String) -> Bool {
        let currentCase = keyboardContext.keyboardCase
        let autoCap = keyboardContext.settings.isAutocapitalizationEnabled

        logger.debug("[AUTOCAP][INPUT] char='\(char)' keyboardCase=\(String(describing: currentCase)) autoCap=\(autoCap)")

        guard !char.isEmpty else {
            return false
        }

        // Unified case transformation
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
               !TPSConverter.isTPSToneMark(lastChar), lastChar != " "
            {
                composingManager.appendCharacter(" ")
                return true
            }
        }

        // Taigi mode (POJ/TL)
        if composingManager.isComposing {
            let committedText = composingManager.composingText

            // Commit composing + insert space (no candidate selection)
            composingManager.commitComposition()
            keyboardContext.textDocumentProxy.insertText(" ")

            // Record committed text for future associations (space doesn't trigger NextWord prediction)
            nextWordController.process(text: committedText, roman: committedText, triggerPrediction: false)
        } else {
            keyboardContext.textDocumentProxy.insertText(" ")
        }
        return true
    }

    // MARK: - Backspace

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
                composingManager.commitRawInput()
                nextWordController.process(text: capturedRawInput, roman: capturedRawInput, requireRomanMode: true)
            } else {
                // Non-zero index: confirm selected candidate
                let suggestions = keyboardController?.state.autocompleteContext.suggestions ?? []
                _ = composingManager.confirmSelectedCandidate(availableSuggestions: suggestions)
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
