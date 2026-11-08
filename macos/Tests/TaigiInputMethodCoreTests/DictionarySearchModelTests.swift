@testable import TaigiInputMethodCore
import XCTest

/// The Manage Dictionaries search model over a stand-in lookup: what reaches
/// the seam, and which answers land. What the core finds is
/// `taigi-macos-ffi`'s `dictionary_search` tests' subject.
@MainActor
final class DictionarySearchModelTests: XCTestCase {
    private nonisolated static func hit(_ roman: String) -> DictionarySearchModel.Hit {
        var hit = DictionarySearchModel.Hit()
        hit.key = "system:\(roman)"
        hit.roman = roman
        return hit
    }

    /// Records the queries the seam was asked, and answers each with one hit
    /// spelled as the query.
    private final class Recorder: @unchecked Sendable {
        private let lock = NSLock()
        private var asked: [String] = []

        var queries: [String] {
            lock.withLock { asked }
        }

        func lookUp(_ query: String, _: Taigi_DesktopShell_SettingsSnapshot) -> [DictionarySearchModel.Hit]? {
            lock.withLock { asked.append(query) }
            return [DictionarySearchModelTests.hit(query)]
        }
    }

    func testTheTrimmedQueryIsLookedUp() async {
        let recorder = Recorder()
        let model = DictionarySearchModel(lookUp: recorder.lookUp, settings: { .init() })
        model.query = "  tsiah "
        XCTAssertTrue(model.isSearching, "no \"No results\" before the answer")
        await model.search()
        XCTAssertEqual(recorder.queries, ["tsiah"])
        XCTAssertEqual(model.hits.map(\.roman), ["tsiah"])
        XCTAssertFalse(model.isSearching)
    }

    func testABlankQueryAsksNothingAndClears() async {
        let recorder = Recorder()
        let model = DictionarySearchModel(lookUp: recorder.lookUp, settings: { .init() })
        model.query = "tsiah"
        await model.search()
        model.query = "   "
        await model.search()
        XCTAssertEqual(recorder.queries, ["tsiah"])
        XCTAssertTrue(model.hits.isEmpty)
        XCTAssertFalse(model.isSearching)
    }

    /// An answer for a query the user has since changed is dropped.
    func testAnAnswerForAnOldQueryIsDropped() async {
        let recorder = Recorder()
        let release = DispatchSemaphore(value: 0)
        let model = DictionarySearchModel(
            lookUp: { query, settings in
                let found = recorder.lookUp(query, settings)
                // Held until the query has moved on.
                release.wait()
                return found
            },
            settings: { .init() },
        )
        model.query = "tsiah"
        let search = Task { await model.search() }
        // The lookup is under way off the main actor before the user types on.
        while recorder.queries.isEmpty {
            await Task.yield()
        }
        model.query = "tsiah-png"
        release.signal()
        await search.value
        XCTAssertTrue(model.hits.isEmpty, "the stale answer did not land")
        XCTAssertTrue(model.isSearching, "the newer query's answer is still owed")
    }

    /// Before the runtime is configured there are no settings to search
    /// under: nothing is asked.
    func testNoSettingsAsksNothing() async {
        let recorder = Recorder()
        let model = DictionarySearchModel(lookUp: recorder.lookUp, settings: { nil })
        model.query = "tsiah"
        await model.search()
        XCTAssertTrue(recorder.queries.isEmpty)
        XCTAssertTrue(model.hits.isEmpty)
    }
}
