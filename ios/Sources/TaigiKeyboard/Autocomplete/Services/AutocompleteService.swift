import Foundation
import KeyboardKit
import OSLog

/// 台語鍵盤自動完成服務
/// 專門處理台語羅馬字與漢字的候選詞搜尋
/// 支援多種輸入類型：純羅馬字、帶聲調羅馬字、漢字
class AutocompleteService: KeyboardKit.AutocompleteService {
    // MARK: - KeyboardKit 協議要求的屬性
    var locale: Locale = .current

    // MARK: - 學習功能相關屬性（台語鍵盤不使用）
    /// 是否支援忽略詞彙功能（台語鍵盤不使用此功能）
    var canIgnoreWords: Bool { false }

    /// 是否支援學習詞彙功能（台語鍵盤不使用此功能）
    var canLearnWords: Bool { false }

    /// 忽略的詞彙列表（台語鍵盤不使用此功能）
    var ignoredWords: [String] = []

    /// 學習的詞彙列表（台語鍵盤不使用此功能）
    var learnedWords: [String] = []

    // MARK: - 學習功能相關方法（台語鍵盤不實作）
    /// 檢查是否已忽略指定詞彙（台語鍵盤不使用）
    func hasIgnoredWord(_: String) -> Bool { false }

    /// 檢查是否已學習指定詞彙（台語鍵盤不使用）
    func hasLearnedWord(_: String) -> Bool { false }

    /// 忽略指定詞彙（台語鍵盤不實作）
    func ignoreWord(_: String) {}

    /// 學習指定詞彙（台語鍵盤不實作）
    func learnWord(_: String) {}

    /// 移除忽略的詞彙（台語鍵盤不實作）
    func removeIgnoredWord(_: String) {}

    /// 停止學習指定詞彙（台語鍵盤不實作）
    func unlearnWord(_: String) {}

    // MARK: - 核心服務屬性
    /// 詞典搜尋服務
    private let lexiconService = LexiconService.shared

    /// 共用設定管理器
    private let settings = SharedSettings.shared

    /// 組字管理器的弱引用
    private weak var composingManager: ComposingManager?

    /// 日誌記錄器
    internal let logger = Logger(
        subsystem: LexiconConstants.Logging.subsystem,
        category: "AutocompleteService",
    )

    // MARK: - 公開介面
    /// 設定組字管理器
    /// - Parameter manager: 組字管理器實例
    func setComposingManager(_ manager: ComposingManager) {
        composingManager = manager
    }

    /// 自動完成核心方法
    /// 根據輸入文字搜尋台語候選詞
    /// 第 0 個候選詞永遠是當前的組字文字，第 1 個位置開始才是建議的候選詞
    /// - Parameter text: 輸入文字
    /// - Returns: 候選詞搜尋結果
    func autocomplete(_ text: String) async throws -> Autocomplete.ServiceResult {
        guard !text.isEmpty else {
            return Autocomplete.ServiceResult(inputText: text, suggestions: [])
        }

        do {
            let searchText: String

            if let composingManager,
               composingManager.isComposing, !composingManager.composingText.isEmpty
            {
                searchText = composingManager.composingText
            } else {
                // 沒有組字狀態時不顯示候選詞
                return Autocomplete.ServiceResult(inputText: text, suggestions: [])
            }

            let inputMode = settings.inputMode
            let preprocessedText = ToneConverter.convertToToneMarks(searchText, mode: inputMode)
            let inputType = determineInputType(preprocessedText)

            let words = try await lexiconService.search(for: preprocessedText, inputType: inputType, inputMode: inputMode, limit: 100)

            // 根據使用者實際輸入的大小寫模式轉換候選詞
            let casePattern = determineCasePattern(from: searchText)
            var suggestions = convertToSuggestions(words, casePattern: casePattern)

            // 在第 0 個位置插入當前組字文字候選詞
            let composingTextSuggestion = createComposingTextSuggestion(searchText)
            suggestions.insert(composingTextSuggestion, at: 0)

            let result = Autocomplete.ServiceResult(inputText: text, suggestions: suggestions)
            return result
        } catch {
            logger.error("Autocomplete failed for text '\(text)': \(error.localizedDescription)")
            return Autocomplete.ServiceResult(inputText: text, suggestions: [])
        }
    }

    // MARK: - 私有輔助方法

    /// 建立當前組字文字的候選詞物件
    /// 這個候選詞會被放在候選詞列的第 0 個位置，顯示使用者目前正在輸入的內容
    /// - Parameter composingText: 當前組字文字
    /// - Returns: 組字文字的候選詞物件
    private func createComposingTextSuggestion(_ composingText: String) -> Autocomplete.Suggestion {
        return Autocomplete.Suggestion(
            text: composingText,
            title: composingText,
            subtitle: nil,
            additionalInfo: ["isComposingText": "true"]
        )
    }

    /// 大小寫模式
    private enum CasePattern {
        case lowercase      // 全小寫
        case capitalized    // 首字母大寫
        case uppercase      // 全大寫
    }

    /// 判斷輸入文字的大小寫模式
    /// - Parameter text: 輸入文字
    /// - Returns: 大小寫模式
    private func determineCasePattern(from text: String) -> CasePattern {
        let letters = text.filter { $0.isLetter && ("a"..."z").contains($0.lowercased()) }
        guard !letters.isEmpty else { return .lowercase }

        let uppercaseCount = letters.filter { $0.isUppercase }.count
        let totalCount = letters.count

        // 首字母大寫（第一個字母大寫，包含單一大寫字母）
        if uppercaseCount == 1 && letters.first?.isUppercase == true {
            return .capitalized
        }

        // 全大寫（至少兩個字母且全部大寫）
        if uppercaseCount == totalCount && totalCount >= 2 {
            return .uppercase
        }

        // 其他情況視為小寫
        return .lowercase
    }

    /// 判斷輸入文字的類型
    /// - Parameter text: 輸入文字
    /// - Returns: 輸入類型（漢字、帶聲調羅馬字、無聲調羅馬字）
    private func determineInputType(_ text: String) -> InputType {
        if containsHanzi(text) {
            return .hanzi
        }

        if containsToneMarks(text) {
            return .romanWithTone
        }

        return .romanWithoutTone
    }

    /// 檢查文字是否包含漢字
    /// - Parameter text: 待檢查的文字
    /// - Returns: 是否包含漢字
    private func containsHanzi(_ text: String) -> Bool {
        text.unicodeScalars.contains { scalar in
            (0x4E00 ... 0x9FFF).contains(scalar.value) ||
                (0xF900 ... 0xFAFF).contains(scalar.value) ||
                (0x3400 ... 0x4DBF).contains(scalar.value)
        }
    }

    /// 檢查文字是否包含聲調標記
    /// - Parameter text: 待檢查的文字
    /// - Returns: 是否包含聲調標記
    private func containsToneMarks(_ text: String) -> Bool {
        text.contains { char in
            ToneMappings.pojToneToBase[String(char)] != nil ||
                ToneMappings.tlToneToBase[String(char)] != nil
        }
    }

    /// 從完整文字中提取當前詞彙
    /// - Parameter fullText: 完整輸入文字
    /// - Returns: 當前詞彙
    private func extractCurrentWord(from fullText: String) -> String {
        guard !fullText.isEmpty else { return "" }

        let currentWord = extractTaigiCurrentWord(from: fullText)

        return currentWord.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    /// 提取台語當前詞彙（處理羅馬字與漢字混合情況）
    /// - Parameter text: 輸入文字
    /// - Returns: 台語當前詞彙
    private func extractTaigiCurrentWord(from text: String) -> String {
        guard !text.isEmpty else { return "" }

        var currentWord = ""
        var hasFoundRomanChar = false

        for char in text.reversed() {
            if isSeparatorCharacter(char) {
                break
            }

            if isRomanCharacter(char) {
                currentWord = String(char) + currentWord
                hasFoundRomanChar = true
                continue
            }

            if isHanziCharacter(char) {
                if hasFoundRomanChar {
                    break
                }
                currentWord = String(char) + currentWord
                continue
            }

            break
        }

        return currentWord
    }

    /// 判斷字元是否為分隔符
    /// - Parameter char: 待檢查的字元
    /// - Returns: 是否為分隔符
    private func isSeparatorCharacter(_ char: Character) -> Bool {
        if char.isWhitespace || char.isNewline {
            return true
        }

        if char == "-" {
            return false
        }

        if let scalar = char.unicodeScalars.first {
            return CharacterSet.punctuationCharacters.contains(scalar)
        }

        return false
    }

    /// 判斷字元是否為漢字
    /// - Parameter char: 待檢查的字元
    /// - Returns: 是否為漢字
    private func isHanziCharacter(_ char: Character) -> Bool {
        ToneUtilities.isHanzi(String(char))
    }

    /// 判斷字元是否為羅馬字符（包含字母、數字、聲調符號）
    /// - Parameter char: 待檢查的字元
    /// - Returns: 是否為羅馬字符
    private func isRomanCharacter(_ char: Character) -> Bool {
        let charString = String(char)

        if isHanziCharacter(char) {
            return false
        }

        if char.isLetter {
            return true
        }

        if char.isNumber {
            return true
        }

        if char == "-" {
            return true
        }

        let pojToneChars = Set(ToneMappings.pojToneToBase.keys)
        let tlToneChars = Set(ToneMappings.tlToneToBase.keys)

        if pojToneChars.contains(charString) || tlToneChars.contains(charString) {
            return true
        }

        return false
    }

    /// 將台語詞彙轉換為 KeyboardKit 候選詞格式
    /// - Parameters:
    ///   - words: 台語詞彙列表
    ///   - casePattern: 大小寫模式
    /// - Returns: KeyboardKit 候選詞列表
    private func convertToSuggestions(_ words: [TaigiWord], casePattern: CasePattern) -> [Autocomplete.Suggestion] {
        let suggestions = words.compactMap { word -> Autocomplete.Suggestion? in
            // 根據使用者輸入的大小寫模式調整候選詞
            let romanText = applyCaseTransform(to: word.roman, pattern: casePattern)
            let hanziText = word.hanzi ?? ""

            guard !romanText.isEmpty else { return nil }

            return Autocomplete.Suggestion(
                text: romanText,
                title: romanText,
                subtitle: hanziText.isEmpty ? nil : hanziText,
                additionalInfo: ["displayText": word.displayText]
            )
        }

        // 當 showHanjiMode = false 時，對羅馬字進行去重
        if !SharedSettings.shared.showHanjiMode {
            return deduplicateRomanSuggestions(suggestions)
        }

        return suggestions
    }

    /// 對候選詞按羅馬字進行去重，保留第一個出現的
    /// 注意：第 0 個候選詞（組字文字）會被自動保留，因為它永遠是第一個
    /// - Parameter suggestions: 原始候選詞列表
    /// - Returns: 去重後的候選詞列表
    private func deduplicateRomanSuggestions(_ suggestions: [Autocomplete.Suggestion]) -> [Autocomplete.Suggestion] {
        var seenRoman = Set<String>()
        var result: [Autocomplete.Suggestion] = []

        for suggestion in suggestions {
            let romanText = suggestion.title
            if !seenRoman.contains(romanText) {
                seenRoman.insert(romanText)
                result.append(suggestion)
            }
        }

        return result
    }

    /// 根據大小寫模式轉換文字
    /// - Parameters:
    ///   - text: 原始文字
    ///   - pattern: 大小寫模式
    /// - Returns: 轉換後的文字
    private func applyCaseTransform(to text: String, pattern: CasePattern) -> String {
        switch pattern {
        case .lowercase:
            // 全小寫
            return text.map { char in
                if ("A"..."Z").contains(char) {
                    return char.lowercased()
                }
                return String(char)
            }.joined()

        case .capitalized:
            // 首字母大寫
            var result = ""
            var isFirstLetter = true
            for char in text {
                if ("a"..."z").contains(char) || ("A"..."Z").contains(char) {
                    if isFirstLetter {
                        result += char.uppercased()
                        isFirstLetter = false
                    } else {
                        result += char.lowercased()
                    }
                } else {
                    result += String(char)
                }
            }
            return result

        case .uppercase:
            // 全大寫
            return text.map { char in
                if ("a"..."z").contains(char) {
                    return char.uppercased()
                }
                return String(char)
            }.joined()
        }
    }
}
