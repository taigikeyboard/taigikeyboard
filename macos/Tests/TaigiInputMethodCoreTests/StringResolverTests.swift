@testable import TaigiInputMethodCore
import XCTest

/// String resolution against the generated maps. macOS resolves all five production languages the
/// same way — there is no catalog half — so these cases are what proves English and Japanese really
/// are reachable without a resource bundle.
final class StringResolverTests: XCTestCase {
    func testResolve_everyProductionLanguageHasItsOwnText() {
        let expected: [DisplayLanguage: String] = [
            .hanji: "顯示語言",
            .tailo: "hián-sī gí-giân",
            .poj: "hián-sī gí-giân",
            .japanese: "表示言語",
            .english: "Display Language",
        ]
        for (language, text) in expected {
            XCTAssertEqual(StringResolver(language).resolve(.settingsDisplayLanguage), text, "\(language.tag)")
        }
    }

    func testResolve_automaticLabelIsTranslatedNotAnEndonym() {
        // The one picker row that is a real i18n key: the rest are language-invariant endonyms.
        XCTAssertEqual(StringResolver(.english).resolve(.settingsDisplayLanguageAutomatic), "Automatic")
        XCTAssertEqual(StringResolver(.hanji).resolve(.settingsDisplayLanguageAutomatic), "自動")
    }

    /// The generated typed accessor, not a raw `format` call: this is what a call site uses, and the
    /// `Int64` cast it emits is the reason the placeholder spec is `%lld` rather than `%d`.
    func testGeneratedFormatAccessor_interpolatesArgumentsInAuthoredOrder() {
        XCTAssertEqual(
            StringResolver(.english).dictionaryImportResult(imported: 12, skipped: 3),
            "Imported 12, skipped 3",
        )
        XCTAssertEqual(
            StringResolver(.hanji).dictionaryImportResult(imported: 12, skipped: 3),
            "成功匯入 12 項，跳過 3 項",
        )
    }

    /// English is the only plural-bearing language, and its arms are selected at runtime rather than
    /// by an OS plural locale — the generated map has no plural machinery at all.
    func testGeneratedFormatAccessor_selectsEnglishPluralArmsByCount() {
        XCTAssertEqual(
            StringResolver(.english).dictionaryImportBackupResult(customDict: 1, frequency: 1, association: 1),
            "Imported 1 custom entry, 1 frequency record, 1 association record",
        )
        XCTAssertEqual(
            StringResolver(.english).dictionaryImportBackupResult(customDict: 2, frequency: 0, association: 5),
            "Imported 2 custom entries, 0 frequency records, 5 association records",
        )
    }

    func testGeneratedFormatAccessor_nonEnglishRendersTheAuthoredTemplate() {
        // No plural selector runs for these languages; the stored template is used as authored.
        XCTAssertEqual(
            StringResolver(.japanese).dictionaryImportBackupResult(customDict: 1, frequency: 2, association: 3),
            "1 件のカスタム単語、2 件の単語頻度、3 件の単語連携をインポートしました",
        )
    }
}
