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

        func lookUp(
            _ query: String,
            _: Taigi_DesktopShell_SettingsSnapshot,
        ) -> Result<[DictionarySearchModel.Hit], DictionarySearchFailure> {
            lock.withLock { asked.append(query) }
            return .success([DictionarySearchModelTests.hit(query)])
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
        XCTAssertFalse(model.hasFailed)
        XCTAssertNil(model.failureDiagnostic)
        XCTAssertTrue(model.shouldShowResultsSection)
        XCTAssertFalse(model.shouldShowNoResults)
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
        XCTAssertFalse(model.hasFailed)
        XCTAssertFalse(model.shouldShowResultsSection)
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
    /// under: nothing is asked and a failure is surfaced rather than "No results".
    func testNoSettingsAsksNothingAndReportsFailure() async {
        let recorder = Recorder()
        let model = DictionarySearchModel(lookUp: recorder.lookUp, settings: { nil })
        model.query = "tsiah"
        await model.search()
        XCTAssertTrue(recorder.queries.isEmpty)
        XCTAssertTrue(model.hits.isEmpty)
        XCTAssertTrue(model.hasFailed)
        XCTAssertNotNil(model.failureDiagnostic)
        XCTAssertFalse(model.shouldShowNoResults)
        XCTAssertFalse(model.shouldShowResultsSection)
    }

    /// A failed bridge search surfaces its diagnostic, clears stale hits, and
    /// suppresses "No results" even after the alert is dismissed, until a
    /// subsequent query succeeds.
    func testAFailedLookupReportsDiagnosticAndSuppressesNoResults() async {
        let model = DictionarySearchModel(
            lookUp: { query, _ in
                if query == "broken" {
                    return .failure(DictionarySearchFailure(diagnostic: "core returned failInternal"))
                }
                return .success([DictionarySearchModelTests.hit(query)])
            },
            settings: { .init() },
        )

        model.query = "tsiah"
        await model.search()
        XCTAssertEqual(model.hits.map(\.roman), ["tsiah"])

        model.query = "broken"
        await model.search()
        XCTAssertTrue(model.hits.isEmpty)
        XCTAssertTrue(model.hasFailed)
        XCTAssertEqual(model.failureDiagnostic, "core returned failInternal")
        XCTAssertFalse(model.shouldShowNoResults)
        XCTAssertFalse(model.shouldShowResultsSection)

        model.dismissFailureAlert()
        XCTAssertNil(model.failureDiagnostic)
        XCTAssertTrue(model.hasFailed)
        XCTAssertFalse(model.shouldShowNoResults)
        XCTAssertFalse(model.shouldShowResultsSection)

        model.query = "tsiah-png"
        await model.search()
        XCTAssertFalse(model.hasFailed)
        XCTAssertNil(model.failureDiagnostic)
        XCTAssertEqual(model.hits.map(\.roman), ["tsiah-png"])
        XCTAssertTrue(model.shouldShowResultsSection)
    }

    func testHitHasExternalLookup_reflectsURLAvailability() {
        var none = DictionarySearchModel.Hit()
        none.key = "custom:1"
        XCTAssertFalse(none.hasExternalLookup)

        var moeOnly = none
        moeOnly.moeURL = "https://sutian.moe.edu.tw/zh-hant/tshiau/?lui=tai_su&tsha=tai5"
        XCTAssertTrue(moeOnly.hasExternalLookup)

        var chhoeOnly = none
        chhoeOnly.chhoeURL = "https://chhoe.taigi.info/s?s=su&f=e&lmjf=ki&lmj=tai5"
        XCTAssertTrue(chhoeOnly.hasExternalLookup)
    }
}
