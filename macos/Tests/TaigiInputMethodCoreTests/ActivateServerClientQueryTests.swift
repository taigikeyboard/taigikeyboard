// Pins the Chromium activation-deadlock rule.

import InputMethodKit
import XCTest

@testable import TaigiInputMethodCore

/// Chromium hosts deadlock when an input method makes a synchronous
/// client round-trip inside `activateServer` (Chromium issue 503787240 —
/// azooKey-Desktop hit this in Chrome on JS-heavy pages). This drives the real
/// `activateServer` with a client that records every read, so the rule breaks
/// loudly the moment someone adds one.
final class ActivateServerClientQueryTests: XCTestCase {
    func testActivateServer_doesNotReadFromClient() throws {
        let client = RecordingTextInputClient()
        let controller = try XCTUnwrap(
            TaigiInputController(server: nil, delegate: nil, client: nil),
            "could not construct the controller under test",
        )

        controller.activateServer(client)

        XCTAssertEqual(
            client.readCallCount,
            0,
            """
            activateServer read from the client (\(client.readCalls.joined(separator: ", "))). \
            A synchronous client round-trip during activation deadlocks Chromium hosts \
            (Chromium issue 503787240) — move the read to the first key event instead.
            """,
        )
    }
}

/// Records only reads: the rule forbids synchronously *asking* the client for
/// state during activation, so the write methods are inert stubs.
private final class RecordingTextInputClient: NSObject, IMKTextInput {
    private(set) var readCalls: [String] = []

    var readCallCount: Int { readCalls.count }

    // MARK: Writes — inert

    func insertText(_ string: Any!, replacementRange: NSRange) {}

    func setMarkedText(_ string: Any!, selectionRange: NSRange, replacementRange: NSRange) {}

    func overrideKeyboard(withKeyboardNamed keyboardUniqueName: String!) {}

    func selectMode(_ modeIdentifier: String!) {}

    // MARK: Reads

    func selectedRange() -> NSRange {
        readCalls.append(#function)
        return NSRange(location: NSNotFound, length: NSNotFound)
    }

    func markedRange() -> NSRange {
        readCalls.append(#function)
        return NSRange(location: NSNotFound, length: NSNotFound)
    }

    func attributedSubstring(from range: NSRange) -> NSAttributedString! {
        readCalls.append(#function)
        return NSAttributedString()
    }

    func length() -> Int {
        readCalls.append(#function)
        return NSNotFound
    }

    func characterIndex(
        for point: NSPoint,
        tracking mappingMode: IMKLocationToOffsetMappingMode,
        inMarkedRange: UnsafeMutablePointer<ObjCBool>!,
    ) -> Int {
        readCalls.append(#function)
        return NSNotFound
    }

    func attributes(
        forCharacterIndex index: Int,
        lineHeightRectangle lineRect: UnsafeMutablePointer<NSRect>!,
    ) -> [AnyHashable: Any]! {
        readCalls.append(#function)
        return [:]
    }

    func validAttributesForMarkedText() -> [Any]! {
        readCalls.append(#function)
        return []
    }

    func supportsUnicode() -> Bool {
        readCalls.append(#function)
        return true
    }

    func bundleIdentifier() -> String! {
        readCalls.append(#function)
        return "com.example.RecordingTextInputClient"
    }

    func windowLevel() -> CGWindowLevel {
        readCalls.append(#function)
        return 0
    }

    func supportsProperty(_ property: TSMDocumentPropertyTag) -> Bool {
        readCalls.append(#function)
        return false
    }

    func uniqueClientIdentifierString() -> String! {
        readCalls.append(#function)
        return "recording-client"
    }

    func string(from range: NSRange, actualRange: NSRangePointer!) -> String! {
        readCalls.append(#function)
        return ""
    }

    func firstRect(forCharacterRange aRange: NSRange, actualRange: NSRangePointer!) -> NSRect {
        readCalls.append(#function)
        return .zero
    }
}
