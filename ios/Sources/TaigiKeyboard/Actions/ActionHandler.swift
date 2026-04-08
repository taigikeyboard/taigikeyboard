import Foundation
import KeyboardKit

/// Taigi keyboard action handler. Extensions in ActionHandler+*.swift.
/// Handles character input/composing, suggestion selection, and NextWord prediction.
public class ActionHandler: KeyboardAction.StandardActionHandler, SelectionContextProvider {
    // MARK: - Properties

    let logger = DebugLogger(category: "ActionHandler")

    let settings = SharedSettings.shared
    public let composingManager = ComposingManager()

    /// Distinguishes space-drag (cursor move) from space-tap (insert space)
    private var isSpaceDragInProgress = false

    // MARK: - NextWord State

    /// Previous selection for word association recording
    var lastSelectedWord: String?
    var lastSelectedRoman: String?
    var lastSelectionTime: Int64 = 0
    var isShowingNextWord: Bool = false
    private var contextTimeoutTimer: Timer?

    private enum NextWordConstants {
        /// Max interval between selections to record association
        static let associationTimeoutMs: Int64 = 10000
        /// Context timeout — clears NextWord state after inactivity
        static let contextTimeoutSeconds: TimeInterval = 30.0
        /// Sentence-end punctuation resets NextWord context
        static let sentenceEndPunctuation = Set<Character>(["。", "！", "？", ".", "!", "?"])
        /// Noise punctuation — superset of sentenceEndPunctuation, used by isNoiseText()
        static let noisePunctuation = "。！？.!?，,、；;：:「」『』\"\"\u{2018}\u{2019}（）()【】[]{}—–-～~…·"
    }

    // MARK: - Action Dispatch

    /// - Returns: true if handled (skip KeyboardKit default)
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

    override public func handle(_ gesture: Keyboard.Gesture, on action: KeyboardAction) {
        // Space drag state tracking
        if action == .space {
            switch gesture {
            case .longPress:
                isSpaceDragInProgress = true
            case .release:
                if isSpaceDragInProgress {
                    isSpaceDragInProgress = false
                    super.handle(gesture, on: action) // Let KeyboardKit handle drag end
                    return
                }
                isSpaceDragInProgress = false
            case .end:
                isSpaceDragInProgress = false
            default:
                break
            }
        }

        guard gesture == .release else {
            if action == .backspace {
                // Allow backspace repeat-press gesture
                if gesture == .repeatPress {
                    _ = handleBackspaceAction()
                }
                return
            }

            super.handle(gesture, on: action)
            return
        }

        let handled = handleTaigiSpecificAction(action)
        if handled {
            // Align with Android: skip autocomplete to preserve NextWord suggestions
            // when not composing and pressing space or "-" during NextWord
            let skipAutocomplete: Bool = {
                if !composingManager.isComposing {
                    if action == .space {
                        return true
                    }
                    if case .character("-") = action, isShowingNextWord {
                        return true
                    }
                }
                return false
            }()

            if !skipAutocomplete {
                keyboardController?.performAutocomplete()
            }
            return
        }
        super.handle(gesture, on: action)
    }

    override public func handle(_ suggestion: Autocomplete.Suggestion) {
        // English mode: use KeyboardKit default (auto-deletes typed chars then inserts)
        if settings.inputMode == .english {
            super.handle(suggestion)
            return
        }
        // Taigi mode: custom handling
        handleSuggestionSelection(suggestion)
    }

    override public func handle(_ action: KeyboardAction) {
        handle(.release, on: action)
    }

    /// FIXME: Workaround for KeyboardKit 10 auto-capitalization override.
    /// Part of 3-layer workaround — see KeyboardViewController.swift for full context.
    /// Remove when KeyboardKit provides a proper API to disable auto-capitalization.
    ///
    /// - Shift: always let super handle (preserves doubleTap → Caps Lock)
    /// - Other actions: only call super when auto-cap is on
    override public func tryChangeKeyboardCase(
        after gesture: Keyboard.Gesture,
        on action: KeyboardAction,
    ) {
        let beforeCase = keyboardContext.keyboardCase
        let isAutoCap = keyboardContext.settings.isAutocapitalizationEnabled

        logger.debug("[CASE][tryChange] gesture=\(String(describing: gesture)) action=\(String(describing: action)) before=\(String(describing: beforeCase)) isAutoCap=\(isAutoCap)")

        // Shift: always let super handle (preserves doubleTap → Caps Lock)
        if case .shift = action {
            super.tryChangeKeyboardCase(after: gesture, on: action)
            logger.debug("[CASE][tryChange] after shift: \(String(describing: keyboardContext.keyboardCase))")
            return
        }

        // Other actions: only call super when auto-cap is on
        if isAutoCap {
            super.tryChangeKeyboardCase(after: gesture, on: action)
            logger.debug("[CASE][tryChange] after autoCap: \(String(describing: keyboardContext.keyboardCase))")
        } else {
            logger.debug("[CASE][tryChange] skipped (autoCap=false)")
        }
    }

    // MARK: - NextWord State Management

    func resetNextWordContext() {
        lastSelectedWord = nil
        lastSelectedRoman = nil
        lastSelectionTime = 0
        isShowingNextWord = false
        stopContextTimeoutTimer()
    }

    func startContextTimeoutTimer() {
        stopContextTimeoutTimer()
        contextTimeoutTimer = Timer.scheduledTimer(
            withTimeInterval: NextWordConstants.contextTimeoutSeconds,
            repeats: false,
        ) { [weak self] _ in
            self?.handleContextTimeout()
        }
    }

    func stopContextTimeoutTimer() {
        contextTimeoutTimer?.invalidate()
        contextTimeoutTimer = nil
    }

    private func handleContextTimeout() {
        logger.debug("[NEXTWORD] Context timeout - resetting")
        let wasShowingNextWord = isShowingNextWord
        resetNextWordContext()
        if wasShowingNextWord {
            DispatchQueue.main.async { [weak self] in
                self?.keyboardController?.state.autocompleteContext.reset()
            }
        }
    }

    static var currentTimestampMs: Int64 {
        Int64(Date().timeIntervalSince1970 * 1000)
    }

    /// Whether to record word association (selection interval < 10s)
    func shouldRecordAssociation() -> Bool {
        guard lastSelectedWord != nil else { return false }
        return (Self.currentTimestampMs - lastSelectionTime) < NextWordConstants.associationTimeoutMs
    }

    /// Noise filter: punctuation, whitespace, pure digits don't trigger NextWord
    func isNoiseText(_ text: String) -> Bool {
        guard let firstChar = text.first else { return true }
        let punctuation = NextWordConstants.noisePunctuation
        if punctuation.contains(firstChar) { return true }
        if firstChar.isWhitespace { return true }
        if text.allSatisfy({ $0.isASCII && $0.isNumber }) { return true }
        return false
    }

    func isSentenceEndPunctuation(_ text: String) -> Bool {
        guard let firstChar = text.first else { return false }
        return NextWordConstants.sentenceEndPunctuation.contains(firstChar)
    }

    func updateNextWordState(selectedWord: String, roman: String = "") {
        lastSelectedWord = selectedWord
        lastSelectedRoman = roman
        lastSelectionTime = Self.currentTimestampMs
        startContextTimeoutTimer()
    }

    /// Record previous word without triggering prediction.
    /// Used when space commits composing text — records it for future associations.
    func updateLastSelectedWord(_ word: String, roman: String? = nil) {
        guard !word.isEmpty, !isNoiseText(word) else { return }

        lastSelectedWord = word
        lastSelectedRoman = roman ?? word
        lastSelectionTime = Self.currentTimestampMs
        recordCompoundWordAssociations(displayText: word, roman: word)
        startContextTimeoutTimer()
    }

    // MARK: - NextWord Prediction

    /// Trigger next-word prediction for the given word
    func triggerNextWordPrediction(for word: String, roman: String = "") {
        // DEBUG: NextWord trace - triggerNextWordPrediction entry
        logger.debug("[NEXTWORD][TRIGGER] querying for word='\(word)'")

        Task { @MainActor in
            let predictions = await NextWordService.shared.predict(word: word, roman: roman)

            // DEBUG: NextWord trace - predictions returned
            logger.debug("[NEXTWORD][TRIGGER] predictions.count=\(predictions.count) for word='\(word)'")

            if predictions.isEmpty {
                isShowingNextWord = false
                keyboardController?.state.autocompleteContext.reset()
                return
            }

            // Convert to suggestions (filter out entries with empty TL in romanization mode)
            let suggestions = predictions.compactMap { prediction -> Autocomplete.Suggestion? in
                if !settings.isTranslateSwapped && prediction.tl.isEmpty {
                    // DEBUG: NextWord trace - filtered out prediction with empty TL
                    self.logger.debug("[NEXTWORD][FILTER] REMOVED hanzi='\(prediction.hanzi)' tl='\(prediction.tl)' (TL empty in roman mode)")
                    return nil
                }

                // Convert TL -> POJ for display in POJ mode
                let roman = settings.inputMode == .poj
                    ? RomanizationConverter.tlToPOJ(prediction.tl)
                    : prediction.tl
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
                        "displayText": prediction.hanzi,
                    ],
                )
            }

            // DEBUG: NextWord trace - after filter
            self.logger.debug("[NEXTWORD][TRIGGER] after filter: suggestions.count=\(suggestions.count) (from \(predictions.count) predictions)")

            if let controller = keyboardController {
                if suggestions.isEmpty {
                    isShowingNextWord = false
                    controller.state.autocompleteContext.reset()
                } else {
                    controller.state.autocompleteContext.suggestionsFromService = suggestions
                    isShowingNextWord = true
                    startContextTimeoutTimer()
                }
                self.logger.debug("[NEXTWORD][TRIGGER] isShowingNextWord=\(self.isShowingNextWord)")
            } else {
                self.logger.debug("[NEXTWORD][TRIGGER] keyboardController is nil!")
            }
        }
    }
}
