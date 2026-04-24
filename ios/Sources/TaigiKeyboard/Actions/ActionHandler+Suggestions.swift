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
                CompositionRoot.userFrequencyService.recordUsage(for: displayText)
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

            // Fork: `roman` is the commit string (may be POJ/Hanji); the engine
            // expects raw TL (it calls `pojToTL` on it). Next-word candidates
            // carry raw TL on the `additionalInfo["tl"]` sidechannel — use it
            // here to preserve association-recording semantics.
            let associationRoman = isNextWordPrediction
                ? (suggestion.additionalInfo["tl"] ?? "")
                : roman
            nextWordController.process(text: displayText, roman: associationRoman)
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
            // CROSS-PLATFORM INVARIANT: next-word commit string == UI display string.
            // `suggestion.text` is mode-shaped (POJ in POJ mode, TL otherwise) by
            // `NextWordEngine.filterPredictions`, but `CandidateCellHelper.suggestionToHandle`
            // pre-swaps text↔subtitle in swapped/TPS modes before this handler runs —
            // so we must mirror that swap to recover the mode-shaped roman.
            // `additionalInfo["hanzi"]` carries hanzi even for hanzi-only predictions
            // (Case B) where `subtitle == nil`. The raw-TL sidechannel on
            // `additionalInfo["tl"]` is consumed separately at the association call
            // site (see `handleSuggestionSelection`).
            // Mirror: android/.../smartbar/NextWordHandler.kt:355-363 (TaigiWord.roman).
            // Swapped/TPS Case B (hanzi-only, no roman): `subtitle == nil` after
            // `suggestionToHandle` (swap gate requires non-empty subtitle). Fall
            // back to `""` so bracket-mode output stays `"漢字 ()"` — matches the
            // pre-fix sidechannel behavior, avoids Hanji duplication.
            let roman = effectiveSwapped
                ? (suggestion.subtitle ?? "")
                : suggestion.text
            return (roman, suggestion.additionalInfo["hanzi"])
        } else if effectiveSwapped {
            return (suggestion.subtitle ?? suggestion.text, suggestion.text)
        } else {
            return (suggestion.text, suggestion.subtitle)
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
