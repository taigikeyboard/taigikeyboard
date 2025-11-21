import Foundation
import OSLog

/// 詞典服務
/// 提供台語詞彙搜尋功能
class LexiconService: @unchecked Sendable {

    // MARK: - Properties

    static let shared = LexiconService()

    private let repository: DictionaryRepository
    private let userFrequencyService: UserFrequencyService
    private let logger = Logger(
        subsystem: LexiconConstants.Logging.subsystem,
        category: "LexiconService"
    )

    // MARK: - Initialization

    init(
        repository: DictionaryRepository = .shared,
        userFrequencyService: UserFrequencyService = .shared
    ) {
        self.repository = repository
        self.userFrequencyService = userFrequencyService
    }

    // MARK: - Public API

    /// 搜尋詞彙
    func search(
        for input: String,
        inputType: InputType,
        inputMode: InputMode = .poj,
        limit: Int = LexiconConstants.Search.defaultLimit
    ) async throws -> [TaigiWord] {
        guard !input.isEmpty else {
            return []
        }

        // 從 repository 查詢
        let words = try await repository.query(
            for: input,
            inputType: inputType,
            inputMode: inputMode,
            limit: limit
        )

        // 處理文字大小寫
        let processedWords = words.map { word in
            let processedHanzi: String?
            if let hanzi = word.hanzi, TextProcessor.startsWithRomanLetter(hanzi) {
                processedHanzi = TextProcessor.capitalize(hanzi, basedOn: input)
            } else {
                processedHanzi = word.hanzi
            }

            return TaigiWord(
                id: word.id,
                roman: TextProcessor.capitalize(word.roman, basedOn: input),
                hanzi: processedHanzi,
                lengthScore: word.lengthScore
            )
        }

        // 去重
        let uniqueWords = TextProcessor.removeDuplicates(processedWords)

        // 收集頻率並排序
        guard userFrequencyService.isConnected() else {
            return uniqueWords
        }

        let frequencies = TextProcessor.collectFrequencies(
            for: uniqueWords,
            using: { [weak userFrequencyService] word in
                userFrequencyService?.getFrequency(for: word) ?? 0
            }
        )

        let sortedWords = TextProcessor.sortByFrequency(uniqueWords, frequencies: frequencies)
        return sortedWords
    }

    // MARK: - Connection Status

    func isConnected() -> Bool {
        repository.isConnected()
    }
}
