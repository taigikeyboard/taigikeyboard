@testable import TaigiKeyboard
import KeyboardKit
import XCTest

/// Pins `Callouts.TPSCallouts.calloutChars` — the TPS long-press callout
/// list includes the key's own glyph first (long-press ㄗ offers {ㄗ, ㄐ},
/// not just {ㄐ}). Mirrors Android `tpsPopupWithBaseGlyph`. Scope (USER
/// 2026-08-21): letter-variant + number-shortcut keys include their own
/// glyph; punctuation keys ("," / "，") stay variant-only.
final class TPSCalloutCharsTests: XCTestCase {
    func testRepresentativeKeys() {
        // letter variant / number shortcut / multi-variant / mixed variant+digit
        let rows: [(String, [String])] = [
            ("ㄗ", ["ㄗ", "ㄐ"]),
            ("ㆠ", ["ㆠ", "1"]),
            ("ㄫ", ["ㄫ", "ㆭ", "ㄥ"]),
            ("ㆪ", ["ㆪ", "ㆳ", "9"]),
        ]
        for (input, expected) in rows {
            XCTAssertEqual(Callouts.TPSCallouts.calloutChars(for: input), expected, input)
        }
    }

    // Invariant over the whole table — survives table growth: every glyph key
    // gets its own glyph first, every punctuation key stays variant-only.
    func testWholeTable_glyphKeysBaseFirst_punctuationVariantOnly() {
        for (key, variants) in Callouts.TPSCallouts.glyphActions {
            XCTAssertEqual(Callouts.TPSCallouts.calloutChars(for: key), [key] + variants, key)
        }
        for (key, variants) in Callouts.TPSCallouts.punctuationActions {
            XCTAssertEqual(Callouts.TPSCallouts.calloutChars(for: key), variants, key)
        }
    }

    // Keycap hints read `actions` — the merged table must carry every key
    // from both partitions, variants only (no base glyph baked in).
    func testMergedActionsTable_coversBothPartitions_variantsOnly() {
        XCTAssertEqual(
            Callouts.TPSCallouts.actions.count,
            Callouts.TPSCallouts.glyphActions.count + Callouts.TPSCallouts.punctuationActions.count
        )
        for (key, variants) in Callouts.TPSCallouts.glyphActions {
            XCTAssertEqual(Callouts.TPSCallouts.actions[key], variants, key)
        }
        for (key, variants) in Callouts.TPSCallouts.punctuationActions {
            XCTAssertEqual(Callouts.TPSCallouts.actions[key], variants, key)
        }
    }

    func testKeyWithoutCallouts_returnsNil() {
        XCTAssertNil(Callouts.TPSCallouts.calloutChars(for: "ㆦ"))
    }
}
