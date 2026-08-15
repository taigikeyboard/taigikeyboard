// Executable spec for which key events the controller consumes.

import AppKit
import XCTest

@testable import TaigiInputMethodCore

/// Anything `echoableText(of:)` accepts is swallowed by the input method, so a
/// key that belongs to the host — a shortcut, caret navigation, Return, Escape
/// — must come back `nil` or the host silently loses it.
final class TaigiInputControllerEventTests: XCTestCase {
    func testEchoableText_plainTypedCharacters_areEchoed() throws {
        let cases: [(characters: String, modifiers: NSEvent.ModifierFlags)] = [
            ("a", []),
            ("A", .shift),
            ("7", []),
            (" ", []),
            ("-", []),
        ]

        for testCase in cases {
            let event = try keyDownEvent(characters: testCase.characters, modifiers: testCase.modifiers)
            XCTAssertEqual(
                TaigiInputController.echoableText(of: event),
                testCase.characters,
                "'\(testCase.characters)' is ordinary text and must be echoed",
            )
        }
    }

    func testEchoableText_hostOwnedEvents_fallThrough() throws {
        let leftArrow = "\u{F702}" // NSLeftArrowFunctionKey
        let functionKeyOne = "\u{F704}" // NSF1FunctionKey
        let cases: [(name: String, characters: String, modifiers: NSEvent.ModifierFlags)] = [
            ("Command chord", "a", .command),
            ("Control chord", "a", .control),
            ("Option chord", "a", .option),
            ("Return", "\r", []),
            ("Tab", "\t", []),
            ("Escape", "\u{1B}", []),
            ("Backspace", "\u{08}", []),
            ("Left arrow", leftArrow, []),
            ("F1", functionKeyOne, []),
        ]

        for testCase in cases {
            let event = try keyDownEvent(characters: testCase.characters, modifiers: testCase.modifiers)
            XCTAssertNil(
                TaigiInputController.echoableText(of: event),
                "\(testCase.name) belongs to the host and must not be consumed",
            )
        }
    }

    func testEchoableText_emptyCharacters_fallThrough() throws {
        XCTAssertNil(
            TaigiInputController.echoableText(of: try keyDownEvent(characters: "", modifiers: [])),
            "an event carrying no characters has nothing to echo",
        )
    }

    private func keyDownEvent(
        characters: String,
        modifiers: NSEvent.ModifierFlags,
    ) throws -> NSEvent {
        try XCTUnwrap(NSEvent.keyEvent(
            with: .keyDown,
            location: .zero,
            modifierFlags: modifiers,
            timestamp: 0,
            windowNumber: 0,
            context: nil,
            characters: characters,
            charactersIgnoringModifiers: characters,
            isARepeat: false,
            keyCode: 0,
        ))
    }
}
