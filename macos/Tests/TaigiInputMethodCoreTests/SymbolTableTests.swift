// The shipped symbol table, and the validation that keeps a broken one out.

@testable import TaigiInputMethodCore
import XCTest

final class SymbolTableTests: XCTestCase {
    /// Three categories in menu order — what the level-1 cells are and which
    /// slot key reaches which.
    func testTheShippedTable_hasTheThreeCategoriesInMenuOrder() throws {
        let table = try TestFixtures.shippedSymbolTable()

        XCTAssertEqual(table.categories.map(\.id), [.punctuation, .brackets, .specialSymbols])
    }

    /// Every bracket entry is a PAIR: one pick writes both halves, so a user
    /// never opens the picker twice for one quotation (USER 2026-09-09).
    func testEveryBracket_isAPairOfTwoCharacters() throws {
        let brackets = try XCTUnwrap(TestFixtures.shippedSymbolTable().category(.brackets))

        for pair in brackets.symbols {
            XCTAssertEqual(pair.count, 2, "\(pair) is not an opening and a closing half")
        }
    }

    /// Punctuation and special symbols are single characters, so the
    /// auto-space swap can classify a pick the way it classifies a keystroke
    /// (`AutoSpacePunctuation.isAttaching` reads one character).
    func testEveryPunctuationMark_isOneCharacter() throws {
        let table = try TestFixtures.shippedSymbolTable()

        for id in [SymbolCategoryID.punctuation, .specialSymbols] {
            for symbol in try XCTUnwrap(table.category(id)).symbols {
                XCTAssertEqual(symbol.count, 1, "\(symbol) in \(id) is more than one character")
            }
        }
    }

    /// The full-width marks the composition maps a typed key onto are all
    /// reachable from the picker too, so a user in romanization mode — where
    /// the map is off — has a way to write them.
    func testThePunctuationCategory_carriesEveryFullWidthMappedMark() throws {
        let table = try TestFixtures.shippedSymbolTable()
        let punctuation = try XCTUnwrap(table.category(.punctuation)).symbols
        let brackets = try XCTUnwrap(table.category(.brackets)).symbols.joined()

        // The keys `FullWidthPunctuation` maps; a key the policy does not map
        // answers nil and is skipped, so this reads the policy rather than
        // restating it.
        for mapped in ",.?!;:()[]{}<>'@#$%^&*_+".compactMap({ FullWidthPunctuation.mapped(String($0)) }) {
            XCTAssertTrue(
                punctuation.contains(mapped) || brackets.contains(mapped),
                "\(mapped) is typed by the full-width map but not offered by the picker",
            )
        }
    }

    func testAnEmptyCategoryList_isRefused() {
        XCTAssertThrowsError(try SymbolTable(categories: [])) { error in
            XCTAssertEqual(error as? SymbolTable.ValidationError, .noCategories)
        }
    }

    func testADuplicateSymbol_isRefused() {
        let category = SymbolCategory(id: .punctuation, symbols: ["，", "，"])

        XCTAssertThrowsError(try SymbolTable(categories: [category])) { error in
            XCTAssertEqual(error as? SymbolTable.ValidationError, .duplicateSymbol("，", in: .punctuation))
        }
    }

    func testAnEmptySymbol_isRefused() {
        let category = SymbolCategory(id: .brackets, symbols: ["「」", ""])

        XCTAssertThrowsError(try SymbolTable(categories: [category])) { error in
            XCTAssertEqual(error as? SymbolTable.ValidationError, .emptySymbol(in: .brackets))
        }
    }

    func testADuplicateCategory_isRefused() {
        let category = SymbolCategory(id: .brackets, symbols: ["「」"])

        XCTAssertThrowsError(try SymbolTable(categories: [category, category])) { error in
            XCTAssertEqual(error as? SymbolTable.ValidationError, .duplicateCategory(.brackets))
        }
    }

    /// Decoding runs the same validation as construction, so a JSON that
    /// repeats a symbol is refused where it is read.
    func testDecoding_validates() {
        let duplicate = Data(#"{"categories":[{"id":"brackets","symbols":["「」","「」"]}]}"#.utf8)

        XCTAssertThrowsError(try JSONDecoder().decode(SymbolTable.self, from: duplicate)) { error in
            XCTAssertEqual(error as? SymbolTable.ValidationError, .duplicateSymbol("「」", in: .brackets))
        }
    }

    func testAnUnknownCategoryId_failsToDecode() {
        let json = Data(#"{"categories":[{"id":"emoji","symbols":["😀"]}]}"#.utf8)

        XCTAssertThrowsError(try JSONDecoder().decode(SymbolTable.self, from: json))
    }
}
