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

    /// No key macOS displays carries plural arms, so there is no runtime arm-selection to pin here.
    /// That the emitter produces the macOS-shaped selector at all is covered hermetically, against a
    /// synthetic key, by `tools/i18n/test_i18n.py`
    /// (`test_plural_accessor_names_the_generated_map_as_the_fallback_source`) — where it belongs,
    /// since tying it to a shipping string would make product copy answerable to a codegen test.
    func testGeneratedFormatAccessor_interpolatesTextTheProductDidNotAuthor() {
        // The store's own failure reason: a placeholder rather than something the call site
        // concatenates, so each language punctuates around it.
        XCTAssertEqual(
            StringResolver(.hanji).macosRestoreLineFrequencyFailed(reason: "disk I/O error"),
            "詞頻:失敗(disk I/O error)",
        )
        XCTAssertEqual(
            StringResolver(.english).macosRestoreLineFrequencyFailed(reason: "disk I/O error"),
            "Frequency records: failed (disk I/O error)",
        )
        XCTAssertEqual(
            StringResolver(.japanese).macosRestoreLineFrequencyFailed(reason: "disk I/O error"),
            "単語頻度：失敗（disk I/O error）",
        )
    }
}
