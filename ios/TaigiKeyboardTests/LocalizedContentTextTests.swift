@testable import TaigiKeyboard
import XCTest

/// Tests for `LocalizedContentText` — the per-language content string decoded from the shared
/// `features.json` / `faq.json` assets (Round C1 infra). Coverage pillars:
/// - `resolve(for:)` returns the authored value for each effective language (proves resolution
///   beats the bare Hanji fallback once translations land in C2).
/// - An unauthored language falls back to `hanji` — the C1 behavior-freeze guarantee: today every
///   key is Hanji-only, so every effective language renders Hanji.
/// - Decoding the real JSON shape: all-language object populates every field; a Hanji-only object
///   leaves the others `nil`; a missing `hanji` is a hard decode error.
///
/// The resolve fallback order is a `CROSS-PLATFORM INVARIANT` mirrored by the Android
/// `LocalizedContentTextTest` (android .../content/LocalizedContentTextTest.kt). Drift = silent
/// per-platform divergence in which language a content string shows.
final class LocalizedContentTextTests: XCTestCase {
    private let decoder = JSONDecoder()

    /// All five effective display languages, each authored to a distinct value.
    private let allLanguages = LocalizedContentText(
        hanji: "漢", tailo: "TL", poj: "POJ", ja: "JA", en: "EN",
    )

    /// Hanji-only — the production state today (C1): every other language is unauthored.
    private let hanjiOnly = LocalizedContentText(
        hanji: "漢", tailo: nil, poj: nil, ja: nil, en: nil,
    )

    // MARK: - resolve(for:)

    func test_INVARIANT_resolve_authoredLanguage_returnsThatLanguage() {
        XCTAssertEqual(allLanguages.resolve(for: .hanji), "漢")
        XCTAssertEqual(allLanguages.resolve(for: .tailo), "TL")
        XCTAssertEqual(allLanguages.resolve(for: .poj), "POJ")
        XCTAssertEqual(allLanguages.resolve(for: .japanese), "JA")
        XCTAssertEqual(allLanguages.resolve(for: .english), "EN")
    }

    func test_INVARIANT_resolve_unauthoredLanguage_fallsBackToHanji() {
        for language in [DisplayLanguage.hanji, .tailo, .poj, .japanese, .english, .pseudo] {
            XCTAssertEqual(
                hanjiOnly.resolve(for: language), "漢",
                "unauthored \(language) must fall back to hanji (C1 behavior-freeze)",
            )
        }
    }

    func test_resolve_pseudo_returnsHanji() {
        // Content has no pseudo map; the layout-probe language renders Hanji (contract: pseudo → hanji).
        XCTAssertEqual(allLanguages.resolve(for: .pseudo), "漢")
    }

    // MARK: - Decoding

    func test_decode_allLanguages_populatesEveryField() throws {
        let json = Data(#"{"hanji":"漢","tailo":"TL","poj":"POJ","ja":"JA","en":"EN"}"#.utf8)
        let text = try decoder.decode(LocalizedContentText.self, from: json)
        XCTAssertEqual(text.hanji, "漢")
        XCTAssertEqual(text.tailo, "TL")
        XCTAssertEqual(text.poj, "POJ")
        XCTAssertEqual(text.ja, "JA")
        XCTAssertEqual(text.en, "EN")
    }

    func test_decode_hanjiOnly_otherLanguagesNil() throws {
        let json = Data(#"{"hanji":"漢"}"#.utf8)
        let text = try decoder.decode(LocalizedContentText.self, from: json)
        XCTAssertEqual(text.hanji, "漢")
        XCTAssertNil(text.tailo)
        XCTAssertNil(text.poj)
        XCTAssertNil(text.ja)
        XCTAssertNil(text.en)
    }

    func test_decode_missingHanji_throws() {
        let json = Data(#"{"en":"EN"}"#.utf8)
        XCTAssertThrowsError(try decoder.decode(LocalizedContentText.self, from: json))
    }

    // MARK: - FeatureContent shape (summary optional, link/nav text are LocalizedContentText)

    func test_decode_featureContent_summaryAbsent_isNil() throws {
        let json = Data(#"""
        {"id":"x","title":{"hanji":"標題"},"icon":{"ios":"a","android":"b"},
         "paragraphs":[{"text":{"hanji":"段落"}}]}
        """#.utf8)
        let feature = try decoder.decode(FeatureContent.self, from: json)
        XCTAssertEqual(feature.title.hanji, "標題")
        XCTAssertNil(feature.summary)
        XCTAssertEqual(feature.paragraphs.first?.text.hanji, "段落")
    }

    func test_decode_featureContent_linkAttachmentText_isLocalizedContentText() throws {
        let json = Data(#"""
        {"id":"x","title":{"hanji":"標題"},"icon":{"ios":"a","android":"b"},
         "paragraphs":[{"text":{"hanji":"段落"},
           "attachment":{"type":"link","text":{"hanji":"連結","en":"Link"},"url":"https://x"}}]}
        """#.utf8)
        let feature = try decoder.decode(FeatureContent.self, from: json)
        guard case let .link(text, url) = feature.paragraphs.first?.attachment else {
            return XCTFail("expected a link attachment")
        }
        XCTAssertEqual(text.resolve(for: .hanji), "連結")
        XCTAssertEqual(text.resolve(for: .english), "Link")
        XCTAssertEqual(url, "https://x")
    }
}
