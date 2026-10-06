import AppKit
@testable import TaigiInputMethodCore
import XCTest

/// The Dictionary pages hold what happened, not how to say it — an import running
/// across a display-language change has to report itself in the language in
/// force when the alert draws, not the one in force when it started.
///
/// What each test pins is which message routes to which key. The keys' own
/// values are `StringResolverTests`' subject, so where a literal here would
/// merely restate one, the accessor is compared against instead.
final class UserDataPageChromeTests: XCTestCase {
    private let hanji = StringResolver(.hanji)
    private let english = StringResolver(.english)

    func testActivity_carriesAKeyRatherThanAResolvedLabel() {
        XCTAssertEqual(UserDataPageActivity.working(.desktopProgressWorking).labelKey, .desktopProgressWorking)
        XCTAssertNil(UserDataPageActivity.idle.labelKey)
        XCTAssertTrue(UserDataPageActivity.working(.desktopProgressWorking).isWorking)
        XCTAssertFalse(UserDataPageActivity.idle.isWorking)
    }

    func testSameMessage_resolvesInWhicheverLanguageIsAskedFor() {
        let message = UserDataPageMessage.imported(12, skipped: 3)

        XCTAssertEqual(message.title(hanji), hanji.resolve(.desktopImportComplete))
        XCTAssertEqual(message.title(english), english.resolve(.desktopImportComplete))
        XCTAssertEqual(message.detail(hanji), hanji.dictionaryImportResult(imported: 12, skipped: 3))
        XCTAssertEqual(message.detail(english), english.dictionaryImportResult(imported: 12, skipped: 3))
        XCTAssertNotEqual(message.title(hanji), message.title(english))
    }

    /// The store's error text is a SQLite or file-system condition, not
    /// something the product has wording for, so it is reported verbatim under
    /// a title that is translated.
    func testFailure_translatesTheTitleAndKeepsTheDiagnosticVerbatim() {
        struct StoreError: Error, CustomStringConvertible {
            let description = "disk I/O error"
        }
        let message = UserDataPageMessage.failure(.desktopCustomDictWriteFailed, StoreError())

        XCTAssertEqual(message.title(hanji), hanji.resolve(.desktopCustomDictWriteFailed))
        XCTAssertEqual(message.detail(hanji), "disk I/O error")
        XCTAssertEqual(message.detail(english), "disk I/O error")
    }

    /// A file that is not UTF-8 is a different refusal from a malformed CSV,
    /// and says so rather than reusing the parser's message.
    func testNotUTF8_reportsTheEncodingRatherThanTheCSVShape() {
        let message = UserDataPageMessage.notUTF8

        XCTAssertEqual(message.title(hanji), hanji.resolve(.commonImportFailed))
        XCTAssertEqual(message.detail(hanji), hanji.resolve(.desktopNotUTF8Detail))
    }
}

/// A receipt with nothing to add draws a title and no body — an alert with an
/// empty line under its title reads as a message that failed to load.
///
/// It exists because two `.alert` modifiers on one view chain do not stack:
/// the Delete Learning Records receipt had an alert of its own further down the chain and
/// SwiftUI kept the other one, so no message ever appeared (USER 2026-08-26).
/// Every page message goes through the one channel now, and this case is what
/// let the title-only receipt join it.
final class UserDataPageDoneMessageTests: XCTestCase {
    private let hanji = StringResolver(.hanji)

    func testADoneMessage_hasATitleAndNoBody() {
        let message = UserDataPageMessage.done(.dictionaryClearLearningRecordsDone)

        XCTAssertEqual(message.title(hanji), hanji.resolve(.dictionaryClearLearningRecordsDone))
        XCTAssertNil(message.detail(hanji))
    }

    /// The cases that DO carry a body keep it — the nil above must not be what
    /// every message answers.
    func testAFailureMessage_stillCarriesItsDiagnostic() {
        let message = UserDataPageMessage.failure(
            .dictionaryClearLearningRecordsFailed, diagnostic: "disk I/O error",
        )

        XCTAssertEqual(message.detail(hanji), "disk I/O error")
    }
}

/// The custom-dictionary page's one work slot.
/// The custom-dictionary page's one work slot.
///
/// The view greys its controls too, but only after a delay, so this is what
/// actually keeps two actions from overlapping — including a second CSV
/// import started while the first is still parsing behind a closed file panel.
@MainActor
final class CustomDictionaryWorkSlotTests: XCTestCase {
    private func makeModel() -> CustomDictionaryPageModel {
        CustomDictionaryPageModel(client: FakeUserDataClient())
    }

    /// A symbol name that stops resolving renders a blank rectangle and says
    /// nothing about it.
    func testTheEmptyStateSymbol_resolves() {
        XCTAssertNotNil(
            NSImage(systemSymbolName: UserDataListMetrics.emptyStateSymbolName, accessibilityDescription: nil),
        )
    }

    func testBeginWork_refusesASecondClaimWhileTheFirstIsHeld() throws {
        let model = makeModel()

        XCTAssertTrue(model.beginWork(.desktopProgressWorking))
        XCTAssertFalse(
            model.beginWork(.desktopProgressWorking),
            "a second action must not start while one is running",
        )
        XCTAssertEqual(model.activity, .working(.desktopProgressWorking), "the first action keeps the slot")
    }

    /// A real action, start to finish: it takes the slot on the way in and
    /// gives it back on the way out, whether the store answered or failed.
    func testAnAction_leavesTheSlotFreeWhenItEnds() async throws {
        let model = makeModel()

        await model.deleteAll()

        XCTAssertEqual(model.activity, .idle)
        XCTAssertTrue(model.beginWork(.desktopProgressWorking))
    }

    /// And an action that arrives while the slot is held does not run at all.
    func testAnAction_doesNothingWhileAnotherHoldsTheSlot() async throws {
        let model = makeModel()
        XCTAssertTrue(model.beginWork(.desktopProgressWorking))

        await model.deleteAll()

        XCTAssertEqual(
            model.activity, .working(.desktopProgressWorking),
            "the refused action must not release the slot it never took",
        )
    }
}

/// Delete All asks first, as on Windows and Linux (desktop-core
/// `Confirmation`): only Delete runs it; Cancel, Escape and a dismissal — all
/// of which set `isConfirmingDeleteAll` back to false — run nothing.
@MainActor
final class CustomDictionaryDeleteAllConfirmationTests: XCTestCase {
    private func makeModel(rows: Int = 0) async -> CustomDictionaryPageModel {
        let model = CustomDictionaryPageModel(client: FakeUserDataClient())
        for index in 0 ..< rows {
            await model.save(CustomDictionaryRow(roman: "row\(index)", hanji: "字\(index)"))
        }
        return model
    }

    func testAsking_runsNothing() async {
        let model = await makeModel(rows: 2)

        model.askDeleteAll()

        XCTAssertTrue(model.isConfirmingDeleteAll)
        XCTAssertEqual(model.list.matchCount, 2)
        XCTAssertEqual(model.activity, .idle)
    }

    func testConfirming_emptiesTheDictionary() async {
        let model = await makeModel(rows: 2)

        model.askDeleteAll()
        await model.confirmDeleteAll()?.value

        XCTAssertFalse(model.isConfirmingDeleteAll)
        XCTAssertEqual(model.list.matchCount, 0)
        XCTAssertEqual(model.activity, .idle)
    }

    /// Cancel is the dialog writing false through its binding; nothing is
    /// left for Delete to run.
    func testCancelling_runsNothing() async {
        let model = await makeModel(rows: 2)

        model.askDeleteAll()
        model.isConfirmingDeleteAll = false
        XCTAssertNil(model.confirmDeleteAll(), "nothing pending once cancelled")
        await model.load()

        XCTAssertEqual(model.list.matchCount, 2)
    }

    /// The command is taken when Delete is clicked, so the dismissal that
    /// follows it cannot take it away — and it runs once however often
    /// Delete is answered.
    func testConfirming_takesTheCommandBeforeTheDialogDismisses() async {
        let model = await makeModel(rows: 1)

        model.askDeleteAll()
        let run = model.confirmDeleteAll()
        model.isConfirmingDeleteAll = false
        XCTAssertNil(model.confirmDeleteAll(), "already taken")
        await run?.value

        XCTAssertEqual(model.list.matchCount, 0)
    }

    func testABusyPage_doesNotAsk() async {
        let model = await makeModel()
        XCTAssertTrue(model.beginWork(.desktopProgressWorking))

        model.askDeleteAll()

        XCTAssertFalse(model.isConfirmingDeleteAll)
    }

    /// Something took the slot after the dialog went up: the confirmed
    /// command is refused, and leaves the slot with its holder.
    func testAConfirmedCommand_doesNothingWhileAnotherHoldsTheSlot() async {
        let model = await makeModel(rows: 1)

        model.askDeleteAll()
        XCTAssertTrue(model.beginWork(.desktopProgressWorking))
        await model.confirmDeleteAll()?.value

        XCTAssertEqual(model.activity, .working(.desktopProgressWorking))
        XCTAssertEqual(model.list.matchCount, 1)
    }
}

/// Paging the Custom Dictionary list.
///
/// The reported failure: 17000 entries, and the list would not scroll — a flat
/// `LIMIT 100` in a fixed-height `Table` inside a `Form` put one scroll view
/// inside another, and rows past the hundredth were unreachable by anything
/// but the filter (USER 2026-08-26, real device). A page that FITS the table
/// has neither problem.
@MainActor
final class CustomDictionaryPagingTests: XCTestCase {
    /// Over an in-memory dictionary that pages the way the engine does.
    private func makeModel() -> CustomDictionaryPageModel {
        CustomDictionaryPageModel(client: FakeUserDataClient())
    }

    private func seed(_ model: CustomDictionaryPageModel, count: Int) async {
        for index in 0 ..< count {
            await model.save(CustomDictionaryRow(roman: "row\(index)", hanji: "字\(index)"))
        }
        await model.load()
    }

    /// One page per `pageSize` rows, and the remainder gets a page of its own —
    /// a list whose last few rows had no page would be rows the user cannot
    /// reach, which is the bug this replaced.
    func testPageCount_coversTheRemainder() async throws {
        let model = makeModel()
        let size = UserDataListMetrics.pageSize

        await seed(model, count: size + 1)

        XCTAssertEqual(model.list.matchCount, size + 1)
        XCTAssertEqual(model.list.pageCount, 2)
        XCTAssertEqual(model.list.rows.count, size, "a page holds exactly what the table shows")
    }

    /// An empty dictionary still reads as one page, not as a pager with
    /// nothing in it.
    func testAnEmptyDictionary_isOnePage() async throws {
        let model = makeModel()

        await model.load()

        XCTAssertEqual(model.list.pageCount, 1)
        XCTAssertEqual(model.list.page + 1, model.list.pageCount, "the last page")
        XCTAssertEqual(model.list.page, 0)
    }

    func testPagingForward_showsTheNextRowsAndStopsAtTheEnd() async throws {
        let model = makeModel()
        let size = UserDataListMetrics.pageSize
        await seed(model, count: size + 2)
        let firstPage = Set(model.list.rows.map(\.id))

        await model.pageForward()

        XCTAssertEqual(model.list.page, 1)
        XCTAssertEqual(model.list.rows.count, 2)
        XCTAssertTrue(firstPage.isDisjoint(with: model.list.rows.map(\.id)), "page two repeated page one")
        XCTAssertEqual(model.list.page + 1, model.list.pageCount, "the last page")

        await model.pageForward()

        XCTAssertEqual(model.list.page, 1, "there is nowhere past the last page")
    }

    /// The list can shrink under the page the user is on — a delete on the
    /// last page, or a filter that now matches less. Nothing else clamps the
    /// page, so a load that did not would show an empty table with no way back.
    func testAPageThatOutlivesItsRows_isPulledBackIntoTheList() async throws {
        let model = makeModel()
        let size = UserDataListMetrics.pageSize
        await seed(model, count: size + 1)
        await model.pageForward()
        XCTAssertEqual(model.list.page, 1)

        try await model.delete(XCTUnwrap(model.list.rows.first))

        XCTAssertEqual(model.list.page, 0)
        XCTAssertEqual(model.list.rows.count, size)
    }

    /// A new filter is a new list, so the pages it had before are pages of
    /// something else.
    func testLoadingAfterAFilterChange_startsAtPageOne() async throws {
        let model = makeModel()
        await seed(model, count: UserDataListMetrics.pageSize + 1)
        await model.pageForward()

        model.filter = "row"
        await model.loadFirstPage()

        XCTAssertEqual(model.list.page, 0)
    }
}

/// What Custom Dictionary shares with the Windows and Linux pages
/// (desktop-core `settings/listing.rs`, `settings/custom_dictionary.rs`).
///
/// INVARIANT_USER_DATA_LIST_FILTER_RELOAD_SELECTION (§58): the filter goes
/// to the engine as typed, every write reloads the list whatever it
/// answered, and the selection is only ever a row on screen.
@MainActor
final class CustomDictionaryListParityTests: XCTestCase {
    private var client = FakeUserDataClient()

    private func makeModel(rows: Int) async -> CustomDictionaryPageModel {
        client = FakeUserDataClient()
        let model = CustomDictionaryPageModel(client: client)
        for index in 0 ..< rows {
            await model.save(CustomDictionaryRow(roman: "row\(index)", hanji: "字\(index)"))
        }
        await model.load()
        return model
    }

    func testTheFilter_goesToTheEngineAsTyped() async {
        let model = await makeModel(rows: 2)

        model.filter = " row1 "
        await model.loadFirstPage()

        XCTAssertEqual(client.lastCustomFilter, " row1 ", "the engine trims it")
        XCTAssertEqual(model.list.rows.map(\.roman), ["row1"])
    }

    /// A write that failed may still have changed the store; the list shows
    /// what the store holds.
    func testAFailedWrite_stillReloadsTheList() async {
        let model = await makeModel(rows: 1)
        client.failsCustomWritesAfterApplying = true

        await model.save(CustomDictionaryRow(roman: "tsit", hanji: "一"))

        guard case .failure(.desktopCustomDictWriteFailed, _) = model.message else {
            return XCTFail("expected the write failure, got \(String(describing: model.message))")
        }
        XCTAssertEqual(model.list.rows.map(\.roman), ["tsit", "row0"])
        XCTAssertEqual(model.list.countLabel, "2")
        XCTAssertEqual(model.activity, .idle)
    }

    /// The engine commits a large import in chunks: one that fails partway
    /// has still added the chunks before it.
    func testAFailedImport_showsTheRowsItCommitted() async {
        let model = await makeModel(rows: 1)
        client.rowsToImport = [CustomDictionaryRow(roman: "tsit", hanji: "一")]
        client.failsCustomWritesAfterApplying = true

        await model.importCSV(at: URL(fileURLWithPath: "/dev/null"))

        guard case .failure(.commonImportFailed, _) = model.message else {
            return XCTFail("expected the import failure, got \(String(describing: model.message))")
        }
        XCTAssertEqual(model.list.rows.map(\.roman), ["tsit", "row0"])
    }

    /// The one alert shows the last thing that went wrong, as on Windows.
    func testAFailedWriteWhoseReloadFailsToo_reportsTheRead() async {
        let model = await makeModel(rows: 1)
        client.failsCustomWritesAfterApplying = true
        client.failsCustomReads = true

        await model.delete(CustomDictionaryRow(roman: "x", hanji: ""))

        guard case .failure(.desktopCustomDictReadFailed, _) = model.message else {
            return XCTFail("expected the read failure, got \(String(describing: model.message))")
        }
        XCTAssertEqual(model.activity, .idle)
    }

    func testASelectionPagedAway_isNotThereOnPagingBack() async throws {
        let model = await makeModel(rows: UserDataListMetrics.pageSize + 1)
        let selected = try XCTUnwrap(model.list.rows.first)
        model.selectedRowID = selected.id

        await model.load()
        XCTAssertEqual(model.list.selectedRow, selected, "a reload that keeps the row keeps the selection")

        await model.pageForward()
        XCTAssertNil(model.selectedRowID)

        await model.pageBackward()
        XCTAssertTrue(model.list.rows.contains(selected))
        XCTAssertNil(model.list.selectedRow, "− has nothing to act on")
    }

    func testAFailedLoad_keepsTheSelection() async throws {
        let model = await makeModel(rows: UserDataListMetrics.pageSize + 1)
        let selected = try XCTUnwrap(model.list.rows.first)
        model.selectedRowID = selected.id
        client.failsCustomReads = true

        await model.pageForward()

        XCTAssertEqual(model.list.selectedRow, selected, "the rows on screen did not change")
    }
}
