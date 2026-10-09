@testable import TaigiKeyboard
import XCTest

/// Learning Records page policy over a fake engine: rows are paged by the
/// engine (100 at a time, the next page when the last row shows), a change
/// of kind / order / filter lists from the top and drops answers still in
/// flight, every write reloads, and a row already gone is a notice, not a
/// failure. Mirrors `desktop-core` `settings/learning_records.rs` + `listing.rs`.
@MainActor
final class LearningRecordsViewModelTests: XCTestCase {
    /// The engine's user-data surface, answering from `rows` with the
    /// engine's paging rule — or, while `isHolding`, parking each list
    /// request until the test resolves it.
    private final class FakeLearningRecords: UserDataClientStub, @unchecked Sendable {
        struct ListCall: Equatable {
            let kind: Taigi_Engine_LearningRecordKind
            let order: Taigi_Engine_LearningRecordOrder
            let filter: String
            let limit: UInt32
            let offset: UInt32
        }

        struct Unreadable: LocalizedError {
            var errorDescription: String? {
                "disk I/O error"
            }
        }

        private let lock = NSLock()
        private var storedRows: [Taigi_Engine_LearningRecord]
        private var storedCalls: [ListCall] = []
        private var parked: [(call: ListCall, answer: CheckedContinuation<Taigi_Engine_LearningRecords, any Error>)] = []
        private var storedIsHolding = false
        private var storedFailure: Unreadable?
        private var storedListsFail = false
        private var storedAddRefusal: String?
        private var storedAdded: [Taigi_Engine_LearningRecord] = []

        init(rows: [Taigi_Engine_LearningRecord]) {
            storedRows = rows
        }

        var rows: [Taigi_Engine_LearningRecord] {
            get { lock.withLock { storedRows } }
            set { lock.withLock { storedRows = newValue } }
        }

        var calls: [ListCall] {
            lock.withLock { storedCalls }
        }

        var isHolding: Bool {
            get { lock.withLock { storedIsHolding } }
            set { lock.withLock { storedIsHolding = newValue } }
        }

        /// Every request fails while set.
        var failure: Unreadable? {
            get { lock.withLock { storedFailure } }
            set { lock.withLock { storedFailure = newValue } }
        }

        /// Only list requests fail while set; writes still land.
        var listsFail: Bool {
            get { lock.withLock { storedListsFail } }
            set { lock.withLock { storedListsFail = newValue } }
        }

        /// The engine's refusal detail every add throws while set; the row
        /// stays.
        var addRefusal: String? {
            get { lock.withLock { storedAddRefusal } }
            set { lock.withLock { storedAddRefusal = newValue } }
        }

        /// The rows whose words were filed in the custom dictionary, in order.
        var added: [Taigi_Engine_LearningRecord] {
            lock.withLock { storedAdded }
        }

        var parkedCount: Int {
            lock.withLock { parked.count }
        }

        /// Answers the parked request at `index` from the rows as they are now.
        func resolve(_ index: Int) {
            let (call, answer) = lock.withLock { parked.remove(at: index) }
            answer.resume(returning: page(for: call))
        }

        /// Fails the parked request at `index` as an unreadable store.
        func fail(_ index: Int) {
            let (_, answer) = lock.withLock { parked.remove(at: index) }
            answer.resume(throwing: Unreadable())
        }

        override func listLearningRecords(
            kind: Taigi_Engine_LearningRecordKind,
            order: Taigi_Engine_LearningRecordOrder,
            filter: String,
            limit: UInt32,
            offset: UInt32,
        ) async throws -> Taigi_Engine_LearningRecords {
            let call = ListCall(kind: kind, order: order, filter: filter, limit: limit, offset: offset)
            let (isHolding, failure) = lock.withLock {
                storedCalls.append(call)
                return (storedIsHolding, storedFailure ?? (storedListsFail ? Unreadable() : nil))
            }
            if let failure {
                throw failure
            }
            guard isHolding else { return page(for: call) }
            return try await withCheckedThrowingContinuation { answer in
                lock.withLock { parked.append((call, answer)) }
            }
        }

        override func setLearningRecordCount(_ record: Taigi_Engine_LearningRecord, count: Int64) async throws -> Bool {
            if let failure {
                throw failure
            }
            return lock.withLock {
                guard let index = storedRows.firstIndex(where: { $0.isSameRow(as: record) }) else { return false }
                storedRows[index].count = count
                return true
            }
        }

        override func deleteLearningRecord(_ record: Taigi_Engine_LearningRecord) async throws -> Bool {
            if let failure {
                throw failure
            }
            return lock.withLock {
                guard let index = storedRows.firstIndex(where: { $0.isSameRow(as: record) }) else { return false }
                storedRows.remove(at: index)
                return true
            }
        }

        /// The engine's add: a refusal keeps the row; otherwise the word is
        /// filed, and a learned phrase's row forgotten (one already gone is no
        /// failure) while a frequency row stays.
        override func addLearningRecordToCustomDictionary(_ record: Taigi_Engine_LearningRecord) async throws {
            if let failure {
                throw failure
            }
            try lock.withLock {
                if let storedAddRefusal {
                    throw UserDataRefused(detail: storedAddRefusal)
                }
                storedAdded.append(record)
                if record.kind == .learnedPhrase {
                    storedRows.removeAll { $0.isSameRow(as: record) }
                }
            }
        }

        /// The engine's page: substring filter on text / TL, an offset past
        /// the end pulled back to the last page (`engine/userdata/src/paging.rs`).
        private func page(for call: ListCall) -> Taigi_Engine_LearningRecords {
            let ofKind = rows.filter { $0.kind == call.kind }
            let matching = ofKind.filter {
                call.filter.isEmpty || $0.text.contains(call.filter) || $0.tl.contains(call.filter)
            }
            let limit = Int(call.limit)
            var offset = Int(call.offset)
            if offset >= matching.count, offset > 0 {
                offset = max(matching.count - 1, 0) / limit * limit
            }
            var page = Taigi_Engine_LearningRecords()
            page.records = Array(matching.dropFirst(offset).prefix(limit))
            page.total = UInt32(ofKind.count)
            page.matchingTotal = UInt32(matching.count)
            page.offset = UInt32(offset)
            return page
        }
    }

    private func record(
        _ id: Int64,
        _ text: String,
        tl: String = "",
        kind: Taigi_Engine_LearningRecordKind = .frequency,
    ) -> Taigi_Engine_LearningRecord {
        var record = Taigi_Engine_LearningRecord()
        record.kind = kind
        record.id = id
        record.text = text
        record.tl = tl
        record.count = 3
        return record
    }

    private func numberedRows(_ count: Int) -> [Taigi_Engine_LearningRecord] {
        (1 ... count).map { record(Int64($0), "詞\($0)") }
    }

    private func makeViewModel(
        _ fake: FakeLearningRecords,
        kind: Taigi_Engine_LearningRecordKind = .frequency,
    ) -> LearningRecordsViewModel {
        LearningRecordsViewModel(kind: kind, userData: fake, filterSettle: .zero)
    }

    /// A write's outcome and, once its reload lands, the reload's notice.
    private func landed(_ write: LearningRecordsWrite) async -> (outcome: LearningRecordsNotice?, reloadNotice: LearningRecordsNotice?) {
        await (write.outcome, write.reload.value)
    }

    /// Yields until `count` list requests are parked in `fake`.
    private func waitForParked(_ count: Int, in fake: FakeLearningRecords) async {
        for _ in 0 ..< 10000 where fake.parkedCount < count {
            await Task.yield()
        }
        XCTAssertEqual(fake.parkedCount, count, "list requests parked")
    }

    // MARK: - Paging

    func testPaging_theNextPageAppendsWhenTheLastRowShows() async {
        // trace: 250 rows, pages of 100 → offsets 0, 100, 200; the third page holds 50.
        let fake = FakeLearningRecords(rows: numberedRows(250))
        let viewModel = makeViewModel(fake)

        _ = await viewModel.load()
        XCTAssertFalse(viewModel.isLoading)
        XCTAssertEqual(viewModel.records.map(\.id), Array(1 ... 100))

        _ = await viewModel.loadNextPage()
        _ = await viewModel.loadNextPage()
        XCTAssertEqual(viewModel.records.map(\.id), Array(1 ... 250))
        XCTAssertFalse(viewModel.hasMoreRows)

        _ = await viewModel.loadNextPage()
        XCTAssertEqual(fake.calls.map(\.offset), [0, 100, 200], "nothing past the matches")
        XCTAssertEqual(fake.calls.map(\.limit), [100, 100, 100])
    }

    func testPaging_matchesShrunkUnderTheList_reloadsWhatIsListed() async {
        // trace: 150 rows → 100 listed; 60 go behind the page's back → 90 left. Offset 100
        // is past the end, so the engine serves offset 0 (last_page_offset(90, 100, 100) = 0),
        // which does not line up: the list reloads from the top, 100 rows → the 90 left.
        let fake = FakeLearningRecords(rows: numberedRows(150))
        let viewModel = makeViewModel(fake)
        _ = await viewModel.load()
        fake.rows = Array(fake.rows.prefix(90))

        _ = await viewModel.loadNextPage()

        XCTAssertEqual(fake.calls.map(\.offset), [0, 100, 0])
        XCTAssertEqual(viewModel.records.map(\.id), Array(1 ... 90))
    }

    func testThePagingKey_changesAfterAReloadThatKeepsTheLastRow() async {
        // trace: 150 rows, 100 listed, tail id 100. A reorder answers the same 100 rows
        // (the fake ignores order), so the tail row keeps its id — the key must still move.
        let fake = FakeLearningRecords(rows: numberedRows(150))
        let viewModel = makeViewModel(fake)
        _ = await viewModel.load()
        let before = viewModel.pagingKey

        fake.isHolding = true
        let reorder = viewModel.selectOrder(.mostRecent)
        // Asked while the rows answer the old order: nothing is asked.
        _ = await viewModel.loadNextPage()
        await waitForParked(1, in: fake)
        fake.resolve(0)
        _ = await reorder.value
        fake.isHolding = false

        XCTAssertEqual(viewModel.records.last?.id, 100)
        XCTAssertNotEqual(viewModel.pagingKey, before, "the sentinel task re-fires")
        _ = await viewModel.loadNextPage()
        XCTAssertEqual(viewModel.records.count, 150)
        XCTAssertEqual(fake.calls.map(\.offset), [0, 0, 100])
    }

    func testAFailedNextPage_waitsForARetry() async {
        let fake = FakeLearningRecords(rows: numberedRows(150))
        let viewModel = makeViewModel(fake)
        _ = await viewModel.load()

        fake.failure = FakeLearningRecords.Unreadable()
        let notice = await viewModel.loadNextPage()
        XCTAssertEqual(notice, .readFailed(detail: "disk I/O error"))
        XCTAssertEqual(viewModel.failedRead, .nextPage)
        XCTAssertEqual(viewModel.records.count, 100)

        let failedRetry = await viewModel.retry()
        XCTAssertEqual(failedRetry, .readFailed(detail: "disk I/O error"))
        XCTAssertEqual(viewModel.failedRead, .nextPage)

        fake.failure = nil
        let retry = await viewModel.retry()
        XCTAssertNil(retry)
        XCTAssertNil(viewModel.failedRead)
        XCTAssertEqual(viewModel.records.map(\.id), Array(1 ... 150))
        XCTAssertEqual(fake.calls.map(\.offset), [0, 100, 100, 100])
    }

    func testAFailedReReadAfterAWrite_retriesTheListedRowsNotTheNextPage() async {
        // trace: 250 rows, 200 listed (offsets 0, 100). The write lands; its re-read fails.
        // Retry re-reads the 200 listed rows as (limit 100, offset 0), (100, 100) — no offset 200.
        let fake = FakeLearningRecords(rows: numberedRows(250))
        let viewModel = makeViewModel(fake)
        _ = await viewModel.load()
        _ = await viewModel.loadNextPage()

        fake.listsFail = true
        _ = await landed(viewModel.setCount(viewModel.records[0], to: 7))
        XCTAssertEqual(viewModel.failedRead, .list)
        XCTAssertEqual(viewModel.records.count, 200, "the rows stay on screen")

        let failedRetry = await viewModel.retry()
        XCTAssertEqual(failedRetry, .readFailed(detail: "disk I/O error"))
        XCTAssertEqual(viewModel.failedRead, .list)

        fake.listsFail = false
        let before = fake.calls.count
        let retry = await viewModel.retry()
        XCTAssertNil(retry)

        let retried = fake.calls.dropFirst(before)
        XCTAssertEqual(retried.map(\.limit), [100, 100])
        XCTAssertEqual(retried.map(\.offset), [0, 100])
        XCTAssertNil(viewModel.failedRead)
        XCTAssertEqual(viewModel.records.map(\.id), Array(1 ... 200))
        XCTAssertEqual(viewModel.records[0].count, 7)

        _ = await viewModel.loadNextPage()
        XCTAssertEqual(viewModel.records.count, 250, "paging goes on")
    }

    // MARK: - Kind / order / filter

    func testKindOrderAndFilter_goToTheEngineAndListFromTheTop() async {
        let fake = FakeLearningRecords(rows: [
            record(1, "台灣", tl: "tâi-uân"),
            record(2, "食飯", tl: "tsia̍h-pn̄g"),
            record(3, "台語", tl: "tâi-gí", kind: .learnedPhrase),
            record(4, "食飽", tl: "tsia̍h-pá", kind: .learnedPhrase),
        ])
        let viewModel = makeViewModel(fake, kind: .learnedPhrase)
        _ = await viewModel.load()
        XCTAssertEqual(viewModel.records.map(\.text), ["台語", "食飽"], "the page lists its own kind only")

        _ = await viewModel.filterChanged("台").value
        XCTAssertEqual(viewModel.records.map(\.text), ["台語"])

        _ = await viewModel.selectOrder(.mostRecent).value
        XCTAssertEqual(fake.calls.first?.order, .mostUsed, "most used first by default")
        XCTAssertEqual(fake.calls.last, .init(kind: .learnedPhrase, order: .mostRecent, filter: "台", limit: 100, offset: 0))
        XCTAssertEqual(fake.calls.map(\.offset), [0, 0, 0])
    }

    func testALaterAppearance_reReadsTheListedRowsInPlace() async {
        // trace: 250 rows, 200 listed. Back from another tab the keyboard learned row 0:
        // the re-read asks (100, 0), (100, 100) and shows 200 rows, the new one first.
        let fake = FakeLearningRecords(rows: numberedRows(250))
        let viewModel = makeViewModel(fake)
        _ = await viewModel.load()
        _ = await viewModel.loadNextPage()
        fake.rows.insert(record(0, "新詞"), at: 0)

        _ = await viewModel.load()

        XCTAssertEqual(fake.calls.map(\.offset), [0, 100, 0, 100])
        XCTAssertEqual(fake.calls.map(\.limit), [100, 100, 100, 100])
        XCTAssertEqual(viewModel.records.map(\.id), Array(0 ... 199), "the place in the list is kept")
    }

    func testAReReadPulledBack_landsTheServedPage() async {
        // trace: 250 rows, 250 listed. A write's re-read gets (100, 0) → rows 1–100 of 250;
        // then the matches shrink to 20, so (100, 100) is served from offset 0 (last page of 20)
        // → the rows read from offset 0 on are replaced: 20 rows, matching 20.
        let fake = FakeLearningRecords(rows: numberedRows(250))
        let viewModel = makeViewModel(fake)
        _ = await viewModel.load()
        _ = await viewModel.loadNextPage()
        _ = await viewModel.loadNextPage()
        fake.isHolding = true

        let write = Task { await landed(viewModel.setCount(viewModel.records[0], to: 5)) }
        await waitForParked(1, in: fake)
        fake.resolve(0)
        await waitForParked(1, in: fake)
        fake.rows = Array(fake.rows.prefix(20))
        fake.resolve(0)
        _ = await write.value

        XCTAssertEqual(viewModel.records.map(\.id), Array(1 ... 20))
        XCTAssertEqual(viewModel.matchingTotal, 20)
        XCTAssertFalse(viewModel.hasMoreRows)
    }

    func testAnOrderChange_overtakesAFilterStillSettling() async {
        let fake = FakeLearningRecords(rows: [record(1, "台灣"), record(2, "食飯")])
        let viewModel = LearningRecordsViewModel(kind: .frequency, userData: fake, filterSettle: .seconds(60))
        _ = await viewModel.load()

        let settling = viewModel.filterChanged("台")
        _ = await viewModel.selectOrder(.mostRecent).value
        _ = await settling.value

        XCTAssertEqual(fake.calls.count, 2, "the settling filter does not reload again")
        XCTAssertEqual(fake.calls.last?.filter, "台")
        XCTAssertEqual(viewModel.records.map(\.text), ["台灣"])
    }

    func testANewerRequestWins_anOlderAnswerIsDropped() async {
        let fake = FakeLearningRecords(rows: [record(1, "台灣"), record(2, "食飯")])
        let viewModel = makeViewModel(fake)
        _ = await viewModel.load()
        fake.isHolding = true

        let older = viewModel.filterChanged("台")
        await waitForParked(1, in: fake)
        let newer = viewModel.filterChanged("食")
        await waitForParked(2, in: fake)

        // The newer (食) answer lands first; the older (台) one after it.
        fake.resolve(1)
        _ = await newer.value
        fake.resolve(0)
        _ = await older.value

        XCTAssertEqual(viewModel.records.map(\.text), ["食飯"], "the 台 answer is stale")
    }

    func testAStaleAnswer_earnsNoNotice() async {
        let fake = FakeLearningRecords(rows: [record(1, "台灣"), record(2, "食飯")])
        let viewModel = makeViewModel(fake)
        _ = await viewModel.load()
        fake.isHolding = true

        let older = viewModel.filterChanged("台")
        await waitForParked(1, in: fake)
        let newer = viewModel.filterChanged("食")
        await waitForParked(2, in: fake)

        fake.fail(0)
        let olderNotice = await older.value
        XCTAssertNil(olderNotice, "the 台 failure is stale")
        XCTAssertNil(viewModel.failedRead)
        fake.resolve(0)
        let newerNotice = await newer.value
        XCTAssertNil(newerNotice)
        XCTAssertEqual(viewModel.records.map(\.text), ["食飯"])
    }

    func testAFilterOvertakenWhileSettling_earnsNoNotice() async {
        let fake = FakeLearningRecords(rows: [record(1, "台灣"), record(2, "食飯")])
        let viewModel = makeViewModel(fake)
        _ = await viewModel.load()

        let first = viewModel.filterChanged("台")
        let second = viewModel.filterChanged("食")
        let firstNotice = await first.value
        XCTAssertNil(firstNotice)
        _ = await second.value
        XCTAssertEqual(fake.calls.map(\.filter), ["", "食"], "the 台 filter never reached the engine")
    }

    func testAFilterChange_dropsTheAnswerAlreadyInFlight() async {
        let fake = FakeLearningRecords(rows: [record(1, "台灣"), record(2, "食飯")])
        let viewModel = makeViewModel(fake)
        fake.isHolding = true

        let unfiltered = viewModel.selectOrder(.mostRecent)
        await waitForParked(1, in: fake)
        // Stale at the keystroke, before the box settles.
        let filtered = viewModel.filterChanged("食")
        fake.resolve(0)
        _ = await unfiltered.value
        XCTAssertTrue(viewModel.records.isEmpty, "the unfiltered answer is dropped")

        await waitForParked(1, in: fake)
        fake.resolve(0)
        _ = await filtered.value
        XCTAssertEqual(viewModel.records.map(\.text), ["食飯"])
    }

    func testAnOrderChange_isNotGivenTheOldListsNextPage() async {
        let fake = FakeLearningRecords(rows: numberedRows(150))
        let viewModel = makeViewModel(fake)
        _ = await viewModel.load()
        fake.isHolding = true

        let reorder = viewModel.selectOrder(.mostRecent)
        _ = await viewModel.loadNextPage()
        await waitForParked(1, in: fake)
        fake.resolve(0)
        _ = await reorder.value

        XCTAssertEqual(fake.calls.map(\.offset), [0, 0], "no next page while the old order is listed")
        XCTAssertEqual(viewModel.records.count, 100)
    }

    // MARK: - Writes

    func testSetCountAndDelete_reloadTheListedRows() async {
        let fake = FakeLearningRecords(rows: numberedRows(3))
        let viewModel = makeViewModel(fake)
        _ = await viewModel.load()

        let setCount = await landed(viewModel.setCount(viewModel.records[0], to: 40))
        XCTAssertEqual(viewModel.records.map(\.count), [40, 3, 3])

        let delete = await landed(viewModel.delete(viewModel.records[1]))
        XCTAssertEqual(viewModel.records.map(\.id), [1, 3])
        XCTAssertNil(setCount.outcome)
        XCTAssertNil(setCount.reloadNotice)
        XCTAssertNil(delete.outcome)
        XCTAssertNil(delete.reloadNotice)
        // trace: the reloads ask for max(100, rows listed) = 100 from the top.
        XCTAssertEqual(fake.calls.map(\.limit), [100, 100, 100])
    }

    func testAWrite_reReadsTheListedRowsInPagesOfAtMost100() async {
        // trace: 250 rows listed after offsets 0, 100, 200. The reload re-reads
        // max(100, 250) = 250 rows as limits 100, 100, 50 from offsets 0, 100, 200.
        let fake = FakeLearningRecords(rows: numberedRows(250))
        let viewModel = makeViewModel(fake)
        _ = await viewModel.load()
        _ = await viewModel.loadNextPage()
        _ = await viewModel.loadNextPage()

        _ = await landed(viewModel.setCount(viewModel.records[200], to: 9))

        let reload = fake.calls.dropFirst(3)
        XCTAssertEqual(reload.map(\.offset), [0, 100, 200])
        XCTAssertEqual(reload.map(\.limit), [100, 100, 50])
        XCTAssertEqual(viewModel.records.map(\.id), Array(1 ... 250))
        XCTAssertEqual(viewModel.records[200].count, 9)
    }

    func testARowAlreadyGone_isANoticeAndTheListReloads() async {
        let fake = FakeLearningRecords(rows: numberedRows(2))
        let viewModel = makeViewModel(fake)
        _ = await viewModel.load()
        let listed = viewModel.records[0]
        fake.rows = [fake.rows[1]]

        let delete = await landed(viewModel.delete(listed))

        XCTAssertEqual(delete.outcome, .gone)
        XCTAssertNil(delete.reloadNotice)
        XCTAssertEqual(viewModel.records.map(\.id), [2])
    }

    func testFailures_carryTheEngineDetail() async {
        let fake = FakeLearningRecords(rows: numberedRows(2))
        let viewModel = makeViewModel(fake)
        _ = await viewModel.load()
        fake.failure = FakeLearningRecords.Unreadable()

        let write = await landed(viewModel.setCount(viewModel.records[0], to: 5))
        XCTAssertEqual(write.outcome, .writeFailed(detail: "disk I/O error"))
        XCTAssertEqual(write.reloadNotice, .readFailed(detail: "disk I/O error"))
        XCTAssertEqual(viewModel.records.map(\.id), [1, 2], "a failed read keeps the rows on screen")

        let reorderNotice = await viewModel.selectOrder(.mostRecent).value
        XCTAssertEqual(reorderNotice, .readFailed(detail: "disk I/O error"))
        XCTAssertEqual(viewModel.records.map(\.id), [1, 2])
    }

    func testAnUnreadableStore_isNotNothingLearned() async {
        let fake = FakeLearningRecords(rows: numberedRows(2))
        fake.failure = FakeLearningRecords.Unreadable()
        let viewModel = makeViewModel(fake)

        let notice = await viewModel.load()
        XCTAssertEqual(notice, .readFailed(detail: "disk I/O error"))
        XCTAssertTrue(viewModel.records.isEmpty)
        XCTAssertEqual(viewModel.failedRead, .list, "could not read ≠ nothing learned yet")

        fake.failure = nil
        _ = await viewModel.selectOrder(.mostRecent).value
        XCTAssertNil(viewModel.failedRead)
        XCTAssertEqual(viewModel.records.map(\.id), [1, 2])
    }

    // MARK: - Add to the custom dictionary

    func testAddingAPhrase_saysItLandedAndTheReloadDropsThePhrase() async {
        let fake = FakeLearningRecords(rows: [
            record(1, "台語", tl: "tâi-gí", kind: .learnedPhrase),
            record(2, "食飽", tl: "tsia̍h-pá", kind: .learnedPhrase),
        ])
        let viewModel = makeViewModel(fake, kind: .learnedPhrase)
        _ = await viewModel.load()

        let add = await landed(viewModel.addToCustomDictionary(viewModel.records[0]))

        XCTAssertEqual(add.outcome, .addedToCustomDictionary)
        XCTAssertEqual(fake.added.map(\.text), ["台語"])
        XCTAssertEqual(viewModel.records.map(\.id), [2])
        XCTAssertEqual(fake.calls.map(\.offset), [0, 0], "the add reloads the listed rows")
    }

    func testAddingAFrequencyRow_saysItLandedAndKeepsTheRow() async {
        let fake = FakeLearningRecords(rows: [
            record(1, "台語", tl: "tâi-gí", kind: .frequency),
            record(2, "食飽", tl: "tsia̍h-pá", kind: .frequency),
        ])
        let viewModel = makeViewModel(fake, kind: .frequency)
        _ = await viewModel.load()

        let add = await landed(viewModel.addToCustomDictionary(viewModel.records[0]))

        XCTAssertEqual(add.outcome, .addedToCustomDictionary)
        XCTAssertEqual(fake.added.map(\.text), ["台語"])
        XCTAssertEqual(viewModel.records.map(\.id), [1, 2], "a frequency row still weights its word")
    }

    func testARefusedAdd_isAWriteFailureAndKeepsThePhrase() async {
        let fake = FakeLearningRecords(rows: [record(1, "台語", tl: "tâi-gí", kind: .learnedPhrase)])
        let viewModel = makeViewModel(fake, kind: .learnedPhrase)
        _ = await viewModel.load()
        fake.addRefusal = "custom dictionary is full"

        let add = await landed(viewModel.addToCustomDictionary(viewModel.records[0]))

        XCTAssertEqual(add.outcome, .writeFailed(detail: "custom dictionary is full"))
        XCTAssertNil(add.reloadNotice)
        XCTAssertTrue(fake.added.isEmpty)
        XCTAssertEqual(viewModel.records.map(\.id), [1])
        XCTAssertEqual(fake.calls.count, 2, "a refused add still reloads")
    }

    // MARK: - Notices

    func testANoticeArriving_replacesTheOneUpUnlessItIsAReadFailure() {
        let readFailed = LearningRecordsNotice.readFailed(detail: "disk I/O error")
        let writeFailed = LearningRecordsNotice.writeFailed(detail: "disk I/O error")

        XCTAssertEqual(readFailed.arriving(over: nil), readFailed)
        XCTAssertEqual(readFailed.arriving(over: writeFailed), writeFailed, "the write's notice outlives its reload's")
        XCTAssertEqual(readFailed.arriving(over: .gone), .gone)
        XCTAssertEqual(writeFailed.arriving(over: readFailed), writeFailed)
        XCTAssertEqual(LearningRecordsNotice.gone.arriving(over: .addedToCustomDictionary), .gone)
    }

    // MARK: - Edit field and labels

    func testCountFromText_isAWholeNumberOfAtLeastOne() {
        let cases: [(String, Int64?)] = [
            ("12", 12), (" 3 ", 3), ("1", 1), ("1000000", 1_000_000), ("1000001", nil),
            ("0", nil), ("-1", nil), ("1.5", nil), ("", nil), ("abc", nil),
        ]
        for (text, expected) in cases {
            XCTAssertEqual(LearningRecordsViewModel.count(from: text), expected, "count(from: \"\(text)\")")
        }
    }

    func testLastUsedLabel_isEmptyWithoutAReadableTime() {
        XCTAssertEqual(record(1, "台灣", tl: "tâi-uân").lastUsedLabel, "")
    }
}

private extension Taigi_Engine_LearningRecord {
    /// The engine's identity guard: the id AND the identity columns.
    func isSameRow(as other: Self) -> Bool {
        kind == other.kind && id == other.id && text == other.text && tl == other.tl
    }
}
