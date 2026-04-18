// ActionHandler extension: suggestion selection (candidate commit, output formatting).

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
            let isTPSLayout = settings.keyboardLayoutType == .tps
            let effectiveSwapped = isTPSLayout || settings.isTranslateSwapped

            let (roman, hanzi) = parseRomanAndHanzi(from: suggestion, isNextWord: isNextWordPrediction, effectiveSwapped: effectiveSwapped)
            let textToCommit = formatOutputText(roman: roman, hanzi: hanzi, isTPSLayout: isTPSLayout, effectiveSwapped: effectiveSwapped)

            commitSuggestionText(textToCommit, isNextWord: isNextWordPrediction, suggestion: suggestion)

            let displayText = suggestion.additionalInfo["displayText"] ?? hanzi ?? roman
            if settings.isFrequencyRecordingEnabled {
                UserFrequencyService.recordUsage(for: displayText)
            }

            logger.debug("[SELECT] suggestion.text='\(suggestion.text)' subtitle='\(suggestion.subtitle ?? "nil")' additionalInfo=\(suggestion.additionalInfo.description)")
            logger.debug("[SELECT] parsed roman='\(roman)' hanzi='\(hanzi ?? "nil")' displayText='\(displayText)'")

            // Romanization mode: auto-space (unless trailing hyphen)
            // TPS mode disables auto-space (effectiveSwapped is true for TPS)
            if settings.isAutoSpaceEnabled, !effectiveSwapped || settings.isOutputBothScripts {
                if !textToCommit.hasSuffix("-") {
                    keyboardContext.textDocumentProxy.insertText(" ")
                }
            }

            nextWordController.process(text: displayText, roman: roman)
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
            ? TLToTPS.convertFromDisplay(roman, orMapsToER: settings.isTpsOrMappedToER)
            : roman

        if settings.isOutputBothScripts, let hanzi, !hanzi.isEmpty {
            return effectiveSwapped
                ? "\(hanzi) (\(bracketRoman))"
                : "\(bracketRoman) (\(hanzi))"
        } else if effectiveSwapped, let hanzi, !hanzi.isEmpty {
            return hanzi
        } else {
            return roman
        }
    }

    /// Commit text via proxy (NextWord) or composing manager (regular candidate).
    /// The `suggestion` parameter is kept for future telemetry/logging use
    /// but ComposingManager only needs the candidate text.
    private func commitSuggestionText(_ text: String, isNextWord: Bool, suggestion _: Autocomplete.Suggestion) {
        if isNextWord {
            keyboardContext.textDocumentProxy.insertText(text)
        } else {
            composingManager.selectSuggestion(text: text)
        }
    }
}
