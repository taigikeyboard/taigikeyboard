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
final class ShortcutKeyRecorderTests: XCTestCase {
    private var suiteName = ""
    private var userDefaults = UserDefaults.standard
    private var window: NSWindow!
    private var field: ShortcutKeyRecorderField!

    override func setUpWithError() throws {
        try super.setUpWithError()
        suiteName = "ShortcutKeyRecorderTests.\(UUID().uuidString)"
        userDefaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        field = ShortcutKeyRecorderField()
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
        // Resolved through the same store the field uses, so an i18n wording change
        // cannot fail this test (#678 renamed 撳 → 揤 and it did).
        XCTAssertEqual(field.placeholderString, field.language?.string(.desktopShortcutRecording))
        XCTAssertFalse(field.placeholderString?.isEmpty ?? true, "the prompt must be non-empty")
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
        let detached = ShortcutKeyRecorderField()

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

/// What a GLOBAL row accepts, now that it records through the first-party
/// field rather than `KeyboardShortcuts.Recorder`.
///
/// The bug this replaced: the library beeps at every modifier-less key before
/// any validation of ours runs (`RecorderCocoa.swift:404-410`), so 漢羅代先
/// could not be moved onto a bare `z` — while SHIPPING on a bare backtick
/// (USER 2026-08-26, real device).
@MainActor
final class GlobalShortcutPolicyTests: XCTestCase {
    private func key(
        _ character: String,
        _ modifiers: NSEvent.ModifierFlags = [],
        shortcut: KeyboardShortcuts.Shortcut?,
    ) throws -> RecordedShortcutKey {
        try RecordedShortcutKey(
            chord: ComposingKeyChord.make(key: character, modifiers: modifiers).get(),
            globalShortcut: shortcut,
        )
    }

    /// The reported failure, as a test: a bare key the gate passes must pass a
    /// global row too. The key is `'` rather than the `z` of the original
    /// report — every letter is a typing key since Telex claimed the eight
    /// no syllable spells (2026-09-08).
    func testABarePunctuationKey_isAccepted() throws {
        let recorded = try key("'", shortcut: .init(.quote))

        XCTAssertNil(GlobalShortcutPolicy.rejection(for: recorded))
    }

    /// The bare key the action ships on. It was unrecordable for the same
    /// reason `z` was, which is what made the shipped default a one-way door.
    func testTheBareBacktickItShipsOn_isAccepted() throws {
        let recorded = try key("`", shortcut: .init(.backtick))

        XCTAssertNil(GlobalShortcutPolicy.rejection(for: recorded))
    }

    /// The shared gate still defends the keys a syllable is spelled with —
    /// this policy never sees them, because `make` refuses first.
    func testABareSyllableLetter_neverReachesThePolicy() {
        switch ComposingKeyChord.make(key: "a", modifiers: []) {
        case .success: XCTFail("a bare `a` would cost the user the letter")
        case let .failure(reason): XCTAssertEqual(reason, .typesRomanization)
        }
    }

    /// Modifier chords were never the broken half, and must stay accepted.
    func testAModifierChord_isAccepted() throws {
        let recorded = try key("j", [.control, .option], shortcut: .init(.j, modifiers: [.control, .option]))

        XCTAssertNil(GlobalShortcutPolicy.rejection(for: recorded))
    }

    /// ⌘ plus a key is where a Mac application puts its own menu commands, and
    /// a global hotkey takes that key from the host for as long as this input
    /// source is selected — ⌘, would cost the user their app's own settings
    /// command, which the menu would then answer with `showPreferences:` while
    /// closed. The library used to enforce this indirectly, by refusing the
    /// modifier-less keys this recorder now accepts.
    func testABareCommandChord_isRefused() throws {
        let comma = try key(",", [.command], shortcut: .init(.comma, modifiers: [.command]))
        let shifted = try key("s", [.command, .shift], shortcut: .init(.s, modifiers: [.command, .shift]))

        XCTAssertEqual(GlobalShortcutPolicy.rejection(for: comma), .belongsToHost)
        XCTAssertEqual(GlobalShortcutPolicy.rejection(for: shifted), .belongsToHost)
    }

    /// ⌃⌘ and ⌥⌘ are this input method's own family — both shipped defaults
    /// live there, so the refusal above must not reach them.
    func testCommandWithAnotherModifier_isAccepted() throws {
        let recorded = try key("s", [.control, .command], shortcut: .init(.s, modifiers: [.control, .command]))

        XCTAssertNil(GlobalShortcutPolicy.rejection(for: recorded))
    }

    /// Every chord this app hands out has to be recordable, or 恢復預設設定
    /// would leave a row the recorder itself refuses. That is not hypothetical:
    /// `CopySymbolicHotKeys` reported ⌃⌘S — the settings doorway's own default
    /// — as an enabled system shortcut on the development Mac.
    func testEveryShippedDefault_isAccepted() throws {
        for action in ShortcutAction.allCases {
            let shortcut = try XCTUnwrap(action.defaultShortcut, "\(action) ships unbound")
            let chord = try XCTUnwrap(
                ShortcutConflicts.composingChord(occupiedBy: shortcut),
                "\(action)'s default does not cross the registry bridge",
            )
            let recorded = RecordedShortcutKey(chord: chord, globalShortcut: shortcut)

            XCTAssertNil(GlobalShortcutPolicy.rejection(for: recorded), "\(action) cannot be restored")
        }
    }

    /// A bare key is never judged against the system table: that table's
    /// "enabled" entries included bare `a`, `s`, `f`, `q` and the bare backtick
    /// this app ships 漢羅代先 on (probe, 2026-08-26). They are slots, not
    /// shortcuts a user could name, and refusing them would rebuild the one-way
    /// door this recorder exists to remove. Punctuation here because every
    /// letter is a typing key since Telex claimed the free eight (2026-09-08)
    /// — the shared gate refuses those before this policy ever sees them.
    func testBareKeys_areNotJudgedAgainstTheSystemTable() throws {
        for character in ["`", "[", "]", "'", ","] {
            let scalar = try XCTUnwrap(character.first?.asciiValue)
            let recorded = try key(
                character,
                shortcut: .init(carbonKeyCode: Int(scalar)),
            )

            XCTAssertNil(GlobalShortcutPolicy.rejection(for: recorded), "bare \(character) was refused")
        }
    }

    /// A global row stores a Carbon key code; a press that yields none has
    /// nothing to store, however happily the composing tier would take it.
    func testAPressWithNoCarbonKeyCode_isRefused() throws {
        let recorded = try key("`", shortcut: nil)

        XCTAssertEqual(GlobalShortcutPolicy.rejection(for: recorded), .notAGlobalKey)
    }
}
