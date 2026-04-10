// NextWordController: manages NextWord prediction state, association recording, and UI updates.
// Separated from ActionHandler to maintain single responsibility:
// ActionHandler dispatches keyboard actions, NextWordController manages word prediction.

import Foundation
import KeyboardKit

/// Controls NextWord prediction lifecycle: record associations, update state, trigger predictions.
///
/// **Lifecycle** (word selection → prediction display):
/// 1. `process()` — validate input, record word association, update state, trigger prediction
/// 2. `triggerPrediction()` — async query `NextWordService`, convert to suggestions, update UI
///
/// **Special paths**:
/// - `rePredictAfterBackspace()` — re-predict without recording associations (backspace is not a word selection)
/// - `clearDisplay()` — hide suggestions when user starts typing or enters a digit
/// - `resetAndClearUI()` — full reset on sentence-end punctuation or empty document
final class NextWordController: SelectionContextProvider {
    let logger = DebugLogger(category: "NextWord")

    // MARK: - Dependencies

    private let settings = SharedSettings.shared
    weak var contextUpdater: AutocompleteContextUpdater?

    // MARK: - State

    /// Previous word for association recording (exposed via SelectionContextProvider for autocomplete context boost)
    private(set) var lastSelectedWord: String?
    private(set) var lastSelectedRoman: String?
    private(set) var lastSelectionTime: Int64 = 0
    /// Whether NextWord predictions are currently displayed
    private(set) var isShowing: Bool = false
    private var contextTimeoutTimer: Timer?

    private enum Constants {
        /// Max interval between selections to record association
        static let associationTimeoutMs: Int64 = 10000
        /// Context timeout — clears NextWord state after inactivity
        static let contextTimeoutSeconds: TimeInterval = 30.0
        /// Sentence-end punctuation resets NextWord context
        static let sentenceEndPunctuation = Set<Character>(["。", "！", "？", ".", "!", "?"])
        /// Noise punctuation — superset of sentenceEndPunctuation, used by isNoiseText()
        static let noisePunctuation = "。！？.!?，,、；;：:「」『』\"\"\u{2018}\u{2019}（）()【】[]{}—–-～~…·"
    }

    // MARK: - Core Processing

    /// Unified NextWord entry point: validate → record association → update state → optionally predict.
    /// Called by: suggestion selection, Space (triggerPrediction=false), Enter (requireRomanMode=true)
    /// - `requireRomanMode`: when true, skip if in Hanji mode (Enter commits raw romanization only)
    /// - `triggerPrediction`: when false, only record + update state (Space path)
    func process(text: String, roman: String, requireRomanMode: Bool = false, triggerPrediction: Bool = true) {
        if requireRomanMode {
            guard !settings.isTranslateSwapped else { return }
        }

        guard !text.isEmpty, !isNoiseText(text) else {
            if isSentenceEndPunctuation(text) {
                resetAndClearUI()
            }
            return
        }

        // Normalize romanization to TL for consistent storage and query
        // pojToTL is idempotent on TL input, safe for all modes including TPS
        let textTl = RomanizationConverter.pojToTL(roman)
        let prevTl = RomanizationConverter.pojToTL(lastSelectedRoman ?? "")

        if settings.isAssociationRecordingEnabled {
            if shouldRecordAssociation(), let prevWord = lastSelectedWord {
                Task {
                    await NextWordService.shared.recordAssociation(
                        prev: prevWord,
                        prevTl: prevTl,
                        nextHanzi: text,
                        nextTl: textTl,
                    )
                }
            }

            recordCompoundWordAssociations(displayText: text, roman: textTl)
        }

        lastSelectedWord = text
        lastSelectedRoman = textTl
        lastSelectionTime = Self.currentTimestampMs
        startContextTimeoutTimer()

        if triggerPrediction {
            self.triggerPrediction(for: text, roman: textTl)
        }
    }

    /// Re-predict NextWord after backspace based on last remaining character.
    /// Called by: ActionHandler+KeyActions (backspace path)
    /// Intentionally does NOT record associations — backspace is not a word selection.
    func rePredictAfterBackspace(lastChar: String) {
        lastSelectedWord = lastChar
        lastSelectedRoman = nil
        lastSelectionTime = Self.currentTimestampMs

        triggerPrediction(for: lastChar)
    }

    /// Full reset: clear all state and hide UI suggestions.
    /// Called by: backspace (empty document), textDidChange, sentence-end punctuation
    func resetAndClearUI() {
        let wasShowing = isShowing
        resetContext()
        if wasShowing {
            contextUpdater?.resetNextWordSuggestions()
        }
    }

    /// Hide NextWord suggestions without clearing association state.
    /// Called by: digit input, new composing character (not hyphen)
    func clearDisplay() {
        isShowing = false
        contextUpdater?.resetNextWordSuggestions()
    }

    // MARK: - State Management

    private func resetContext() {
        lastSelectedWord = nil
        lastSelectedRoman = nil
        lastSelectionTime = 0
        isShowing = false
        stopContextTimeoutTimer()
    }

    private func startContextTimeoutTimer() {
        stopContextTimeoutTimer()
        contextTimeoutTimer = Timer.scheduledTimer(
            withTimeInterval: Constants.contextTimeoutSeconds,
            repeats: false,
        ) { [weak self] _ in
            self?.handleContextTimeout()
        }
    }

    private func stopContextTimeoutTimer() {
        contextTimeoutTimer?.invalidate()
        contextTimeoutTimer = nil
    }

    private func handleContextTimeout() {
        logger.debug("[TIMEOUT] Context timeout - resetting")
        let wasShowing = isShowing
        resetContext()
        if wasShowing {
            DispatchQueue.main.async { [weak self] in
                self?.contextUpdater?.resetNextWordSuggestions()
            }
        }
    }

    static var currentTimestampMs: Int64 {
        Int64(Date().timeIntervalSince1970 * 1000)
    }

    // MARK: - Prediction

    /// Query NextWordService and update UI with predictions
    private func triggerPrediction(for word: String, roman: String = "") {
        logger.debug("[TRIGGER] querying for word='\(word)'")

        Task { @MainActor in
            let predictions = await NextWordService.shared.predict(word: word, roman: roman)
            logger.debug("[TRIGGER] predictions.count=\(predictions.count) for word='\(word)'")

            if predictions.isEmpty {
                isShowing = false
                contextUpdater?.resetNextWordSuggestions()
                return
            }

            let suggestions = makeSuggestions(from: predictions)
            logger.debug("[TRIGGER] after filter: suggestions.count=\(suggestions.count) (from \(predictions.count) predictions)")

            if suggestions.isEmpty {
                isShowing = false
                contextUpdater?.resetNextWordSuggestions()
            } else {
                contextUpdater?.setNextWordSuggestions(suggestions)
                isShowing = true
                startContextTimeoutTimer()
            }
        }
    }

    /// Convert NextWord predictions to autocomplete suggestions, filtering empty TL in romanization mode
    private func makeSuggestions(from predictions: [NextWordService.Prediction]) -> [Autocomplete.Suggestion] {
        predictions.compactMap { prediction in
            if !settings.isTranslateSwapped && prediction.tl.isEmpty {
                logger.debug("[FILTER] REMOVED hanzi='\(prediction.hanzi)' tl='\(prediction.tl)' (TL empty in roman mode)")
                return nil
            }

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
    }

    // MARK: - Association Helpers

    /// Whether to record word association (previous selection exists and interval < 10s)
    private func shouldRecordAssociation() -> Bool {
        guard lastSelectedWord != nil else { return false }
        return (Self.currentTimestampMs - lastSelectionTime) < Constants.associationTimeoutMs
    }

    /// Noise filter: punctuation, whitespace, pure digits don't trigger NextWord
    private func isNoiseText(_ text: String) -> Bool {
        guard let firstChar = text.first else { return true }
        if Constants.noisePunctuation.contains(firstChar) { return true }
        if firstChar.isWhitespace { return true }
        if text.allSatisfy({ $0.isASCII && $0.isNumber }) { return true }
        return false
    }

    private func isSentenceEndPunctuation(_ text: String) -> Bool {
        guard let firstChar = text.first else { return false }
        return Constants.sentenceEndPunctuation.contains(firstChar)
    }

    private func splitCompoundWord(_ word: String) -> [String] {
        guard !word.isEmpty else { return [] }
        return word.split(separator: "-").map(String.init).filter { !$0.isEmpty }
    }

    /// Record associations between parts of compound words (e.g. tshit-niû → tshit, niû)
    private func recordCompoundWordAssociations(displayText: String, roman: String) {
        let parts = splitCompoundWord(displayText)
        let romanParts = splitCompoundWord(roman)

        guard parts.count > 1 else { return }

        Task {
            for i in 0 ..< (parts.count - 1) {
                let prevPart = parts[i]
                let prevPartRoman = romanParts.indices.contains(i) ? romanParts[i] : ""
                let nextPart = parts[i + 1]
                let nextRoman = romanParts.indices.contains(i + 1) ? romanParts[i + 1] : ""

                await NextWordService.shared.recordAssociation(
                    prev: prevPart,
                    prevTl: prevPartRoman,
                    nextHanzi: nextPart,
                    nextTl: nextRoman,
                )
            }
        }
    }
}
