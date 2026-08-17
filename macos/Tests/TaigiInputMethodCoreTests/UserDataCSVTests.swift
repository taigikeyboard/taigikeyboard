// The CSV dialect the three platforms share.

@testable import TaigiInputMethodCore
import XCTest

/// A file exported on one platform has to import the same way on the others,
/// so these cases pin the dialect itself rather than any one caller's use of
/// it.
final class UserDataCSVTests: XCTestCase {
    // MARK: - Dialect

    func testParseLine_splitsPlainFields() {
        XCTAssertEqual(UserDataCSV.parseLine("gua,我,3"), ["gua", "我", "3"])
    }

    func testParseLine_keepsACommaInsideQuotes() {
        XCTAssertEqual(UserDataCSV.parseLine("\"a,b\",c"), ["a,b", "c"])
    }

    /// The RFC 4180 escape: a doubled quote inside a quoted field is one
    /// literal quote, which is what makes the round-trip lossless.
    func testParseLine_readsADoubledQuoteAsOne() {
        XCTAssertEqual(UserDataCSV.parseLine("\"say \"\"hi\"\"\",x"), ["say \"hi\"", "x"])
    }

    func testParseLine_keepsATrailingEmptyField() {
        XCTAssertEqual(UserDataCSV.parseLine("a,"), ["a", ""])
    }

    /// Grapheme clusters are atomic: a combining tone mark must not be split
    /// off the letter it sits on.
    func testParseLine_keepsToneMarksWithTheirLetter() {
        XCTAssertEqual(UserDataCSV.parseLine("tsia̍h,食"), ["tsia̍h", "食"])
    }

    func testEscape_quotesOnlyWhenItHasTo() {
        XCTAssertEqual(UserDataCSV.escape("plain"), "plain")
        XCTAssertEqual(UserDataCSV.escape("a,b"), "\"a,b\"")
        XCTAssertEqual(UserDataCSV.escape("say \"hi\""), "\"say \"\"hi\"\"\"")
    }

    func testEscapeThenParse_roundTrips() {
        let awkward = ["a,b", "say \"hi\"", "plain", ""]
        let line = awkward.map(UserDataCSV.escape).joined(separator: ",")

        XCTAssertEqual(UserDataCSV.parseLine(line), awkward)
    }

    // MARK: - 詞頻

    func testFrequency_roundTrips() {
        let rows = [
            UserDataCSV.FrequencyCSVRow(word: "我", tl: "guá", count: 7),
            UserDataCSV.FrequencyCSVRow(word: "重", tl: "tāng", count: 2),
        ]

        XCTAssertEqual(UserDataCSV.decodeFrequency(UserDataCSV.encodeFrequency(rows)), rows)
    }

    /// Both readings of one 漢字 survive as separate rows — identity is the
    /// pair, and a codec that collapsed them would merge 重/tāng into 重/tîng.
    func testFrequency_keepsTwoReadingsOfOneWordApart() {
        let csv = "重,tāng,5\n重,tîng,2\n"

        XCTAssertEqual(
            UserDataCSV.decodeFrequency(csv),
            [
                UserDataCSV.FrequencyCSVRow(word: "重", tl: "tāng", count: 5),
                UserDataCSV.FrequencyCSVRow(word: "重", tl: "tîng", count: 2),
            ],
        )
    }

    /// A backup written before the pair key has two columns. It reads into the
    /// tolerant empty-TL bucket rather than being dropped.
    func testFrequency_readsALegacyTwoColumnRow() {
        XCTAssertEqual(
            UserDataCSV.decodeFrequency("我,4\n"),
            [UserDataCSV.FrequencyCSVRow(word: "我", tl: "", count: 4)],
        )
    }

    /// Four columns is not "three plus junk": the column count is an exact
    /// discriminator, so a malformed row is skipped rather than being eaten as
    /// a legacy one.
    func testFrequency_skipsARowWithTooManyColumns() {
        XCTAssertTrue(UserDataCSV.decodeFrequency("我,guá,4,extra\n").isEmpty)
    }

    func testFrequency_skipsRowsThatCannotBeCounted() {
        XCTAssertTrue(UserDataCSV.decodeFrequency("我,guá,none\n").isEmpty)
        XCTAssertTrue(UserDataCSV.decodeFrequency("我,guá,0\n").isEmpty)
        XCTAssertTrue(UserDataCSV.decodeFrequency(",guá,3\n").isEmpty)
    }

    func testFrequency_ignoresBlankLines() {
        XCTAssertEqual(UserDataCSV.decodeFrequency("\n我,guá,4\n\n").count, 1)
    }

    /// A file saved by an editor that writes CRLF still reads: the trailing
    /// carriage return is trimmed with the rest of the whitespace.
    func testFrequency_readsCarriageReturnLineEndings() {
        XCTAssertEqual(UserDataCSV.decodeFrequency("我,guá,4\r\n").count, 1)
    }

    // MARK: - 詞關聯

    func testAssociation_roundTrips() {
        let rows = [
            UserDataCSV.AssociationCSVRow(
                previousWord: "台語", previousTl: "tâi-gí",
                nextWord: "真好", nextTl: "tsin-hó", count: 3,
            ),
        ]

        XCTAssertEqual(UserDataCSV.decodeAssociation(UserDataCSV.encodeAssociation(rows)), rows)
    }

    /// A word with no predecessor is legitimate; a bigram with no next word is
    /// not.
    func testAssociation_allowsAnEmptyPreviousWordButNotAnEmptyNextWord() {
        XCTAssertEqual(UserDataCSV.decodeAssociation(",,好,hó,2\n").count, 1)
        XCTAssertTrue(UserDataCSV.decodeAssociation("台語,tâi-gí,,,2\n").isEmpty)
    }

    func testAssociation_skipsShortRows() {
        XCTAssertTrue(UserDataCSV.decodeAssociation("台語,tâi-gí,好,hó\n").isEmpty)
    }
}
