// The CSV dialect the three platforms share.

@testable import TaigiInputMethodCore
import XCTest

/// A file exported on one platform has to import the same way on the others,
/// so these cases pin the dialect itself rather than any one caller's use of
/// it.
final class UserDataCSVTests: XCTestCase {
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
}
