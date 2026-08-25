// What a recorded chord does to the session it lands in.

@testable import TaigiInputMethodCore
import XCTest

/// The shortcut actions go through the controller so a setting change and the
/// candidate bar it invalidates move together.
@MainActor
final class TaigiInputControllerShortcutTests: XCTestCase {
    private var suiteName = ""
    private var userDefaults = UserDefaults.standard
    private var controller: TaigiInputController!
    private var presenter: RecordingCandidatePresenter!

    override func setUpWithError() throws {
        try super.setUpWithError()
        suiteName = "TaigiInputControllerShortcutTests.\(UUID().uuidString)"
        userDefaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        controller = try TestFixtures.makeInputController()
        controller.settings = SettingsStore(userDefaults: userDefaults)
        presenter = RecordingCandidatePresenter()
        controller.candidatePresenter = presenter
    }

    override func tearDown() {
        userDefaults.removePersistentDomain(forName: suiteName)
        super.tearDown()
    }

    func testRomanizationShortcut_switchesBetweenTlAndPoj() {
        controller.performShortcutAction(.toggleRomanization)

        XCTAssertEqual(controller.settings.inputMode, .poj)

        controller.performShortcutAction(.toggleRomanization)

        XCTAssertEqual(controller.settings.inputMode, .tl)
    }

    /// A default-on setting must read as on before it is flipped:
    /// `UserDefaults.bool(forKey:)` would answer `false` for a key nobody has
    /// written and turn the first press into a no-op.
    func testTranslateSwappedShortcut_flipsTheSettingFromItsDefault() {
        let defaults = EngineSettings.defaults

        controller.performShortcutAction(.toggleTranslateSwapped)

        XCTAssertEqual(controller.settings.isTranslateSwapped, !defaults.isTranslateSwapped)
    }

    /// What happens to the bar follows what the setting invalidates: the
    /// romanization switch changes what a fetch would return, so it takes the
    /// bar down; the 漢羅 swap changes only how the same candidates display,
    /// so it must NOT dismiss — the re-render path is pinned in
    /// `TaigiInputControllerCandidateTests`.
    func testTheRomanizationSwitch_takesTheBarDown_andTheSwapDoesNot() {
        var callsBefore = presenter.calls.count
        controller.performShortcutAction(.toggleRomanization)
        XCTAssertTrue(
            presenter.calls.dropFirst(callsBefore).contains {
                if case .hide = $0 { true } else { false }
            },
            "a romanization switch left stale candidates on screen",
        )

        callsBefore = presenter.calls.count
        controller.performShortcutAction(.toggleTranslateSwapped)
        XCTAssertFalse(
            presenter.calls.dropFirst(callsBefore).contains {
                if case .hide = $0 { true } else { false }
            },
            "a display-only swap must not route through dismissal",
        )
    }

    /// The settings doorway is handled before any session is consulted, so
    /// reaching a session with it must do nothing at all — not change a
    /// setting, and not disturb the composition on screen.
    func testTheSettingsDoorwayAction_isInertAtTheSession() {
        let settingsBefore = controller.settings.current
        let callsBefore = presenter.calls.count

        controller.performShortcutAction(.openLastSettingsPane)

        XCTAssertEqual(controller.settings.current, settingsBefore)
        XCTAssertEqual(presenter.calls.count, callsBefore)
    }
}
