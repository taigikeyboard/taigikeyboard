import Foundation
import OSLog

/// 詞典服務
/// 提供台語詞彙搜尋功能
class LexiconService: @unchecked Sendable {

    // MARK: - Properties

    static let shared = LexiconService()

    private let repository: DictionaryRepository
    private let userFrequencyService: UserFrequencyService
    private let trieService: TrieService
    private let logger = Logger(
        subsystem: LexiconConstants.Logging.subsystem,
        category: "LexiconService"
    )

    // MARK: - Initialization

    init(
        repository: DictionaryRepository = .shared,
        userFrequencyService: UserFrequencyService = .shared,
        trieService: TrieService = .shared
    ) {
        self.repository = repository
        self.userFrequencyService = userFrequencyService
        self.trieService = trieService

        // 初始化 Trie
        initializeTrie()
    }

    // MARK: - Private Methods

    /// 初始化 Trie（背景執行）
    private func initializeTrie() {
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            let success = self?.trieService.initialize() ?? false
            if success {
                self?.logger.info("[INIT] Trie initialized successfully")
            } else {
                self?.logger.warning("[INIT] Trie initialization failed, using fallback")
            }
        }
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
            if let hanzi = word.hanzi, CandidateProcessor.startsWithRomanLetter(hanzi) {
                processedHanzi = CandidateProcessor.capitalize(hanzi, basedOn: input)
            } else {
                processedHanzi = word.hanzi
            }

            return TaigiWord(
                id: word.id,
                roman: CandidateProcessor.capitalize(word.roman, basedOn: input),
                hanzi: processedHanzi,
                lengthScore: word.lengthScore
            )
        }

        // 去重
        let uniqueWords = CandidateProcessor.removeDuplicates(processedWords)

        // 收集頻率資料並排序
        guard userFrequencyService.isConnected() else {
            return uniqueWords
        }

        // 批次查詢使用者頻率資料（包含 count 和 lastUsed）
        let wordTexts = uniqueWords.compactMap(\.displayText)
        let frequencyDataMap = userFrequencyService.frequencyDataBatch(for: wordTexts)

        // 正規化輸入用於完全匹配判斷（包含調符或 POJ 特殊字符時需要轉換）
        let normalizedInput = InputNormalizer.normalize(input, mode: inputMode)

        let sortedWords = CandidateProcessor.sortByScore(
            uniqueWords,
            normalizedInput: normalizedInput,
            frequencyDataMap: frequencyDataMap
        )
        return sortedWords
    }

    // MARK: - Connection Status

    func isConnected() -> Bool {
        repository.isConnected()
    }
}
