// What the recording field shows, and when.

import AppKit
import KeyboardShortcuts
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

    /// The regression the arming hook moved for: recording must be armed by
    /// FOCUS, not by the editing session — `controlTextDidBeginEditing` only
    /// fires on the first text change, which loses the first key press. With
    /// no window, focus is refused outright (a SwiftUI hierarchy still
    /// assembling itself).
    func testBecomingFirstResponder_withoutAWindow_isRefused() {
        let detached = ComposingKeyRecorderField()

        XCTAssertFalse(detached.becomeFirstResponder())
    }

    /// The window's initial key-view pass must walk past the field — a pane
    /// that started recording into its first row unasked would swallow every
    /// key. One runloop turn later the user's own click or Tab lands.
    func testInitialKeyViewFocus_isRefused_untilTheNextRunloopTurn() {
        XCTAssertFalse(field.canBecomeKeyView, "the initial key-view pass must pass by")

        let turn = expectation(description: "next runloop turn")
        DispatchQueue.main.async { turn.fulfill() }
        wait(for: [turn], timeout: 1)

        XCTAssertTrue(field.canBecomeKeyView, "the user's own click or Tab must land")
    }

    /// A chord that collides with a live global hotkey must be recorded, not
    /// acted on: the hotkeys are off for exactly the recording session.
    func testRecording_pausesTheGlobalHotkeys() {
        field.beginRecording()
        XCTAssertFalse(KeyboardShortcuts.isEnabled)

        field.endRecording()
        XCTAssertTrue(KeyboardShortcuts.isEnabled)
    }

    /// Teardown has several entry points — editing end, window resign, view
    /// detach — that overlap on one exit and also fire with no session live.
    /// The global pause must pair one begin to one end regardless.
    func testTeardownEntryPoints_maySafelyOverlap() {
        field.endRecording()
        XCTAssertTrue(KeyboardShortcuts.isEnabled, "an idle teardown must not touch a session it never had")

        field.beginRecording()
        field.endRecording()
        field.endRecording()
        XCTAssertTrue(KeyboardShortcuts.isEnabled, "a doubled teardown ends the session once")
    }
}
