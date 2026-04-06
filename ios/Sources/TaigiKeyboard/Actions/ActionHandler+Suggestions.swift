import Foundation
import KeyboardKit

/// 候選詞選擇處理
///
/// 處理使用者點選候選詞的邏輯，包含：
/// - 一般組字候選詞
/// - NextWord 下一詞預測候選詞
/// - 詞彙關聯記錄
extension ActionHandler {

    // MARK: - 候選詞選擇

    func handleSuggestionSelection(_ suggestion: Autocomplete.Suggestion) {
        // Raw input candidate: commit literal keystrokes directly (no tone conversion)
        if suggestion.additionalInfo["isRawInput"] == "true" {
            composingManager.commitRawInput()
            if settings.isAutoSpaceEnabled && !settings.isTranslateSwapped {
                keyboardContext.textDocumentProxy.insertText(" ")
            }
            return
        }

        let isNextWordPrediction = suggestion.additionalInfo["isNextWord"] == "true"

        if composingManager.isComposing || isNextWordPrediction {
            let wasSwapped = settings.isTranslateSwapped
            let isTPSLayout = SharedSettings.shared.keyboardLayoutType == .tps
            let effectiveSwapped = isTPSLayout || wasSwapped

            // Capture rawInput BEFORE selectSuggestion clears it
            let capturedRawInput = composingManager.rawInput

            // 解析羅馬字與漢字
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

            // 決定輸出文字
            let textToCommit: String
            if settings.outputBothScripts && hanzi != nil && !hanzi!.isEmpty {
                textToCommit = effectiveSwapped
                    ? "\(hanzi!) (\(bracketRoman))"
                    : "\(bracketRoman) (\(hanzi!))"
            } else if effectiveSwapped && hanzi != nil && !hanzi!.isEmpty {
                textToCommit = hanzi!
            } else {
                textToCommit = roman
            }

            // 提交文字
            if isNextWordPrediction {
                keyboardContext.textDocumentProxy.insertText(textToCommit)
            } else {
                let modifiedSuggestion = Autocomplete.Suggestion(
                    text: textToCommit,
                    title: suggestion.title,
                    subtitle: suggestion.subtitle,
                    additionalInfo: suggestion.additionalInfo
                )
                composingManager.selectSuggestion(modifiedSuggestion)
            }

            // 記錄使用頻率
            let displayText = suggestion.additionalInfo["displayText"] ?? hanzi ?? roman
            if SharedSettings.shared.frequencyRecordingEnabled {
                UserFrequencyService.recordUsage(for: displayText)
            }

            // DEBUG: NextWord trace - suggestion selection parsing
            logger.debug("[NEXTWORD][SELECT] suggestion.text='\(suggestion.text, privacy: .public)' subtitle='\(suggestion.subtitle ?? "nil", privacy: .public)' additionalInfo=\(suggestion.additionalInfo.description, privacy: .public)")
            logger.debug("[NEXTWORD][SELECT] parsed roman='\(roman, privacy: .public)' hanzi='\(hanzi ?? "nil", privacy: .public)' displayText='\(displayText, privacy: .public)'")

            // 羅馬字模式：自動加空白（字尾非連字符時）
            // TPS mode disables auto-space (effectiveSwapped is true for TPS)
            if settings.isAutoSpaceEnabled && (!effectiveSwapped || settings.outputBothScripts) {
                if !textToCommit.hasSuffix("-") {
                    keyboardContext.textDocumentProxy.insertText(" ")
                }
            }

            handleNextWordPrediction(
                displayText: displayText,
                roman: roman,
                hanzi: hanzi,
                rawInput: capturedRawInput
            )
        } else {
            keyboardContext.textDocumentProxy.insertText(suggestion.text)
        }
    }

    // MARK: - NextWord 處理

    /// 選詞後觸發 NextWord 預測並記錄關聯
    private func handleNextWordPrediction(displayText: String, roman: String, hanzi: String? = nil, rawInput: String = "") {
        // DEBUG: NextWord trace - handleNextWordPrediction entry
        logger.debug("[NEXTWORD][HANDLE] displayText='\(displayText, privacy: .public)' roman='\(roman, privacy: .public)' hanzi='\(hanzi ?? "nil", privacy: .public)' isNoise=\(self.isNoiseText(displayText), privacy: .public)")

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

        // 記錄與前一詞的關聯
        if SharedSettings.shared.associationRecordingEnabled {
            if shouldRecordAssociation(), let prevWord = lastSelectedWord {
                Task {
                    await NextWordService.shared.recordAssociation(
                        prev: prevWord,
                        prevTl: prevTl,
                        nextHanzi: displayText,
                        nextTl: romanTl
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

    /// 記錄複合詞內部關聯（如 tshit-niû → tshit, niû）
    func recordCompoundWordAssociations(displayText: String, roman: String) {
        let parts = splitCompoundWord(displayText)
        let romanParts = splitCompoundWord(roman)

        guard parts.count > 1 else { return }

        Task {
            for i in 0..<(parts.count - 1) {
                let prevPart = parts[i]
                let prevPartRoman = romanParts.indices.contains(i) ? romanParts[i] : ""
                let nextPart = parts[i + 1]
                let nextRoman = romanParts.indices.contains(i + 1) ? romanParts[i + 1] : ""

                await NextWordService.shared.recordAssociation(
                    prev: prevPart,
                    prevTl: prevPartRoman,
                    nextHanzi: nextPart,
                    nextTl: nextRoman
                )
            }
        }
    }

    /// Enter 確認組字後觸發 NextWord 預測（僅羅馬字模式）
    func handleEnterNextWordPrediction(committedText: String, rawInput: String = "") {
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
                        nextTl: committedTl
                    )
                }
            }

            recordCompoundWordAssociations(displayText: committedText, roman: committedTl)
        }
        updateNextWordState(selectedWord: committedText, roman: committedTl)
        triggerNextWordPrediction(for: committedText, roman: committedTl)
    }
}
