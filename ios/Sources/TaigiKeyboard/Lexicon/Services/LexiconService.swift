import Foundation

/// 詞典服務
/// 提供台語詞彙搜尋功能
class LexiconService: @unchecked Sendable {
    // MARK: - Properties

    static let shared = LexiconService()

    private let repository: DictionaryRepository
    private let userFrequencyService: UserFrequencyService
    private let trieService: TrieService
    private let hanziTrieService: TrieService
    private let customDictionaryRepository: CustomDictionaryRepository
    private let logger = DebugLogger(category: "LexiconService")

    // MARK: - Initialization

    init(
        repository: DictionaryRepository? = nil,
        userFrequencyService: UserFrequencyService = .shared,
        trieService: TrieService = .shared,
        customDictionaryRepository: CustomDictionaryRepository = .shared,
    ) {
        self.trieService = trieService
        self.hanziTrieService = TrieService(
            fileName: "hanzi",
            fileExtension: "trie",
            logCategory: "HanziTrieService",
        )
        self.userFrequencyService = userFrequencyService
        self.customDictionaryRepository = customDictionaryRepository

        // DictionaryRepository 使用 binary reader + hanzi trie
        self.repository = repository ?? DictionaryRepository(
            trieService: trieService,
            hanziTrieService: hanziTrieService,
        )

        // 初始化 Trie
        initializeTrie()
        // 初始化 Custom Dictionary（keyboard extension 需要提前初始化）
        initializeCustomDictionary()
    }

    // MARK: - Private Methods

    /// 初始化 Trie（背景執行）
    private func initializeTrie() {
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            guard let self else { return }

            let dictSuccess = self.trieService.initialize()
            if dictSuccess {
                self.logger.info("[INIT] Dictionary trie initialized successfully")
            } else {
                self.logger.warning("[INIT] Dictionary trie initialization failed")
            }

            let hanziSuccess = self.hanziTrieService.initialize()
            if hanziSuccess {
                self.logger.info("[INIT] Hanzi trie initialized successfully")
            } else {
                self.logger.warning("[INIT] Hanzi trie initialization failed")
            }
        }
    }

    /// 初始化 Custom Dictionary DB（背景執行）
    /// searchSync doesn't call ensureInitialized, so we must initialize eagerly
    private func initializeCustomDictionary() {
        guard SharedSettings.shared.isCustomDictEnabled else { return }
        Task {
            do {
                try await customDictionaryRepository.ensureInitialized()
                logger.info("[INIT] Custom dictionary initialized successfully")
            } catch {
                logger.warning("[INIT] Custom dictionary initialization failed: \(error.localizedDescription)")
            }
        }
    }

    // MARK: - Public API

    /// 搜尋詞彙
    /// - Parameters:
    ///   - input: Segmented search key for system dictionary (e.g. "li-ho")
    ///   - rawInput: Unsegmented input for custom dictionary (e.g. "liho"). Falls back to `input` if nil.
    func search(
        for input: String,
        inputType: InputType,
        inputMode: InputMode = .poj,
        limit: Int = LexiconConstants.Search.defaultLimit,
        rawInput: String? = nil,
    ) async throws -> [TaigiWord] {
        guard !input.isEmpty else {
            return []
        }

        // Query custom dictionary by unsegmented input (highest priority)
        // Tone-aware: match roman_num column; toneless: match notone column
        let customWords: [TaigiWord]
        if SharedSettings.shared.isCustomDictEnabled {
            let customSearchKey = rawInput ?? input
            let isToneAware = customSearchKey.contains { $0.isNumber }
            let searchPrefix = isToneAware
                ? customSearchKey.lowercased()
                .replacingOccurrences(of: "-", with: "")
                .replacingOccurrences(of: " ", with: "")
                : CustomDictionaryService.generateNotone(customSearchKey)
            let customEntries = customDictionaryRepository.searchSync(
                prefix: searchPrefix,
                isToneAware: isToneAware,
                limit: 20,
            )
            logger.debug("[SEARCH] customDict key='\(customSearchKey)' prefix='\(searchPrefix)' toneAware=\(isToneAware) segmented='\(input)' results=\(customEntries.count)")
            customWords = customEntries.map { entry in
                let processedRoman = CandidateProcessor.capitalize(entry.roman, basedOn: input)
                let processedHanzi: String? = if CandidateProcessor.startsWithRomanLetter(entry.hanzi) {
                    CandidateProcessor.capitalize(entry.hanzi, basedOn: input)
                } else {
                    entry.hanzi
                }
                return TaigiWord(
                    id: -2, // Custom dictionary marker
                    roman: processedRoman,
                    hanzi: processedHanzi,
                    lengthScore: nil,
                )
            }
        } else {
            customWords = []
        }

        // Query system dictionaries
        let words = try await repository.query(
            for: input,
            inputType: inputType,
            inputMode: inputMode,
            limit: limit,
        )

        // Process case for system results
        let processedWords = words.map { word in
            let processedHanzi: String? = if let hanzi = word.hanzi, CandidateProcessor.startsWithRomanLetter(hanzi) {
                CandidateProcessor.capitalize(hanzi, basedOn: input)
            } else {
                word.hanzi
            }

            return TaigiWord(
                id: word.id,
                roman: CandidateProcessor.capitalize(word.roman, basedOn: input),
                hanzi: processedHanzi,
                lengthScore: word.lengthScore,
            )
        }

        // Merge: custom words first, then system words
        let mergedWords = customWords + processedWords

        // Deduplicate
        let uniqueWords = CandidateProcessor.removeDuplicates(mergedWords)

        // Ensure user frequency DB is initialized (lazy: first search triggers connection)
        if !userFrequencyService.isConnected() {
            try? await UserFrequencyRepository.shared.ensureInitialized()
        }

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
            frequencyDataMap: frequencyDataMap,
        )

        // TPS mode: remove visual duplicates (same hanzi, different roman)
        if inputMode == .tps {
            return CandidateProcessor.removeDisplayDuplicates(sortedWords)
        }
        return sortedWords
    }

    // MARK: - Connection Status

    func isConnected() -> Bool {
        repository.isConnected()
    }
}
