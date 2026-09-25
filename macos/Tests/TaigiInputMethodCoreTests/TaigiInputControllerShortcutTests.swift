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
    /// What the HUD was asked to say, in order. Recorded rather than shown: a
    /// real flash is a panel ordered in front of whoever is running the tests.
    private var flashes: [String] = []

    override func setUpWithError() throws {
        try super.setUpWithError()
        suiteName = "TaigiInputControllerShortcutTests.\(UUID().uuidString)"
        userDefaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        controller = try TestFixtures.makeInputController()
        controller.settings = SettingsStore(userDefaults: userDefaults)
        presenter = RecordingCandidatePresenter()
        controller.candidatePresenter = presenter
        // Pinned, so the flash reads in the language this case asked for rather
        // than the language of whatever machine is running it.
        controller.displayLanguageOverride = TestFixtures.makeDisplayLanguageStore(
            .hanji, userDefaults: userDefaults,
        )
        flashes = []
        controller.modeFlashOverride = { [weak self] text in self?.flashes.append(text) }
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

        XCTAssertEqual(controller.settings.storedIsTranslateSwapped, !defaults.isTranslateSwapped)
    }

    /// What happens to the bar follows what the setting invalidates: the
    /// romanization switch changes what a fetch would return, so it takes the
    /// bar down; the Hanji/romanization swap changes only how the same candidates display,
    /// so it must NOT dismiss — the re-render path is pinned in
    /// `TaigiInputControllerCandidateTests`.
    func testTheRomanizationSwitch_takesTheBarDown_andTheSwapDoesNot() {
        var callsBefore = presenter.calls.count
        controller.performShortcutAction(.toggleRomanization)
        XCTAssertTrue(
            presenter.calls.dropFirst(callsBefore).contains {
                if case .hide = $0 {
                    true
                } else {
                    false
                }
            },
            "a romanization switch left stale candidates on screen",
        )

        callsBefore = presenter.calls.count
        controller.performShortcutAction(.toggleTranslateSwapped)
        XCTAssertFalse(
            presenter.calls.dropFirst(callsBefore).contains {
                if case .hide = $0 {
                    true
                } else {
                    false
                }
            },
            "a display-only swap must not route through dismissal",
        )
    }

    /// The chord fires from anywhere, so nothing on screen would otherwise say
    /// which romanization is now live — and a switch with no notice reads as
    /// the keyboard breaking. Same HUD the Shift tap raises (USER 2026-08-26),
    /// naming the mode switched INTO.
    func testTheRomanizationSwitch_announcesTheModeItSwitchedInto() {
        controller.performShortcutAction(.toggleRomanization)
        XCTAssertEqual(flashes, ["白話字"])

        controller.performShortcutAction(.toggleRomanization)
        XCTAssertEqual(flashes, ["白話字", "台羅"])
    }

    /// The Hanji/romanization swap does not announce itself: it changes how the candidates on
    /// screen render, and they re-render where the user is already looking.
    func testTheTranslateSwap_raisesNoFlash() {
        controller.performShortcutAction(.toggleTranslateSwapped)

        XCTAssertEqual(flashes, [])
    }

    /// Under the romanization-only display the swap has nothing to swap, so
    /// the chord is inert — silently (USER 2026-09-01, Q11): the STORED value
    /// is untouched, so leaving the mode gives the user their swap back, the
    /// presenter is not disturbed, and no flash pretends something happened.
    func testTheTranslateSwap_underRomanOnly_isSilentlyInert() {
        controller.settings.storedIsTranslateSwapped = true
        controller.settings.candidateDisplayMode = .romanOnly
        let callsBefore = presenter.calls.count

        controller.performShortcutAction(.toggleTranslateSwapped)

        XCTAssertTrue(controller.settings.storedIsTranslateSwapped, "the chord flipped a stored value it must not touch")
        XCTAssertFalse(controller.settings.current.isTranslateSwapped, "the effective swap stays off under romanization-only")
        XCTAssertEqual(presenter.calls.count, callsBefore)
        XCTAssertEqual(flashes, [])

        controller.settings.candidateDisplayMode = .sideBySide

        XCTAssertTrue(controller.settings.current.isTranslateSwapped, "leaving the mode must give the stored swap back")
    }

    /// Under the combined display each script is its own adjacent cell and
    /// the Hanji cell always comes first, so the candidate projection stays
    /// `true` whatever the chord does — but the chord is NOT inert: it flips
    /// the STORED swap, which is what picks the punctuation width there
    /// (USER 2026-09-13: "Hanji with Romanization needs an isTranslateSwapped button"), and the
    /// flipped value is what the user gets back on returning to side-by-side.
    func testTheTranslateSwap_underCombined_flipsThePunctuationWidthOnly() {
        controller.settings.storedIsTranslateSwapped = false
        controller.settings.candidateDisplayMode = .combined

        controller.performShortcutAction(.toggleTranslateSwapped)

        XCTAssertTrue(controller.settings.storedIsTranslateSwapped, "the chord flips the stored swap")
        XCTAssertTrue(controller.settings.current.isFullWidthPunctuation, "…which is the punctuation width under combined")
        XCTAssertTrue(controller.settings.current.isTranslateSwapped, "the effective swap stays on under combined")
        XCTAssertEqual(flashes, [])

        controller.performShortcutAction(.toggleTranslateSwapped)

        XCTAssertFalse(controller.settings.current.isFullWidthPunctuation, "a second press goes back to half-width")
    }

    /// The cycle walks the Appearance picker's order and comes back round, so three
    /// presses return the user where they started; each press names the mode
    /// switched INTO, in the picker's own words — the strip changes shape,
    /// and a strip that did so with no notice reads as breakage. The STORED
    /// swap is not the cycle's to touch: it is what the user gets back on
    /// returning to side by side.
    func testCycleCandidateDisplayShortcut_advancesThePickerOrder_andAnnouncesIt() {
        controller.settings.storedIsTranslateSwapped = true
        XCTAssertEqual(controller.settings.candidateDisplayMode, .sideBySide, "the cycle starts from the default")

        controller.performShortcutAction(.cycleCandidateDisplayMode)
        XCTAssertEqual(controller.settings.candidateDisplayMode, .combined)
        XCTAssertEqual(flashes, ["漢羅濫"])

        controller.performShortcutAction(.cycleCandidateDisplayMode)
        XCTAssertEqual(controller.settings.candidateDisplayMode, .romanOnly)
        XCTAssertEqual(flashes, ["漢羅濫", "羅馬字"])

        controller.performShortcutAction(.cycleCandidateDisplayMode)
        XCTAssertEqual(controller.settings.candidateDisplayMode, .sideBySide)
        XCTAssertEqual(flashes, ["漢羅濫", "羅馬字", "漢羅對應"])

        XCTAssertTrue(controller.settings.storedIsTranslateSwapped, "the cycle flipped a stored swap it must not touch")
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
