import Foundation
import KeyboardKit
import OSLog

/// 自動完成服務
///
/// 處理台語羅馬字與漢字的候選詞搜尋，支援多種輸入類型。
class AutocompleteService: KeyboardKit.AutocompleteService {

    // MARK: - KeyboardKit 協議屬性
    var locale: Locale = .current

    // MARK: - 學習功能屬性（未使用）
    /// 是否支援忽略詞彙功能（台語鍵盤不使用此功能）
    var canIgnoreWords: Bool { false }

    /// 是否支援學習詞彙功能（台語鍵盤不使用此功能）
    var canLearnWords: Bool { false }

    /// 忽略的詞彙列表（台語鍵盤不使用此功能）
    var ignoredWords: [String] = []

    /// 學習的詞彙列表（台語鍵盤不使用此功能）
    var learnedWords: [String] = []

    // MARK: - 學習功能方法（未實作）
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

    // MARK: - 核心屬性
    /// 詞典搜尋服務
    private let lexiconService = LexiconService.shared

    /// 共用設定管理器
    private let settings = SharedSettings.shared

    /// 組字管理器的弱引用
    private weak var composingManager: ComposingManager?

    /// ActionHandler 的弱引用（用於取得 lastSelectedWord 進行上下文提升）
    private weak var actionHandler: ActionHandler?

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

    /// 設定 ActionHandler（用於存取 lastSelectedWord 進行上下文候選詞提升）
    /// - Parameter handler: ActionHandler 實例
    func setActionHandler(_ handler: ActionHandler) {
        actionHandler = handler
    }

    /// 自動完成核心方法
    /// 根據輸入文字搜尋台語候選詞
    /// 第 0 個候選詞永遠是當前的組字文字，第 1 個位置開始才是建議的候選詞
    /// - Parameter text: 輸入文字
    /// - Returns: 候選詞搜尋結果
    func autocomplete(_ text: String) async throws -> Autocomplete.Result {
        guard !text.isEmpty else {
            return Autocomplete.Result(inputText: text, suggestions: [])
        }

        do {
            // 取得 rawInput（搜尋用）和 composingText（顯示用）
            guard let composingManager,
                  composingManager.isComposing,
                  !composingManager.rawInput.isEmpty
            else {
                // 沒有組字狀態時不顯示候選詞
                return Autocomplete.Result(inputText: text, suggestions: [])
            }

            let rawInput = composingManager.rawInput           // 搜尋用（如 gua2）
            let displayText = composingManager.composingText   // 顯示用（如 guá）

            logger.debug("[AUTOCOMPLETE] rawInput='\(rawInput, privacy: .public)' display='\(displayText, privacy: .public)'")

            let inputMode = settings.inputMode
            // 使用 rawInput 判斷（因為 displayText 可能已移除聲調數字，如 soo1 → soo）
            let inputType = determineInputType(rawInput)

            // Segment continuous input and normalize for Trie search
            let searchInput = buildSearchKey(from: rawInput)
            let words = try await lexiconService.search(for: searchInput, inputType: inputType, inputMode: inputMode, limit: 100, rawInput: rawInput)

            // TPS ㄜ expansion: also search "or" variant when toggle ON
            var allWords = words
            if TPSConverter.containsTPS(rawInput),
               settings.tpsOrMapsToER,
               searchInput.contains("er") {
                let orVariantKey = searchInput.replacingOccurrences(of: "er", with: "or")
                let orWords = try await lexiconService.search(
                    for: orVariantKey, inputType: inputType,
                    inputMode: inputMode, limit: 100, rawInput: rawInput
                )
                let existingIds = Set(allWords.map { $0.id })
                allWords += orWords.filter { !existingIds.contains($0.id) }
            }

            // Apply context boost: promote candidates matching bigram predictions from lastSelectedWord
            let contextBoostedWords = await applyContextBoost(words: allWords)

            // 將詞彙轉換為候選詞（不做大小寫轉換，由 SuggestionCaseTransformer 在 View 層處理）
            var suggestions = convertToSuggestions(contextBoostedWords)

            // 在第 0 個位置插入當前組字文字候選詞（使用顯示文字）
            let composingTextSuggestion = createComposingTextSuggestion(displayText)
            suggestions.insert(composingTextSuggestion, at: 0)

            let result = Autocomplete.Result(inputText: text, suggestions: suggestions)
            return result
        } catch {
            logger.error("[AUTOCOMPLETE] failed for text '\(text, privacy: .public)': \(error.localizedDescription, privacy: .public)")
            return Autocomplete.Result(inputText: text, suggestions: [])
        }
    }

    // MARK: - 私有方法

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

    /// 判斷輸入文字的類型
    /// - Parameter text: 輸入文字
    /// - Returns: 輸入類型（漢字、帶聲調羅馬字、無聲調羅馬字）
    private func determineInputType(_ text: String) -> InputType {
        if CandidateProcessor.isHanzi(text) {
            return .hanzi
        }

        if InputNormalizer.hasToneMarks(text) {
            return .romanWithTone
        }

        // 檢查數字聲調（如 gua2, soo1）
        if containsNumericTone(text) {
            return .romanWithTone
        }

        return .romanWithoutTone
    }

    /// 檢查文字是否包含數字聲調（2, 3, 5, 6, 7, 8, 9）
    /// 排除 1, 4, 0：1 和 4 是無調號聲調，0 是無效輸入
    /// - Parameter text: 待檢查的文字
    /// - Returns: 是否包含數字聲調
    private func containsNumericTone(_ text: String) -> Bool {
        text.contains { char in
            char.isNumber && char != "1" && char != "4" && char != "0"
        }
    }

    /// Apply context boost by promoting candidates whose displayText starts with a bigram-predicted character.
    ///
    /// Uses `lastSelectedWord` from ActionHandler to query NextWordService for bigram predictions,
    /// then partitions candidates: context-matched first, then the rest (preserving original order within each group).
    private func applyContextBoost(words: [TaigiWord]) async -> [TaigiWord] {
        guard let lastWord = actionHandler?.lastSelectedWord,
              !lastWord.isEmpty else {
            return words
        }

        let predictions = await NextWordService.shared.predict(word: lastWord, limit: 30)
        guard !predictions.isEmpty else { return words }

        // Build context set: predicted hanzi characters
        let contextSet = Set(predictions.map(\.hanzi))

        // Partition: candidates whose displayText starts with a predicted character go first
        var boosted: [TaigiWord] = []
        var rest: [TaigiWord] = []

        for word in words {
            let display = word.displayText
            if let firstChar = display.first, contextSet.contains(String(firstChar)) {
                boosted.append(word)
            } else {
                rest.append(word)
            }
        }

        return boosted + rest
    }

    /// Build a Trie-compatible search key from continuous raw input
    ///
    /// Segments continuous input into syllables and joins with hyphens
    /// so the existing InputNormalizer + Trie pipeline handles it correctly.
    ///
    /// - Toned input (has digits): non-final toneless syllables get a default
    ///   tone (1 or 4) so the joined key matches tl_num entries in the Trie.
    /// - Toneless input (no digits): no default tones added, so the joined key
    ///   matches notone entries in the Trie for broader results.
    private func buildSearchKey(from rawInput: String) -> String {
        // Convert TPS to TL before segmentation so the segmenter receives romanization
        let processedInput = TPSConverter.containsTPS(rawInput)
            ? TPSConverter.toTL(rawInput)
            : rawInput

        let prefix = LexiconConstants.TriePrefix.prefix(for: settings.inputMode)
        let checker: SyllableSegmenter.WordPrefixChecker = { key in
            !TrieService.shared.prefixSearch(prefix + key, limit: 1).isEmpty
        }
        let segments = SyllableSegmenter.segment(processedInput, wordPrefixChecker: checker, mode: settings.inputMode)
        guard segments.count > 1 else { return processedInput }

        let hasTones = processedInput.contains { $0.isNumber }

        let processed = segments.enumerated().map { (index, seg) -> String in
            let base = seg.hasSuffix("-") ? String(seg.dropLast()) : seg
            guard !base.isEmpty else { return "" }

            let isLast = index == segments.count - 1

            // Only add default tones when input already contains tone digits.
            // For toneless input, skip to match notone keys in the Trie.
            if hasTones, !isLast, let lastChar = base.last, !lastChar.isNumber {
                return base + (TaigiPhonetics.isStopTone(base) ? "4" : "1")
            }

            return base
        }

        return processed.joined(separator: "-")
    }

    /// 將台語詞彙轉換為 KeyboardKit 候選詞格式
    ///
    /// 不做大小寫轉換，保持詞典原始格式（小寫）。
    /// 大小寫轉換由 SuggestionCaseTransformer 在 View 層處理。
    ///
    /// - Parameter words: 台語詞彙列表
    /// - Returns: KeyboardKit 候選詞列表
    private func convertToSuggestions(_ words: [TaigiWord]) -> [Autocomplete.Suggestion] {
        let suggestions = words.compactMap { word -> Autocomplete.Suggestion? in
            let romanText = word.roman
            let hanziText = word.hanzi ?? ""

            guard !romanText.isEmpty else { return nil }

            return Autocomplete.Suggestion(
                text: romanText,
                title: romanText,
                subtitle: hanziText.isEmpty ? nil : hanziText,
                additionalInfo: ["displayText": word.displayText]
            )
        }

        return suggestions
    }
}
