// Floating HUD panels follow the Appearance setting (Light / Dark / Automatic).

import AppKit
@testable import TaigiInputMethodCore
import XCTest

@MainActor
final class HUDPanelAppearanceTests: XCTestCase {
    private var suiteName = ""
    private var userDefaults = UserDefaults.standard

    override func setUp() async throws {
        try await super.setUp()
        InstalledLexicon.installOnce()
        suiteName = "HUDPanelAppearanceTests.\(UUID().uuidString)"
        userDefaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        TelexGuidePanel.shared.hideNow()
        TpsKeyboardPanel.shared.hideNow()
    }

    override func tearDown() async throws {
        TelexGuidePanel.shared.hideNow()
        TpsKeyboardPanel.shared.hideNow()
        userDefaults.removePersistentDomain(forName: suiteName)
        try await super.tearDown()
    }

    func testModeFlashPanel_appliesAppearanceMode() {
        XCTAssertEqual(
            ModeFlashPanel.makePanel(text: "台羅", appearanceMode: .light).appearance,
            NSAppearance(named: .aqua),
        )
        XCTAssertEqual(
            ModeFlashPanel.makePanel(text: "台羅", appearanceMode: .dark).appearance,
            NSAppearance(named: .darkAqua),
        )
        XCTAssertNil(ModeFlashPanel.makePanel(text: "台羅", appearanceMode: .auto).appearance)
    }

    func testTelexGuidePanel_appliesAndLiveUpdatesAppearanceMode() async throws {
        userDefaults.set(AppearanceMode.light.rawValue, forKey: SettingsStore.Keys.appearanceMode.name)
        let controller = try makeActivatedController()
        defer { controller.deactivateServer(nil) }

        controller.performShortcutAction(.showTelexGuide)
        XCTAssertTrue(TelexGuidePanel.shared.isShowing)
        XCTAssertEqual(TelexGuidePanel.shared.currentAppearance, NSAppearance(named: .aqua))
        let buildsAfterShow = TelexGuidePanel.shared.panelBuildCount

        userDefaults.set(AppearanceMode.dark.rawValue, forKey: SettingsStore.Keys.appearanceMode.name)
        for _ in 0 ..< 50 where TelexGuidePanel.shared.currentAppearance != NSAppearance(named: .darkAqua) {
            await Task.yield()
        }
        XCTAssertEqual(TelexGuidePanel.shared.currentAppearance, NSAppearance(named: .darkAqua))
        XCTAssertEqual(TelexGuidePanel.shared.panelBuildCount, buildsAfterShow)

        userDefaults.set(AppearanceMode.auto.rawValue, forKey: SettingsStore.Keys.appearanceMode.name)
        for _ in 0 ..< 50 where TelexGuidePanel.shared.currentAppearance != nil {
            await Task.yield()
        }
        XCTAssertNil(TelexGuidePanel.shared.currentAppearance)
    }

    func testTpsKeyboardPanel_appliesAndLiveUpdatesAppearanceMode() async throws {
        userDefaults.set(InputMode.tps.rawValue, forKey: SettingsStore.Keys.inputMode.name)
        userDefaults.set(AppearanceMode.dark.rawValue, forKey: SettingsStore.Keys.appearanceMode.name)
        let controller = try makeActivatedController()
        defer { controller.deactivateServer(nil) }

        controller.performShortcutAction(.showTpsKeyboard)
        XCTAssertTrue(TpsKeyboardPanel.shared.isShowing)
        XCTAssertEqual(TpsKeyboardPanel.shared.currentAppearance, NSAppearance(named: .darkAqua))

        userDefaults.set(AppearanceMode.light.rawValue, forKey: SettingsStore.Keys.appearanceMode.name)
        for _ in 0 ..< 50 where TpsKeyboardPanel.shared.currentAppearance != NSAppearance(named: .aqua) {
            await Task.yield()
        }
        XCTAssertEqual(TpsKeyboardPanel.shared.currentAppearance, NSAppearance(named: .aqua))

        userDefaults.set(AppearanceMode.auto.rawValue, forKey: SettingsStore.Keys.appearanceMode.name)
        for _ in 0 ..< 50 where TpsKeyboardPanel.shared.currentAppearance != nil {
            await Task.yield()
        }
        XCTAssertNil(TpsKeyboardPanel.shared.currentAppearance)
    }

    private func makeActivatedController() throws -> TaigiInputController {
        let controller = try TestFixtures.makeInputController()
        controller.settings = SettingsStore(userDefaults: userDefaults)
        controller.displayLanguageOverride = TestFixtures.makeDisplayLanguageStore(.hanji, userDefaults: userDefaults)
        controller.candidatePresenter = RecordingCandidatePresenter()
        controller.activateServer(RecordingTextInputClient())
        return controller
    }
}
