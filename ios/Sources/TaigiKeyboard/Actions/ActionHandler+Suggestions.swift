import Foundation
import KeyboardKit

// MARK: - Suggestion Handling

extension ActionHandler {
    /// 處理候選詞選擇的實作邏輯
    /// - Parameter suggestion: 使用者選擇的候選詞
    func handleSuggestionSelection(_ suggestion: Autocomplete.Suggestion) {
        // 檢查是否為 NextWord 候選詞
        let isNextWordPrediction = suggestion.additionalInfo["isNextWord"] == "true"

        if composingManager.isComposing || isNextWordPrediction {
            // 判斷 suggestion 是否已被 CandidateView 交換
            // showHanjiMode 固定為 true，CandidateView 在 isTranslateSwapped 時會交換 text/subtitle
            let wasSwapped = settings.isTranslateSwapped

            // 取得真正的羅馬字與漢字
            let roman: String
            let hanzi: String?

            if isNextWordPrediction {
                // NextWord 候選詞：直接使用 additionalInfo 中的資料
                hanzi = suggestion.additionalInfo["hanzi"]
                roman = suggestion.additionalInfo["tl"] ?? suggestion.additionalInfo["poj"] ?? suggestion.text
            } else if wasSwapped {
                // CandidateView 已交換：text = 漢字, subtitle = 羅馬字
                roman = suggestion.subtitle ?? suggestion.text
                hanzi = suggestion.text
            } else {
                // 未交換：text = 羅馬字, subtitle = 漢字
                roman = suggestion.text
                hanzi = suggestion.subtitle
            }

            // 決定輸出文字
            // showHanjiMode 固定為 true
            let textToCommit: String
            if settings.outputBothScripts && hanzi != nil && !hanzi!.isEmpty {
                // 漢羅做伙出模式
                if settings.isTranslateSwapped {
                    textToCommit = "\(hanzi!) (\(roman))"
                } else {
                    textToCommit = "\(roman) (\(hanzi!))"
                }
            } else if settings.isTranslateSwapped && hanzi != nil && !hanzi!.isEmpty {
                // 漢字模式
                textToCommit = hanzi!
            } else {
                // 羅馬字模式
                textToCommit = roman
            }

            // 提交文字
            if isNextWordPrediction {
                // NextWord 候選詞：直接插入文字
                keyboardContext.textDocumentProxy.insertText(textToCommit)
            } else {
                // 一般候選詞：使用 composingManager 選擇
                let modifiedSuggestion = Autocomplete.Suggestion(
                    text: textToCommit,
                    title: suggestion.title,
                    subtitle: suggestion.subtitle,
                    additionalInfo: suggestion.additionalInfo
                )
                composingManager.selectSuggestion(modifiedSuggestion)
            }

            // 使用 displayText 作為記錄 Key（與排序查詢一致）
            let displayText = suggestion.additionalInfo["displayText"] ?? hanzi ?? roman
            UserFrequencyService.recordUsage(for: displayText)

            // 羅馬字模式或漢羅做伙出模式：選擇候選詞後自動加空白（字尾非連字符時）
            if settings.isAutoSpaceEnabled && (!settings.isTranslateSwapped || settings.outputBothScripts) {
                // 檢查字尾是否為連字符
                if !textToCommit.hasSuffix("-") {
                    keyboardContext.textDocumentProxy.insertText(" ")
                }
            }

            // 處理 NextWord 預測
            handleNextWordPrediction(displayText: displayText, roman: roman)

        } else {
            keyboardContext.textDocumentProxy.insertText(suggestion.text)
        }
    }

    // MARK: - NextWord Prediction

    /// 處理 NextWord 預測
    private func handleNextWordPrediction(displayText: String, roman: String) {
        // 檢查是否為雜訊
        guard !isNoiseText(displayText) else {
            // 雜訊不觸發 NextWord，但檢查是否為句末標點需要重置
            if isSentenceEndPunctuation(displayText) {
                resetNextWordContext()
            }
            return
        }

        // 記錄關聯（如果符合條件）
        if shouldRecordAssociation(), let prevWord = lastSelectedWord {
            Task {
                await NextWordService.shared.recordAssociation(
                    prev: prevWord,
                    nextHanzi: displayText,
                    nextTl: roman
                )
            }
        }

        // 記錄複合詞內部的關聯（如 tshit-niû → tshit → niû）
        recordCompoundWordAssociations(displayText: displayText, roman: roman)

        // 更新狀態
        updateNextWordState(selectedWord: displayText)

        // 觸發 NextWord 預測
        triggerNextWordPrediction(for: displayText)
    }

    /// 拆分複合詞
    private func splitCompoundWord(_ word: String) -> [String] {
        guard !word.isEmpty else { return [] }
        return word.split(separator: "-").map(String.init).filter { !$0.isEmpty }
    }

    /// 記錄複合詞內部的關聯
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

                let nextTl = useTl ? nextRoman : ""
                let nextPoj = useTl ? "" : nextRoman

                await NextWordService.shared.recordAssociation(
                    prev: prevPart,
                    nextHanzi: nextPart,
                    nextTl: nextTl,
                    nextPoj: nextPoj
                )
            }
        }
    }

    /// 處理 Enter 確認後的 NextWord 預測（羅馬字模式）
    ///
    /// - Parameter committedText: 確認的組字文字
    func handleEnterNextWordPrediction(committedText: String) {
        // 羅馬字模式（isTranslateSwapped=false）：記錄羅馬字到 NextWord
        // 漢字模式（isTranslateSwapped=true）：不記錄，因為組字不會產生漢字
        guard !settings.isTranslateSwapped, !committedText.isEmpty else {
            return
        }

        // 檢查是否為雜訊
        guard !isNoiseText(committedText) else { return }

        // 記錄關聯（如果符合條件）
        if shouldRecordAssociation(), let prevWord = lastSelectedWord {
            Task {
                await NextWordService.shared.recordAssociation(
                    prev: prevWord,
                    nextHanzi: committedText,
                    nextTl: committedText  // 羅馬字模式下 committedText 就是羅馬字
                )
            }
        }

        // 記錄複合詞內部的關聯
        recordCompoundWordAssociations(displayText: committedText, roman: committedText)

        // 更新狀態
        updateNextWordState(selectedWord: committedText)

        // 觸發 NextWord 預測
        triggerNextWordPrediction(for: committedText)
    }

    /// 觸發 NextWord 預測並更新候選詞
    private func triggerNextWordPrediction(for word: String) {
        Task { @MainActor in
            let predictions = await NextWordService.shared.predict(word: word)

            guard !predictions.isEmpty else {
                isShowingNextWord = false
                return
            }

            // 將預測結果轉換為 Autocomplete.Suggestion
            // 羅馬字模式下過濾無羅馬字的候選詞
            let suggestions = predictions.compactMap { prediction -> Autocomplete.Suggestion? in
                // 羅馬字模式下，若無羅馬字則跳過
                if !settings.isTranslateSwapped && prediction.tl.isEmpty && prediction.poj.isEmpty {
                    return nil
                }

                // 統一格式：text = 羅馬字, subtitle = 漢字
                // 讓 CandidateView 根據 isTranslateSwapped 統一處理顯示交換
                // 這樣與一般候選詞格式一致，避免雙重交換問題
                let roman = prediction.tl.isEmpty ? prediction.poj : prediction.tl
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
                        "poj": prediction.poj,
                        "displayText": prediction.hanzi
                    ]
                )
            }

            // 更新 AutocompleteContext
            if let controller = keyboardViewController {
                if suggestions.isEmpty {
                    isShowingNextWord = false
                    controller.state.autocompleteContext.reset()
                } else {
                    controller.state.autocompleteContext.suggestionsFromService = suggestions
                    isShowingNextWord = true
                }
            }
        }
    }
}
