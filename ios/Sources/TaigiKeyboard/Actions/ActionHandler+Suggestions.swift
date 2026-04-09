// ActionHandler extension: suggestion selection (candidate commit, output formatting)
// and NextWord prediction flow (association recording, state update, prediction trigger).

import Foundation
import KeyboardKit

extension ActionHandler {
    // MARK: - Suggestion Selection

    func handleSuggestionSelection(_ suggestion: Autocomplete.Suggestion) {
        // Raw input candidate: commit literal keystrokes directly (no tone conversion)
        if suggestion.additionalInfo["isRawInput"] == "true" {
            composingManager.commitRawInput()
            if settings.isAutoSpaceEnabled, !settings.isTranslateSwapped {
                keyboardContext.textDocumentProxy.insertText(" ")
            }
            return
        }

        let isNextWordPrediction = suggestion.additionalInfo["isNextWord"] == "true"

        if composingManager.isComposing || isNextWordPrediction {
            let isTPSLayout = SharedSettings.shared.keyboardLayoutType == .tps
            let effectiveSwapped = isTPSLayout || settings.isTranslateSwapped

            let (roman, hanzi) = parseRomanAndHanzi(from: suggestion, isNextWord: isNextWordPrediction, effectiveSwapped: effectiveSwapped)
            let textToCommit = formatOutputText(roman: roman, hanzi: hanzi, isTPSLayout: isTPSLayout, effectiveSwapped: effectiveSwapped)

            commitSuggestionText(textToCommit, isNextWord: isNextWordPrediction, suggestion: suggestion)

            let displayText = suggestion.additionalInfo["displayText"] ?? hanzi ?? roman
            if SharedSettings.shared.frequencyRecordingEnabled {
                UserFrequencyService.recordUsage(for: displayText)
            }

            logger.debug("[NEXTWORD][SELECT] suggestion.text='\(suggestion.text)' subtitle='\(suggestion.subtitle ?? "nil")' additionalInfo=\(suggestion.additionalInfo.description)")
            logger.debug("[NEXTWORD][SELECT] parsed roman='\(roman)' hanzi='\(hanzi ?? "nil")' displayText='\(displayText)'")

            // Romanization mode: auto-space (unless trailing hyphen)
            // TPS mode disables auto-space (effectiveSwapped is true for TPS)
            if settings.isAutoSpaceEnabled, !effectiveSwapped || settings.outputBothScripts {
                if !textToCommit.hasSuffix("-") {
                    keyboardContext.textDocumentProxy.insertText(" ")
                }
            }

            processNextWord(text: displayText, roman: roman)
        } else {
            keyboardContext.textDocumentProxy.insertText(suggestion.text)
        }
    }

    // MARK: - Suggestion Helpers

    /// Extract romanization and Hanji from suggestion based on display mode
    private func parseRomanAndHanzi(
        from suggestion: Autocomplete.Suggestion,
        isNextWord: Bool,
        effectiveSwapped: Bool,
    ) -> (roman: String, hanzi: String?) {
        if isNextWord {
            (suggestion.additionalInfo["tl"] ?? suggestion.text, suggestion.additionalInfo["hanzi"])
        } else if effectiveSwapped {
            (suggestion.subtitle ?? suggestion.text, suggestion.text)
        } else {
            (suggestion.text, suggestion.subtitle)
        }
    }

    /// Format output text based on display mode (roman, Hanji, or both scripts)
    private func formatOutputText(roman: String, hanzi: String?, isTPSLayout: Bool, effectiveSwapped: Bool) -> String {
        let bracketRoman = isTPSLayout
            ? TPSConverter.toTPSFromDisplay(roman, orMapsToER: SharedSettings.shared.tpsOrMapsToER)
            : roman

        if settings.outputBothScripts, let hanzi, !hanzi.isEmpty {
            return effectiveSwapped
                ? "\(hanzi) (\(bracketRoman))"
                : "\(bracketRoman) (\(hanzi))"
        } else if effectiveSwapped, let hanzi, !hanzi.isEmpty {
            return hanzi
        } else {
            return roman
        }
    }

    /// Commit text via proxy (NextWord) or composing manager (regular candidate)
    private func commitSuggestionText(_ text: String, isNextWord: Bool, suggestion: Autocomplete.Suggestion) {
        if isNextWord {
            keyboardContext.textDocumentProxy.insertText(text)
        } else {
            let modifiedSuggestion = Autocomplete.Suggestion(
                text: text,
                title: suggestion.title,
                subtitle: suggestion.subtitle,
                additionalInfo: suggestion.additionalInfo,
            )
            composingManager.selectSuggestion(modifiedSuggestion)
        }
    }

    // MARK: - NextWord Handling

    /// Unified NextWord processing: record association, update state, optionally trigger prediction.
    /// - `requireRomanMode`: when true, skip if in Hanji mode (Enter commits raw romanization only)
    /// - `triggerPrediction`: when false, only record + update state (Space path)
    func processNextWord(text: String, roman: String, requireRomanMode: Bool = false, triggerPrediction: Bool = true) {
        if requireRomanMode {
            guard !settings.isTranslateSwapped else { return }
        }

        guard !text.isEmpty, !isNoiseText(text) else {
            if isSentenceEndPunctuation(text) {
                resetNextWordContext()
            }
            return
        }

        // Normalize romanization to TL for consistent storage and query
        // pojToTL is idempotent on TL input, safe for all modes including TPS
        let textTl = RomanizationConverter.pojToTL(roman)
        let prevTl = RomanizationConverter.pojToTL(lastSelectedRoman ?? "")

        if SharedSettings.shared.associationRecordingEnabled {
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
            triggerNextWordPrediction(for: text, roman: textTl)
        }
    }

    func splitCompoundWord(_ word: String) -> [String] {
        guard !word.isEmpty else { return [] }
        return word.split(separator: "-").map(String.init).filter { !$0.isEmpty }
    }

    /// Record associations between parts of compound words (e.g. tshit-niû → tshit, niû)
    func recordCompoundWordAssociations(displayText: String, roman: String) {
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
