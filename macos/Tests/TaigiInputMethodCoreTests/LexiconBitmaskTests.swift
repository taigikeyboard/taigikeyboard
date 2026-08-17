// Which dictionary each bit of a search row's bitmask names.

@testable import TaigiInputMethodCore
import XCTest

/// This table is the fourth copy of a layout owned by
/// `dictionary/common/source_bits.py` — the others are the Rust reader, iOS
/// and Android. The toggle tests exercise the Rust ENCODER; nothing else
/// would notice this DECODER drifting, so every bit is asserted by hand.
final class LexiconBitmaskTests: XCTestCase {
    private func source(ofBit bit: UInt32) -> [DictionarySource] {
        LexiconBitmask.sources(from: 1 << bit)
    }

    func testEachBitNamesItsOwnDictionary() {
        XCTAssertEqual(source(ofBit: 0), [.kautian])
        XCTAssertEqual(source(ofBit: 1), [.taigitv])
        XCTAssertEqual(source(ofBit: 2), [.itaigi])
        XCTAssertEqual(source(ofBit: 3), [.sitbut])
        XCTAssertEqual(source(ofBit: 4), [.taihoa])
        XCTAssertEqual(source(ofBit: 5), [.taijit])
        XCTAssertEqual(source(ofBit: 6), [.kungge])
        XCTAssertEqual(source(ofBit: 7), [.stti])
        XCTAssertEqual(source(ofBit: 8), [.khpoo])
        XCTAssertEqual(source(ofBit: 9), [.khiin])
        XCTAssertEqual(source(ofBit: 10), [.dev])
        XCTAssertEqual(source(ofBit: 11), [.lkk])
    }

    func testNoBitsNamesNothing() {
        XCTAssertTrue(LexiconBitmask.sources(from: 0).isEmpty)
    }

    /// Bit 12 marks a record as a variant spelling rather than naming a
    /// dictionary, so it has never been a badge.
    func testTheVariantBitIsNotADictionary() {
        XCTAssertTrue(source(ofBit: 12).isEmpty)
    }

    /// Bits 13 and up are the kautian subcollection region of the QUERY mask.
    /// They describe what was asked for, not what a row belongs to, and a row
    /// carrying them must not sprout badges from them.
    func testTheSubcollectionRegionIsNotADictionary() {
        for bit in UInt32(13) ... 25 {
            XCTAssertTrue(source(ofBit: bit).isEmpty, "bit \(bit) produced a badge")
        }
    }

    /// The badges are drawn in the order this returns them, which is bit
    /// order — not the order the cases happen to be declared in.
    func testSourcesComeBackInBitOrder() {
        let mask: UInt32 = (1 << 11) | (1 << 0) | (1 << 6)

        XCTAssertEqual(LexiconBitmask.sources(from: mask), [.kautian, .kungge, .lkk])
    }

    /// `custom` has no bit: it marks a row from the user's own dictionary,
    /// which is not in `dictionary.bin` at all.
    func testTheCustomDictionaryIsNeverDecodedFromABitmask() {
        XCTAssertFalse(LexiconBitmask.sources(from: .max).contains(.custom))
    }
}
