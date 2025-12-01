import Foundation
import KeyboardKit

// MARK: - Suggestion Handling

extension ActionHandler {
    /// 處理候選詞選擇的實作邏輯
    /// - Parameter suggestion: 使用者選擇的候選詞
    func handleSuggestionSelection(_ suggestion: Autocomplete.Suggestion) {
        if composingManager.isComposing {
            // 判斷 suggestion 是否已被 CandidateView 交換
            // showHanjiMode 固定為 true，CandidateView 在 isTranslateSwapped 時會交換 text/subtitle
            let wasSwapped = settings.isTranslateSwapped

            // 取得真正的羅馬字與漢字
            let roman: String
            let hanzi: String?

            if wasSwapped {
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
            let modifiedSuggestion = Autocomplete.Suggestion(
                text: textToCommit,
                title: suggestion.title,
                subtitle: suggestion.subtitle,
                additionalInfo: suggestion.additionalInfo
            )
            composingManager.selectSuggestion(modifiedSuggestion)

            // 使用 displayText 作為記錄 Key（與排序查詢一致）
            if let displayText = suggestion.additionalInfo["displayText"] {
                UserFrequencyService.recordUsage(for: displayText)
            }

            // 羅馬字模式或漢羅做伙出模式：選擇候選詞後自動加空白（字尾非連字符時）
            if settings.isAutoSpaceEnabled && (!settings.isTranslateSwapped || settings.outputBothScripts) {
                // 檢查字尾是否為連字符
                if !textToCommit.hasSuffix("-") {
                    keyboardContext.textDocumentProxy.insertText(" ")
                }
            }

            // 注意：resetAutocomplete 已經在 selectSuggestion 中處理
        } else {
            keyboardContext.textDocumentProxy.insertText(suggestion.text)
        }
    }
}
