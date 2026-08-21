// What the recording field shows, and when.

import AppKit
@testable import TaigiInputMethodCore
import XCTest

/// The field is an `NSSearchField`, so a placeholder is only visible while its
/// text is empty. That one AppKit rule is what these cases are about: a row that
/// already holds a chord has to be blanked to ask for a new one, or the prompt
/// and any refusal would be written where nobody can read them.
@MainActor
final class ComposingKeyRecorderTests: XCTestCase {
    private var suiteName = ""
    private var userDefaults = UserDefaults.standard
    private var window: NSWindow!
    private var field: ComposingKeyRecorderField!

    override func setUpWithError() throws {
        try super.setUpWithError()
        suiteName = "ComposingKeyRecorderTests.\(UUID().uuidString)"
        userDefaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        field = ComposingKeyRecorderField()
        field.language = TestFixtures.makeDisplayLanguageStore(.hanji, userDefaults: userDefaults)
        // A field with no window cannot become first responder.
        window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 300, height: 100),
            styleMask: [.titled],
            backing: .buffered,
            defer: false,
        )
        window.isReleasedWhenClosed = false
        window.contentView?.addSubview(field)
    }

    override func tearDown() {
        window.contentView?.subviews.forEach { $0.removeFromSuperview() }
        window = nil
        field = nil
        userDefaults.removePersistentDomain(forName: suiteName)
        super.tearDown()
    }

    private func chord(_ key: String, _ modifiers: NSEvent.ModifierFlags = []) throws -> ComposingKeyChord {
        try ComposingKeyChord.make(key: key, modifiers: modifiers).get()
    }

    func testABoundRow_showsItsChord() throws {
        field.chord = try chord("\r", .shift)

        XCTAssertEqual(field.stringValue, "⇧↩")
    }

    /// The regression this file exists for: a row holding a chord kept showing
    /// it while recording, so the prompt — and any refusal — had nowhere to be
    /// seen.
    func testRecordingABoundRow_blanksItSoThePromptIsVisible() throws {
        field.chord = try chord("\r", .shift)

        field.beginRecording()

        XCTAssertEqual(field.stringValue, "", "a placeholder only shows while the text is empty")
        XCTAssertEqual(field.placeholderString, "撳一个鍵…")
    }

    func testLeavingWithoutRecording_putsTheChordBack() throws {
        field.chord = try chord("\r", .shift)
        field.beginRecording()

        field.endRecording()

        XCTAssertEqual(field.stringValue, "⇧↩")
        XCTAssertEqual(field.chord, try chord("\r", .shift), "leaving changed nothing")
    }

    func testAnEmptyRow_promptsToRecord() {
        XCTAssertEqual(field.stringValue, "")
        XCTAssertEqual(field.placeholderString, "無設定")
    }
}
