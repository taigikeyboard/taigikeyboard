// Looking a word up in the dictionaries, from the 詞庫 tab.

import Foundation

/// One row of a dictionary search, as the settings list shows it.
struct DictionarySearchResult: Equatable, Identifiable, Sendable {
    /// Which list a row came from, and its key within that list.
    ///
    /// Namespaced because the two lists number their rows independently: a
    /// custom entry's UUID and a dictionary record's row id share no space,
    /// and a single sentinel for every custom row (which is what iOS uses)
    /// would make SwiftUI treat them as one row repeated.
    ///
    /// This is display identity, not word identity — anything matching or
    /// deduping WORDS still keys on the `(漢字, canonical TL)` pair.
    enum ID: Hashable, Sendable {
        case system(Int64)
        case custom(String)
    }

    let id: ID
    /// What the row shows: TL, or POJ while the user is typing POJ.
    let roman: String
    /// The engine's own TL for the word, or `nil` for a row the two
    /// dictionary sites cannot be asked about.
    ///
    /// The lookups are built from this rather than from `roman`, because the
    /// sites index TL and the display may be POJ. It is `nil` for the user's
    /// own entries: a word they added themselves is one the bundled
    /// dictionaries did not have, and the spelling they typed it under is
    /// whichever script they were in — a link built from it would query for a
    /// word that is not there, in a form the site does not index.
    let lookupTl: String?
    let hanzi: String?
    let sources: [DictionarySource]

    var moeURL: URL? {
        lookupTl.flatMap(ExternalLookupURLBuilder.moeURL(forTl:))
    }

    var chhoeURL: URL? {
        lookupTl.flatMap(ExternalLookupURLBuilder.chhoeURL(forTl:))
    }
}

/// Runs one dictionary search: the bundled dictionaries, plus the user's own.
///
/// Everything a query needs is read ONCE at the top — the settings snapshot
/// and the resolved source filters — and passed down. Reading either again
/// mid-query would let a toggle changed between the two reads produce results
/// filtered one way and labelled another.
struct DictionarySearchService: Sendable {
    /// What the engine is asked for. The list shows fewer; the rest are the
    /// margin the kautian-first sort reorders within.
    static let resultLimit = 20

    private let customDictionaryStore: CustomDictionaryStore
    private let settingsProvider: any EngineSettingsProvider

    init(
        customDictionaryStore: CustomDictionaryStore,
        settingsProvider: any EngineSettingsProvider,
    ) {
        self.customDictionaryStore = customDictionaryStore
        self.settingsProvider = settingsProvider
    }

    /// The results for `query`, best first.
    ///
    /// Order is a contract, in three parts: the user's own entries lead,
    /// because a word they added themselves is the one they meant; then the
    /// bundled results, 教育部 first because it is the reference dictionary;
    /// then by the engine's own ranking, with its original order as the final
    /// tie-break so the same query twice gives the same list.
    func search(_ query: String) -> [DictionarySearchResult] {
        guard !query.isEmpty else { return [] }
        let settings = settingsProvider.current
        let filters = RustEngineBridge.lexiconDictionaryFilters(toggles: settings.dictionarySources)

        let isHanziQuery = RustEngineBridge.isHanzi(query)
        let systemResults = systemResults(
            query: query,
            settings: settings,
            isHanziQuery: isHanziQuery,
            filters: filters,
        )
        // The custom dictionary is keyed by romanization, so a 漢字 query has
        // nothing to look up in it.
        let customResults = isHanziQuery ? [] : customResults(query: query, settings: settings)

        return customResults + systemResults
    }

    // MARK: - The bundled dictionaries

    private func systemResults(
        query: String,
        settings: EngineSettings,
        isHanziQuery: Bool,
        filters: DictionaryFilters?,
    ) -> [DictionarySearchResult] {
        // A failed resolve searches everything and labels everything, rather
        // than showing an empty dictionary because one FFI call did not come
        // back. The search path's all-on sentinel is `UInt32.max`, NOT `0` —
        // `0` there means "no sources enabled" and would empty the list.
        let wireMask = filters?.wireMask ?? RustEngineBridge.allSourcesEnabledSearchBitmask
        let enabledSources = filters?.enabledSources
        let mode: LexiconInputMode = settings.inputMode == .poj ? .poj : .tl

        let rows = isHanziQuery
            ? RustEngineBridge.lexiconSearchByHanzi(
                query: query,
                inputMode: mode,
                limit: UInt32(Self.resultLimit),
                enabledSourcesBitmask: wireMask,
            )
            : RustEngineBridge.lexiconSearchWithSources(
                input: query,
                inputMode: mode,
                limit: UInt32(Self.resultLimit),
                enabledSourcesBitmask: wireMask,
            )

        // Sorted BEFORE the badges are trimmed: the sort asks whether a row is
        // a 教育部 row, and a row that is one is still one when the user has
        // that dictionary switched off but reached the list through another
        // source it also belongs to.
        return Ordering.sorted(rows).map { row in
            let sources = LexiconBitmask.sources(from: row.sourceBitmask ?? 0)
            return DictionarySearchResult(
                id: .system(row.id),
                roman: displayRoman(row.roman, settings: settings),
                lookupTl: row.roman,
                hanzi: row.hanzi,
                // Trimmed to what is switched on, so the badges describe the
                // dictionaries the user actually has. A row survives the
                // engine's filter when ANY of its sources is enabled, so a
                // multi-source row can arrive carrying tags for dictionaries
                // that are off.
                sources: enabledSources.map { enabled in
                    sources.filter(enabled.contains)
                } ?? sources,
            )
        }
    }

    /// The rows the engine returned, in the order the list shows them.
    ///
    /// The engine's own position is carried through the sort only to break
    /// ties: `sorted(by:)` is not documented stable, so without it the same
    /// query could list equal-scoring rows differently between two runs.
    /// Its own type so the order can be tested against rows a fixture names,
    /// rather than only against whatever the shipped dictionary returns.
    enum Ordering {
        /// 教育部 first — it is the reference dictionary — then by the
        /// engine's own score, then by the order the engine returned them in.
        ///
        /// Reads the row's own bitmask, never a trimmed badge list: the sort
        /// runs before the badges are trimmed, so a 教育部 row is one even
        /// when the user has that dictionary switched off and the row reached
        /// the list through another source it also belongs to.
        static func sorted(_ rows: [LexiconRow]) -> [LexiconRow] {
            rows.enumerated().sorted { first, second in
                let firstRow = first.element
                let secondRow = second.element

                let firstIsKautian = LexiconBitmask
                    .sources(from: firstRow.sourceBitmask ?? 0).contains(.kautian)
                let secondIsKautian = LexiconBitmask
                    .sources(from: secondRow.sourceBitmask ?? 0).contains(.kautian)
                if firstIsKautian != secondIsKautian {
                    return firstIsKautian
                }

                let firstScore = firstRow.lengthScore ?? 0
                let secondScore = secondRow.lengthScore ?? 0
                if firstScore != secondScore {
                    return firstScore > secondScore
                }

                return first.offset < second.offset
            }.map(\.element)
        }
    }

    /// The romanization as the user is currently typing it. The engine answers
    /// in TL; POJ is a rendering, and only of the bundled rows — a custom
    /// entry is shown exactly as it was typed.
    private func displayRoman(_ tl: String, settings: EngineSettings) -> String {
        guard settings.inputMode == .poj else { return tl }
        return RustEngineBridge.tlToPoj(tl) ?? tl
    }

    // MARK: - The user's own dictionary

    private func customResults(
        query: String,
        settings: EngineSettings,
    ) -> [DictionarySearchResult] {
        guard settings.isCustomDictEnabled,
              let queryKey = RustEngineBridge.deriveCustomQueryKey(
                  input: query,
                  mode: settings.inputMode,
              )
        else { return [] }

        return customDictionaryStore
            .rows(matching: queryKey, limit: Self.resultLimit)
            .map { row in
                DictionarySearchResult(
                    id: .custom(row.id),
                    roman: row.roman,
                    lookupTl: nil,
                    hanzi: row.hanzi.isEmpty ? nil : row.hanzi,
                    sources: [.custom],
                )
            }
    }
}
