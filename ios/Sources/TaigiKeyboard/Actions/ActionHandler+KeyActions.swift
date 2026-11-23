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
            isNasalMarkerUppercaseEnabled: settings.isNasalMarkerUppercaseEnabled,
        )

        // TPS keys go to the engine one key per op (`TpsKey`): it applies the
        // TPS auto-correct against the composition itself.
        let isTPS = settings.inputMode == .tps

        logger.debug("[AUTOCAP][INPUT] processedChar='\(processedChar)'")

        // English mode: insert directly, skip composing; punctuation still swaps
        // with an auto space a suggestion pick wrote (`hello ` + `,` → `hello, `).
        if settings.inputMode == .english {
            insertNonComposingCharacter(processedChar)
            // Reset single-shift (preserve Caps Lock)
            if keyboardContext.keyboardCase == .uppercased {
                keyboardContext.keyboardCase = .lowercased
            }
            return true
        }

        // Taigi mode (POJ/TL/TPS) composing logic

        // Standalone digit: commit directly without entering composing mode.
        // Digits only enter composing as tone markers appended to existing romanization.
        if !composingManager.isComposing, processedChar.first?.isNumber == true {
            if nextWordController.isShowing {
                nextWordController.clearDisplay()
            }
            hostText.insert(processedChar)
            return true
        }

        // Composing characters (letters, TPS symbols, hyphen, ˙) → enter composing
        if isComposingCharacter(processedChar) {
            if composingManager.isComposing {
                if isTPS {
                    composingManager.tpsKey(processedChar)
                } else if processedChar == "-" {
                    composingManager.appendHyphen()
                } else {
                    composingManager.appendCharacter(processedChar)
                }
            } else {
                // Not composing: check NextWord state
                if processedChar == "-", nextWordController.isShowing {
                    // "-" during NextWord: output directly, keep NextWord suggestions
                    hostText.insert("-")
                    logger.debug("[INPUT] '-' committed in NextWord mode, keeping suggestions")
                } else {
                    if nextWordController.isShowing {
                        nextWordController.clearDisplay()
                    }
                    if isTPS {
                        composingManager.tpsKey(processedChar)
                    } else {
                        composingManager.startComposing(with: processedChar)
                    }
                }
            }
        } else if composingManager.isComposing, processedChar.first?.isNumber == true {
            // Digit while composing → tone marker
            if isTPS {
                composingManager.tpsKey(processedChar)
            } else {
                composingManager.appendCharacter(processedChar)
            }
        } else {
            // Non-composing char (punctuation, symbols, etc.) → commit then output
            if composingManager.isComposing {
                composingManager.commitComposition()
            }
            // Model B §10.3: the engine commit (commitComposition→CommitRaw)
            // already applies the terminal next-word handshake (records the
            // association). Punctuation SUPPRESSES the next-word *display*
            // (converges with Space; resolves the iOS/Android Model-B
            // divergence). Unconditional on this path — mirrors Android's
            // unconditional `clearCandidates()` on the punctuation path so a
            // stale strip is also cleared when NextWord was showing and we
            // were NOT composing (Codex pre-impl P1). clearDisplay() bumps the
            // NextWord generation so the in-flight prediction query is dropped
            // stale; it is a cheap no-op when nothing is showing.
            nextWordController.clearDisplay()
            insertNonComposingCharacter(processedChar)
            // Text going into the document without passing through a
            // composition is still context: a full stop typed here is what
            // ends the sentence the next-word learning would otherwise carry
            // across (§40; mirrors desktop-core `note_character_typed_outside_composition`).
            if NextWordController.isContextCharacterOutsideComposition(processedChar) {
                nextWordController.noteCharacterTypedOutsideComposition(processedChar)
            }
            return true
        }

        // Reset single-shift (preserve Caps Lock) — only a cased letter spends it.
        if keyboardContext.keyboardCase == .uppercased, Self.consumesSingleShift(processedChar) {
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
    /// punctuation (`guá ` + `?` → `guá? `, never `guá ?`). A full-width
    /// attaching mark and a joiner take the space's place instead
    /// (`taigi ` + `，` → `taigi，`; `hōo ` + `·` → `hōo·`).
    // CROSS-PLATFORM INVARIANT — mirrors Android
    // TextInputKeyHandler.commitNonComposingCharacter. Drift causes silent divergence.
    private func insertNonComposingCharacter(_ char: String) {
        let isArmed = isAutoSpaceSwapArmed
        let attaches = isArmed && RustEngineBridge.isAttachingPunctuation(char)
        let joins = isArmed
            && Self.joinerReplacesAutoSpace(
                isEnglishMode: settings.inputMode == .english,
                syllableSeparator: settings.syllableSeparator,
            )
            && RustEngineBridge.isJoinerPunctuation(char)
        let proxy = controllerContext.textDocumentProxy
        // No-selection guard: with an active selection the preceding space is
        // text before the selection, not an auto-space; the punctuation must
        // replace the selection normally (Codex P2).
        if attaches || joins,
           (proxy.selectedText ?? "").isEmpty,
           proxy.documentContextBeforeInput?.last == " "
        {
            let isSpaced = attaches && RustEngineBridge.isSpacedPunctuation(char)
            hostText.deleteBackward()
            hostText.insert(isSpaced ? char + " " : char)
            // Re-armed on the space the swap just wrote, so `?!` chains keep
            // swapping (`guá? ` + `!` → `guá?! `). A full-width mark or a
            // joiner wrote no space.
            if isSpaced {
                armAutoSpaceSwap()
            }
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

        // TPS mode: space as tone 1/4 syllable boundary marker (like Microsoft
        // Zhuyin's space for tone 1). The engine takes it while the syllable has
        // no tone mark and does not already end in a space; otherwise it refuses
        // the key and Space commits below.
        if settings.inputMode == .tps, composingManager.isComposing, composingManager.tpsKey(" ") {
            return true
        }

        // Taigi mode (POJ/TL)
        if composingManager.isComposing {
            // Model B §10.3: the engine commit (commitComposition→CommitRaw)
            // applies the terminal next-word handshake → records the
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

    /// Writes a space, or — when the previous event was a plain space, nothing
    /// is selected and the engine's `doubleSpacePeriod` earns a period —
    /// replaces that space with it. Always on, no setting. Returns the period
    /// written, or nil for a plain space.
    // CROSS-PLATFORM INVARIANT — mirrors Android
    // TextInputKeyHandler.commitSpaceOrDoubleSpacePeriod. Drift causes silent divergence.
    // Characters of context the double-space rule reads: the space and the one before it.
    // CROSS-PLATFORM INVARIANT — mirrors Android TextInputKeyHandler.DOUBLE_SPACE_LOOKBEHIND (UTF-16 units).
    private static let doubleSpaceLookbehind = 2

    @discardableResult
    private func insertSpaceOrDoubleSpacePeriod(isFullWidthPunctuation: Bool) -> String? {
        let proxy = controllerContext.textDocumentProxy
        // The selection is read last: most second spaces earn no period.
        let replacement: String? = if let millis = millisSincePlainSpace,
                                      let context = proxy.documentContextBeforeInput,
                                      let period = RustEngineBridge.doubleSpacePeriod(
                                          textBeforeCaret: String(context.suffix(Self.doubleSpaceLookbehind)),
                                          millisSincePlainSpace: millis,
                                          fullWidth: isFullWidthPunctuation,
                                      ),
                                      (proxy.selectedText ?? "").isEmpty
        {
            period
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
        let textBeforeCursor = controllerContext.textDocumentProxy.documentContextBeforeInput ?? ""
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
            let isActionField = Self.isActionReturnKey(controllerContext.textDocumentProxy.returnKeyType)

            // Enter commits raw input (literal keystrokes).
            // This allows English words to pass through without tone conversion
            // (Google Pinyin convention: Enter = raw Latin text, Space = converted text)
            //
            // Model B §10.3: `commitComposition()`→CommitRaw → the engine's
            // Continuous final-commit emits the terminal
            // next-word handshake (trigger on), applied in the same call and run
            // by `nextWordController.applyComposingDecision` → records the association +
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
