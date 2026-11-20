import Foundation

/// Search service backing dictionary search on the Manage Dictionaries page.
///
/// The engine's `DictionarySearch` op owns the search — Hanji vs romanization
/// lookup, source filter and badges, POJ / TL rendering, custom entries
/// first. This service hands it one snapshot of the settings per query.
/// CROSS-PLATFORM INVARIANT — mirrors Android `ime/dictionary/DictionarySearchService.kt`.
final class DictionarySearchService: @unchecked Sendable {
    private let lexicon: any LexiconClient
    private let settingsProvider: EngineSettingsProvider

    init(
        lexicon: any LexiconClient = EngineLexiconClient(),
        settingsProvider: EngineSettingsProvider = SharedSettings.shared,
    ) {
        self.lexicon = lexicon
        self.settingsProvider = settingsProvider
    }

    /// Search the user's enabled dictionaries for `query`: up to `limit`
    /// custom entries, then up to `limit` dictionary rows.
    func search(query: String, limit: Int = 20) async -> [DictionarySearchResult] {
        guard !query.isEmpty else { return [] }
        // One snapshot per query: the toggles and the custom gate must agree.
        let settings = settingsProvider.current
        return await lexicon.dictionarySearch(
            query: query,
            inputMode: settings.inputMode,
            toggles: RustEngineBridge.DictionaryToggles(from: settings),
            customDictionaryEnabled: settings.isCustomDictEnabled,
            limit: limit,
        )
    }
}
