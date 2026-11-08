// The Manage Dictionaries search's state and its one call across the seam.

import Foundation

/// The hits for one query, as desktop-core finds them — the lookup the
/// Windows and Linux settings windows run (`engine::dictionary_search`),
/// asked across `desktop_request_bytes` (#792).
enum DictionarySearch {
    /// The hits for `query` under `settings`, or nil (logged by the bridge)
    /// when the core gave no answer. Safe off the main thread: the core
    /// answers it outside the session lock, and the bridge holds no state.
    static func hits(
        for query: String,
        under settings: Taigi_DesktopShell_SettingsSnapshot,
    ) -> [Taigi_DesktopShell_DictionarySearchHit]? {
        var request = Taigi_DesktopShell_DictionarySearchRequest()
        request.query = query
        let reply = DesktopCoreBridge.roundtrip(
            .dictionarySearch(request),
            settings: settings,
            op: "dictionarySearch",
        ) {
            if case let .dictionarySearch(reply) = $0 {
                reply
            } else {
                nil
            }
        }
        return reply?.hits
    }
}

/// What the search panel shows for its query: the hits, and whether an
/// answer is still on its way (so "No results" never flashes before it).
@MainActor
@Observable
final class DictionarySearchModel {
    typealias Hit = Taigi_DesktopShell_DictionarySearchHit
    typealias LookUp = @Sendable (String, Taigi_DesktopShell_SettingsSnapshot) -> [Hit]?

    var query = ""
    private(set) var hits: [Hit] = []
    /// The trimmed query `hits` answers.
    private var answeredQuery = ""

    private let lookUp: LookUp
    private let settings: @MainActor () -> Taigi_DesktopShell_SettingsSnapshot?

    /// `lookUp` and `settings` are the seam and the stored settings — built
    /// here, on the main actor, since `DesktopCoreRuntime` is not
    /// `Sendable`; tests pass their own.
    init(
        lookUp: @escaping LookUp = DictionarySearch.hits(for:under:),
        settings: @escaping @MainActor () -> Taigi_DesktopShell_SettingsSnapshot? = {
            DesktopCoreRuntime.snapshot(in: SettingsStore().userDefaults, op: "dictionarySearch")
        },
    ) {
        self.lookUp = lookUp
        self.settings = settings
    }

    var trimmedQuery: String {
        query.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    var isSearching: Bool {
        !trimmedQuery.isEmpty && trimmedQuery != answeredQuery
    }

    /// Runs the settled query off the main thread. An answer that arrives
    /// after the query moved on is dropped: the newest query wins.
    func search() async {
        let query = trimmedQuery
        guard !query.isEmpty, let settings = settings() else {
            hits = []
            answeredQuery = query
            return
        }
        let lookUp = lookUp
        let found = await Task.detached { lookUp(query, settings) }.value
        guard query == trimmedQuery else { return }
        hits = found ?? []
        answeredQuery = query
    }
}
