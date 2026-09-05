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
            nextWordController.clearDisplay()
            insertNonComposingCharacter(finalChar)
            return true
        }

        // Reset single-shift (preserve Caps Lock)
        if keyboardContext.keyboardCase == .uppercased {
            keyboardContext.keyboardCase = .lowercased
        }

        return true
    }

    /// Insert a non-composing character (punctuation / symbol), applying the
    /// auto-space "smart punctuation" swap: when auto-space is active and the
    /// character just before the cursor is the auto-inserted trailing space,
    /// attaching punctuation deletes that space and re-inserts it AFTER the
    /// punctuation (`guá ` + `?` → `guá? `, never `guá ?`).
    // CROSS-PLATFORM INVARIANT — mirrors Android
    // TextInputKeyHandler.commitNonComposingCharacter. Drift causes silent divergence.
    private func insertNonComposingCharacter(_ char: String) {
        let proxy = keyboardContext.textDocumentProxy
        // No-selection guard: with an active selection the preceding space is
        // text before the selection, not an auto-space; the punctuation must
        // replace the selection normally (Codex P2).
        if isAutoSpaceSwapArmed,
           AutoSpacePunctuation.isAttaching(char),
           (proxy.selectedText ?? "").isEmpty,
           proxy.documentContextBeforeInput?.last == " "
        {
            proxy.deleteBackward()
            proxy.insertText(char + " ")
            // Re-armed on the space the swap just wrote, so `?!` chains keep
            // swapping (`guá? ` + `!` → `guá?! `).
            armAutoSpaceSwap()
            return
        }
        proxy.insertText(char)
    }

    // MARK: - Space

    // Do not add drag detection here — ActionHandler.handle intercepts the spacebar drag gesture first.
    func handleSpaceAction() -> Bool {
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
            composingManager.commitComposition()
            keyboardContext.textDocumentProxy.insertText(" ")
            nextWordController.clearDisplay()
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
            // Captured before the commit clears them: the raw input, the
            // highlighted index, and the suggestion that index names.
            let capturedRawInput = composingManager.rawInput
            let selectedIndex = composingManager.selectedCandidateIndex
            let suggestions = keyboardController?.state.autocompleteContext.suggestions ?? []

            // The document string each arm writes, and whether it is
            // romanization — resolved per arm, because the two arms write
            // different things: index 0 commits the composition verbatim,
            // any other index commits that candidate's own display text.
            let committedText: String
            let wroteRomanization: Bool
            if selectedIndex == 0 {
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
                composingManager.commitRawInput()
                committedText = capturedRawInput
                wroteRomanization = Self.rawPreeditWritesRomanization(isTPSLayout: isTPSLayout)
            } else {
                // Non-zero index: confirm selected candidate. This commits the
                // suggestion's own `text` verbatim, so THAT string — not the
                // raw input, and not a mode-derived rendering — is what the
                // auto space answers for.
                // Strip KK type at the boundary; ComposingManager is engine-pure.
                _ = composingManager.confirmSelectedCandidate(availableTexts: suggestions.map(\.text))
                let selected = suggestions.indices.contains(selectedIndex) ? suggestions[selectedIndex] : nil
                committedText = selected?.text ?? ""
                wroteRomanization = selected.map(highlightedCandidateWroteRomanization) ?? false
            }

            // Auto-space follows what the commit WROTE, not the output mode:
            // a raw commit is romanization in TL/POJ and Bopomofo in TPS,
            // while a confirmed candidate can be the 漢字 itself. The hyphen
            // check runs on the committed string, so a 連字 the user is
            // continuing suppresses the space on either arm.
            appendAutoSpaceIfEarned(
                documentText: committedText,
                wroteRomanization: wroteRomanization,
            )
            return true
        } else {
            keyboardContext.textDocumentProxy.insertText("\n")
        }
        return true
    }
}
