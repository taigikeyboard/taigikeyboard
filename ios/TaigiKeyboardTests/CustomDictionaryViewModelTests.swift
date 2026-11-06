@testable import TaigiKeyboard
import XCTest

/// Custom Dictionary list policy over a fake engine: rows are paged by the
/// engine, a new filter lists from the top and drops answers still in flight,
/// and a write re-reads the listed rows. Mirrors Android's
/// `CustomDictionaryListingTest`.
@MainActor
final class CustomDictionaryViewModelTests: XCTestCase {
    /// The engine's list request, answering from `rows` with the engine's
    /// paging rule (`engine/userdata/src/paging.rs`).
    private final class FakeDictionary: UserDataClientStub, @unchecked Sendable {
        struct ListCall: Equatable {
            let filter: String
            let limit: UInt32
            let offset: UInt32
        }

        struct Unreadable: Error {}

        private let lock = NSLock()
        private var storedRows: [CustomDictionaryEntry]
        private var storedCalls: [ListCall] = []
        private var storedFails = false
        private var storedIsHolding = false
        private var parked: [CheckedContinuation<Void, Never>] = []

        init(rows: [CustomDictionaryEntry]) {
            storedRows = rows
        }

        var rows: [CustomDictionaryEntry] {
            get { lock.withLock { storedRows } }
            set { lock.withLock { storedRows = newValue } }
        }

        var calls: [ListCall] {
            get { lock.withLock { storedCalls } }
            set { lock.withLock { storedCalls = newValue } }
        }

        var fails: Bool {
            get { lock.withLock { storedFails } }
            set { lock.withLock { storedFails = newValue } }
        }

        /// While set, each request waits for `releaseParked()`.
        var isHolding: Bool {
            get { lock.withLock { storedIsHolding } }
            set { lock.withLock { storedIsHolding = newValue } }
        }

        var parkedCount: Int {
            lock.withLock { parked.count }
        }

        func releaseParked() {
            let waiting = lock.withLock {
                defer { parked = [] }
                return parked
            }
            waiting.forEach { $0.resume() }
        }

        override func listCustomEntries(filter: String, limit: UInt32, offset: UInt32) async throws -> CustomDictionaryPage {
            // The answer is cut on arrival, so a parked request answers what
            // it asked even after the test moved on.
            let answer: CustomDictionaryPage? = lock.withLock {
                storedCalls.append(ListCall(filter: filter, limit: limit, offset: offset))
                return storedFails ? nil : page(filter: filter, limit: Int(limit), offset: Int(offset))
            }
            if isHolding {
                await withCheckedContinuation { continuation in
                    lock.withLock { parked.append(continuation) }
                }
            }
            guard let answer else { throw Unreadable() }
            return answer
        }

        private func page(filter: String, limit: Int, offset: Int) -> CustomDictionaryPage {
            let query = filter.trimmingCharacters(in: .whitespaces)
            let matching = storedRows.filter { query.isEmpty || $0.roman.contains(query) }
            let served = min(offset, max(matching.count - 1, 0) / limit * limit)
            return CustomDictionaryPage(
                entries: Array(matching.dropFirst(served).prefix(limit)),
                total: storedRows.count,
                matchingTotal: matching.count,
                offset: served,
            )
        }
    }

    private func words(_ count: Int) -> [CustomDictionaryEntry] {
        (0 ..< count).map { CustomDictionaryEntry(id: "\($0)", roman: "word\($0)", hanji: "字") }
    }

    private func makeViewModel(_ fake: FakeDictionary, filterSettle: Duration = .zero) -> CustomDictionaryViewModel {
        CustomDictionaryViewModel(userData: fake, filterSettle: filterSettle)
    }

    func testListsOnePage_thenEveryRowAPageAtATime() async {
        let fake = FakeDictionary(rows: words(250))
        let viewModel = makeViewModel(fake)

        await viewModel.load()
        XCTAssertEqual(viewModel.entries.count, 100)
        XCTAssertFalse(viewModel.isLoading)
        XCTAssertTrue(viewModel.hasMoreRows)

        await viewModel.loadNextPage()
        await viewModel.loadNextPage()

        // trace: 250 rows, pages at 0 / 100 / 200 — row 101 is reachable with no filter.
        XCTAssertEqual(fake.calls.map(\.offset), [0, 100, 200])
        XCTAssertEqual(viewModel.entries.count, 250)
        XCTAssertEqual(viewModel.entries[100].id, "100")
        XCTAssertFalse(viewModel.hasMoreRows)

        await viewModel.loadNextPage()
        XCTAssertEqual(fake.calls.count, 3)
    }

    func testFilterGoesToTheEngine() async {
        let fake = FakeDictionary(rows: words(250))
        let viewModel = makeViewModel(fake)

        await viewModel.filterChanged("word249").value

        XCTAssertEqual(fake.calls, [.init(filter: "word249", limit: 100, offset: 0)])
        XCTAssertEqual(viewModel.entries.map(\.id), ["249"])
        XCTAssertEqual(viewModel.total, 250)
        XCTAssertEqual(viewModel.matchingTotal, 1)
    }

    func testLoad_rereadsTheListedRowsInOneRequest() async {
        let fake = FakeDictionary(rows: words(250))
        let viewModel = makeViewModel(fake)
        await viewModel.load()
        await viewModel.loadNextPage()
        await viewModel.loadNextPage()
        fake.calls = []

        await viewModel.load()

        XCTAssertEqual(fake.calls, [.init(filter: "", limit: 250, offset: 0)])
        XCTAssertEqual(viewModel.entries.count, 250)
    }

    func testEmptySearchKeepsTheStoredTotal_emptyDictionaryHasNone() async {
        let viewModel = makeViewModel(FakeDictionary(rows: words(3)))
        await viewModel.filterChanged("nothing").value
        XCTAssertTrue(viewModel.entries.isEmpty)
        XCTAssertEqual(viewModel.total, 3)

        let empty = makeViewModel(FakeDictionary(rows: []))
        await empty.load()
        XCTAssertEqual(empty.total, 0)
    }

    func testPageServedFromAnEarlierOffset_rereadsTheList() async {
        let fake = FakeDictionary(rows: words(150))
        let viewModel = makeViewModel(fake)
        await viewModel.load()

        // trace: 60 rows left, offset 100 asked → the fake serves (59 / 100) * 100 = 0.
        fake.rows = Array(fake.rows.prefix(60))
        await viewModel.loadNextPage()

        XCTAssertEqual(viewModel.entries.count, 60)
        XCTAssertEqual(viewModel.matchingTotal, 60)
        XCTAssertEqual(Set(viewModel.entries.map(\.id)).count, 60)
    }

    func testWordAddedToAFullyListedList_isReachedThroughThePulledBackPage() async {
        let fake = FakeDictionary(rows: words(120))
        let viewModel = makeViewModel(fake)
        await viewModel.load()
        await viewModel.loadNextPage()
        XCTAssertFalse(viewModel.hasMoreRows)

        fake.rows.insert(CustomDictionaryEntry(id: "new", roman: "new", hanji: "字"), at: 0)
        await viewModel.load()
        XCTAssertEqual(viewModel.entries.count, 120)
        XCTAssertTrue(viewModel.hasMoreRows)

        // trace: 121 matches, limit 100 → last page 120 / 100 * 100 = 100, so offset 120 serves 100.
        await viewModel.loadNextPage()
        XCTAssertEqual(viewModel.entries.count, 121)
        XCTAssertEqual(Set(viewModel.entries.map(\.id)).count, 121)
        XCTAssertFalse(viewModel.hasMoreRows)
    }

    func testFailedReadKeepsTheRows_andRetryResumesPaging() async {
        let fake = FakeDictionary(rows: words(150))
        let viewModel = makeViewModel(fake)
        await viewModel.load()

        fake.fails = true
        await viewModel.loadNextPage()
        XCTAssertEqual(viewModel.entries.count, 100)
        XCTAssertTrue(viewModel.hasReadFailed)

        await viewModel.load()
        XCTAssertEqual(viewModel.entries.count, 100)
        XCTAssertEqual(viewModel.total, 150)

        fake.calls = []
        await viewModel.loadNextPage()
        XCTAssertTrue(fake.calls.isEmpty, "nothing is asked until the retry")

        fake.fails = false
        await viewModel.load()
        XCTAssertFalse(viewModel.hasReadFailed)
        await viewModel.loadNextPage()
        XCTAssertEqual(viewModel.entries.count, 150)
    }

    func testNewerFilterDropsTheAnswerInFlight() async {
        let fake = FakeDictionary(rows: words(3))
        let viewModel = makeViewModel(fake)
        fake.isHolding = true
        async let stale: Void = viewModel.load()
        while fake.parkedCount == 0 {
            await Task.yield()
        }

        fake.isHolding = false
        await viewModel.filterChanged("word1").value
        fake.releaseParked()
        await stale

        XCTAssertEqual(viewModel.entries.map(\.id), ["1"])
    }
}
