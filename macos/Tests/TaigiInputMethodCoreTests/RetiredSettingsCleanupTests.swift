// The upgrade path for settings retired after a release: their stored state is removed.

@testable import TaigiInputMethodCore
import XCTest

/// An install upgraded across a retirement must behave like a fresh one: the
/// retired keys a released build wrote are cleared, and the live settings
/// beside them survive the sweep.
@MainActor
final class RetiredSettingsCleanupTests: XCTestCase {
    private var suiteName = ""
    private var userDefaults = UserDefaults.standard

    override func setUp() async throws {
        try await super.setUp()
        suiteName = "RetiredSettingsCleanupTests.\(UUID().uuidString)"
        userDefaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
    }

    override func tearDown() async throws {
        userDefaults.removePersistentDomain(forName: suiteName)
        try await super.tearDown()
    }

    func testStoredTrueValuesOfRetiredToggles_areRemoved() {
        userDefaults.set(true, forKey: "outputBothScripts")

        RetiredSettingsCleanup.run(userDefaults: userDefaults)

        XCTAssertNil(userDefaults.object(forKey: "outputBothScripts"))
    }

    /// No Hyphens left a stored `true` behind: it becomes Syllable Separator
    /// `none`, a picker choice already stored wins, and the switch goes either way.
    func testAStoredNoHyphensSwitch_carriesOverAsNone() {
        let separatorName = SettingsStore.Keys.syllableSeparator.name
        let retiredName = RetiredSettingsCleanup.retiredHyphenlessRomanName

        userDefaults.set(true, forKey: retiredName)
        RetiredSettingsCleanup.run(userDefaults: userDefaults)
        XCTAssertEqual(userDefaults.string(forKey: separatorName), "none")
        XCTAssertNil(userDefaults.object(forKey: retiredName))

        userDefaults.removeObject(forKey: separatorName)
        userDefaults.set(false, forKey: retiredName)
        RetiredSettingsCleanup.run(userDefaults: userDefaults)
        XCTAssertNil(userDefaults.object(forKey: separatorName), "off carries nothing over")
        XCTAssertNil(userDefaults.object(forKey: retiredName))

        userDefaults.set("space", forKey: separatorName)
        userDefaults.set(true, forKey: retiredName)
        RetiredSettingsCleanup.run(userDefaults: userDefaults)
        XCTAssertEqual(userDefaults.string(forKey: separatorName), "space", "a stored choice wins")
        XCTAssertNil(userDefaults.object(forKey: retiredName))
    }

    /// A tombstone for Show Typed Text First is what left its earlier pane row dead —
    /// the sweep cleared what the row wrote, every launch (`behavioral-invariants.md`
    /// §34 desktop notes). The row is back, so the sweep must leave the key alone.
    func testTheLiteralRomanCandidateSetting_survivesTheSweep() {
        userDefaults.set(true, forKey: SettingsStore.Keys.isLiteralRomanCandidateEnabled.name)

        RetiredSettingsCleanup.run(userDefaults: userDefaults)

        XCTAssertEqual(
            userDefaults.object(forKey: SettingsStore.Keys.isLiteralRomanCandidateEnabled.name) as? Bool,
            true,
            "the sweep must not take a setting the 一般 pane still writes",
        )
    }

    /// Candidate Selection Keys, retired 2026-09-08: the slot key set follows the tone scheme
    /// now, and none of the values this key could hold (`shift`, `control`,
    /// `option`) has a set left to migrate to.
    func testTheCandidateSlotModifier_isRemoved() {
        userDefaults.set("option", forKey: "candidateSlotModifier")

        RetiredSettingsCleanup.run(userDefaults: userDefaults)

        XCTAssertNil(userDefaults.object(forKey: "candidateSlotModifier"))
        XCTAssertEqual(
            SettingsStore(userDefaults: userDefaults).toneInputScheme.slotKeySet,
            .bareKeys,
        )
    }

    /// The setting that replaced it must NOT be swept up with its neighbours.
    func testTheToneInputScheme_survivesTheSweep() {
        userDefaults.set(ToneInputScheme.telex.rawValue, forKey: SettingsStore.Keys.toneInputScheme.name)

        RetiredSettingsCleanup.run(userDefaults: userDefaults)

        XCTAssertEqual(
            SettingsStore(userDefaults: userDefaults).toneInputScheme,
            .telex,
        )
    }

    /// The learning toggles went with the Frequency Records / Association Records panes.
    /// Neither key is read any more (recording is unconditional), so a stored
    /// `false` is simply swept.
    func testStoredFalseOnTheRetiredRecordingToggles_isRemoved() {
        userDefaults.set(false, forKey: "frequencyRecordingEnabled")
        userDefaults.set(false, forKey: "associationRecordingEnabled")

        RetiredSettingsCleanup.run(userDefaults: userDefaults)

        XCTAssertNil(userDefaults.object(forKey: "frequencyRecordingEnabled"))
        XCTAssertNil(userDefaults.object(forKey: "associationRecordingEnabled"))
    }

    func testSelectionOnASurvivingPane_isKept() {
        userDefaults.set("appearance", forKey: SettingsStore.Keys.selectedSettingsPane.name)

        RetiredSettingsCleanup.run(userDefaults: userDefaults)

        XCTAssertEqual(
            userDefaults.string(forKey: SettingsStore.Keys.selectedSettingsPane.name),
            "appearance",
        )
    }
}
