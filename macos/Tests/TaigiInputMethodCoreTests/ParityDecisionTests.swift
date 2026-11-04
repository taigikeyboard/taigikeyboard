// The settled parity items, end to end through the controller.

import AppKit
@testable import TaigiInputMethodCore
import XCTest

/// The rare inputs the parity decisions settled (docs/architecture/
/// macos-desktop-core-roadmap.md § Parity decisions, P11), pinned end to end
/// through the controller and the core back end. The classification itself
/// is pinned in desktop-core (`keys/intent.rs`, `tests/composing_manager.rs`).
///
/// - E2 (P11c) — a key event carrying several characters.
/// - E4 (P11d) — a format character (Cf). Its next-word cases are the
///   core's (`tests/composing_manager.rs`).
/// - E5 (P11e) — the next-word gate, per scalar: the core's
///   (`tests/composing_manager.rs`).
/// - C4 — a refetch with the window off.
///
/// Not here: E2b (P11c) — a plain Escape inside a multi-character event. The
/// picker reads the core's rule since P14 (`symbol_picker.rs`
/// `e2b_an_escape_inside_a_longer_event_closes_and_passes_through`).
@MainActor
final class ParityDecisionTests: XCTestCase {
    override func setUpWithError() throws {
        try super.setUpWithError()
        restoreStandardSettingsAtTeardown()
        InstalledLexicon.installOnce()
        // These event-classification and spacing cases confirm the literal row, not a dictionary word.
        UserDefaults.standard.set(true, forKey: SettingsStore.Keys.isLiteralRomanCandidateEnabled.name)
    }

    /// E2 (C3), settled P11c: one key event carrying two characters,
    /// mid-composition. Every character has to qualify (`keys/intent.rs`),
    /// so `a.` is document text, not a letter.
    func testE2_aKeyEventCarryingSeveralCharacters() throws {
        let session = try composedSession()

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: "a."), client: session.client,
        )

        XCTAssertTrue(handled)
        XCTAssertEqual(session.client.insertedTexts, ["taigia."], "committed with the event as text")
    }

    /// E2 through the commit-then-insert policies: `a.` is not attaching
    /// punctuation, so Auto-Space puts the space BEFORE it, as for any other
    /// text; and it is not one mapped
    /// key, so the full-width map leaves it alone while a lone `.` maps.
    func testE2_aSeveralCharacterEventThroughAutoSpaceAndFullWidth() throws {
        UserDefaults.standard.set(true, forKey: SettingsStore.Keys.isAutoSpaceEnabled.name)
        let spaced = try composedSession()
        _ = try spaced.controller.handle(TestFixtures.keyDownEvent(characters: "a."), client: spaced.client)
        XCTAssertEqual(spaced.client.insertedTexts, ["taigi a."], "Auto-Space: the space leads non-attaching text")

        UserDefaults.standard.set(false, forKey: SettingsStore.Keys.isAutoSpaceEnabled.name)
        UserDefaults.standard.set(true, forKey: SettingsStore.Keys.isHanjiFirst.name)
        let fullWidth = try composedSession()
        _ = try fullWidth.controller.handle(TestFixtures.keyDownEvent(characters: "a."), client: fullWidth.client)
        XCTAssertEqual(fullWidth.client.insertedTexts, ["taigia."], "full width maps one key, not `a.`")

        // Positive control: the same session's map is on.
        let control = try composedSession()
        _ = try control.controller.handle(TestFixtures.keyDownEvent(characters: "."), client: control.client)
        XCTAssertEqual(control.client.insertedTexts, ["taigi\u{3002}"])
    }

    /// E2 under Telex, idle: `vx` is two letters, not a tone key with nothing
    /// to mark, so it starts a composition (until P11c the Mac
    /// read `v` and passed the event to the host).
    func testE2_aSeveralLetterEventUnderTelex_startsAComposition() throws {
        UserDefaults.standard.set(ToneInputScheme.telex.rawValue, forKey: SettingsStore.Keys.toneInputScheme.name)
        let session = try idleSession()

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: "vx"), client: session.client,
        )

        XCTAssertTrue(handled)
        XCTAssertEqual(session.client.insertedTexts, [])
        guard case let .setMarkedText(marked, _) = try XCTUnwrap(session.client.writes.last) else {
            return XCTFail("`vx` should have started a composition — got \(session.client.writes)")
        }
        XCTAssertEqual(marked, "vx")
    }

    /// E4 (K7), settled P11d: a format character (Cf) typed
    /// mid-composition, isolated or inside a longer event. Text
    /// (`keys/intent.rs`), so the composition commits with it in one write;
    /// until P11d the Mac committed and handed the key to the host.
    func testE4_aFormatCharacterMidComposition() throws {
        for text in ["\u{200B}", "x\u{200D}y", "👩\u{200D}💻"] {
            let session = try composedSession()

            let handled = try session.controller.handle(
                TestFixtures.keyDownEvent(characters: text), client: session.client,
            )

            XCTAssertTrue(handled, "\(text.unicodeScalars)")
            XCTAssertEqual(session.client.insertedTexts, ["taigi" + text], "\(text.unicodeScalars)")
        }
    }

    /// E4 through Auto-Space: a format character is not attaching
    /// punctuation, so the space leads it, as for any other text.
    func testE4_aFormatCharacterThroughAutoSpace() throws {
        UserDefaults.standard.set(true, forKey: SettingsStore.Keys.isAutoSpaceEnabled.name)
        let session = try composedSession()

        _ = try session.controller.handle(TestFixtures.keyDownEvent(characters: "\u{200B}"), client: session.client)

        XCTAssertEqual(session.client.insertedTexts, ["taigi \u{200B}"])
    }

    /// E4 idle: the host types a format character with no composition to
    /// end, as it did before P11d.
    func testE4_anIdleFormatCharacter_isTheHosts() throws {
        let session = try idleSession()

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: "\u{200B}"), client: session.client,
        )

        XCTAssertFalse(handled)
        XCTAssertEqual(session.client.writes, [])
    }

    /// C4: Candidate Display changed while the window is switched off and a
    /// list is still held. Through the controller this needs a narrow
    /// ordering — the window going off takes the list down
    /// (`applyCandidateWindowSettingChange`), but each setting's observer
    /// runs as its own main-actor task: a display-mode write, then the
    /// window-off write, and the display-mode refetch runs while the list is
    /// still up — so it is asked of the back end itself: the core's refetch
    /// (`represent_list`) closes, which is where the window-off observer
    /// leaves the list anyway (the Swift key path repainted; accepted, P10).
    func testC4_aRefetchWithTheCandidateWindowOff() throws {
        let backend = ComposingBackend.shared
        let session = ComposingSessionToken()
        let store = SettingsStore()
        backend.activate(session, settings: store)
        defer { backend.release(session, settings: store) }
        var isListOnScreen = false
        func request() -> ComposingRequest {
            Self.request(session: session, settings: store, isListOnScreen: isListOnScreen)
        }
        for character in "taigi".map(String.init) {
            let reply = backend.key(
                KeyEventSnapshot(characters: character, modifiers: [], isNamedSpecialKey: false),
                in: request(),
            )
            if reply?.effects.contains(where: {
                if case .candidatesChanged = $0 {
                    true
                } else {
                    false
                }
            }) == true {
                isListOnScreen = true
            }
        }
        XCTAssertTrue(isListOnScreen, "taigi puts a list up")
        store.userDefaults.set(false, forKey: SettingsStore.Keys.isCandidateWindowEnabled.name)

        let represented = backend.represent(refetch: true, in: request())

        XCTAssertEqual(represented, .closed, "C4: the window is off, so the list goes")
    }

    // MARK: - Session

    /// A request with no slot map and no swap, the highlight on the first
    /// cell while a list is up.
    private static func request(
        session: ComposingSessionToken,
        settings: SettingsStore,
        isListOnScreen: Bool,
    ) -> ComposingRequest {
        ComposingRequest(
            session: session,
            settings: settings,
            panel: ComposingPanelState(
                isListOnScreen: isListOnScreen,
                selectedIndex: { isListOnScreen ? 0 : nil },
                indexForKeySlot: { _ in nil },
                canSwapPrecedingSpace: nil,
            ),
        )
    }

    private struct Session: CandidateBarSession {
        let controller: TaigiInputController
        let client: RecordingTextInputClient
        let presenter: RecordingCandidatePresenter
    }

    /// A session over `.standard` that has typed `taigi`.
    private func composedSession() throws -> Session {
        let session = try idleSession()
        try session.type("taigi")
        XCTAssertEqual(session.presenter.cells.first?.text, "taigi")
        return session
    }

    /// An activated session over `.standard` with nothing typed.
    private func idleSession() throws -> Session {
        let client = RecordingTextInputClient()
        client.caretRects = [4: CGRect(x: 120, y: 400, width: 1, height: 18)]
        let controller = try TestFixtures.makeInputController()
        let presenter = RecordingCandidatePresenter()
        controller.candidatePresenter = presenter
        controller.settings = SettingsStore()
        controller.activateServer(client)
        return Session(controller: controller, client: client, presenter: presenter)
    }
}
