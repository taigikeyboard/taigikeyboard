// Pins the English current-word rule; Android twin: EnglishCurrentWordTest.

@testable import TaigiKeyboard
import XCTest

final class EnglishAutocompleteServiceTests: XCTestCase {
    private func word(_ text: String) -> String {
        EnglishAutocompleteService.extractCurrentWord(from: text)
    }

    func testWordAfterLastSeparator() {
        XCTAssertEqual(word("say hel"), "hel")
        XCTAssertEqual(word("hello"), "hello")
    }

    func testPunctuationSplits() {
        XCTAssertEqual(word("hi,"), "")
        XCTAssertEqual(word("hi,the"), "the")
        XCTAssertEqual(word("end. Wor"), "Wor")
        XCTAssertEqual(word("line\nwor"), "wor")
    }

    func testApostropheAndHyphenStayInWord() {
        XCTAssertEqual(word("I don't"), "don't")
        XCTAssertEqual(word("a well-known"), "well-known")
    }

    func testTrailingWhitespaceDropped() {
        XCTAssertEqual(word("hello "), "hello")
        XCTAssertEqual(word(""), "")
        XCTAssertEqual(word("   "), "")
    }
}
