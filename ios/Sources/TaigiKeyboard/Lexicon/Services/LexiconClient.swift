import Foundation

/// The engine's dictionary search as `DictionarySearchService` sees it: a
/// seam so the service's settings snapshot can be tested with a fake.
/// Mirrors Android `ime/dictionary/LexiconClient.kt`; the shipped
/// implementation is `EngineLexiconClient`, the same shape as `UserDataClient`.
protocol LexiconClient: Sendable {
    /// The rows for `query` — custom entries first, then dictionary rows —
    /// as the engine's `DictionarySearch` answers them; empty when the engine
    /// did not answer.
    func dictionarySearch(
        query: String,
        inputMode: InputMode,
        toggles: RustEngineBridge.DictionaryToggles,
        customDictionaryEnabled: Bool,
        limit: Int,
    ) async -> [DictionarySearchResult]
}

/// The engine's `DictionarySearch` op, on the user-data request queue: the
/// op waits for the user data to finish opening, and that wait must not hold
/// a thread of the shared pool (`EngineUserDataClient.requests`).
struct EngineLexiconClient: LexiconClient {
    func dictionarySearch(
        query: String,
        inputMode: InputMode,
        toggles: RustEngineBridge.DictionaryToggles,
        customDictionaryEnabled: Bool,
        limit: Int,
    ) async -> [DictionarySearchResult] {
        let rows = await withCheckedContinuation { continuation in
            EngineUserDataClient.requests.async {
                continuation.resume(returning: RustEngineBridge.dictionarySearch(
                    query: query,
                    inputMode: inputMode.rawValue,
                    toggles: toggles,
                    customDictionaryEnabled: customDictionaryEnabled,
                    limit: limit,
                ))
            }
        }
        return rows?.map(DictionarySearchResult.init) ?? []
    }
}

extension DictionarySearchResult {
    init(_ row: Taigi_Engine_DictionarySearchRow) {
        self.init(
            roman: row.roman,
            tl: row.lookupTl,
            hanji: row.hasHanji ? row.hanji : nil,
            sources: row.sources.compactMap(RustEngineBridge.platformDictionarySource(from:)),
        )
    }
}
