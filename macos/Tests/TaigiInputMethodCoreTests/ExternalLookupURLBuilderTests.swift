// The query form the two dictionary websites index by.

@testable import TaigiInputMethodCore
import XCTest

/// Real FFI: the tone stripping and the `o͘` / nasal normalisation belong to
/// the engine, and a reading that reaches the sites in the wrong form finds
/// nothing.
final class ExternalLookupURLBuilderTests: XCTestCase {
    func testAToneMarkBecomesADigit() {
        XCTAssertEqual(ExternalLookupURLBuilder.digitToneForm("guá"), "gua2")
    }

    /// Both sites omit the tone on the open and checked syllables, so the
    /// digit is dropped rather than written.
    func testTonesOneAndFourAreOmitted() {
        XCTAssertEqual(ExternalLookupURLBuilder.digitToneForm("tai"), "tai")
        XCTAssertEqual(ExternalLookupURLBuilder.digitToneForm("tai1"), "tai")
        XCTAssertEqual(ExternalLookupURLBuilder.digitToneForm("kat4"), "kat")
    }

    func testAnAlreadyDigitedReadingKeepsItsTone() {
        XCTAssertEqual(ExternalLookupURLBuilder.digitToneForm("gua2"), "gua2")
    }

    /// Every syllable is converted, and the 連字 between them survives —
    /// the sites index the hyphenated form.
    func testEverySyllableIsConvertedAndHyphensSurvive() {
        XCTAssertEqual(ExternalLookupURLBuilder.digitToneForm("tâi-gí"), "tai5-gi2")
    }

    func testTheNasalMarkerBecomesAscii() {
        XCTAssertEqual(ExternalLookupURLBuilder.digitToneForm("peⁿ"), "penn")
    }

    /// `o͘` is a letter with a mark that is not a tone; the engine's
    /// preprocessing is what turns it into the spelling the sites use.
    func testTheDottedOBecomesAscii() {
        XCTAssertEqual(ExternalLookupURLBuilder.digitToneForm("hó͘"), "hoo2")
    }

    func testCaseIsFolded() {
        XCTAssertEqual(ExternalLookupURLBuilder.digitToneForm("Tâi-gí"), "tai5-gi2")
    }

    // MARK: - URLs

    func testTheMoeUrlCarriesTheDigitToneForm() throws {
        let url = try XCTUnwrap(ExternalLookupURLBuilder.moeURL(forTl: "tâi-gí"))

        XCTAssertEqual(url.host(), "sutian.moe.edu.tw")
        XCTAssertTrue(url.absoluteString.hasSuffix("tsha=tai5-gi2"), url.absoluteString)
    }

    func testTheChhoeUrlCarriesTheDigitToneForm() throws {
        let url = try XCTUnwrap(ExternalLookupURLBuilder.chhoeURL(forTl: "tâi-gí"))

        XCTAssertEqual(url.host(), "chhoe.taigi.info")
        XCTAssertTrue(url.absoluteString.hasSuffix("lmj=tai5-gi2"), url.absoluteString)
    }

    /// A digit-toned reading can still carry `o͘`, which is a letter with a
    /// mark rather than a tone — the tone being resolved does not mean the
    /// spelling is.
    func testADigitTonedReadingIsStillNormalised() {
        XCTAssertEqual(ExternalLookupURLBuilder.digitToneForm("ho\u{0358}2"), "hoo2")
    }

    /// The reading is ONE query value. `.urlQueryAllowed` leaves `&` and `=`
    /// alone, and a custom-dictionary romanization is whatever the user typed.
    func testAReadingCannotAddQueryParameters() throws {
        let url = try XCTUnwrap(ExternalLookupURLBuilder.moeURL(forTl: "gua2&lui=evil"))
        let components = try XCTUnwrap(URLComponents(url: url, resolvingAgainstBaseURL: false))
        let lui = components.queryItems?.filter { $0.name == "lui" } ?? []

        XCTAssertEqual(lui.map(\.value), ["tai_su"], "the reading added a query parameter")
    }

    /// A reading that produces nothing has no page to link to, and an empty
    /// query would take the user to a search for nothing.
    func testAnEmptyReadingHasNoUrl() {
        XCTAssertNil(ExternalLookupURLBuilder.moeURL(forTl: ""))
        XCTAssertNil(ExternalLookupURLBuilder.chhoeURL(forTl: ""))
    }

    /// Both sites are HTTPS, and a reading is not a place to smuggle a scheme.
    func testTheUrlsAreHttps() throws {
        XCTAssertEqual(try XCTUnwrap(ExternalLookupURLBuilder.moeURL(forTl: "gua2")).scheme, "https")
        XCTAssertEqual(
            try XCTUnwrap(ExternalLookupURLBuilder.chhoeURL(forTl: "gua2")).scheme,
            "https",
        )
    }
}
