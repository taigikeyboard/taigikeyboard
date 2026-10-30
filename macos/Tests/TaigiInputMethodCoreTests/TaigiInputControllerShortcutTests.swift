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

    @MainActor
    override func setUp() async throws {
        try await super.setUp()
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

    @MainActor
    override func tearDown() async throws {
        userDefaults.removePersistentDomain(forName: suiteName)
        try await super.tearDown()
    }

    func testRomanizationShortcut_switchesBetweenTlAndPoj() {
        controller.performShortcutAction(.toggleRomanization)

        XCTAssertEqual(controller.settings.inputMode, .poj)

        controller.performShortcutAction(.toggleRomanization)

        XCTAssertEqual(controller.settings.inputMode, .tl)
    }

    /// Switch TPS goes into TPS and back to the romanization it left — POJ
    /// here, remembered as `lastRomanizationMode` — flashing each mode.
    func testTpsShortcut_roundTripsToTheRomanizationLastUsed() {
        controller.settings.switchInputMode(.pick(.poj))

        controller.performShortcutAction(.toggleTps)

        XCTAssertEqual(controller.settings.inputMode, .tps)
        XCTAssertEqual(userDefaults.string(forKey: SettingsStore.Keys.lastRomanizationMode.name), "poj")

        controller.performShortcutAction(.toggleTps)

        XCTAssertEqual(controller.settings.inputMode, .poj)
        XCTAssertEqual(flashes, ["方音符號", "白話字"])
    }

    /// TPS is not a romanization: Switch Romanization leaves it for the
    /// romanization NOT last used — Switch TPS already returns to that one.
    func testRomanizationShortcut_underTps_leavesForTheOtherRomanization() {
        controller.settings.switchInputMode(.pick(.poj))
        controller.performShortcutAction(.toggleTps)

        controller.performShortcutAction(.toggleRomanization)

        XCTAssertEqual(controller.settings.inputMode, .tl)
    }

    /// Under TPS the swap, the Candidate Display cycle and the Telex guide do
    /// nothing: no write, no flash, no card.
    func testRomanizationOnlyChords_underTps_areInert() {
        controller.settings.switchInputMode(.pick(.tps))
        let storedSwap = controller.settings.storedIsHanjiFirst
        let displayMode = controller.settings.candidateDisplayMode

        controller.performShortcutAction(.toggleTranslateSwapped)
        controller.performShortcutAction(.cycleCandidateDisplayMode)
        controller.performShortcutAction(.showTelexGuide)

        XCTAssertEqual(controller.settings.storedIsHanjiFirst, storedSwap)
        XCTAssertEqual(controller.settings.candidateDisplayMode, displayMode)
        XCTAssertNil(TelexGuidePanel.shared.shownInputMode)
        XCTAssertEqual(flashes, [])
    }

    /// A default-on setting must read as on before it is flipped:
    /// `UserDefaults.bool(forKey:)` would answer `false` for a key nobody has
    /// written and turn the first press into a no-op.
    func testTranslateSwappedShortcut_flipsTheSettingFromItsDefault() {
        controller.performShortcutAction(.toggleTranslateSwapped)

        XCTAssertEqual(controller.settings.storedIsHanjiFirst, !SettingsStore.Keys.isHanjiFirst.defaultValue)
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

    /// The swap names what a commit now writes: the menu-bar icon is one
    /// static mark (D5), so nothing else would. (Over an open list too —
    /// `TaigiInputControllerCandidateTests`.)
    func testTheTranslateSwap_announcesWhatACommitNowWrites() {
        controller.performShortcutAction(.toggleTranslateSwapped)
        XCTAssertEqual(flashes, ["羅馬字"])

        controller.performShortcutAction(.toggleTranslateSwapped)
        XCTAssertEqual(flashes, ["羅馬字", "漢字"])
    }

    /// Under the romanization-only display the swap has nothing to swap, so
    /// the chord is inert — silently (USER 2026-09-01, Q11): the STORED value
    /// is untouched, so leaving the mode gives the user their swap back, the
    /// presenter is not disturbed, and no flash pretends something happened.
    func testTheTranslateSwap_underRomanOnly_isSilentlyInert() {
        controller.settings.storedIsHanjiFirst = true
        controller.settings.candidateDisplayMode = .romanOnly
        let callsBefore = presenter.calls.count

        controller.performShortcutAction(.toggleTranslateSwapped)

        XCTAssertTrue(controller.settings.storedIsHanjiFirst, "the chord flipped a stored value it must not touch")
        XCTAssertEqual(presenter.calls.count, callsBefore)
        XCTAssertEqual(flashes, [])
    }

    /// Under the combined display each script is its own adjacent cell and
    /// the Hanji cell always comes first, so the candidate projection stays
    /// `true` whatever the chord does — but the chord is NOT inert: it flips
    /// the STORED swap, which is what picks the punctuation width there
    /// (USER 2026-09-13: "Hanji with Romanization needs an isTranslateSwapped button"), and the
    /// flipped value is what the user gets back on returning to side-by-side.
    func testTheTranslateSwap_underCombined_flipsThePunctuationWidthOnly() {
        controller.settings.storedIsHanjiFirst = false
        controller.settings.candidateDisplayMode = .combined

        controller.performShortcutAction(.toggleTranslateSwapped)

        XCTAssertTrue(controller.settings.storedIsHanjiFirst, "the chord flips the stored swap")
        XCTAssertEqual(flashes, [])

        controller.performShortcutAction(.toggleTranslateSwapped)

        XCTAssertFalse(controller.settings.storedIsHanjiFirst, "a second press flips it back")
    }

    /// The cycle walks the Appearance picker's order and comes back round, so three
    /// presses return the user where they started; each press names the mode
    /// switched INTO, in the picker's own words — the strip changes shape,
    /// and a strip that did so with no notice reads as breakage. The STORED
    /// swap is not the cycle's to touch: it is what the user gets back on
    /// returning to side by side.
    func testCycleCandidateDisplayShortcut_advancesThePickerOrder_andAnnouncesIt() {
        controller.settings.storedIsHanjiFirst = true
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

        XCTAssertTrue(controller.settings.storedIsHanjiFirst, "the cycle flipped a stored swap it must not touch")
    }

    /// The settings doorway is handled before any session is consulted, so
    /// reaching a session with it must do nothing at all — not change a
    /// setting, and not disturb the composition on screen.
    func testTheSettingsDoorwayAction_isInertAtTheSession() {
        let settingsBefore = userDefaults.persistentDomain(forName: suiteName) as NSDictionary?
        let callsBefore = presenter.calls.count

        controller.performShortcutAction(.openLastSettingsPane)

        XCTAssertEqual(userDefaults.persistentDomain(forName: suiteName) as NSDictionary?, settingsBefore)
        XCTAssertEqual(presenter.calls.count, callsBefore)
    }
}
