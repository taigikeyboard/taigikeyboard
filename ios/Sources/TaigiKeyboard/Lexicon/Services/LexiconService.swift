import Foundation

/// 詞典服務
/// 提供台語詞彙搜尋功能
final class LexiconService: @unchecked Sendable {
    // MARK: - Properties

    static let shared = LexiconService()

    private let repository: DictionaryRepository
    private let userFrequencyService: UserFrequencyService
    private let trieService: TrieService
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
        self.userFrequencyService = userFrequencyService
        self.customDictionaryRepository = customDictionaryRepository

        self.repository = repository ?? DictionaryRepository(
            trieService: trieService,
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

            let success = trieService.initialize()
            if success {
                logger.info("[INIT] Dictionary trie initialized successfully")
            } else {
                logger.warning("[INIT] Dictionary trie initialization failed")
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
    ///
    /// Orchestrates four search phases:
    /// 1. Custom dictionary lookup (user-added entries, highest priority)
    /// 2. System dictionary query (with optional TPS `er`↔`or` expansion)
    /// 3. Merge + dedup + case processing
    /// 4. Rank by user frequency (if available)
    ///
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
        guard !input.isEmpty else { return [] }

        let customWords = lookupCustomDictionary(rawInput: rawInput, segmentedInput: input)

        let systemWords = try await querySystemDictionaries(
            segmentedInput: input,
            inputType: inputType,
            inputMode: inputMode,
            limit: limit,
            rawInput: rawInput,
        )

        let processedSystem = applyCaseProcessing(systemWords, basedOn: input)
        let uniqueWords = CandidateProcessor.removeDuplicates(customWords + processedSystem)

        let ranked = await rankByFrequency(uniqueWords, segmentedInput: input, inputMode: inputMode)

        // TPS mode: remove visual duplicates (same hanzi, different roman)
        if inputMode == .tps {
            return CandidateProcessor.removeDisplayDuplicates(ranked)
        }
        return ranked
    }

    // MARK: - Connection Status

    func isConnected() -> Bool {
        repository.isConnected()
    }

    // MARK: - Search Pipeline

    /// Look up user-added custom dictionary entries by unsegmented input.
    ///
    /// Tone-aware inputs (contain a digit) match the `roman_num` column; toneless
    /// inputs match the `notone` column. Returns `[]` when the feature is disabled.
    private func lookupCustomDictionary(
        rawInput: String?,
        segmentedInput: String,
    ) -> [TaigiWord] {
        guard SharedSettings.shared.isCustomDictEnabled else { return [] }

        let customSearchKey = rawInput ?? segmentedInput
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
        logger.debug("[SEARCH] customDict key='\(customSearchKey)' prefix='\(searchPrefix)' toneAware=\(isToneAware) segmented='\(segmentedInput)' results=\(customEntries.count)")

        return customEntries.map { entry in
            let processedRoman = CandidateProcessor.capitalize(entry.roman, basedOn: segmentedInput)
            let processedHanzi: String? = if CandidateProcessor.startsWithRomanLetter(entry.hanzi) {
                CandidateProcessor.capitalize(entry.hanzi, basedOn: segmentedInput)
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
    }

    /// Query system dictionaries for the segmented input, with optional TPS
    /// `er`↔`or` variant expansion when the user has that toggle on.
    private func querySystemDictionaries(
        segmentedInput: String,
        inputType: InputType,
        inputMode: InputMode,
        limit: Int,
        rawInput: String?,
    ) async throws -> [TaigiWord] {
        var systemWords = try await repository.query(
            for: segmentedInput,
            inputType: inputType,
            inputMode: inputMode,
            limit: limit,
        )

        // TPS ㄜ expansion: also search "or" variant when toggle ON (matching Android)
        if let raw = rawInput, TPSConverter.containsTPS(raw),
           SharedSettings.shared.isTpsOrMappedToER,
           segmentedInput.contains("er")
        {
            let orVariantKey = segmentedInput.replacingOccurrences(of: "er", with: "or")
            let orWords = try await repository.query(
                for: orVariantKey,
                inputType: inputType,
                inputMode: inputMode,
                limit: limit,
            )
            let existingIds = Set(systemWords.map(\.id))
            systemWords += orWords.filter { !existingIds.contains($0.id) }
        }

        return systemWords
    }

    /// Apply auto-capitalization to roman and hanzi forms based on the input shape.
    private func applyCaseProcessing(
        _ words: [TaigiWord],
        basedOn input: String,
    ) -> [TaigiWord] {
        words.map { word in
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
    }

    /// Rank candidates by user frequency. Lazily initialises the frequency DB
    /// on first use; returns `words` unchanged when the DB is not available.
    private func rankByFrequency(
        _ words: [TaigiWord],
        segmentedInput: String,
        inputMode: InputMode,
    ) async -> [TaigiWord] {
        // Ensure user frequency DB is initialized (lazy: first search triggers connection)
        if !userFrequencyService.isConnected() {
            try? await UserFrequencyRepository.shared.ensureInitialized()
        }
        guard userFrequencyService.isConnected() else { return words }

        let wordTexts = words.compactMap(\.displayText)
        let frequencyDataMap = userFrequencyService.frequencyDataBatch(for: wordTexts)

        // 正規化輸入用於完全匹配判斷（包含調符或 POJ 特殊字符時需要轉換）
        let normalizedInput = InputNormalizer.normalize(segmentedInput, mode: inputMode)

        return CandidateProcessor.sortByScore(
            words,
            normalizedInput: normalizedInput,
            frequencyDataMap: frequencyDataMap,
        )
    }
}
