import Foundation
import KeyboardKit

/// 自動完成服務
///
/// 處理台語羅馬字與漢字的候選詞搜尋，支援多種輸入類型。
///
/// `autocomplete(_:)` 以 orchestrator 形式串接 4 個命名階段：
/// `classifyInput` → `searchLexicon` → `applyContextBoost` → `buildSuggestions`。
class AutocompleteService: KeyboardKit.AutocompleteService {
    // MARK: - KeyboardKit 協議屬性

    var locale: Locale = .current

    // MARK: - KeyboardKit Learning Stubs (protocol requirement, unused by Taigi)

    // KeyboardKit's AutocompleteService protocol mandates these members without defaults.
    // Taigi keyboard does not surface ignore/learn UX, so all implementations are no-ops.

    var canIgnoreWords: Bool {
        false
    }

    var canLearnWords: Bool {
        false
    }

    var ignoredWords: [String] = []
    var learnedWords: [String] = []

    func hasIgnoredWord(_: String) -> Bool {
        false
    }

    func hasLearnedWord(_: String) -> Bool {
        false
    }

    func ignoreWord(_: String) {}
    func learnWord(_: String) {}
    func removeIgnoredWord(_: String) {}
    func unlearnWord(_: String) {}

    // MARK: - 核心屬性

    private let lexiconService: LexiconService
    private let settingsProvider: EngineSettingsProvider
    private let nextWordService: NextWordService

    /// Composing state provider (decoupled from ComposingManager)
    private weak var composingState: (any ComposingStateProvider)?

    /// Selection context provider (decoupled from ActionHandler)
    private weak var selectionContext: (any SelectionContextProvider)?

    let logger = DebugLogger(category: "AutocompleteService")

    // MARK: - Initialization

    init(
        lexiconService: LexiconService = .shared,
        settingsProvider: EngineSettingsProvider = SharedSettings.shared,
        nextWordService: NextWordService = .shared,
    ) {
        self.lexiconService = lexiconService
        self.settingsProvider = settingsProvider
        self.nextWordService = nextWordService
    }

    // MARK: - 公開介面

    func setComposingManager(_ provider: any ComposingStateProvider) {
        composingState = provider
    }

    func setSelectionContextProvider(_ provider: any SelectionContextProvider) {
        selectionContext = provider
    }

    /// 自動完成核心方法
    /// 根據輸入文字搜尋台語候選詞
    /// 第 0 個候選詞永遠是當前的組字文字，第 1 個位置開始才是建議的候選詞
    func autocomplete(_ text: String) async throws -> Autocomplete.Result {
        guard !text.isEmpty, let composing = activeComposingContext() else {
            return Autocomplete.Result(inputText: text, suggestions: [])
        }

        logger.debug("[AUTOCOMPLETE] rawInput='\(composing.rawInput)' display='\(composing.displayText)'")

        do {
            let classification = AutocompleteInputClassifier.classify(rawInput: composing.rawInput)
            let words = try await searchLexicon(using: classification, rawInput: composing.rawInput)
            let boosted = await applyContextBoost(words: words)
            let suggestions = buildSuggestions(from: boosted, composingText: composing.displayText)
            return Autocomplete.Result(inputText: text, suggestions: suggestions)
        } catch {
            logger.error("[AUTOCOMPLETE] failed for text '\(text)': \(error.localizedDescription)")
            return Autocomplete.Result(inputText: text, suggestions: [])
        }
    }

    // MARK: - Orchestration phases

    /// 當前組字上下文；沒有組字狀態時回傳 nil，呼叫端即不顯示候選詞。
    private func activeComposingContext() -> (rawInput: String, displayText: String)? {
        guard let composingState,
              composingState.isComposing,
              !composingState.rawInput.isEmpty
        else {
            return nil
        }
        return (composingState.rawInput, composingState.composingText)
    }

    /// 使用分類結果向 LexiconService 查詢候選詞。
    private func searchLexicon(
        using classification: AutocompleteInputClassifier.Classification,
        rawInput: String,
    ) async throws -> [TaigiWord] {
        try await lexiconService.search(
            for: classification.searchKey,
            inputType: classification.inputType,
            inputMode: settingsProvider.current.inputMode,
            rawInput: rawInput,
        )
    }

    /// 依 `lastSelectedWord` 的 bigram 預測，將符合預測首字的候選詞拉到前面。
    private func applyContextBoost(words: [TaigiWord]) async -> [TaigiWord] {
        guard let lastWord = selectionContext?.lastSelectedWord,
              !lastWord.isEmpty
        else {
            return words
        }

        let predictions = await nextWordService.predict(word: lastWord, limit: 30)
        guard !predictions.isEmpty else { return words }

        let contextSet = Set(predictions.map(\.hanzi))
        return AutocompleteContextBooster.boost(words: words, predictedFirstChars: contextSet)
    }

    /// 組合 position-0 組字文字 + 查詢結果為 KeyboardKit 候選詞列表。
    private func buildSuggestions(
        from words: [TaigiWord],
        composingText: String,
    ) -> [Autocomplete.Suggestion] {
        var suggestions = convertToSuggestions(words)
        suggestions.insert(createComposingTextSuggestion(composingText), at: 0)
        return suggestions
    }

    // MARK: - Suggestion construction

    /// 建立 position-0 的組字文字候選詞，顯示使用者目前正在輸入的內容。
    private func createComposingTextSuggestion(_ composingText: String) -> Autocomplete.Suggestion {
        Autocomplete.Suggestion(
            text: composingText,
            title: composingText,
            subtitle: nil,
            additionalInfo: ["isComposingText": "true"],
        )
    }

    /// 把詞典回傳的 TaigiWord 轉為 KeyboardKit 候選詞。
    ///
    /// 不做大小寫轉換，保持詞典原始格式（小寫）；大小寫由 `SuggestionCaseTransformer` 在 View 層處理。
    private func convertToSuggestions(_ words: [TaigiWord]) -> [Autocomplete.Suggestion] {
        words.compactMap { word -> Autocomplete.Suggestion? in
            let romanText = word.roman
            guard !romanText.isEmpty else { return nil }

            let hanziText = word.hanzi ?? ""
            return Autocomplete.Suggestion(
                text: romanText,
                title: romanText,
                subtitle: hanziText.isEmpty ? nil : hanziText,
                additionalInfo: ["displayText": word.displayText],
            )
        }
    }
}
