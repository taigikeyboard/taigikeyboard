// The Manage Dictionaries search's state and its one call across the seam.

import Foundation

/// Why a Manage Dictionaries search could not complete.
struct DictionarySearchFailure: Error, Equatable, Sendable {
    let diagnostic: String
}

/// The hits for one query, as desktop-core finds them — the lookup the
/// Windows and Linux settings windows run (`engine::dictionary_search`),
/// asked across `desktop_request_bytes` (#792).
enum DictionarySearch {
    private static let logger = DebugLogger(category: "DictionarySearch")

    /// The hits for `query` under `settings`, or a diagnostic failure when the
    /// core or bridge could not answer. Safe off the main thread: the core
    /// answers it outside the session lock, and the bridge holds no state.
    static func search(
        for query: String,
        under settings: Taigi_DesktopShell_SettingsSnapshot,
    ) -> Result<[Taigi_DesktopShell_DictionarySearchHit], DictionarySearchFailure> {
        var request = Taigi_DesktopShell_DictionarySearchRequest()
        request.query = query
        switch DesktopCoreBridge.response(to: .dictionarySearch(request), settings: settings, op: "dictionarySearch") {
        case .failure(.notSent):
            return .failure(DictionarySearchFailure(diagnostic: "the search request could not be encoded"))
        case .failure(.undecodable):
            return .failure(DictionarySearchFailure(diagnostic: "the search response could not be decoded"))
        case let .success(response):
            guard response.error == .ok else {
                logger.error("[dictionarySearch] core returned \(response.error)")
                return .failure(DictionarySearchFailure(diagnostic: "core returned \(response.error)"))
            }
            guard case let .dictionarySearch(reply) = response.reply else {
                logger.error("[dictionarySearch] expected reply missing, got \(String(describing: response.reply))")
                return .failure(DictionarySearchFailure(diagnostic: "the search returned no reply"))
            }
            return .success(reply.hits)
        }
    }
}

extension Taigi_DesktopShell_DictionarySearchHit {
    var hasExternalLookup: Bool {
        hasMoeURL || hasChhoeURL
    }
}

/// What the search panel shows for its query: the hits, whether an answer is
/// still on its way (so "No results" never flashes before it), and any search
/// failure diagnostic.
@MainActor
@Observable
final class DictionarySearchModel {
    typealias Hit = Taigi_DesktopShell_DictionarySearchHit
    typealias LookUp = @Sendable (String, Taigi_DesktopShell_SettingsSnapshot) -> Result<[Hit], DictionarySearchFailure>

    var query = ""
    private(set) var hits: [Hit] = []
    private(set) var hasFailed = false
    private(set) var failureDiagnostic: String?
    /// The trimmed query `hits` answers.
    private var answeredQuery = ""

    private let lookUp: LookUp
    private let settings: @MainActor () -> Taigi_DesktopShell_SettingsSnapshot?

    /// `lookUp` and `settings` are the seam and the stored settings — built
    /// here, on the main actor, since `DesktopCoreRuntime` is not
    /// `Sendable`; tests pass their own.
    init(
        lookUp: @escaping LookUp = DictionarySearch.search(for:under:),
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

    var shouldShowNoResults: Bool {
        !trimmedQuery.isEmpty && !isSearching && !hasFailed && hits.isEmpty
    }

    var shouldShowResultsSection: Bool {
        !trimmedQuery.isEmpty && (!hits.isEmpty || shouldShowNoResults)
    }

    func dismissFailureAlert() {
        failureDiagnostic = nil
    }

    /// Runs the settled query off the main thread. An answer that arrives
    /// after the query moved on is dropped: the newest query wins.
    func search() async {
        let query = trimmedQuery
        guard !query.isEmpty else {
            hits = []
            hasFailed = false
            failureDiagnostic = nil
            answeredQuery = ""
            return
        }
        guard let settings = settings() else {
            hits = []
            hasFailed = true
            failureDiagnostic = "the dictionaries are not available"
            answeredQuery = query
            return
        }
        let lookUp = lookUp
        let outcome = await Task.detached { lookUp(query, settings) }.value
        guard query == trimmedQuery else { return }
        switch outcome {
        case let .success(found):
            hits = found
            hasFailed = false
            failureDiagnostic = nil
        case let .failure(error):
            hits = []
            hasFailed = true
            failureDiagnostic = error.diagnostic
        }
        answeredQuery = query
    }
}
