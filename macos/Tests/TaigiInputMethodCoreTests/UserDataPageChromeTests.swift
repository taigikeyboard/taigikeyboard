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
        let message = UserDataPageMessage.failure(.macosFrequencyWriteFailed, StoreError())

        XCTAssertEqual(message.title(hanji), hanji.resolve(.macosFrequencyWriteFailed))
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

    func testRestore_whenEveryCategoryLands_reportsCompleteWithOneLineEach() {
        let message = UserDataPageMessage.restored(BackupImportResult(
            customDictionary: .restored(12),
            frequency: .restored(1),
            association: .restored(0),
        ))

        XCTAssertEqual(message.title(hanji), hanji.resolve(.macosRestoreComplete))
        XCTAssertEqual(message.detail(hanji), """
        自訂詞庫:新增 12 筆
        詞頻:處理 1 筆
        詞關聯:處理 0 筆
        """)
    }

    /// The three databases cannot be restored in one transaction, so the report
    /// keeps its per-category shape when one of them fails rather than
    /// collapsing to a single sentence that cannot say which — and the failed
    /// category names its reason in place, in the language's own punctuation.
    func testRestore_whenOneCategoryFails_keepsTheShapeAndNamesTheReason() {
        let message = UserDataPageMessage.restored(BackupImportResult(
            customDictionary: .restored(12),
            frequency: .failed("disk I/O error"),
            association: .restored(3),
        ))

        XCTAssertEqual(message.title(hanji), hanji.resolve(.macosRestorePartial))
        XCTAssertEqual(message.detail(hanji), """
        自訂詞庫:新增 12 筆
        詞頻:失敗(disk I/O error)
        詞關聯:處理 3 筆
        """)
        XCTAssertEqual(message.detail(english), """
        Custom dictionary: 12 added
        Frequency records: failed (disk I/O error)
        Association records: 3 processed
        """)
    }
}
