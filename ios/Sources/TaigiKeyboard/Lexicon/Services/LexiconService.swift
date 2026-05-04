import Foundation

/// 詞典服務
/// 提供台語詞彙搜尋功能
final class LexiconService: @unchecked Sendable {
    // MARK: - Properties

    private let userFrequencyService: UserFrequencyService
    private let customDictionaryRepository: CustomDictionaryRepository
    private let settingsProvider: EngineSettingsProvider
    private let logger = DebugLogger(category: "LexiconService")

    // MARK: - Initialization

    init(
        userFrequencyService: UserFrequencyService = CompositionRoot.userFrequencyService,
        customDictionaryRepository: CustomDictionaryRepository = CompositionRoot.customDictionaryRepository,
        settingsProvider: EngineSettingsProvider = SharedSettings.shared,
    ) {
        self.userFrequencyService = userFrequencyService
        self.customDictionaryRepository = customDictionaryRepository
        self.settingsProvider = settingsProvider

        // 初始化 Custom Dictionary（keyboard extension 需要提前初始化）
        initializeCustomDictionary()
        // Trie / dictionary.bin / association.bin lexicon engine state is
        // installed once at extension launch via `RustEngineBridge.lexiconInstall(...)`
        // (see `KeyboardViewController+Setup.swift`). No per-service init needed.
    }

    // MARK: - Private Methods

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
    /// 1. **D-8 hanzi guard** — `inputType == .hanzi` short-circuits to `[]`
    ///    BEFORE custom-dict and system-dict, mirroring Android. Pinned by
    ///    `INVARIANT_LEX_HANZI_GUARD` (parity correction toward Android per
    ///    `rules/cross-platform-alignment.md` §1b; `behavioral-invariants.md` §14).
    /// 2. Custom dictionary lookup (user-added entries, highest priority)
    /// 3. System dictionary query through `RustEngineBridge.lexiconSearch`
    ///    (Rust engine handles trie / binary readers / TPS er↔or expansion)
    /// 4. Case processing on the merged list
    /// 5. Rank by user frequency through `RustEngineBridge.processCandidates`
    ///    (dedup + score + sort + optional TPS display-dedup, atomic in
    ///    Rust shared core); cold-start before the freq DB warms up routes
    ///    through the same call with `mergeOrderOnly: true` to skip
    ///    score-sort while still running engine dedup.
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

        // D-8 hard guard: hanzi inputs short-circuit before any reader is touched.
        // See `behavioral-invariants.md` §14.
        guard inputType != .hanzi else { return [] }

        let customWords = lookupCustomDictionary(rawInput: rawInput, segmentedInput: input, inputMode: inputMode)
        let systemWords = querySystemDictionaries(
            segmentedInput: input,
            inputType: inputType,
            inputMode: inputMode,
            limit: limit,
        )

        let processedSystem = applyCaseProcessing(systemWords, basedOn: input, inputMode: inputMode)
        let merged = customWords + processedSystem

        return await processCandidates(merged, segmentedInput: input, inputMode: inputMode)
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

    /// Query system dictionaries through the Rust shared-core lexicon engine.
    ///
    /// TPS `er`↔`or` variant expansion runs INSIDE the engine when
    /// `tpsOrMappedToER` is set and the normalized key contains "er", per
    /// `engine/lexicon/src/search.rs`. iOS no longer runs the variant-search
    /// loop platform-side (audit D-1 + D-2 resolution).
    private func querySystemDictionaries(
        segmentedInput: String,
        inputType: InputType,
        inputMode: InputMode,
        limit: Int,
    ) -> [TaigiWord] {
        let bridgeInputType: RustEngineBridge.LexiconInputType = switch inputType {
        case .hanzi: .hanzi // unreachable due to D-8 guard, kept for completeness
        case .romanWithTone: .romanWithTone
        case .romanWithoutTone: .romanNoTone
        }
        // .english unreachable — keyboard passthrough never invokes lexicon
        // search; mirrors Android `InputMode.ENGLISH -> LexiconInputMode.TL`.
        let bridgeInputMode: RustEngineBridge.LexiconInputMode = switch inputMode {
        case .tl: .tl
        case .poj: .poj
        case .tps: .tps
        case .english: .tl
        }
        // Resolve the user's 12 dictionary toggles → ready-to-send bitmask
        // via the engine. Pre-v3.5.8 this branched on `allEnabled` and sent
        // `UInt32.max`, which forced variant + khiin on regardless of user
        // toggles (r3173440126); we now route through Rust per audit
        // residue § A.1.
        let toggles = RustEngineBridge.DictionaryToggles(from: settingsProvider.current)
        let bitmask = RustEngineBridge.lexiconDictionaryFilters(toggles: toggles).dictionaryFilterBitmask
        let rows = RustEngineBridge.lexiconSearch(
            input: segmentedInput,
            inputType: bridgeInputType,
            inputMode: bridgeInputMode,
            limit: UInt32(limit),
            tpsOrMappedToER: settingsProvider.current.isTpsOrMappedToER,
            enabledSourcesBitmask: bitmask,
        )
        return rows.map { row in
            // Engine returns raw `tl`; if the user is in POJ mode, render to POJ.
            let roman = inputMode == .poj ? RustEngineBridge.tlToPoj(row.roman) : row.roman
            return TaigiWord(
                id: Int(row.id),
                roman: roman,
                hanzi: row.hanzi,
                lengthScore: row.lengthScore.map(Int.init),
                sourceBitmask: row.sourceBitmask.map(UInt16.init(truncatingIfNeeded:)),
            )
        }
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
    /// available): routes through `RustEngineBridge.processCandidates`
    /// with `mergeOrderOnly: true` so dedup runs without scoring/sorting.
    /// Preserves the legacy iOS "merged-order on cold-start" behavior —
    /// custom-dictionary entries continue to surface ahead of system
    /// candidates until the freq DB warms up. Android has no equivalent
    /// branch because its `UserFrequencyService.frequencyDataBatch` is
    /// always callable.
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
            return RustEngineBridge.processCandidates(
                raw: merged,
                normalizedInput: "",
                tpsDedupEnabled: isTPS,
                frequencyData: [:],
                nowMs: 0,
                mergeOrderOnly: true,
            )
        }

        let normalizedInput = RustEngineBridge.normalizeInput(segmentedInput)
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
