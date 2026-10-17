// The TPS key panel end to end: the chord that asks for it, the modes it shows under, and who may take it down.

import InputMethodKit
@testable import TaigiInputMethodCore
import XCTest

/// Drives the real controller against the shared panel. What is pinned is
/// the routing — the stored wish, when the panel is up, whose teardown may
/// close it — not how the caps are drawn.
@MainActor
final class TaigiInputControllerTpsKeyboardTests: XCTestCase {
    private var suiteName = ""
    private var userDefaults = UserDefaults.standard

    override func setUpWithError() throws {
        try super.setUpWithError()
        InstalledLexicon.installOnce()
        suiteName = "TaigiInputControllerTpsKeyboardTests.\(UUID().uuidString)"
        userDefaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        // Process-wide, like the guide: a panel left up by one case would be
        // the panel the next case finds.
        TpsKeyboardPanel.shared.hideNow()
    }

    override func tearDown() {
        TpsKeyboardPanel.shared.flashSleep = { try await Task.sleep(for: $0) }
        TpsKeyboardPanel.shared.hideNow()
        userDefaults.removePersistentDomain(forName: suiteName)
        super.tearDown()
    }

    // MARK: - The chord

    func testTheChord_underTps_showsThePanel_andStoresTheWish() throws {
        let session = try makeSession(under: .tps)

        session.controller.performShortcutAction(.showTpsKeyboard)

        XCTAssertTrue(session.controller.settings.isTpsKeyboardShown)
        XCTAssertTrue(TpsKeyboardPanel.shared.isShowing)
        XCTAssertNotNil(TpsKeyboardPanel.shared.owner, "a panel on screen names the session that raised it")
    }

    /// trace: `tps_keyboard_rows` — 12 + 10 + 10 + 10 caps, read from the
    /// core through `KeyRules.tpsKeyboardRows`.
    func testThePanel_drawsTheCoresRows() throws {
        let session = try makeSession(under: .tps)

        session.controller.performShortcutAction(.showTpsKeyboard)

        XCTAssertEqual(TpsKeyboardPanel.shared.capCount, 42)
    }

    func testTheChord_again_hidesThePanel_andClearsTheWish() throws {
        let session = try makeSession(under: .tps)
        session.controller.performShortcutAction(.showTpsKeyboard)

        session.controller.performShortcutAction(.showTpsKeyboard)

        XCTAssertFalse(session.controller.settings.isTpsKeyboardShown)
        XCTAssertFalse(TpsKeyboardPanel.shared.isShowing)
    }

    /// Inert outside TPS, as desktop-core's `is_inert_under` says: no write,
    /// nothing shown.
    func testTheChord_underARomanization_isInert() throws {
        for mode in [InputMode.tl, .poj] {
            let session = try makeSession(under: mode)

            session.controller.performShortcutAction(.showTpsKeyboard)

            XCTAssertNil(userDefaults.object(forKey: SettingsStore.Keys.isTpsKeyboardShown.name), "\(mode)")
            XCTAssertFalse(TpsKeyboardPanel.shared.isShowing, "\(mode)")
        }
    }

    // MARK: - The mode

    /// Leaving TPS takes the panel down and keeps the wish, so coming back
    /// to TPS brings it back.
    func testLeavingTps_hidesThePanel_andComingBack_showsItAgain() throws {
        let session = try makeSession(under: .tps)
        session.controller.performShortcutAction(.showTpsKeyboard)

        session.controller.performShortcutAction(.toggleTps)

        XCTAssertEqual(session.controller.settings.inputMode, .tl)
        XCTAssertFalse(TpsKeyboardPanel.shared.isShowing)
        XCTAssertTrue(session.controller.settings.isTpsKeyboardShown, "the wish is kept for the way back")

        session.controller.performShortcutAction(.toggleTps)

        XCTAssertTrue(TpsKeyboardPanel.shared.isShowing)
    }

    /// The wish stored by an earlier session: the next one to take focus
    /// under TPS puts the panel up.
    func testActivation_underTps_withTheWishStored_showsThePanel() throws {
        userDefaults.set(true, forKey: SettingsStore.Keys.isTpsKeyboardShown.name)

        _ = try makeSession(under: .tps)

        XCTAssertTrue(TpsKeyboardPanel.shared.isShowing)
    }

    func testHidePalettes_hidesThePanel() throws {
        let session = try makeSession(under: .tps)
        session.controller.performShortcutAction(.showTpsKeyboard)

        session.controller.hidePalettes()

        XCTAssertFalse(TpsKeyboardPanel.shared.isShowing)
        XCTAssertTrue(session.controller.settings.isTpsKeyboardShown, "the system's hide is not the user's")
    }

    // MARK: - Ownership

    /// IMK activates the incoming session before it deactivates the outgoing
    /// one: the outgoing session's teardown must not close the panel the
    /// arriving one now shows — the guide's rule.
    func testDeactivate_ofAnotherSession_leavesTheLiveSessionsPanelAlone() throws {
        userDefaults.set(true, forKey: SettingsStore.Keys.isTpsKeyboardShown.name)
        let leaving = try makeSession(under: .tps)
        let arriving = try makeSession(under: .tps)

        leaving.controller.deactivateServer(leaving.client)

        XCTAssertTrue(TpsKeyboardPanel.shared.isShowing, "a session that no longer showed the panel took it down")

        arriving.controller.deactivateServer(arriving.client)

        XCTAssertFalse(TpsKeyboardPanel.shared.isShowing, "the panel goes with the focus of the session showing it")
        XCTAssertTrue(arriving.controller.settings.isTpsKeyboardShown, "focus leaving is not the user's hide")
    }

    // MARK: - Clicks

    /// A click types its glyph into the client of the session showing the
    /// panel, through the core's `TpsKey` (`a_tps_keyboard_press_types_its_glyph_even_over_an_open_list`).
    func testAPress_typesTheGlyphIntoTheSessionShowingThePanel() throws {
        let session = try makeSession(under: .tps)
        session.controller.performShortcutAction(.showTpsKeyboard)

        TpsKeyboardPanel.shared.press("ㄍ")
        TpsKeyboardPanel.shared.press("ㄚ")

        XCTAssertEqual(session.client.writes.last, .setMarkedText("ㄍㄚ", selectionLocation: 2))
    }

    /// The session the panel was handed to types; the one that left it
    /// types nothing, even called directly.
    func testAPress_afterAHandover_reachesOnlyTheArrivingSession() throws {
        userDefaults.set(true, forKey: SettingsStore.Keys.isTpsKeyboardShown.name)
        let leaving = try makeSession(under: .tps)
        let arriving = try makeSession(under: .tps)
        leaving.controller.deactivateServer(leaving.client)
        leaving.client.clearWrites()

        TpsKeyboardPanel.shared.press("ㄍ")
        leaving.controller.typeTpsKeyboardGlyph("ㄚ", generation: 1)

        XCTAssertEqual(leaving.client.writes, [])
        XCTAssertEqual(arriving.client.writes.last, .setMarkedText("ㄍ", selectionLocation: 1))
    }

    /// A press carrying a tenure that has ended — the panel shown before
    /// this session went and came back — types nothing.
    func testAPress_fromAnEndedTenure_typesNothing() throws {
        let session = try makeSession(under: .tps)
        session.controller.performShortcutAction(.showTpsKeyboard)
        session.controller.deactivateServer(session.client)
        session.controller.activateServer(session.client)
        session.client.clearWrites()

        // trace: tenure 1 at the first activation, 2 at the teardown, 3 now.
        session.controller.typeTpsKeyboardGlyph("ㄍ", generation: 1)
        XCTAssertEqual(session.client.writes, [])

        TpsKeyboardPanel.shared.press("ㄍ")
        XCTAssertEqual(session.client.writes.last, .setMarkedText("ㄍ", selectionLocation: 1))
    }

    /// Outside TPS nothing is typed and nothing is taken down: a click that
    /// raced a switch.
    func testAPress_underARomanization_typesNothing() throws {
        let session = try makeSession(under: .tl)

        // trace: one activation → tenure 1, the live one; only the mode refuses.
        session.controller.typeTpsKeyboardGlyph("ㄍ", generation: 1)

        XCTAssertEqual(session.client.writes, [])
    }

    /// trace: `tps_keyboard_rows` — `E` ㄍ / ㆣ, `Q` ㄆ with no Shift glyph.
    func testThePressedGlyph_isTheShiftGlyphOnlyWhereTheKeyHasOne() throws {
        let caps = TpsKeyboardPanel.shared.caps
        let capE = try XCTUnwrap(caps.first { $0.label == "E" })
        let capQ = try XCTUnwrap(caps.first { $0.label == "Q" })

        XCTAssertEqual(TpsKeyboardPanel.pressedGlyph(of: capE, isShiftLayer: false), "ㄍ")
        XCTAssertEqual(TpsKeyboardPanel.pressedGlyph(of: capE, isShiftLayer: true), "ㆣ")
        XCTAssertEqual(TpsKeyboardPanel.pressedGlyph(of: capQ, isShiftLayer: true), "ㄆ")
    }

    // MARK: - Flash

    /// A key the engine takes lights its cap; the next key moves the light.
    /// trace: `tps_keyboard_rows` — `e` (ㄍ) is row 1 cap 2, `8` (ㄚ) row 0 cap 7.
    func testAKeyTypingAGlyph_flashesItsCap() throws {
        let session = try makeSession(under: .tps)
        session.controller.performShortcutAction(.showTpsKeyboard)

        _ = try session.controller.handle(TestFixtures.keyDownEvent(characters: "e"), client: session.client)
        XCTAssertEqual(TpsKeyboardPanel.shared.flashedCapIndex?.row, 1)
        XCTAssertEqual(TpsKeyboardPanel.shared.flashedCapIndex?.cap, 2)

        _ = try session.controller.handle(TestFixtures.keyDownEvent(characters: "8"), client: session.client)
        XCTAssertEqual(TpsKeyboardPanel.shared.flashedCapIndex?.row, 0)
        XCTAssertEqual(TpsKeyboardPanel.shared.flashedCapIndex?.cap, 7)
    }

    /// A click runs the same key, so its cap lights too; the light goes out
    /// on its own.
    func testAPress_flashesItsCap_untilTheFlashEnds() async throws {
        let sleep = FlashSleep()
        defer { sleep.resumeAll() }
        let sleeping = expectation(description: "flash timer started")
        sleep.onSleep = { sleeping.fulfill() }
        TpsKeyboardPanel.shared.flashSleep = { await sleep.pause(for: $0) }
        let session = try makeSession(under: .tps)
        session.controller.performShortcutAction(.showTpsKeyboard)

        TpsKeyboardPanel.shared.press("ㆣ")
        // trace: ㆣ is `E` with Shift — the same cap as ㄍ, row 1 cap 2.
        XCTAssertEqual(TpsKeyboardPanel.shared.flashedCapIndex?.row, 1)
        XCTAssertEqual(TpsKeyboardPanel.shared.flashedCapIndex?.cap, 2)

        let flash = try XCTUnwrap(TpsKeyboardPanel.shared.flashTask)
        await fulfillment(of: [sleeping], timeout: 5)
        XCTAssertEqual(sleep.durations, [.milliseconds(150)])
        sleep.resumeFirst()
        await flash.value
        XCTAssertNil(TpsKeyboardPanel.shared.flashedCapIndex)
    }

    /// A key typed mid-flash restarts the light on its own cap: the first
    /// flash's end, when its time comes, does not put out the second.
    /// Complete the cancelled `e` sleep after `8` has lit, then complete `8`.
    /// No elapsed-time window is needed to observe either state.
    func testAFlash_endingLate_leavesTheNewerFlashLit() async throws {
        let sleep = FlashSleep()
        defer { sleep.resumeAll() }
        TpsKeyboardPanel.shared.flashSleep = { await sleep.pause(for: $0) }
        let session = try makeSession(under: .tps)
        session.controller.performShortcutAction(.showTpsKeyboard)

        let firstSleeping = expectation(description: "first flash timer started")
        sleep.onSleep = { firstSleeping.fulfill() }
        _ = try session.controller.handle(TestFixtures.keyDownEvent(characters: "e"), client: session.client)
        let first = try XCTUnwrap(TpsKeyboardPanel.shared.flashTask)
        await fulfillment(of: [firstSleeping], timeout: 5)

        let secondSleeping = expectation(description: "second flash timer started")
        sleep.onSleep = { secondSleeping.fulfill() }
        _ = try session.controller.handle(TestFixtures.keyDownEvent(characters: "8"), client: session.client)
        let second = try XCTUnwrap(TpsKeyboardPanel.shared.flashTask)
        await fulfillment(of: [secondSleeping], timeout: 5)

        sleep.resumeFirst()
        await first.value
        XCTAssertEqual(TpsKeyboardPanel.shared.flashedCapIndex?.cap, 7, "the `8` cap is still lit")

        sleep.resumeFirst()
        await second.value
        XCTAssertNil(TpsKeyboardPanel.shared.flashedCapIndex)
    }

    @MainActor
    private final class FlashSleep {
        var onSleep: (() -> Void)?
        private(set) var durations: [Duration] = []
        private var continuations: [CheckedContinuation<Void, Never>] = []

        func pause(for duration: Duration) async {
            await withCheckedContinuation { continuation in
                durations.append(duration)
                continuations.append(continuation)
                onSleep?()
            }
        }

        func resumeFirst() {
            continuations.removeFirst().resume()
        }

        func resumeAll() {
            let pending = continuations
            continuations.removeAll()
            pending.forEach { $0.resume() }
        }
    }

    /// A handover puts the leaving session's light out; the leaving
    /// session's teardown leaves the arriving one's alone.
    func testAHandover_endsTheLeavingFlash_andTheTeardownKeepsTheArrivingOne() throws {
        userDefaults.set(true, forKey: SettingsStore.Keys.isTpsKeyboardShown.name)
        let leaving = try makeSession(under: .tps)
        _ = try leaving.controller.handle(TestFixtures.keyDownEvent(characters: "e"), client: leaving.client)
        XCTAssertNotNil(TpsKeyboardPanel.shared.flashedCapIndex)

        let arriving = try makeSession(under: .tps)
        XCTAssertNil(TpsKeyboardPanel.shared.flashedCapIndex, "the handover ended the flash")

        _ = try arriving.controller.handle(TestFixtures.keyDownEvent(characters: "8"), client: arriving.client)
        leaving.controller.deactivateServer(leaving.client)
        XCTAssertEqual(TpsKeyboardPanel.shared.flashedCapIndex?.cap, 7)
    }

    /// Nothing lights while the panel is down, and taking it down ends a flash.
    func testAFlash_needsThePanelUp_andEndsWithIt() throws {
        let session = try makeSession(under: .tps)
        _ = try session.controller.handle(TestFixtures.keyDownEvent(characters: "e"), client: session.client)
        XCTAssertNil(TpsKeyboardPanel.shared.flashedCapIndex, "no panel, no flash")

        session.controller.performShortcutAction(.showTpsKeyboard)
        _ = try session.controller.handle(TestFixtures.keyDownEvent(characters: "8"), client: session.client)
        XCTAssertNotNil(TpsKeyboardPanel.shared.flashedCapIndex)

        session.controller.performShortcutAction(.showTpsKeyboard)
        XCTAssertNil(TpsKeyboardPanel.shared.flashedCapIndex)
    }

    // MARK: - Harness

    private struct Session {
        let controller: TaigiInputController
        let client: RecordingTextInputClient
    }

    /// An activated session under `mode`, reading its settings from this
    /// case's own suite.
    private func makeSession(under mode: InputMode) throws -> Session {
        userDefaults.set(mode.rawValue, forKey: SettingsStore.Keys.inputMode.name)
        let client = RecordingTextInputClient()
        let controller = try TestFixtures.makeInputController()
        controller.settings = SettingsStore(userDefaults: userDefaults)
        controller.displayLanguageOverride = TestFixtures.makeDisplayLanguageStore(.hanji, userDefaults: userDefaults)
        controller.candidatePresenter = RecordingCandidatePresenter()
        controller.activateServer(client)
        return Session(controller: controller, client: client)
    }
}
