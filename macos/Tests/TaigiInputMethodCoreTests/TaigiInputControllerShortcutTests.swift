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

    /// Default-on settings must read as on before they are flipped:
    /// `UserDefaults.bool(forKey:)` would answer `false` for a key nobody has
    /// written and turn the first press into a no-op.
    func testCandidateShortcuts_flipTheirSettingFromItsDefault() {
        let defaults = EngineSettings.defaults

        controller.performShortcutAction(.toggleTranslateSwapped)
        controller.performShortcutAction(.toggleBothScripts)
        controller.performShortcutAction(.toggleLiteralRomanCandidate)

        XCTAssertEqual(controller.settings.isTranslateSwapped, !defaults.isTranslateSwapped)
        XCTAssertEqual(controller.settings.isOutputBothScripts, !defaults.isOutputBothScripts)
        XCTAssertEqual(
            controller.settings.isLiteralRomanCandidateEnabled,
            !defaults.isLiteralRomanCandidateEnabled,
        )
    }

    /// The candidates on screen were produced under the setting that just
    /// changed, and the key contract lets Space commit the highlighted one —
    /// so every candidate-affecting shortcut takes the bar down, the same rule
    /// the input-source menu's romanization items follow.
    func testEveryCandidateAffectingShortcut_takesTheBarDown() {
        for action in ShortcutAction.allCases where action != .openSettings {
            let callsBefore = presenter.calls.count

            controller.performShortcutAction(action)

            let newCalls = presenter.calls.dropFirst(callsBefore)
            XCTAssertTrue(
                newCalls.contains {
                    if case .hide = $0 {
                        true
                    } else {
                        false
                    }
                },
                "\(action.label) left stale candidates on screen",
            )
        }
    }

    /// 開啟設定 is handled before any session is consulted, so reaching a
    /// session with it must do nothing at all — not change a setting, and not
    /// disturb the composition on screen.
    func testOpenSettingsAction_isInertAtTheSession() {
        let settingsBefore = controller.settings.current
        let callsBefore = presenter.calls.count

        controller.performShortcutAction(.openSettings)

        XCTAssertEqual(controller.settings.current, settingsBefore)
        XCTAssertEqual(presenter.calls.count, callsBefore)
    }
}
