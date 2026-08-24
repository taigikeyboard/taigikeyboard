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
        XCTAssertEqual(UserDataPageActivity.working(.macosProgressImporting).labelKey, .macosProgressImporting)
        XCTAssertNil(UserDataPageActivity.idle.labelKey)
        XCTAssertTrue(UserDataPageActivity.working(.macosProgressImporting).isWorking)
        XCTAssertFalse(UserDataPageActivity.idle.isWorking)
    }

    func testSameMessage_resolvesInWhicheverLanguageIsAskedFor() {
        let message = UserDataPageMessage.imported(12, skipped: 3)

        XCTAssertEqual(message.title(hanji), hanji.resolve(.macosImportComplete))
        XCTAssertEqual(message.title(english), english.resolve(.macosImportComplete))
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
        let message = UserDataPageMessage.failure(.macosCustomDictWriteFailed, StoreError())

        XCTAssertEqual(message.title(hanji), hanji.resolve(.macosCustomDictWriteFailed))
        XCTAssertEqual(message.detail(hanji), "disk I/O error")
        XCTAssertEqual(message.detail(english), "disk I/O error")
    }

    /// A file that is not UTF-8 is a different refusal from a malformed CSV,
    /// and says so rather than reusing the parser's message.
    func testNotUTF8_reportsTheEncodingRatherThanTheCSVShape() {
        let message = UserDataPageMessage.notUTF8

        XCTAssertEqual(message.title(hanji), hanji.resolve(.commonImportFailed))
        XCTAssertEqual(message.detail(hanji), hanji.resolve(.macosNotUTF8Detail))
    }
}

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

        XCTAssertTrue(model.beginWork(.macosProgressImporting))
        XCTAssertFalse(
            model.beginWork(.macosProgressDeleting),
            "a second action must not start while one is running",
        )
        XCTAssertEqual(model.activity, .working(.macosProgressImporting), "the first action keeps the slot")
    }

    /// A real action, start to finish: it takes the slot on the way in and
    /// gives it back on the way out, whether the store answered or failed.
    func testAnAction_leavesTheSlotFreeWhenItEnds() async throws {
        let model = try makeModel()

        await model.deleteAll()

        XCTAssertEqual(model.activity, .idle)
        XCTAssertTrue(model.beginWork(.macosProgressSaving))
    }

    /// And an action that arrives while the slot is held does not run at all.
    func testAnAction_doesNothingWhileAnotherHoldsTheSlot() async throws {
        let model = try makeModel()
        XCTAssertTrue(model.beginWork(.macosProgressImporting))

        await model.deleteAll()

        XCTAssertEqual(
            model.activity, .working(.macosProgressImporting),
            "the refused action must not release the slot it never took",
        )
    }
}
