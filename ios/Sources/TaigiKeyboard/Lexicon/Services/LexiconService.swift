import Foundation

/// 詞典服務
/// 提供台語詞彙搜尋功能
final class LexiconService: @unchecked Sendable {
    // MARK: - Properties

    private let repository: DictionaryRepository
    private let userFrequencyService: UserFrequencyService
    private let trieService: TrieService
    private let customDictionaryRepository: CustomDictionaryRepository
    private let settingsProvider: EngineSettingsProvider
    private let logger = DebugLogger(category: "LexiconService")

    // MARK: - Initialization

    init(
        repository: DictionaryRepository? = nil,
        userFrequencyService: UserFrequencyService = CompositionRoot.userFrequencyService,
        trieService: TrieService = CompositionRoot.trieService,
        customDictionaryRepository: CustomDictionaryRepository = CompositionRoot.customDictionaryRepository,
        settingsProvider: EngineSettingsProvider = SharedSettings.shared,
    ) {
        self.trieService = trieService
        self.userFrequencyService = userFrequencyService
        self.customDictionaryRepository = customDictionaryRepository
        self.settingsProvider = settingsProvider

        self.repository = repository ?? DictionaryRepository(
            trieService: trieService,
            settingsProvider: settingsProvider,
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
        guard settingsProvider.current.isCustomDictEnabled else { return }
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
    /// 3. Case processing on the merged list
    /// 4. Rank by user frequency through `RustEngineBridge.processCandidates`
    ///    (dedup + score + sort + optional TPS display-dedup, atomic in
    ///    Rust shared core); cold-start before the freq DB warms up falls
    ///    back to platform dedup helpers and skips the score-sort.
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

        let customWords = lookupCustomDictionary(rawInput: rawInput, segmentedInput: input, inputMode: inputMode)

        let systemWords = try await querySystemDictionaries(
            segmentedInput: input,
            inputType: inputType,
            inputMode: inputMode,
            limit: limit,
            rawInput: rawInput,
        )

        let processedSystem = applyCaseProcessing(systemWords, basedOn: input, inputMode: inputMode)
        let merged = customWords + processedSystem

        return await processCandidates(merged, segmentedInput: input, inputMode: inputMode)
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
        inputMode: InputMode,
    ) -> [TaigiWord] {
        guard settingsProvider.current.isCustomDictEnabled else { return [] }

        let isAutoCap = settingsProvider.current.isAutoCap
        let customSearchKey = rawInput ?? segmentedInput
        let (searchPrefix, isToneAware) = CustomDictionaryDerivation.searchPrefix(for: customSearchKey)

        let customEntries = customDictionaryRepository.searchSync(
            prefix: searchPrefix,
            isToneAware: isToneAware,
            limit: 20,
        )
        logger.debug("[SEARCH] customDict key='\(customSearchKey)' prefix='\(searchPrefix)' toneAware=\(isToneAware) segmented='\(segmentedInput)' results=\(customEntries.count)")

        return customEntries.map { entry in
            let processedRoman = CandidateProcessor.capitalize(entry.roman, basedOn: segmentedInput, inputMode: inputMode, isAutoCap: isAutoCap)
            let processedHanzi: String? = if CandidateProcessor.startsWithRomanLetter(entry.hanzi) {
                CandidateProcessor.capitalize(entry.hanzi, basedOn: segmentedInput, inputMode: inputMode, isAutoCap: isAutoCap)
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
        if let raw = rawInput, RustEngineBridge.containsTPS(raw),
           settingsProvider.current.isTpsOrMappedToER,
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
        inputMode: InputMode,
    ) -> [TaigiWord] {
        let isAutoCap = settingsProvider.current.isAutoCap
        return words.map { word in
            let processedHanzi: String? = if let hanzi = word.hanzi, CandidateProcessor.startsWithRomanLetter(hanzi) {
                CandidateProcessor.capitalize(hanzi, basedOn: input, inputMode: inputMode, isAutoCap: isAutoCap)
            } else {
                word.hanzi
            }
            return TaigiWord(
                id: word.id,
                roman: CandidateProcessor.capitalize(word.roman, basedOn: input, inputMode: inputMode, isAutoCap: isAutoCap),
                hanzi: processedHanzi,
                lengthScore: word.lengthScore,
                sourceBitmask: word.sourceBitmask,
            )
        }
    }

    /// Run the merged candidate list through the lexicon ranking pipeline.
    ///
    /// Connected path: hands the full pipeline (dedup → score → sort →
    /// optional TPS display-dedup) to the Rust shared core via
    /// `RustEngineBridge.processCandidates`. Atomic — no intermediate
    /// platform passes.
    ///
    /// Disconnected path (cold-start before the user-frequency DB is
    /// available): falls back to platform `CandidateProcessor.removeDuplicates`
    /// + optional TPS `removeDisplayDuplicates`. Skipping the score-sort
    /// preserves the legacy iOS "merged-order on cold-start" behavior so
    /// custom-dictionary entries continue to surface ahead of system
    /// candidates until the freq DB warms up. This is an **intentional
    /// exception** to the v3.5.2 ranking-slice rule that production
    /// routes through Rust — see `docs/engine/ranking-slice-audit.md` § 8
    /// row "iOS cold-start fallback". Android has no equivalent because
    /// its `UserFrequencyService.frequencyDataBatch` is always callable.
    private func processCandidates(
        _ merged: [TaigiWord],
        segmentedInput: String,
        inputMode: InputMode,
    ) async -> [TaigiWord] {
        if !userFrequencyService.isConnected() {
            try? await userFrequencyService.ensureInitialized()
        }

        let isTPS = inputMode == .tps
        guard userFrequencyService.isConnected() else {
            let uniqueWords = CandidateProcessor.removeDuplicates(merged)
            return isTPS ? CandidateProcessor.removeDisplayDuplicates(uniqueWords) : uniqueWords
        }

        let normalizedInput = InputNormalizer.normalize(segmentedInput, mode: inputMode)
        let displayKeys = merged.map(\.displayText)
        let frequencyDataMap = userFrequencyService.frequencyDataBatch(for: displayKeys)
        let nowMs = Int64(Date().timeIntervalSince1970 * 1000)

        return RustEngineBridge.processCandidates(
            raw: merged,
            normalizedInput: normalizedInput,
            tpsDedupEnabled: isTPS,
            frequencyData: frequencyDataMap,
            nowMs: nowMs,
        )
    }
}
