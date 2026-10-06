@testable import TaigiInputMethodCore
import XCTest

/// The Learning Records page over an in-memory store that pages, filters,
/// orders and matches rows the way the engine does (`FakeUserDataClient`).
/// What the engine itself does with a request is `engine/userdata`'s subject.
@MainActor
final class LearningRecordsPageTests: XCTestCase {
    private func record(
        _ kind: Taigi_Engine_LearningRecordKind,
        id: Int64,
        count: Int64 = 1,
        lastUsedMs: Int64 = 0,
    ) -> Taigi_Engine_LearningRecord {
        var record = Taigi_Engine_LearningRecord()
        record.kind = kind
        record.id = id
        record.text = "字\(id)"
        record.tl = "ji\(id)"
        record.count = count
        record.lastUsedMs = lastUsedMs
        return record
    }

    private func makeModel(_ records: [Taigi_Engine_LearningRecord]) -> (LearningRecordsPageModel, FakeUserDataClient) {
        let client = FakeUserDataClient()
        client.seedLearningRecords(records)
        return (LearningRecordsPageModel(client: client), client)
    }

    /// The pickers' values are what the engine is asked for — word frequency
    /// first, most used first, as the page opens.
    func testLoad_sendsTheKindOrderAndFilterPickedNow() async throws {
        let (model, client) = makeModel([])

        await model.load()
        XCTAssertEqual(client.lastLearningRecordsQuery?.kind, .frequency)
        XCTAssertEqual(client.lastLearningRecordsQuery?.order, .mostUsed)

        model.kind = .learnedPhrase
        model.order = .mostRecent
        model.filter = "ji"
        await model.loadFirstPage()
        XCTAssertEqual(client.lastLearningRecordsQuery?.kind, .learnedPhrase)
        XCTAssertEqual(client.lastLearningRecordsQuery?.order, .mostRecent)
        XCTAssertEqual(client.lastLearningRecordsQuery?.filter, "ji")
    }

    /// One store's rows only: a phrase never shows under word frequency.
    func testRows_comeFromTheKindPicked() async throws {
        let (model, _) = makeModel([record(.frequency, id: 1), record(.learnedPhrase, id: 2)])

        await model.load()
        XCTAssertEqual(model.list.rows.map(\.id), [1])

        model.kind = .learnedPhrase
        await model.loadFirstPage()
        XCTAssertEqual(model.list.rows.map(\.id), [2])
        XCTAssertEqual(model.list.countLabel, "1")
    }

    /// Another kind, order or filter is another list: its first page.
    func testAChangeOfKind_startsAtPageOne() async throws {
        let size = UserDataListMetrics.pageSize
        let (model, _) = makeModel((0 ..< Int64(size + 1)).map { record(.frequency, id: $0) })
        await model.load()
        await model.pageForward()
        XCTAssertEqual(model.list.page, 1)

        model.kind = .learnedPhrase
        await model.loadFirstPage()

        XCTAssertEqual(model.list.page, 0)
        XCTAssertTrue(model.list.rows.isEmpty)
    }

    func testOrder_mostUsedThenMostRecent() async throws {
        let (model, _) = makeModel([
            record(.frequency, id: 1, count: 9, lastUsedMs: 1000),
            record(.frequency, id: 2, count: 1, lastUsedMs: 2000),
        ])

        await model.load()
        XCTAssertEqual(model.list.rows.map(\.id), [1, 2])

        model.order = .mostRecent
        await model.loadFirstPage()
        XCTAssertEqual(model.list.rows.map(\.id), [2, 1])
    }

    func testSetCount_storesItAndReloads() async throws {
        let (model, _) = makeModel([record(.frequency, id: 1, count: 3)])
        await model.load()

        try await model.setCount(of: XCTUnwrap(model.list.rows.first), to: 40)

        XCTAssertEqual(model.list.rows.first?.count, 40)
        XCTAssertNil(model.message)
        XCTAssertFalse(model.activity.isWorking)
    }

    func testDelete_removesTheRowAndReloads() async throws {
        let (model, _) = makeModel([record(.frequency, id: 1), record(.frequency, id: 2)])
        await model.load()

        try await model.delete(XCTUnwrap(model.list.rows.first { $0.id == 1 }))

        XCTAssertEqual(model.list.rows.map(\.id), [2])
        XCTAssertEqual(model.list.countLabel, "1")
        XCTAssertNil(model.message)
    }

    /// The phrase lands in the custom dictionary, leaves this list, and the
    /// add is said.
    func testAddToCustomDictionary_filesThePhraseAndReloadsWithoutIt() async throws {
        let (model, client) = makeModel([record(.learnedPhrase, id: 1), record(.learnedPhrase, id: 2)])
        model.kind = .learnedPhrase
        await model.loadFirstPage()

        try await model.addToCustomDictionary(XCTUnwrap(model.list.rows.first { $0.id == 1 }))

        XCTAssertEqual(model.message, .done(.dictionaryLearningRecordsAddedToCustomDictionary))
        XCTAssertEqual(model.list.rows.map(\.id), [2])
        XCTAssertEqual(model.list.countLabel, "1")
        let custom = try client.list(filter: "", limit: 10, offset: 0).rows
        XCTAssertEqual(custom.map(\.roman), ["ji1"])
        XCTAssertEqual(custom.map(\.hanji), ["字1"])
        XCTAssertFalse(model.activity.isWorking)
    }

    /// A frequency row's word lands in the custom dictionary and the add is
    /// said; the row stays listed, still weighting its word.
    func testAddToCustomDictionary_filesTheWordAndKeepsTheFrequencyRow() async throws {
        let (model, client) = makeModel([record(.frequency, id: 1), record(.frequency, id: 2)])
        await model.loadFirstPage()

        try await model.addToCustomDictionary(XCTUnwrap(model.list.rows.first { $0.id == 1 }))

        XCTAssertEqual(model.message, .done(.dictionaryLearningRecordsAddedToCustomDictionary))
        XCTAssertEqual(model.list.rows.map(\.id), [1, 2])
        let custom = try client.list(filter: "", limit: 10, offset: 0).rows
        XCTAssertEqual(custom.map(\.hanji), ["字1"])
    }

    /// A refusal — a full dictionary — is a write failure, and the row stays.
    func testAddToCustomDictionary_refused_isAWriteFailureAndKeepsTheRow() async throws {
        let (model, client) = makeModel([record(.learnedPhrase, id: 1)])
        client.refusesLearningRecordAdds = true
        model.kind = .learnedPhrase
        await model.loadFirstPage()

        try await model.addToCustomDictionary(XCTUnwrap(model.list.rows.first))

        guard case .failure(.dictionaryLearningRecordsWriteFailed, _)? = model.message else {
            return XCTFail("expected the write-failed alert, got \(String(describing: model.message))")
        }
        XCTAssertEqual(model.list.rows.map(\.id), [1])
    }

    /// A row gone since it was listed is said as such — not a failure — and
    /// the list is reloaded so the row stops showing.
    func testAWriteToARowAlreadyGone_saysItIsGone() async throws {
        let (model, client) = makeModel([record(.frequency, id: 1)])
        await model.load()
        let listed = try XCTUnwrap(model.list.rows.first)
        client.seedLearningRecords([])

        await model.setCount(of: listed, to: 5)
        XCTAssertEqual(model.message, .done(.dictionaryLearningRecordGone))
        XCTAssertTrue(model.list.rows.isEmpty)

        model.message = nil
        await model.delete(listed)
        XCTAssertEqual(model.message, .done(.dictionaryLearningRecordGone))
    }

    /// "Could not be read" is an alert; "nothing learned yet" is an empty
    /// table and no alert.
    func testAnUnreadableStore_alertsWhereAnEmptyOneDoesNot() async throws {
        let (model, client) = makeModel([])

        await model.load()
        XCTAssertNil(model.message)

        client.failsLearningRecordReads = true
        await model.load()
        guard case .failure(.dictionaryLearningRecordsReadFailed, _)? = model.message else {
            return XCTFail("expected the read-failed alert, got \(String(describing: model.message))")
        }
    }

    /// Starts a load that is held in the store, changes `change`, then lets
    /// the load return — the filter's own load waits for it to settle, so
    /// none has started in between.
    private func returnAnOldLoad(
        _ model: LearningRecordsPageModel,
        _ client: FakeUserDataClient,
        after change: () -> Void,
    ) async {
        let gate = DispatchSemaphore(value: 0)
        let entered = expectation(description: "the old load reached the store")
        client.learningRecordsListGate = gate
        client.onLearningRecordsListEntered = { entered.fulfill() }
        let inFlight = Task { await model.load() }
        await fulfillment(of: [entered], timeout: 5)

        change()
        gate.signal()
        await inFlight.value
    }

    /// The old list's rows never land once the filter has changed.
    func testAFilterChange_dropsTheRowsOfALoadAlreadyInFlight() async throws {
        let (model, client) = makeModel([record(.frequency, id: 1)])

        await returnAnOldLoad(model, client) { model.filter = "nothing" }

        XCTAssertTrue(model.list.rows.isEmpty, "rows of the old filter landed")
    }

    /// Nor does its failure: the user has moved past that list.
    func testAFilterChange_dropsTheFailureOfALoadAlreadyInFlight() async throws {
        let (model, client) = makeModel([])
        client.failsLearningRecordReads = true

        await returnAnOldLoad(model, client) { model.filter = "nothing" }

        XCTAssertNil(model.message, "the old filter's failure was raised")
    }

    func testAKindOrOrderChange_dropsALoadAlreadyInFlight() async throws {
        let (model, client) = makeModel([record(.frequency, id: 1)])

        await returnAnOldLoad(model, client) { model.kind = .learnedPhrase }
        XCTAssertTrue(model.list.rows.isEmpty)

        await returnAnOldLoad(model, client) { model.order = .mostRecent }
        XCTAssertTrue(model.list.rows.isEmpty)
    }

    // INVARIANT_USER_DATA_LIST_FILTER_RELOAD_SELECTION (§58): desktop-core
    // `Listing::rewind`. Row ids are per store, so the selection goes with
    // the kind; a new order is a new list.
    func testAChangeOfKindOrOrder_dropsTheSelection() async throws {
        let (model, _) = makeModel([record(.frequency, id: 1)])
        await model.load()

        model.selectedRowID = 1
        model.kind = .learnedPhrase
        XCTAssertNil(model.selectedRowID)

        model.kind = .frequency
        await model.loadFirstPage()
        model.selectedRowID = 1
        model.order = .mostRecent
        XCTAssertNil(model.selectedRowID)
    }

    /// The "above 40 ranks the same" note belongs to word frequency alone.
    func testCountNote_isForWordFrequencyOnly() {
        XCTAssertEqual(LearningRecordsPageModel.countNoteKey(for: .frequency), .dictionaryLearningRecordsCountCapInfo)
        XCTAssertNil(LearningRecordsPageModel.countNoteKey(for: .learnedPhrase))
    }

    func testLastUsedLabel_isTheDayOrEmpty() throws {
        let utc = try XCTUnwrap(TimeZone(identifier: "UTC"))
        // trace: 1_759_449_600_000 ms = 2025-10-03T00:00:00Z
        XCTAssertEqual(LearningRecordsPageModel.lastUsedLabel(1_759_449_600_000, timeZone: utc), "2025-10-03")
        XCTAssertEqual(LearningRecordsPageModel.lastUsedLabel(0, timeZone: utc), "")
    }

    // MARK: - Delete Learning Records

    /// Asking runs nothing; Delete empties every store — whichever kind is on
    /// screen — says so, and reloads to the empty list.
    func testConfirmingClearAll_emptiesEveryStoreAndReloads() async throws {
        let (model, client) = makeModel([record(.frequency, id: 1), record(.learnedPhrase, id: 2)])
        await model.load()

        model.askClearAll()
        XCTAssertTrue(model.isConfirmingClearAll)
        XCTAssertEqual(client.learningRecordClearCount, 0, "asking runs nothing")
        await model.confirmClearAll()?.value

        XCTAssertFalse(model.isConfirmingClearAll)
        XCTAssertEqual(client.learningRecordClearCount, 1)
        XCTAssertEqual(model.message, .done(.dictionaryClearLearningRecordsDone))
        XCTAssertTrue(model.list.rows.isEmpty, "the list reloaded")
        XCTAssertEqual(model.activity, .idle, "the clear gives its slot back")
        model.kind = .learnedPhrase
        await model.load()
        XCTAssertTrue(model.list.rows.isEmpty, "the other kind went too")
    }

    func testAFailedClearAll_isReportedAndStillReloads() async throws {
        let (model, client) = makeModel([record(.frequency, id: 1)])
        client.failsLearningRecordClears = true

        model.askClearAll()
        await model.confirmClearAll()?.value

        guard case .failure(.dictionaryClearLearningRecordsFailed, _)? = model.message else {
            return XCTFail("expected the clear-failed alert, got \(String(describing: model.message))")
        }
        XCTAssertEqual(model.list.rows.map(\.id), [1], "reloaded to what the store still holds")
    }

    /// Cancel is the dialog writing false through its binding; the flag is
    /// taken when Delete is clicked, so it runs once however often answered.
    func testClearAll_runsOnlyOnceConfirmedAndOnlyOnce() async throws {
        let (model, client) = makeModel([record(.frequency, id: 1)])

        model.askClearAll()
        model.isConfirmingClearAll = false
        XCTAssertNil(model.confirmClearAll(), "nothing pending once cancelled")

        model.askClearAll()
        let run = model.confirmClearAll()
        XCTAssertNil(model.confirmClearAll(), "already taken")
        await run?.value

        XCTAssertEqual(client.learningRecordClearCount, 1)
    }

    func testABusyPage_doesNotAskToClearAll() throws {
        let (model, _) = makeModel([])
        XCTAssertTrue(model.beginWork())

        model.askClearAll()

        XCTAssertFalse(model.isConfirmingClearAll)
    }

    /// Something took the slot after the dialog went up: the confirmed clear
    /// is refused, and leaves the slot with its holder.
    func testAConfirmedClearAll_doesNothingWhileAnotherHoldsTheSlot() async throws {
        let (model, client) = makeModel([record(.frequency, id: 1)])

        model.askClearAll()
        XCTAssertTrue(model.beginWork())
        await model.confirmClearAll()?.value

        XCTAssertEqual(client.learningRecordClearCount, 0)
        XCTAssertEqual(model.activity, .working(.desktopProgressWorking))
    }
}

/// The paged list both user-data pages keep: a load started before another
/// never lands over it, and the selection is only ever a row on screen.
final class UserDataPagedListTests: XCTestCase {
    /// A row is anything with an identity; the pages' own rows are the
    /// models' subject.
    private struct Row: Identifiable, Equatable {
        let id: Int
    }

    private func listing(_ ids: [Int], offset: Int = 0) -> UserDataListing<Row> {
        UserDataListing(rows: ids.map(Row.init), total: 20, matchingTotal: 20, offset: offset)
    }

    func testAnOlderLoad_neverLandsOverANewerOne() {
        var list = UserDataPagedList<Row>()
        let older = list.beginLoad()
        let newer = list.beginLoad()

        list.land(listing([2]), from: newer)
        list.land(listing([1]), from: older)

        XCTAssertEqual(list.rows, [Row(id: 2)])
        XCTAssertFalse(list.isCurrent(older))
        XCTAssertTrue(list.isCurrent(newer))
    }

    func testInvalidate_makesALoadInFlightStaleWithoutStartingOne() {
        var list = UserDataPagedList<Row>()
        let load = list.beginLoad()

        list.invalidate()
        list.land(listing([1]), from: load)

        XCTAssertFalse(list.isCurrent(load))
        XCTAssertTrue(list.rows.isEmpty)
    }

    // INVARIANT_USER_DATA_LIST_FILTER_RELOAD_SELECTION (§58): desktop-core
    // `an_adopted_page_is_the_engines_and_drops_an_off_page_selection`.
    func testASelection_staysWhileOnScreenAndIsNotRestoredOnceDropped() {
        var list = UserDataPagedList<Row>()
        list.land(listing([1, 2]), from: list.beginLoad())
        list.selectedID = 1

        list.land(listing([2, 1]), from: list.beginLoad())
        XCTAssertEqual(list.selectedRow, Row(id: 1), "an on-page selection stays, at its new index")

        list.land(listing([3, 4], offset: UserDataListMetrics.pageSize), from: list.beginLoad())
        XCTAssertNil(list.selectedID, "off the page, nothing is selected")

        list.land(listing([1, 2]), from: list.beginLoad())
        XCTAssertNil(list.selectedID, "back on its page, it does not come back")
    }

    func testAStaleLoad_leavesTheSelectionAlone() {
        var list = UserDataPagedList<Row>()
        list.land(listing([1, 2]), from: list.beginLoad())
        list.selectedID = 1
        let stale = list.beginLoad()
        list.invalidate()

        list.land(listing([3, 4]), from: stale)

        XCTAssertEqual(list.selectedID, 1)
    }
}
