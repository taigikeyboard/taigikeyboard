// Dictionary tab(Tab 3)用的搜尋服務 — 與 keyboard 候選詞流程不同,
// 提供 CJK / roman 路徑分流、kautian 優先排序、source bitmask filter 與 retag。

import Foundation

/// Search service backing the Dictionary tab.
///
/// Orchestrates dictionary browsing (distinct from keyboard-candidate generation
/// in `LexiconService`): CJK vs roman path selection, custom-dict prefix search,
/// kautian-first ordering, and source-tag filtering for badge display.
///
/// System-dict queries flow through `RustEngineBridge.lexiconSearchByHanzi` /
/// `lexiconSearchWithSources`. The Rust engine's bitmask filter drops rows
/// where ZERO enabled bits match — but multi-source rows that overlap at
/// least one enabled source still arrive with their full source bitmask.
/// `retagSources` then trims each row's `sources` array to enabled-only so
/// the badge UI reflects the user's current toggles.
// Tab 3 詞典搜尋 service — 與 keyboard 路徑分離,有自己的 source filter / 排序邏輯。
final class DictionarySearchService: @unchecked Sendable {
    // MARK: - Dependencies

    private let customDictionaryRepository: CustomDictionaryRepository
    private let settingsProvider: EngineSettingsProvider
    private let logger = DebugLogger(category: "DictionarySearchService")

    // MARK: - Init

    init(
        customDictionaryRepository: CustomDictionaryRepository = CompositionRoot.customDictionaryRepository,
        settingsProvider: EngineSettingsProvider = SharedSettings.shared,
    ) {
        self.customDictionaryRepository = customDictionaryRepository
        self.settingsProvider = settingsProvider
        // Lexicon engine state (fst + dictionary.bin + association.bin) is
        // installed once at extension launch via `RustEngineBridge.lexiconInstall(...)`.
        // The Tab3 host process invokes the same bridge call from its
        // composition root; reinstalling is idempotent.
        bootstrapCustomDictionary()
    }

    /// Eagerly open the custom-dictionary DB when the injected settings enable
    /// it, so `searchSync` has a live connection the moment a search arrives.
    /// Failures are logged and left to graceful degradation at lookup time.
    // 設定有開啟自訂詞庫時,提早把 DB 連線打開,讓 searchSync 第一次呼叫就有連線可用。
    // 連線失敗只 log,查詢時會 graceful degrade。
    private func bootstrapCustomDictionary() {
        guard settingsProvider.current.isCustomDictEnabled else { return }
        Task { [customDictionaryRepository, logger] in
            do {
                try await customDictionaryRepository.ensureInitialized()
                logger.info("[INIT] Custom dictionary initialized")
            } catch {
                logger.warning("[INIT] Custom dictionary init failed: \(error.localizedDescription)")
            }
        }
    }

    // MARK: - Public API

    /// Search the user's enabled dictionaries for `query`.
    ///
    /// Hanzi queries use the CJK path; roman queries also consult the user's
    /// custom dictionary. Results are sorted with kautian (教育部) first, then
    /// by frequency; custom-dict hits lead the list. Awaits Trie readiness so
    /// searches arriving during the bootstrap window don't return empty.
    // Tab 3 主搜尋 — 漢字走 CJK 路徑,羅馬字額外查自訂詞庫。
    // 排序為 custom-dict → 教育部教典優先 → 頻率,filter / retag 用同一份 toggles snapshot。
    func search(
        query: String,
        limit: Int = 20,
    ) async throws -> [DictionarySearchResult] {
        guard !query.isEmpty else { return [] }

        let inputMode = settingsProvider.current.inputMode
        let isCJK = RustEngineBridge.isHanzi(query)
        logger.debug("[SEARCH] query='\(query)' isCJK=\(isCJK) inputMode=\(String(describing: inputMode))")

        // Resolve filter bitmask + enabled-source set ONCE per query and
        // hand both down the pipeline. Splitting the snapshot (resolving
        // again in retag) would let toggle changes mid-search produce a
        // mask/badge mismatch (Codex pre-impl BLOCK 5).
        let toggles = RustEngineBridge.DictionaryToggles(from: settingsProvider.current)
        let filters = RustEngineBridge.lexiconDictionaryFilters(toggles: toggles)

        let systemResults = fetchSystemResults(
            query: query,
            inputMode: inputMode,
            isCJK: isCJK,
            limit: limit,
            filterBitmask: filters.dictionaryFilterBitmask,
        )
        let customResults = isCJK ? [] : lookupCustomDictionary(query: query)

        let prepared = sortByMoeThenFrequency(systemResults)
            .map { retagSources($0, enabled: filters.enabledSources) }

        return customResults + prepared
    }

    // MARK: - Private

    private func fetchSystemResults(
        query: String,
        inputMode: InputMode,
        isCJK: Bool,
        limit: Int,
        filterBitmask: UInt32,
    ) -> [DictionarySearchResult] {
        // .english unreachable — keyboard passthrough never invokes lexicon
        // search; mirrors Android `InputMode.ENGLISH -> LexiconInputMode.TL`.
        let bridgeMode: RustEngineBridge.LexiconInputMode = switch inputMode {
        case .tl: .tl
        case .poj: .poj
        case .tps: .tps
        case .english: .tl
        }
        let rows = isCJK
            ? RustEngineBridge.lexiconSearchByHanzi(
                query: query,
                inputMode: bridgeMode,
                limit: UInt32(limit),
                enabledSourcesBitmask: filterBitmask,
            )
            : RustEngineBridge.lexiconSearchWithSources(
                input: query,
                inputMode: bridgeMode,
                limit: UInt32(limit),
                enabledSourcesBitmask: filterBitmask,
            )
        return rows.map { row in
            // Engine returns raw `tl`; render to POJ when in POJ mode.
            let roman = inputMode == .poj ? RustEngineBridge.tlToPoj(row.roman) : row.roman
            let bitmask = row.sourceBitmask ?? 0
            return DictionarySearchResult(
                id: Int(row.id),
                roman: roman,
                tl: row.roman,
                hanzi: row.hanzi,
                frequency: row.lengthScore.map(Int.init) ?? 0,
                sources: LexiconBitmask.sources(from: bitmask),
            )
        }
    }

    private func lookupCustomDictionary(query: String) -> [DictionarySearchResult] {
        guard settingsProvider.current.isCustomDictEnabled,
              let q = CustomDictionaryDerivation.queryKey(for: query, mode: settingsProvider.current.inputMode)
        else { return [] }
        let entries = customDictionaryRepository.searchSync(
            family: q.family,
            form: q.form,
            key: q.key,
            limit: 20,
        )
        return entries.map { entry in
            DictionarySearchResult(
                id: DictionarySearchResult.customDictMarkerId,
                roman: entry.roman,
                tl: entry.roman,
                hanzi: entry.hanzi,
                frequency: Int.max,
                sources: [.custom],
            )
        }
    }

    /// Kautian (教育部) results first, then descending frequency.
    // 教育部教典優先,其餘依 frequency 由大到小排序。
    private func sortByMoeThenFrequency(_ results: [DictionarySearchResult]) -> [DictionarySearchResult] {
        results.sorted { a, b in
            let aMoe = a.sources.contains(.kautian)
            let bMoe = b.sources.contains(.kautian)
            if aMoe != bMoe { return aMoe }
            return a.frequency > b.frequency
        }
    }

    /// Drop source tags the user has disabled so badges reflect current toggles.
    /// DB-layer filtering has already excluded results whose sources are all disabled.
    // 把使用者關閉的 source tag 從顯示陣列移除,讓 badge 反映當前設定。
    // DB 已經把全部 source 都關掉的 row 過濾掉了,這裡只處理多 source row 的呈現裁切。
    private func retagSources(
        _ result: DictionarySearchResult,
        enabled: Set<DictionarySource>,
    ) -> DictionarySearchResult {
        DictionarySearchResult(
            id: result.id,
            roman: result.roman,
            tl: result.tl,
            hanzi: result.hanzi,
            frequency: result.frequency,
            sources: result.sources.filter { enabled.contains($0) },
        )
    }
}
