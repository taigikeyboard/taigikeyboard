@testable import TaigiKeyboard
import XCTest

/// Unit tests for `AutocompleteContextBooster` partition logic.
final class AutocompleteContextBoosterTests: XCTestCase {
    // MARK: - Fixtures

    private func word(id: Int, roman: String, hanzi: String?) -> TaigiWord {
        TaigiWord(id: id, roman: roman, hanzi: hanzi, lengthScore: nil)
    }

    // MARK: - Tests

    func testBoost_emptyPredictionsReturnsInputUntouched() {
        let words = [
            word(id: 1, roman: "guá", hanzi: "我"),
            word(id: 2, roman: "tshù", hanzi: "厝"),
        ]

        let result = AutocompleteContextBooster.boost(words: words, predictedFirstChars: [])

        XCTAssertEqual(result.map(\.id), [1, 2])
    }

    func testBoost_partitionsMatchingFirstCharToFront() {
        let words = [
            word(id: 1, roman: "guá", hanzi: "我"), // displayText = 我
            word(id: 2, roman: "tshù", hanzi: "厝"), // displayText = 厝
            word(id: 3, roman: "tsia̍h", hanzi: "食"), // displayText = 食
        ]

        let result = AutocompleteContextBooster.boost(
            words: words,
            predictedFirstChars: ["食", "厝"],
        )

        // 厝 (id:2) 與 食 (id:3) 匹配預測，應拉到前面並保留原順序；未匹配的 我 (id:1) 排最後。
        XCTAssertEqual(result.map(\.id), [2, 3, 1])
    }

    func testBoost_preservesOrderWithinPartitions() {
        let words = [
            word(id: 1, roman: "a", hanzi: "一"),
            word(id: 2, roman: "b", hanzi: "二"),
            word(id: 3, roman: "c", hanzi: "三"),
            word(id: 4, roman: "d", hanzi: "四"),
        ]

        let result = AutocompleteContextBooster.boost(
            words: words,
            predictedFirstChars: ["二", "四"],
        )

        XCTAssertEqual(result.map(\.id), [2, 4, 1, 3])
    }

    func testBoost_noHanziFallsBackToRomanFirstChar() {
        let words = [
            word(id: 1, roman: "guá", hanzi: nil), // displayText = guá
            word(id: 2, roman: "tshù", hanzi: nil), // displayText = tshù
        ]

        let result = AutocompleteContextBooster.boost(
            words: words,
            predictedFirstChars: ["t"],
        )

        XCTAssertEqual(result.map(\.id), [2, 1])
    }

    func testBoost_emptyHanziStringStillFallsBackToRoman() {
        // `displayText` returns `roman` when hanzi is empty OR nil — both branches must work.
        let words = [
            word(id: 1, roman: "alpha", hanzi: ""),
            word(id: 2, roman: "beta", hanzi: ""),
        ]
        let result = AutocompleteContextBooster.boost(words: words, predictedFirstChars: ["b"])
        XCTAssertEqual(result.map(\.id), [2, 1])
    }

    func testBoost_noMatchesReturnsInputOrderUnchanged() {
        let words = [
            word(id: 1, roman: "a", hanzi: "一"),
            word(id: 2, roman: "b", hanzi: "二"),
        ]
        // Predicted chars don't match any candidate — partitioning must be a no-op.
        let result = AutocompleteContextBooster.boost(words: words, predictedFirstChars: ["九"])
        XCTAssertEqual(result.map(\.id), [1, 2])
    }

    func testBoost_stablePartitionOnLargeInput() {
        let total = 100
        let words = (0 ..< total).map { idx in
            word(id: idx, roman: "w\(idx)", hanzi: idx.isMultiple(of: 2) ? "偶" : "奇")
        }
        let result = AutocompleteContextBooster.boost(words: words, predictedFirstChars: ["偶"])
        let evenIds = result.prefix(total / 2).map(\.id)
        let oddIds = result.suffix(total / 2).map(\.id)
        XCTAssertEqual(evenIds, stride(from: 0, to: total, by: 2).map(\.self))
        XCTAssertEqual(oddIds, stride(from: 1, to: total, by: 2).map(\.self))
    }
}
