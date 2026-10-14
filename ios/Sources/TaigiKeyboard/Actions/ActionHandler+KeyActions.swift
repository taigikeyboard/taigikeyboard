// ActionHandler extension: per-key action handlers (character, space, backspace, return).
// Each handler returns true if handled (skips KeyboardKit default).

import Foundation
import KeyboardKit
import UIKit

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
        // logging only — `transformInputCase` ignores it, as the legacy
        // `CaseTransformer.transformForInput` did.
        let processedChar = RustEngineBridge.transformInputCase(
            char,
            letterCase: currentCase.asLetterCase,
            mode: settings.inputMode,
            isNasalMarkerUppercaseEnabled: settings.isNasalMarkerUppercaseEnabled,
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

        // English mode: insert directly, skip composing; punctuation still swaps
        // with an auto space a suggestion pick wrote (`hello ` + `,` → `hello, `).
        if settings.inputMode == .english {
            insertNonComposingCharacter(finalChar)
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
            hostText.insert(finalChar)
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
                    hostText.insert("-")
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
            // Text going into the document without passing through a
            // composition is still context: a full stop typed here is what
            // ends the sentence the next-word learning would otherwise carry
            // across (§40; mirrors desktop-core `note_character_typed_outside_composition`).
            if NextWordController.isContextCharacterOutsideComposition(finalChar) {
                nextWordController.noteCharacterTypedOutsideComposition(finalChar)
            }
            return true
        }

        // Reset single-shift (preserve Caps Lock) — only a cased letter spends it.
        if keyboardContext.keyboardCase == .uppercased, Self.consumesSingleShift(finalChar) {
            keyboardContext.keyboardCase = .lowercased
        }

        return true
    }

    /// Whether typing `char` spends a one-shot Shift: only a cased letter does.
    /// Tone digits, the hyphen, TPS Bopomofo and TPS tone marks carry no case,
    /// so a Shift stays armed across them for the next letter.
    // CROSS-PLATFORM INVARIANT — mirrors Android TextInputKeyHandler.kt
    // `consumesSingleShift`. Drift causes silent divergence.
    static func consumesSingleShift(_ char: String) -> Bool {
        guard let first = char.unicodeScalars.first else { return false }
        return first.properties.isUppercase || first.properties.isLowercase
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
           RustEngineBridge.isAttachingPunctuation(char),
           (proxy.selectedText ?? "").isEmpty,
           proxy.documentContextBeforeInput?.last == " "
        {
            hostText.deleteBackward()
            hostText.insert(char + " ")
            // Re-armed on the space the swap just wrote, so `?!` chains keep
            // swapping (`guá? ` + `!` → `guá?! `).
            armAutoSpaceSwap()
            return
        }
        hostText.insert(char)
    }

    // MARK: - Space

    // Do not add drag detection here — ActionHandler.handle intercepts the spacebar drag gesture first.
    func handleSpaceAction() -> Bool {
        // English mode: insert space directly
        if settings.inputMode == .english {
            insertSpaceOrDoubleSpacePeriod(isFullWidthPunctuation: false)
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
            // prediction query is dropped stale. The space joins the held
            // commit, so the host gets one write (`HostTextWriter`, #352).
            composingManager.commitComposition()
            hostText.insert(" ")
            nextWordController.clearDisplay()
        } else if let period = insertSpaceOrDoubleSpacePeriod(isFullWidthPunctuation: settings.isFullWidthPunctuation) {
            // The period ends the sentence for next-word learning, as a typed one would (§40).
            nextWordController.noteCharacterTypedOutsideComposition(period)
        }
        return true
    }

    /// Writes a space, or — when the previous event was a plain space a moment
    /// ago and the text before it earns a period — replaces that space with
    /// `doubleSpacePeriodReplacement`. Always on, no setting. Returns the
    /// period written, or nil for a plain space.
    // CROSS-PLATFORM INVARIANT — mirrors Android
    // TextInputKeyHandler.commitSpaceOrDoubleSpacePeriod. Drift causes silent divergence.
    @discardableResult
    private func insertSpaceOrDoubleSpacePeriod(isFullWidthPunctuation: Bool) -> String? {
        let proxy = keyboardContext.textDocumentProxy
        let replacement: String? = if followsRecentPlainSpace, (proxy.selectedText ?? "").isEmpty {
            Self.doubleSpacePeriodReplacement(
                textBeforeCaret: proxy.documentContextBeforeInput,
                isFullWidthPunctuation: isFullWidthPunctuation,
            )
        } else {
            nil
        }
        guard let replacement else {
            hostText.insert(" ")
            notePlainSpace()
            return nil
        }
        hostText.deleteBackward()
        hostText.insert(replacement)
        return replacement.trimmingCharacters(in: .whitespaces)
    }

    /// What a double space writes in place of its first space, or nil when
    /// `textBeforeCaret` does not earn a period: it must end in one ASCII space
    /// after a letter, number or combining mark — a code point, so a
    /// supplementary-plane hanji and POJ's U+0358 both count, and punctuation,
    /// emoji or a second space do not. Full-width punctuation writes `。` with
    /// no space; half-width writes `. `.
    // CROSS-PLATFORM INVARIANT — mirrors Android TextInputKeyHandler.kt
    // `doubleSpacePeriodReplacement`. Drift causes silent divergence.
    static func doubleSpacePeriodReplacement(textBeforeCaret: String?, isFullWidthPunctuation: Bool) -> String? {
        guard let scalars = textBeforeCaret?.unicodeScalars, scalars.last == " " else { return nil }
        guard let previous = scalars.dropLast().last else { return nil }
        switch previous.properties.generalCategory {
        case .uppercaseLetter, .lowercaseLetter, .titlecaseLetter, .modifierLetter, .otherLetter,
             .decimalNumber, .letterNumber, .otherNumber,
             .nonspacingMark, .enclosingMark, .spacingMark:
            return isFullWidthPunctuation ? "。" : ". "
        default:
            return nil
        }
    }

    // MARK: - Backspace

    func handleBackspaceAction() -> Bool {
        let caseBefore = String(describing: keyboardContext.keyboardCase)
        logger.debug("[AUTOCAP][BACKSPACE] BEFORE delete: \(caseBefore)")

        // English mode: delete directly
        if settings.inputMode == .english {
            hostText.deleteBackward()
            let caseAfter = String(describing: keyboardContext.keyboardCase)
            logger.debug("[AUTOCAP][BACKSPACE] AFTER delete: \(caseAfter)")
            return true
        }

        // Taigi mode
        if composingManager.isComposing {
            composingManager.deleteBackward()
        } else {
            hostText.deleteBackward()
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
            hostText.insert("\n")
            return true
        }

        // Taigi mode
        if composingManager.isComposing {
            // Captured before the commit clears it.
            let capturedRawInput = composingManager.rawInput
            // Read before the commit, so a host callback cannot change the field first.
            let isActionField = Self.isActionReturnKey(keyboardContext.textDocumentProxy.returnKeyType)

            // Enter commits raw input (literal keystrokes).
            // This allows English words to pass through without tone conversion
            // (Google Pinyin convention: Enter = raw Latin text, Space = converted text)
            //
            // Model B §10.3: `commitComposition()`→CommitRaw → the engine's
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
            composingManager.commitComposition()

            // The first Return in an action field (Send / Search / Go …) only
            // confirms the composition, with no auto space; the next Return
            // runs the action (CROSS-PLATFORM INVARIANT — mirrors Android
            // TextInputKeyHandler.handleEnter).
            if isActionField {
                return true
            }

            // Auto-space follows what the commit WROTE, not the output mode:
            // a raw commit is romanization in TL/POJ and Bopomofo in TPS. The
            // hyphen check runs on the committed string, so a hyphen the user is
            // continuing suppresses the space.
            appendAutoSpaceIfEarned(
                documentText: capturedRawInput,
                wroteRomanization: Self.rawPreeditWritesRomanization(isTPSLayout: isTPSLayout),
            )
            return true
        } else {
            hostText.insert("\n")
        }
        return true
    }

    /// Whether the host labels Return as an action (Send, Search, Go …) rather
    /// than a newline. A hint only — the host decides what Return does.
    // CROSS-PLATFORM INVARIANT — mirrors Android TextInputKeyHandler.kt
    // `editorActionForEnter`.
    static func isActionReturnKey(_ returnKeyType: UIReturnKeyType?) -> Bool {
        switch returnKeyType {
        case nil, .default:
            false
        case .go, .google, .join, .next, .route, .search, .send, .yahoo, .done, .emergencyCall, .continue:
            true
        @unknown default:
            false
        }
    }
}
