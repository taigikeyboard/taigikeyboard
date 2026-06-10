@testable import TaigiKeyboard
import XCTest

/// Frequency CSV carries the `(漢字, 羅馬字)` pair.
///
/// Pins the CSV side of `INVARIANT_USER_FREQ_PAIR_KEY`
/// (`docs/architecture/behavioral-invariants.md` §28): the hand-editable
/// frequency CSV is `word,tl,count` (3 columns), a legacy 2-column
/// `word,count` file decodes with `tl == ""`, and any other column count is
/// skipped. The format must match Android `DictionaryCsvCodec.encodeFrequencyCSV`
/// / `decodeFrequencyCSV` byte-for-byte — this is a cross-platform CSV contract.
final class FrequencyCSVTests: XCTestCase {
    // Flatten a decoded tuple list into comparable strings (tuples aren't Equatable).
    private func flatten(_ rows: [(word: String, tl: String, count: Int)]) -> [String] {
        rows.map { "\($0.word)|\($0.tl)|\($0.count)" }
    }

    // MARK: - Encode

    func test_INVARIANT_USER_FREQ_PAIR_KEY_csv_encodeWritesThreeColumns() {
        let csv = CSVDocument.encodeFrequencyCSV([
            (word: "重", tl: "tāng", count: 5),
            (word: "食", tl: "tsia̍h", count: 8),
        ])
        XCTAssertEqual(csv, "重,tāng,5\n食,tsia̍h,8\n")
    }

    func test_INVARIANT_USER_FREQ_PAIR_KEY_csv_legacyEmptyTlEncodesAsEmptyMiddleField() {
        // A legacy `tl=""` bucket exports as `word,,count` (empty middle field).
        let csv = CSVDocument.encodeFrequencyCSV([(word: "食", tl: "", count: 8)])
        XCTAssertEqual(csv, "食,,8\n")
    }

    func test_INVARIANT_USER_FREQ_PAIR_KEY_csv_homographReadingsStayDistinctRows() {
        // 一字多音: same hanji, two readings → two rows (never merged).
        let csv = CSVDocument.encodeFrequencyCSV([
            (word: "重", tl: "tāng", count: 5),
            (word: "重", tl: "tîng", count: 3),
        ])
        XCTAssertEqual(csv, "重,tāng,5\n重,tîng,3\n")
    }

    // MARK: - Decode

    func test_INVARIANT_USER_FREQ_PAIR_KEY_csv_decodeThreeColumns() {
        let rows = CSVDocument.decodeFrequencyCSV("重,tāng,5\n重,tîng,3\n食,tsia̍h,8\n")
        XCTAssertEqual(flatten(rows), ["重|tāng|5", "重|tîng|3", "食|tsia̍h|8"])
    }

    func test_INVARIANT_USER_FREQ_PAIR_KEY_csv_decodeLegacyTwoColumnsToEmptyTl() {
        // Pre-roman-column export / hand-edited 2-column file → legacy tl="" bucket.
        let rows = CSVDocument.decodeFrequencyCSV("食,8\n重,5\n")
        XCTAssertEqual(flatten(rows), ["食||8", "重||5"])
    }

    func test_INVARIANT_USER_FREQ_PAIR_KEY_csv_decodeEmptyMiddleFieldIsEmptyTl() {
        let rows = CSVDocument.decodeFrequencyCSV("食,,8\n")
        XCTAssertEqual(flatten(rows), ["食||8"])
    }

    func test_INVARIANT_USER_FREQ_PAIR_KEY_csv_decodeSkipsOtherColumnCounts() {
        // Exact discriminator: 1 col and 4 cols are skipped, not eaten as legacy.
        let rows = CSVDocument.decodeFrequencyCSV("solo\n食,tsia̍h,8\na,b,c,d\n")
        XCTAssertEqual(flatten(rows), ["食|tsia̍h|8"])
    }

    func test_INVARIANT_USER_FREQ_PAIR_KEY_csv_decodeSkipsInvalidRows() {
        // Non-positive / non-numeric count, empty word → skipped.
        let rows = CSVDocument.decodeFrequencyCSV("食,tsia̍h,0\n食,tsia̍h,abc\n,tsia̍h,5\n重,tāng,5\n")
        XCTAssertEqual(flatten(rows), ["重|tāng|5"])
    }

    func test_INVARIANT_USER_FREQ_PAIR_KEY_csv_decodeToleratesBlankLines() {
        let rows = CSVDocument.decodeFrequencyCSV("\n食,tsia̍h,8\n\n重,tāng,5\n\n")
        XCTAssertEqual(flatten(rows), ["食|tsia̍h|8", "重|tāng|5"])
    }

    // MARK: - Round-trip

    func test_INVARIANT_USER_FREQ_PAIR_KEY_csv_roundTripPreservesPair() {
        let original: [(word: String, tl: String, count: Int)] = [
            (word: "重", tl: "tāng", count: 5),
            (word: "重", tl: "tîng", count: 3),
            (word: "食", tl: "tsia̍h", count: 8),
            (word: "舊", tl: "", count: 2), // legacy bucket survives the round-trip
        ]
        let decoded = CSVDocument.decodeFrequencyCSV(CSVDocument.encodeFrequencyCSV(original))
        XCTAssertEqual(flatten(decoded), flatten(original))
    }

    func test_INVARIANT_USER_FREQ_PAIR_KEY_csv_roundTripEscapesCommaField() {
        // A field containing a comma is quote-wrapped on encode and recovered on
        // decode — pins escape↔parse consistency with Android DictionaryCsvCodec.
        let original: [(word: String, tl: String, count: Int)] = [(word: "a,b", tl: "tāng", count: 5)]
        let decoded = CSVDocument.decodeFrequencyCSV(CSVDocument.encodeFrequencyCSV(original))
        XCTAssertEqual(flatten(decoded), ["a,b|tāng|5"])
    }

    func test_INVARIANT_USER_FREQ_PAIR_KEY_csv_roundTripEscapesQuoteField() {
        // A field containing a literal " survives the RFC 4180 "" round-trip —
        // pins parity with Android DictionaryCsvCodec (regression guard for the
        // pre-fix iOS parseLine that dropped doubled quotes).
        let original: [(word: String, tl: String, count: Int)] = [(word: "a\"b", tl: "tāng", count: 5)]
        let decoded = CSVDocument.decodeFrequencyCSV(CSVDocument.encodeFrequencyCSV(original))
        XCTAssertEqual(flatten(decoded), ["a\"b|tāng|5"])
    }
}
