// The two-column CSV the 自訂詞庫 page reads and writes.

@testable import TaigiInputMethodCore
import XCTest

final class CustomDictionaryCSVTests: XCTestCase {
    private let limit = CustomDictionaryStore.maxEntries

    func testRoundTrip_keepsBothColumns() throws {
        let rows = [
            CustomDictionaryRow(roman: "gâu-tsá", hanzi: "𠢕早"),
            CustomDictionaryRow(roman: "tsia̍h-pá--buē", hanzi: "食飽未"),
        ]

        let decoded = try CustomDictionaryCSV.decode(
            CustomDictionaryCSV.encode(rows),
            entryLimit: limit,
        )

        XCTAssertEqual(decoded.map(\.roman), rows.map(\.roman))
        XCTAssertEqual(decoded.map(\.hanzi), rows.map(\.hanzi))
    }

    /// The shared quoting, not iOS's private parser: an entry containing a
    /// quote has to survive its own export.
    func testRoundTrip_survivesAQuoteInTheEntry() throws {
        let rows = [CustomDictionaryRow(roman: "say \"hi\"", hanzi: "講,好")]

        let decoded = try CustomDictionaryCSV.decode(
            CustomDictionaryCSV.encode(rows),
            entryLimit: limit,
        )

        XCTAssertEqual(decoded.first?.roman, "say \"hi\"")
        XCTAssertEqual(decoded.first?.hanzi, "講,好")
    }

    /// A romanization-only entry is legitimate; a row with no romanization is
    /// nothing the dictionary could be searched by.
    func testDecode_keepsRomanizationOnlyRowsAndDropsHanziOnlyOnes() throws {
        let decoded = try CustomDictionaryCSV.decode("gua,\n,我\n", entryLimit: limit)

        XCTAssertEqual(decoded.map(\.roman), ["gua"])
    }

    /// One bad line does not fail a hand-edited file — the rest still imports.
    func testDecode_skipsUnusableLinesButKeepsTheRest() throws {
        let decoded = try CustomDictionaryCSV.decode("gua,我\nnonsense\nli,你\n", entryLimit: limit)

        XCTAssertEqual(decoded.map(\.hanzi), ["我", "你"])
    }

    /// A file with content but nothing usable is a wrong-format file, and
    /// saying "imported 0" would leave the user wondering what happened.
    func testDecode_refusesAFileWithNoUsableRows() {
        XCTAssertThrowsError(try CustomDictionaryCSV.decode("nonsense\n???\n", entryLimit: limit)) {
            guard case CustomDictionaryCSVError.noUsableRows = $0 else {
                return XCTFail("wrong error: \($0)")
            }
        }
    }

    func testDecode_acceptsAnEmptyFile() throws {
        XCTAssertEqual(try CustomDictionaryCSV.decode("", entryLimit: limit).count, 0)
        XCTAssertEqual(try CustomDictionaryCSV.decode("\n\n", entryLimit: limit).count, 0)
    }

    /// Refused before anything is written, so a too-big file cannot land half
    /// of itself.
    func testDecode_refusesMoreRowsThanTheDictionaryHolds() {
        let csv = (0 ... 3).map { "r\($0),字\($0)\n" }.joined()

        XCTAssertThrowsError(try CustomDictionaryCSV.decode(csv, entryLimit: 3)) {
            guard case CustomDictionaryCSVError.tooManyRows = $0 else {
                return XCTFail("wrong error: \($0)")
            }
        }
    }

    func testDecodeFile_refusesAFileLargerThanTheCap() throws {
        let url = try TestFixtures.scratchDirectory().appendingPathComponent("big.csv")
        let oversized = String(repeating: "a", count: CustomDictionaryCSV.maxFileSizeBytes + 1)
        try Data(oversized.utf8).write(to: url)

        XCTAssertThrowsError(try CustomDictionaryCSV.decodeFile(at: url, entryLimit: limit)) {
            guard case CustomDictionaryCSVError.fileTooLarge = $0 else {
                return XCTFail("wrong error: \($0)")
            }
        }
    }

    func testDecodeFile_refusesAFileThatIsNotUTF8() throws {
        let url = try TestFixtures.scratchDirectory().appendingPathComponent("latin1.csv")
        try Data([0xFF, 0xFE, 0x41]).write(to: url)

        XCTAssertThrowsError(try CustomDictionaryCSV.decodeFile(at: url, entryLimit: limit)) {
            guard case CustomDictionaryCSVError.notUTF8 = $0 else {
                return XCTFail("wrong error: \($0)")
            }
        }
    }
}
