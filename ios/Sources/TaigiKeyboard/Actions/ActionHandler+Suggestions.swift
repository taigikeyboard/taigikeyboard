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
            let wasSwapped = settings.isTranslateSwapped
            let isTPSLayout = SharedSettings.shared.keyboardLayoutType == .tps
            let effectiveSwapped = isTPSLayout || wasSwapped

            // Parse romanization and Hanji (漢字)
            let roman: String
            let hanzi: String?

            if isNextWordPrediction {
                hanzi = suggestion.additionalInfo["hanzi"]
                roman = suggestion.additionalInfo["tl"] ?? suggestion.text
            } else if effectiveSwapped {
                roman = suggestion.subtitle ?? suggestion.text
                hanzi = suggestion.text
            } else {
                roman = suggestion.text
                hanzi = suggestion.subtitle
            }

            // Convert roman to TPS for bracket annotation when in TPS mode
            let bracketRoman = isTPSLayout
                ? TPSConverter.toTPSFromDisplay(roman, orMapsToER: SharedSettings.shared.tpsOrMapsToER)
                : roman

            // Determine output text
            let textToCommit: String = if settings.outputBothScripts, hanzi != nil, !hanzi!.isEmpty {
                effectiveSwapped
                    ? "\(hanzi!) (\(bracketRoman))"
                    : "\(bracketRoman) (\(hanzi!))"
            } else if effectiveSwapped, hanzi != nil, !hanzi!.isEmpty {
                hanzi!
            } else {
                roman
            }

            // Commit text
            if isNextWordPrediction {
                keyboardContext.textDocumentProxy.insertText(textToCommit)
            } else {
                let modifiedSuggestion = Autocomplete.Suggestion(
                    text: textToCommit,
                    title: suggestion.title,
                    subtitle: suggestion.subtitle,
                    additionalInfo: suggestion.additionalInfo,
                )
                composingManager.selectSuggestion(modifiedSuggestion)
            }

            // Record usage frequency
            let displayText = suggestion.additionalInfo["displayText"] ?? hanzi ?? roman
            if SharedSettings.shared.frequencyRecordingEnabled {
                UserFrequencyService.recordUsage(for: displayText)
            }

            // DEBUG: NextWord trace - suggestion selection parsing
            logger.debug("[NEXTWORD][SELECT] suggestion.text='\(suggestion.text)' subtitle='\(suggestion.subtitle ?? "nil")' additionalInfo=\(suggestion.additionalInfo.description)")
            logger.debug("[NEXTWORD][SELECT] parsed roman='\(roman)' hanzi='\(hanzi ?? "nil")' displayText='\(displayText)'")

            // Romanization mode: auto-space (unless trailing hyphen)
            // TPS mode disables auto-space (effectiveSwapped is true for TPS)
            if settings.isAutoSpaceEnabled, !effectiveSwapped || settings.outputBothScripts {
                if !textToCommit.hasSuffix("-") {
                    keyboardContext.textDocumentProxy.insertText(" ")
                }
            }

            handleNextWordPrediction(
                displayText: displayText,
                roman: roman,
                hanzi: hanzi,
            )
        } else {
            keyboardContext.textDocumentProxy.insertText(suggestion.text)
        }
    }

    // MARK: - NextWord Handling

    /// Trigger NextWord prediction and record association after word selection
    private func handleNextWordPrediction(displayText: String, roman: String, hanzi: String? = nil) {
        // DEBUG: NextWord trace - handleNextWordPrediction entry
        logger.debug("[NEXTWORD][HANDLE] displayText='\(displayText)' roman='\(roman)' hanzi='\(hanzi ?? "nil")' isNoise=\(isNoiseText(displayText))")

        guard !isNoiseText(displayText) else {
            if isSentenceEndPunctuation(displayText) {
                resetNextWordContext()
            }
            return
        }

        // Normalize romanization to TL for consistent storage and query
        // pojToTL is idempotent on TL input, safe for all modes including TPS
        let romanTl = RomanizationConverter.pojToTL(roman)
        let prevTl = RomanizationConverter.pojToTL(lastSelectedRoman ?? "")

        // Record association with previous word
        if SharedSettings.shared.associationRecordingEnabled {
            if shouldRecordAssociation(), let prevWord = lastSelectedWord {
                Task {
                    await NextWordService.shared.recordAssociation(
                        prev: prevWord,
                        prevTl: prevTl,
                        nextHanzi: displayText,
                        nextTl: romanTl,
                    )
                }
            }

            recordCompoundWordAssociations(displayText: displayText, roman: romanTl)
        }
        updateNextWordState(selectedWord: displayText, roman: romanTl)
        triggerNextWordPrediction(for: displayText, roman: romanTl)
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

    /// Trigger NextWord prediction after Enter commits composing (romanization mode only)
    func handleEnterNextWordPrediction(committedText: String) {
        guard !settings.isTranslateSwapped, !committedText.isEmpty else { return }
        guard !isNoiseText(committedText) else { return }

        // Normalize romanization to TL for consistent storage
        let committedTl = RomanizationConverter.pojToTL(committedText)
        let prevTl = RomanizationConverter.pojToTL(lastSelectedRoman ?? "")

        if SharedSettings.shared.associationRecordingEnabled {
            if shouldRecordAssociation(), let prevWord = lastSelectedWord {
                Task {
                    await NextWordService.shared.recordAssociation(
                        prev: prevWord,
                        prevTl: prevTl,
                        nextHanzi: committedText,
                        nextTl: committedTl,
                    )
                }
            }

            recordCompoundWordAssociations(displayText: committedText, roman: committedTl)
        }
        updateNextWordState(selectedWord: committedText, roman: committedTl)
        triggerNextWordPrediction(for: committedText, roman: committedTl)
    }
}
