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
        let isNextWordPrediction = suggestion.additionalInfo["isNextWord"] == "true"

        if composingManager.isComposing || isNextWordPrediction {
            let wasSwapped = settings.isTranslateSwapped

            // 解析羅馬字與漢字
            let roman: String
            let hanzi: String?

            if isNextWordPrediction {
                hanzi = suggestion.additionalInfo["hanzi"]
                roman = suggestion.additionalInfo["tl"] ?? suggestion.additionalInfo["poj"] ?? suggestion.text
            } else if wasSwapped {
                roman = suggestion.subtitle ?? suggestion.text
                hanzi = suggestion.text
            } else {
                roman = suggestion.text
                hanzi = suggestion.subtitle
            }

            // 決定輸出文字
            let textToCommit: String
            if settings.outputBothScripts && hanzi != nil && !hanzi!.isEmpty {
                textToCommit = settings.isTranslateSwapped
                    ? "\(hanzi!) (\(roman))"
                    : "\(roman) (\(hanzi!))"
            } else if settings.isTranslateSwapped && hanzi != nil && !hanzi!.isEmpty {
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
            UserFrequencyService.recordUsage(for: displayText)

            // 羅馬字模式：自動加空白（字尾非連字符時）
            if settings.isAutoSpaceEnabled && (!settings.isTranslateSwapped || settings.outputBothScripts) {
                if !textToCommit.hasSuffix("-") {
                    keyboardContext.textDocumentProxy.insertText(" ")
                }
            }

            handleNextWordPrediction(displayText: displayText, roman: roman)
        } else {
            keyboardContext.textDocumentProxy.insertText(suggestion.text)
        }
    }

    // MARK: - NextWord 處理

    /// 選詞後觸發 NextWord 預測並記錄關聯
    private func handleNextWordPrediction(displayText: String, roman: String) {
        guard !isNoiseText(displayText) else {
            if isSentenceEndPunctuation(displayText) {
                resetNextWordContext()
            }
            return
        }

        // 記錄與前一詞的關聯
        if shouldRecordAssociation(), let prevWord = lastSelectedWord {
            Task {
                await NextWordService.shared.recordAssociation(
                    prev: prevWord,
                    nextHanzi: displayText,
                    nextTl: roman
                )
            }
        }

        recordCompoundWordAssociations(displayText: displayText, roman: roman)
        updateNextWordState(selectedWord: displayText)
        triggerNextWordPrediction(for: displayText)
    }

    private func splitCompoundWord(_ word: String) -> [String] {
        guard !word.isEmpty else { return [] }
        return word.split(separator: "-").map(String.init).filter { !$0.isEmpty }
    }

    /// 記錄複合詞內部關聯（如 tshit-niû → tshit, niû）
    private func recordCompoundWordAssociations(displayText: String, roman: String) {
        let parts = splitCompoundWord(displayText)
        let romanParts = splitCompoundWord(roman)

        guard parts.count > 1 else { return }

        let useTl = (settings.inputMode == .tl)

        Task {
            for i in 0..<(parts.count - 1) {
                let prevPart = parts[i]
                let nextPart = parts[i + 1]
                let nextRoman = romanParts.indices.contains(i + 1) ? romanParts[i + 1] : ""

                await NextWordService.shared.recordAssociation(
                    prev: prevPart,
                    nextHanzi: nextPart,
                    nextTl: useTl ? nextRoman : "",
                    nextPoj: useTl ? "" : nextRoman
                )
            }
        }
    }

    /// Enter 確認組字後觸發 NextWord 預測（僅羅馬字模式）
    func handleEnterNextWordPrediction(committedText: String) {
        guard !settings.isTranslateSwapped, !committedText.isEmpty else { return }
        guard !isNoiseText(committedText) else { return }

        if shouldRecordAssociation(), let prevWord = lastSelectedWord {
            Task {
                await NextWordService.shared.recordAssociation(
                    prev: prevWord,
                    nextHanzi: committedText,
                    nextTl: committedText
                )
            }
        }

        recordCompoundWordAssociations(displayText: committedText, roman: committedText)
        updateNextWordState(selectedWord: committedText)
        triggerNextWordPrediction(for: committedText)
    }
}
