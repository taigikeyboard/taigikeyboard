// Executable spec for the input method's key contract.

import AppKit
import XCTest

@testable import TaigiInputMethodCore

/// Every key classified as anything but `.passThrough` is a key the host never
/// receives, so this table is also the list of things the user can no longer do
/// in their app while typing. It is asserted in both composition states because
/// most keys mean different things in each.
final class ComposingKeyIntentTests: XCTestCase {
    func testRomanizationCharacters_areComposingInput_inBothStates() throws {
        for characters in ["t", "A", "-"] {
            let event = try TestFixtures.keyDownEvent(characters: characters)
            XCTAssertEqual(
                ComposingKeyIntent.intent(for: KeyEventSnapshot(event), isComposing: false),
                .input(characters),
                "'\(characters)' must be able to start a composition",
            )
            XCTAssertEqual(
                ComposingKeyIntent.intent(for: KeyEventSnapshot(event), isComposing: true),
                .input(characters),
                "'\(characters)' must extend a running composition",
            )
        }
    }

    func testDigits_areToneMarkersOnlyWhileComposing() throws {
        let event = try TestFixtures.keyDownEvent(characters: "5")

        XCTAssertEqual(
            ComposingKeyIntent.intent(for: KeyEventSnapshot(event), isComposing: true),
            .input("5"),
            "a digit typed after romanization is the numeric tone of `tai5`",
        )
        XCTAssertEqual(
            ComposingKeyIntent.intent(for: KeyEventSnapshot(event), isComposing: false),
            .passThrough,
            "a bare digit is a digit — starting a composition with it would make numbers untypable",
        )
    }

    func testCompositionControlKeys_belongToTheHostWhenThereIsNoComposition() throws {
        let cases: [(name: String, characters: String, composing: ComposingKeyIntent)] = [
            ("Return", "\r", .commit),
            ("Escape", "\u{1B}", .cancel),
            ("Backspace", "\u{8}", .deleteBackward),
            ("Delete", "\u{7F}", .deleteBackward),
            ("Space", " ", .commitThenInsert(" ")),
            ("Period", ".", .commitThenInsert(".")),
        ]

        for testCase in cases {
            let event = try TestFixtures.keyDownEvent(characters: testCase.characters)
            XCTAssertEqual(
                ComposingKeyIntent.intent(for: KeyEventSnapshot(event), isComposing: true),
                testCase.composing,
                "\(testCase.name) acts on the composition while one is running",
            )
            XCTAssertEqual(
                ComposingKeyIntent.intent(for: KeyEventSnapshot(event), isComposing: false),
                .passThrough,
                "\(testCase.name) belongs to the host when there is no composition to act on",
            )
        }
    }

    func testHostOwnedEvents_fallThrough_evenMidComposition() throws {
        let cases: [(name: String, characters: String, modifiers: NSEvent.ModifierFlags)] = [
            ("Command chord", "s", .command),
            ("Control chord", "a", .control),
            ("Option chord", "a", .option),
            ("Tab", "\t", []),
            ("Left arrow", "\u{F702}", []),
            ("F1", "\u{F704}", []),
            ("Line separator", "\u{2028}", []),
        ]

        for testCase in cases {
            let event = try TestFixtures.keyDownEvent(characters: testCase.characters, modifiers: testCase.modifiers)
            XCTAssertEqual(
                ComposingKeyIntent.intent(for: KeyEventSnapshot(event), isComposing: true),
                .commitThenPassThrough,
                """
                \(testCase.name) is the host's even mid-composition, but the composition must be \
                finished first — the host is about to act on the document it sits in
                """,
            )
            XCTAssertEqual(
                ComposingKeyIntent.intent(for: KeyEventSnapshot(event), isComposing: false),
                .passThrough,
                "\(testCase.name) with no composition running needs no finishing step",
            )
        }
    }

    func testNonRomanizationLetters_areDocumentTextRatherThanEngineInput() throws {
        let event = try TestFixtures.keyDownEvent(characters: "字")

        XCTAssertEqual(
            ComposingKeyIntent.intent(for: KeyEventSnapshot(event), isComposing: true),
            .commitThenInsert("字"),
            "a character the engine cannot parse ends the composition and goes to the document",
        )
        XCTAssertEqual(
            ComposingKeyIntent.intent(for: KeyEventSnapshot(event), isComposing: false),
            .passThrough,
        )
    }

    func testEmptyCharacters_fallThrough() throws {
        XCTAssertEqual(
            ComposingKeyIntent.intent(for: KeyEventSnapshot(try TestFixtures.keyDownEvent(characters: "")), isComposing: false),
            .passThrough,
            "an event carrying no characters has nothing to compose",
        )
    }

    func testNonAsciiDigits_areDocumentTextRatherThanTones() throws {
        let event = try TestFixtures.keyDownEvent(characters: "\u{FF15}") // full-width ５

        XCTAssertEqual(
            ComposingKeyIntent.intent(for: KeyEventSnapshot(event), isComposing: true),
            .commitThenInsert("\u{FF15}"),
            "the engine's tone markers are ASCII digits — a full-width numeral is document text",
        )
    }
}
