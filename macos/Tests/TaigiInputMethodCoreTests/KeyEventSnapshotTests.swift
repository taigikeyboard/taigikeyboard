// Executable spec for the key predicates the Swift side still reads.

import AppKit
@testable import TaigiInputMethodCore
import XCTest

/// What a key event is to the controller and the core back end: a plain
/// Escape, document text, the TPS punctuation chord. What a key means to a
/// composition, the symbol picker and the Shortcuts pane is desktop-core's
/// (`keys/`).
final class KeyEventSnapshotTests: XCTestCase {
    // MARK: - Document text

    /// The pass-through path reports document text to the engine so a full stop
    /// can end a learning context. What it must NOT report is a key the host
    /// acts on — Escape and Return are pass-through too, and neither is a
    /// character anyone typed into a document.
    func testIsDocumentText_acceptsPrintableCharactersAndRejectsKeysTheHostActsOn() {
        for text in ["。", "、", "!", "?", " ", "台", "x"] {
            XCTAssertTrue(
                isDocumentText(textSnapshot(text)),
                "'\(text)' is text the host puts into its document",
            )
        }
        for text in ["\u{1B}", "\r", "\u{8}", "\u{7F}"] {
            XCTAssertFalse(
                isDocumentText(textSnapshot(text)),
                "a control character is a command, not document text",
            )
        }
    }

    /// `⌘.` and a typed `.` carry the same character. One of them inserts
    /// nothing, and reporting it as document text would end a learning context
    /// on a keystroke that never reached the document.
    func testIsDocumentText_rejectsAHostChordCarryingAPrintableCharacter() {
        // ⌃. included: outside TPS no chord is document text (the TPS
        // punctuation chord: `testTpsPunctuationChord…`).
        for modifier in [NSEvent.ModifierFlags.command, .control, .option] {
            for text in ["x", "."] {
                XCTAssertFalse(
                    isDocumentText(textSnapshot(text, modifiers: modifier)),
                    "a chord is a host command however printable its character is",
                )
            }
        }
        XCTAssertTrue(
            isDocumentText(textSnapshot(".", modifiers: .shift)),
            "Shift is how the character was typed, not a command",
        )
    }

    func testIsDocumentText_rejectsArrowsAndFunctionKeys() throws {
        let leftArrow = try String(XCTUnwrap(UnicodeScalar(NSLeftArrowFunctionKey)))
        let functionKey = try String(XCTUnwrap(UnicodeScalar(NSF5FunctionKey)))

        XCTAssertFalse(isDocumentText(textSnapshot(leftArrow)))
        XCTAssertFalse(isDocumentText(textSnapshot(functionKey)))
        XCTAssertFalse(
            isDocumentText(textSnapshot("\u{2028}", isNamedSpecialKey: true)),
            "a line separator is a named key AppKit gives us, not typed text",
        )
    }

    func testIsDocumentText_rejectsNothingAtAll() {
        XCTAssertFalse(isDocumentText(textSnapshot(nil)))
        XCTAssertFalse(isDocumentText(textSnapshot("")))
    }

    /// A format character (Cf) is text the user typed, isolated or inside a
    /// longer event — the desktop core's rule (roadmap E4, settled P11d).
    /// Only a control character (Cc) is a command.
    func testFormatCharacters_areDocumentText_andControlCharactersAreNot() {
        // trace: Cf is not Cc and not in F700…F8FF → every scalar is text.
        for text in [
            "\u{200B}", "\u{200C}", "\u{AD}", "\u{FEFF}", "\u{2066}", "x\u{200D}y", "👩\u{200D}💻",
            "🏴\u{E0067}\u{E0062}\u{E0073}\u{E0063}\u{E0074}\u{E007F}", // emoji tag sequence (plane 14 Cf)
        ] {
            let key = textSnapshot(text)
            XCTAssertEqual(key.documentText(inputMode: .tl), text, "\(text.unicodeScalars)")
        }
        // Negative control: C0, C1 (NEL) and DEL are not text.
        for text in ["\u{1}", "\u{85}"] {
            let key = textSnapshot(text)
            XCTAssertFalse(isDocumentText(key), "\(text.unicodeScalars)")
        }
        XCTAssertFalse(isDocumentText(textSnapshot("\u{7F}")))
    }

    // MARK: - TPS punctuation chord

    /// Under TPS, ⌃ on a punctuation key types its full-width mark. The
    /// predicate answers the key as typed — the mapping comes later — and the
    /// key is read under the modifier: `⌃,` arrives as `,`, `⌃[` as Escape,
    /// `⌃⇧,` as `<`. Outside TPS the same keys are the host's chords.
    func testTpsPunctuationChord_typesTheMappedKeyInBothStates() throws {
        // trace: AppKit's `charactersIgnoringModifiers` keeps Shift, so ⌃⇧,
        // reads `<`; Control rewrites `[` to `\u{1B}` in `characters` only.
        let comma = try KeyEventSnapshot(TestFixtures.keyDownEvent(
            characters: ",", modifiers: .control, charactersIgnoringModifiers: ",",
        ))
        let bracket = try KeyEventSnapshot(TestFixtures.keyDownEvent(
            characters: "\u{1B}", modifiers: .control, charactersIgnoringModifiers: "[",
        ))
        let angle = try KeyEventSnapshot(TestFixtures.keyDownEvent(
            characters: "<", modifiers: [.control, .shift], charactersIgnoringModifiers: "<",
        ))
        for (key, expected) in [(comma, ","), (bracket, "["), (angle, "<")] {
            XCTAssertEqual(key.tpsPunctuationChord(inputMode: .tps), expected.first)
            XCTAssertEqual(key.documentText(inputMode: .tps), expected)
            for mode in [InputMode.tl, .poj] {
                XCTAssertNil(key.tpsPunctuationChord(inputMode: mode), "\(mode)")
                XCTAssertNil(key.documentText(inputMode: mode), "\(mode): the host's chord")
            }
        }
    }

    /// Exactly ⌃ on a mapped key: another chording modifier beside it, a key
    /// the policy does not map or leaves to the host (⌃⇧` is VS Code's New
    /// Terminal), or a bare key is no chord.
    func testTpsPunctuationChord_needsExactlyControlOnAMappedKey() throws {
        let withCommand = try KeyEventSnapshot(TestFixtures.keyDownEvent(
            characters: ",", modifiers: [.control, .command], charactersIgnoringModifiers: ",",
        ))
        let withOption = try KeyEventSnapshot(TestFixtures.keyDownEvent(
            characters: ",", modifiers: [.control, .option], charactersIgnoringModifiers: ",",
        ))
        let letter = try KeyEventSnapshot(TestFixtures.keyDownEvent(
            characters: "\u{13}", modifiers: .control, charactersIgnoringModifiers: "s",
        ))
        let hyphen = try KeyEventSnapshot(TestFixtures.keyDownEvent(
            characters: "-", modifiers: .control, charactersIgnoringModifiers: "-",
        ))
        let quote = try KeyEventSnapshot(TestFixtures.keyDownEvent(
            characters: "\"", modifiers: .control, charactersIgnoringModifiers: "\"",
        ))
        let tilde = try KeyEventSnapshot(TestFixtures.keyDownEvent(
            characters: "`", modifiers: [.control, .shift], charactersIgnoringModifiers: "~",
        ))
        for key in [withCommand, withOption, letter, hyphen, quote, tilde] {
            XCTAssertNil(key.tpsPunctuationChord(inputMode: .tps), "\(key)")
        }
        XCTAssertNil(textSnapshot(",").tpsPunctuationChord(inputMode: .tps))
    }

    /// Text the host will put into its document, rather than a key it will act on.
    private func isDocumentText(_ key: KeyEventSnapshot) -> Bool {
        key.documentText(inputMode: .tl) != nil
    }

    private func textSnapshot(
        _ characters: String?,
        modifiers: NSEvent.ModifierFlags = [],
        isNamedSpecialKey: Bool = false,
    ) -> KeyEventSnapshot {
        KeyEventSnapshot(
            characters: characters,
            modifiers: modifiers,
            isNamedSpecialKey: isNamedSpecialKey,
        )
    }
}
