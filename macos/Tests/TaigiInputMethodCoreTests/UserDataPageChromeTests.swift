import AppKit
@testable import TaigiInputMethodCore
import XCTest

/// The 詞庫 pages hold what happened, not how to say it — an import running
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
        XCTAssertEqual(UserDataPageActivity.working(.desktopProgressImporting).labelKey, .desktopProgressImporting)
        XCTAssertNil(UserDataPageActivity.idle.labelKey)
        XCTAssertTrue(UserDataPageActivity.working(.desktopProgressImporting).isWorking)
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
/// the 刪除學習紀錄 receipt had an alert of its own further down the chain and
/// SwiftUI kept the other one, so no message ever appeared (USER 2026-08-26).
/// Every page message goes through the one channel now, and this case is what
/// let the title-only receipt join it.
final class UserDataPageDoneMessageTests: XCTestCase {
    private let hanji = StringResolver(.hanji)

    func testADoneMessage_hasATitleAndNoBody() {
        let message = UserDataPageMessage.done(.desktopClearLearningRecordsDone)

        XCTAssertEqual(message.title(hanji), hanji.resolve(.desktopClearLearningRecordsDone))
        XCTAssertNil(message.detail(hanji))
    }

    /// The cases that DO carry a body keep it — the nil above must not be what
    /// every message answers.
    func testAFailureMessage_stillCarriesItsDiagnostic() {
        let message = UserDataPageMessage.failure(
            .desktopClearLearningRecordsFailed, diagnostic: "disk I/O error",
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
    private func makeModel() throws -> CustomDictionaryPageModel {
        let directory = try TestFixtures.scratchDirectory()
        return CustomDictionaryPageModel(store: CustomDictionaryStore(directory: { directory }))
    }

    /// A symbol name that stops resolving renders a blank rectangle and says
    /// nothing about it.
    func testTheEmptyStateSymbol_resolves() {
        XCTAssertNotNil(
            NSImage(systemSymbolName: CustomDictionaryPage.emptyStateSymbolName, accessibilityDescription: nil),
        )
    }

    func testBeginWork_refusesASecondClaimWhileTheFirstIsHeld() throws {
        let model = try makeModel()

        XCTAssertTrue(model.beginWork(.desktopProgressImporting))
        XCTAssertFalse(
            model.beginWork(.desktopProgressDeleting),
            "a second action must not start while one is running",
        )
        XCTAssertEqual(model.activity, .working(.desktopProgressImporting), "the first action keeps the slot")
    }

    /// A real action, start to finish: it takes the slot on the way in and
    /// gives it back on the way out, whether the store answered or failed.
    func testAnAction_leavesTheSlotFreeWhenItEnds() async throws {
        let model = try makeModel()

        await model.deleteAll()

        XCTAssertEqual(model.activity, .idle)
        XCTAssertTrue(model.beginWork(.desktopProgressSaving))
    }

    /// And an action that arrives while the slot is held does not run at all.
    func testAnAction_doesNothingWhileAnotherHoldsTheSlot() async throws {
        let model = try makeModel()
        XCTAssertTrue(model.beginWork(.desktopProgressImporting))

        await model.deleteAll()

        XCTAssertEqual(
            model.activity, .working(.desktopProgressImporting),
            "the refused action must not release the slot it never took",
        )
    }
}

/// Paging the 自訂詞庫 list.
///
/// The reported failure: 17000 entries, and the list would not scroll — a flat
/// `LIMIT 100` in a fixed-height `Table` inside a `Form` put one scroll view
/// inside another, and rows past the hundredth were unreachable by anything
/// but the filter (USER 2026-08-26, real device). A page that FITS the table
/// has neither problem.
@MainActor
final class CustomDictionaryPagingTests: XCTestCase {
    /// Through `makeUserDataStores`, which OPENS the databases — a store built
    /// straight from a directory answers every query with "not open", which is
    /// enough for the work-slot cases above and useless here.
    private func makeModel() throws -> CustomDictionaryPageModel {
        CustomDictionaryPageModel(store: try TestFixtures.makeUserDataStores().customDictionary)
    }

    private func seed(_ model: CustomDictionaryPageModel, count: Int) async {
        for index in 0 ..< count {
            await model.save(CustomDictionaryRow(roman: "row\(index)", hanzi: "字\(index)"))
        }
        await model.load()
    }

    /// One page per `pageSize` rows, and the remainder gets a page of its own —
    /// a list whose last few rows had no page would be rows the user cannot
    /// reach, which is the bug this replaced.
    func testPageCount_coversTheRemainder() async throws {
        let model = try makeModel()
        let size = CustomDictionaryPageModel.pageSize

        await seed(model, count: size + 1)

        XCTAssertEqual(model.matchCount, size + 1)
        XCTAssertEqual(model.pageCount, 2)
        XCTAssertEqual(model.rows.count, size, "a page holds exactly what the table shows")
    }

    /// An empty dictionary still reads as one page, not as a pager with
    /// nothing in it.
    func testAnEmptyDictionary_isOnePage() async throws {
        let model = try makeModel()

        await model.load()

        XCTAssertEqual(model.pageCount, 1)
        XCTAssertFalse(model.canPageForward)
        XCTAssertFalse(model.canPageBackward)
    }

    func testPagingForward_showsTheNextRowsAndStopsAtTheEnd() async throws {
        let model = try makeModel()
        let size = CustomDictionaryPageModel.pageSize
        await seed(model, count: size + 2)
        let firstPage = Set(model.rows.map(\.id))

        await model.pageForward()

        XCTAssertEqual(model.page, 1)
        XCTAssertEqual(model.rows.count, 2)
        XCTAssertTrue(firstPage.isDisjoint(with: model.rows.map(\.id)), "page two repeated page one")
        XCTAssertFalse(model.canPageForward)

        await model.pageForward()

        XCTAssertEqual(model.page, 1, "there is nowhere past the last page")
    }

    /// The list can shrink under the page the user is on — a delete on the
    /// last page, or a filter that now matches less. Nothing else clamps the
    /// page, so a load that did not would show an empty table with no way back.
    func testAPageThatOutlivesItsRows_isPulledBackIntoTheList() async throws {
        let model = try makeModel()
        let size = CustomDictionaryPageModel.pageSize
        await seed(model, count: size + 1)
        await model.pageForward()
        XCTAssertEqual(model.page, 1)

        await model.delete(try XCTUnwrap(model.rows.first))

        XCTAssertEqual(model.page, 0)
        XCTAssertEqual(model.rows.count, size)
    }

    /// A new filter is a new list, so the pages it had before are pages of
    /// something else.
    func testLoadingAfterAFilterChange_startsAtPageOne() async throws {
        let model = try makeModel()
        await seed(model, count: CustomDictionaryPageModel.pageSize + 1)
        await model.pageForward()

        model.filter = "row"
        await model.loadFirstPage()

        XCTAssertEqual(model.page, 0)
    }
}
