// Keyboard layouts are applied to the IMK client before it supplies key events.

import InputMethodKit
@testable import TaigiInputMethodCore
import XCTest

@MainActor
final class TaigiInputControllerKeyboardLayoutTests: XCTestCase {
    private var suiteName = ""
    private var userDefaults = UserDefaults.standard
    private var sessions: [(controller: TaigiInputController, client: RecordingTextInputClient)] = []

    override func setUpWithError() throws {
        try super.setUpWithError()
        suiteName = "TaigiInputControllerKeyboardLayoutTests.\(UUID().uuidString)"
        userDefaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
    }

    override func tearDown() {
        for session in sessions {
            session.controller.deactivateServer(session.client)
        }
        sessions.removeAll()
        userDefaults.removePersistentDomain(forName: suiteName)
        super.tearDown()
    }

    func testActivation_appliesTheChosenLayoutForTlAndPoj_withoutDocumentCalls() throws {
        // Identifiers read from macOS Text Input Source Services.
        let layouts: [(KeyboardLayout, String)] = [
            (.qwerty, "com.apple.keylayout.US"),
            (.dvorak, "com.apple.keylayout.Dvorak"),
            (.colemak, "com.apple.keylayout.Colemak"),
        ]
        for mode in [InputMode.tl, .poj] {
            for (layout, identifier) in layouts {
                userDefaults.set(layout.rawValue, forKey: SettingsStore.Keys.keyboardLayout.name)
                userDefaults.set(mode.rawValue, forKey: SettingsStore.Keys.inputMode.name)
                let session = try makeSession()

                XCTAssertEqual(session.client.keyboardLayoutOverrides, [identifier], "\(mode), \(layout)")
                XCTAssertEqual(session.client.readCallCount, 0, "activation must not query a Chromium client")
                XCTAssertEqual(session.client.writes, [])
                session.controller.deactivateServer(session.client)
            }
        }
    }

    func testActivation_withNoStoredLayout_usesQwerty() throws {
        let session = try makeSession()
        XCTAssertEqual(session.client.keyboardLayoutOverrides, ["com.apple.keylayout.US"])
    }

    func testReactivation_reappliesTheLayoutToTheNewClient() throws {
        userDefaults.set("dvorak", forKey: SettingsStore.Keys.keyboardLayout.name)
        let session = try makeSession()
        session.controller.deactivateServer(session.client)
        let newClient = RecordingTextInputClient()

        session.controller.activateServer(newClient)

        XCTAssertEqual(newClient.keyboardLayoutOverrides, ["com.apple.keylayout.Dvorak"])
        session.controller.deactivateServer(newClient)
    }

    func testLayoutChange_appliesLive_withoutRewritingTheComposition() async throws {
        let session = try makeSession()
        for letter in "tai" {
            let event = try TestFixtures.keyDownEvent(characters: String(letter))
            XCTAssertTrue(session.controller.handle(event, client: session.client))
        }
        let writes = session.client.writes
        let changed = expectation(description: "Colemak applied to the active client")
        session.client.afterKeyboardLayoutOverride = { identifier in
            if identifier == "com.apple.keylayout.Colemak" {
                changed.fulfill()
            }
        }

        userDefaults.set("colemak", forKey: SettingsStore.Keys.keyboardLayout.name)
        await fulfillment(of: [changed], timeout: 2)

        XCTAssertEqual(session.client.writes, writes, "already typed text keeps its meaning")
        XCTAssertEqual(session.client.keyboardLayoutOverrides, ["com.apple.keylayout.US", "com.apple.keylayout.Colemak"])
        session.client.afterKeyboardLayoutOverride = nil
    }

    func testTpsSwitch_usesQwertyImmediately_andRestoresTheRomanizationLayout() throws {
        userDefaults.set("dvorak", forKey: SettingsStore.Keys.keyboardLayout.name)
        let session = try makeSession()

        session.controller.performShortcutAction(.toggleTps)

        XCTAssertEqual(session.client.keyboardLayoutOverrides.last, "com.apple.keylayout.US")
        XCTAssertEqual(session.controller.settings.keyboardLayout, .dvorak, "TPS must retain the user's choice")
        session.controller.performShortcutAction(.toggleTps)
        XCTAssertEqual(session.client.keyboardLayoutOverrides.last, "com.apple.keylayout.Dvorak")
    }

    func testExternalInputModeChange_updatesTheLayout() async throws {
        userDefaults.set("colemak", forKey: SettingsStore.Keys.keyboardLayout.name)
        userDefaults.set("tps", forKey: SettingsStore.Keys.inputMode.name)
        let session = try makeSession()
        XCTAssertEqual(session.client.keyboardLayoutOverrides, ["com.apple.keylayout.US"])
        let changed = expectation(description: "leaving TPS restores Colemak")
        session.client.afterKeyboardLayoutOverride = { identifier in
            if identifier == "com.apple.keylayout.Colemak" {
                changed.fulfill()
            }
        }

        userDefaults.set("poj", forKey: SettingsStore.Keys.inputMode.name)
        await fulfillment(of: [changed], timeout: 2)

        XCTAssertEqual(session.client.keyboardLayoutOverrides.last, "com.apple.keylayout.Colemak")
        session.client.afterKeyboardLayoutOverride = nil
    }

    func testGeneralReset_restoresQwertyInTheActiveClient() async throws {
        userDefaults.set("dvorak", forKey: SettingsStore.Keys.keyboardLayout.name)
        let session = try makeSession()
        let changed = expectation(description: "reset applies QWERTY")
        session.client.afterKeyboardLayoutOverride = { identifier in
            if identifier == "com.apple.keylayout.US" {
                changed.fulfill()
            }
        }

        session.controller.settings.resetGeneralSettings()
        await fulfillment(of: [changed], timeout: 2)

        XCTAssertEqual(session.controller.settings.keyboardLayout, .qwerty)
        session.client.afterKeyboardLayoutOverride = nil
    }

    func testHandover_aLayoutChangeReachesOnlyTheActiveSession() async throws {
        let leaving = try makeSession()
        let arriving = try makeSession()
        leaving.controller.deactivateServer(leaving.client)
        let changed = expectation(description: "current owner applies Dvorak")
        arriving.client.afterKeyboardLayoutOverride = { identifier in
            if identifier == "com.apple.keylayout.Dvorak" {
                changed.fulfill()
            }
        }

        userDefaults.set("dvorak", forKey: SettingsStore.Keys.keyboardLayout.name)
        await fulfillment(of: [changed], timeout: 2)
        // A retired client's layout must not be changed by a queued observer.
        for _ in 0 ..< 10 {
            await Task.yield()
        }

        XCTAssertEqual(leaving.client.keyboardLayoutOverrides, ["com.apple.keylayout.US"])
        XCTAssertEqual(arriving.client.keyboardLayoutOverrides, ["com.apple.keylayout.US", "com.apple.keylayout.Dvorak"])
        arriving.client.afterKeyboardLayoutOverride = nil
    }

    func testAWaitingSession_receivesLayoutChangesBeforeItsFirstKey() async throws {
        _ = try makeSession()
        let waiting = try makeSession()
        let changed = expectation(description: "waiting client applies Colemak before macOS translates its first key")
        waiting.client.afterKeyboardLayoutOverride = { identifier in
            if identifier == "com.apple.keylayout.Colemak" {
                changed.fulfill()
            }
        }

        userDefaults.set("colemak", forKey: SettingsStore.Keys.keyboardLayout.name)
        await fulfillment(of: [changed], timeout: 2)

        XCTAssertEqual(waiting.client.keyboardLayoutOverrides, ["com.apple.keylayout.US", "com.apple.keylayout.Colemak"])
        XCTAssertEqual(waiting.client.readCallCount, 0)
        XCTAssertEqual(waiting.client.writes, [])
        waiting.client.afterKeyboardLayoutOverride = nil
    }

    private func makeSession() throws -> (controller: TaigiInputController, client: RecordingTextInputClient) {
        let controller = try TestFixtures.makeInputController()
        controller.settings = SettingsStore(userDefaults: userDefaults)
        controller.candidatePresenter = RecordingCandidatePresenter()
        controller.modeFlashOverride = { _ in }
        let client = RecordingTextInputClient()
        controller.activateServer(client)
        let session = (controller, client)
        sessions.append(session)
        return session
    }
}
