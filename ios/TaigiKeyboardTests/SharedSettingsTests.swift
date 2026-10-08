import SwiftUI
@testable import TaigiKeyboard
import XCTest

/// Tests for `SharedSettings.setInputMode(_:)` / `setKeyboardLayoutType(_:)`
/// — the TPS ↔ layout state machine that replaced the cross-setter cascade
/// guarded by `TPSSyncCoordinator` (deleted in this slice).
///
/// Coverage targets the essentials from the B8a pre-impl plan:
/// - enter TPS from input side / layout side
/// - exit TPS from input side / layout side (target-aware restore)
/// - idempotent same-value writes
/// - stale-state guard (writing `.tps` when the paired field is already `.tps`)
/// - `resetToDefaults()` lands on `.tl` + `.phahTaigi` from any starting state
/// - `snapshot()` reflects the post-transition pair
/// - live-read: external mutation of the backing `UserDefaults` is visible
///
/// Each test builds a `SharedSettings` against a unique transient suite so
/// they neither touch the App Group store nor interfere with each other.
final class SharedSettingsTests: XCTestCase {
    private var suiteName: String!
    private var defaults: UserDefaults!
    private var settings: SharedSettings!

    override func setUp() {
        super.setUp()
        suiteName = "SharedSettingsTests.\(UUID().uuidString)"
        defaults = UserDefaults(suiteName: suiteName)!
        settings = SharedSettings(userDefaults: defaults)
    }

    override func tearDown() {
        defaults.removePersistentDomain(forName: suiteName)
        defaults = nil
        settings = nil
        suiteName = nil
        super.tearDown()
    }

    // MARK: - Enter TPS

    func test_setInputMode_tps_fromPoj_flipsLayoutAndSavesBackup() {
        settings.setInputMode(.poj)
        settings.setKeyboardLayoutType(.qwerty)

        settings.setInputMode(.tps)

        XCTAssertEqual(settings.inputMode, .tps)
        XCTAssertEqual(settings.keyboardLayoutType, .tps)
        XCTAssertEqual(defaults.string(forKey: "layoutBeforeTps"), KeyboardLayoutType.qwerty.rawValue)
    }

    func test_setKeyboardLayoutType_tps_fromQwerty_flipsInputModeAndSavesBackup() {
        settings.setInputMode(.poj)
        settings.setKeyboardLayoutType(.qwerty)

        settings.setKeyboardLayoutType(.tps)

        XCTAssertEqual(settings.inputMode, .tps)
        XCTAssertEqual(settings.keyboardLayoutType, .tps)
        XCTAssertEqual(defaults.string(forKey: "inputModeBeforeTps"), InputMode.poj.rawValue)
    }

    // MARK: - Exit TPS (target-aware)

    func test_setInputMode_nonTps_fromTps_restoresLayoutFromBackup() {
        settings.setInputMode(.poj)
        settings.setKeyboardLayoutType(.qwerty)
        settings.setInputMode(.tps) // saves layoutBeforeTps = .qwerty, sets layout = .tps

        settings.setInputMode(.tl)

        XCTAssertEqual(settings.inputMode, .tl)
        XCTAssertEqual(settings.keyboardLayoutType, .qwerty, "Layout should restore from layoutBeforeTps backup")
    }

    func test_setKeyboardLayoutType_nonTps_fromTps_restoresInputModeFromBackup() {
        settings.setInputMode(.poj)
        settings.setKeyboardLayoutType(.qwerty)
        settings.setKeyboardLayoutType(.tps) // saves inputModeBeforeTps = .poj, sets inputMode = .tps

        settings.setKeyboardLayoutType(.moe1)

        XCTAssertEqual(settings.inputMode, .poj, "InputMode should restore from inputModeBeforeTps backup")
        XCTAssertEqual(settings.keyboardLayoutType, .moe1)
    }

    // MARK: - Stale-state guard

    func test_setInputMode_tps_whenLayoutAlreadyTps_doesNotOverwriteBackup() {
        // Stage a stale state where layout is .tps but inputMode is not.
        defaults.set(KeyboardLayoutType.tps.rawValue, forKey: "keyboardLayoutType")
        defaults.set(KeyboardLayoutType.moe2.rawValue, forKey: "layoutBeforeTps")
        settings.setInputMode(.poj) // oldMode = .tl (default), newMode = .poj — no TPS branch fires

        settings.setInputMode(.tps)

        XCTAssertEqual(settings.inputMode, .tps)
        XCTAssertEqual(settings.keyboardLayoutType, .tps)
        // Backup must NOT be overwritten to `.tps` — the guard skipped the save.
        XCTAssertEqual(defaults.string(forKey: "layoutBeforeTps"), KeyboardLayoutType.moe2.rawValue)
    }

    func test_setInputMode_nonTps_whenLayoutNotTps_doesNotRestore() {
        // User entered TPS, then manually picked a different layout before
        // switching input back to a non-TPS mode. The non-TPS input switch
        // must NOT clobber the manual layout choice — the input-side exit
        // is guarded on `keyboardLayoutType == .tps`.
        settings.setInputMode(.poj)
        settings.setKeyboardLayoutType(.qwerty)
        settings.setInputMode(.tps) // layoutBeforeTps = .qwerty
        defaults.set(KeyboardLayoutType.moe2.rawValue, forKey: "keyboardLayoutType") // raw layout swap

        settings.setInputMode(.tl)

        XCTAssertEqual(settings.inputMode, .tl)
        XCTAssertEqual(settings.keyboardLayoutType, .moe2, "Manual layout choice must survive non-TPS input switch")
    }

    func test_setKeyboardLayoutType_nonTps_whenInputModeNotTps_stillOverwritesFromBackup() {
        // Deliberate asymmetry preserved from HEAD~1: the layout-side exit
        // restores `inputMode` from backup UNCONDITIONALLY, even when the
        // live `inputMode` was already non-TPS (stale state from an
        // out-of-band write). This mirrors the pre-refactor cascade where
        // the old `keyboardLayoutType` setter wrote `inputMode = inputModeBeforeTps`
        // without an `if inputMode == .tps` guard. See doc on
        // `setKeyboardLayoutType(_:)`.
        defaults.set(KeyboardLayoutType.tps.rawValue, forKey: "keyboardLayoutType")
        defaults.set(InputMode.poj.rawValue, forKey: "inputModeBeforeTps")
        // inputMode stays at default `.tl`.

        settings.setKeyboardLayoutType(.moe1)

        XCTAssertEqual(settings.keyboardLayoutType, .moe1)
        XCTAssertEqual(settings.inputMode, .poj, "Layout-side exit is unguarded (HEAD~1 parity)")
    }

    // MARK: - romanizationInputMode (layout overlay preview)

    func test_romanizationInputMode_matchesModeAfterPickingRomanizationLayout() {
        // Each case: arrange a state, read the prediction, pick MOE1, compare with the live mode.
        let arrangements: [(String, (SharedSettings, UserDefaults) -> Void)] = [
            ("default tl", { _, _ in }),
            ("poj", { settings, _ in settings.setInputMode(.poj) }),
            ("english", { settings, _ in settings.setInputMode(.english) }),
            ("TPS layout card from poj", { settings, _ in
                settings.setInputMode(.poj)
                settings.setKeyboardLayoutType(.tps)
            }),
            ("toolbar TPS keeps the older backup", { settings, _ in
                settings.setInputMode(.poj)
                settings.setKeyboardLayoutType(.tps) // backup = poj
                settings.setKeyboardLayoutType(.qwerty) // restores poj
                settings.setInputMode(.tl)
                settings.setInputMode(.tps) // input side: backup stays poj
            }),
            ("stale: TPS layout, tl mode, poj backup", { _, defaults in
                defaults.set(KeyboardLayoutType.tps.rawValue, forKey: "keyboardLayoutType")
                defaults.set(InputMode.poj.rawValue, forKey: "inputModeBeforeTps")
            }),
        ]
        for (name, arrange) in arrangements {
            let suite = "SharedSettingsTests.romanization.\(UUID().uuidString)"
            let caseDefaults = UserDefaults(suiteName: suite)!
            defer { caseDefaults.removePersistentDomain(forName: suite) }
            let caseSettings = SharedSettings(userDefaults: caseDefaults)
            arrange(caseSettings, caseDefaults)

            let predicted = caseSettings.romanizationInputMode
            caseSettings.setKeyboardLayoutType(.moe1)

            XCTAssertEqual(predicted, caseSettings.inputMode, name)
        }
    }

    // MARK: - Idempotency

    func test_setInputMode_sameValue_isNoOpForPairedField() {
        settings.setInputMode(.poj)
        settings.setKeyboardLayoutType(.qwerty)

        settings.setInputMode(.poj)

        XCTAssertEqual(settings.inputMode, .poj)
        XCTAssertEqual(settings.keyboardLayoutType, .qwerty)
    }

    func test_setInputMode_tpsWhenAlreadyTps_isNoOp() {
        settings.setInputMode(.poj)
        settings.setKeyboardLayoutType(.qwerty)
        settings.setInputMode(.tps)
        XCTAssertEqual(defaults.string(forKey: "layoutBeforeTps"), KeyboardLayoutType.qwerty.rawValue)

        settings.setInputMode(.tps)

        XCTAssertEqual(settings.inputMode, .tps)
        XCTAssertEqual(settings.keyboardLayoutType, .tps)
        // Backup unchanged — second .tps write is idempotent.
        XCTAssertEqual(defaults.string(forKey: "layoutBeforeTps"), KeyboardLayoutType.qwerty.rawValue)
    }

    // MARK: - Property setter forwarding

    func test_inputModeSetter_forwardsToStateMachine() {
        settings.setInputMode(.poj)
        settings.setKeyboardLayoutType(.qwerty)

        settings.inputMode = .tps

        XCTAssertEqual(settings.keyboardLayoutType, .tps, "Setter must forward to setInputMode and trigger TPS sync")
    }

    func test_keyboardLayoutTypeSetter_forwardsToStateMachine() {
        settings.setInputMode(.poj)
        settings.setKeyboardLayoutType(.qwerty)

        settings.keyboardLayoutType = .tps

        XCTAssertEqual(settings.inputMode, .tps, "Setter must forward to setKeyboardLayoutType and trigger TPS sync")
    }

    // MARK: - resetToDefaults

    func test_resetToDefaults_fromTpsState_landsOnDefaults() {
        settings.setInputMode(.tps)

        settings.resetToDefaults()

        XCTAssertEqual(settings.inputMode, .tl)
        XCTAssertEqual(settings.keyboardLayoutType, .phahTaigi)
    }

    // MARK: - snapshot

    func test_snapshot_reflectsPostTransitionPair() {
        settings.setInputMode(.poj)
        settings.setKeyboardLayoutType(.qwerty)
        settings.setInputMode(.tps)

        let snap = settings.snapshot(for: .light)

        XCTAssertEqual(snap.inputMode, .tps)
        XCTAssertEqual(snap.keyboardLayoutType, .tps)
    }

    // MARK: - Live-read

    func test_inputMode_liveReadsBackingDefaults() {
        settings.setInputMode(.poj)
        XCTAssertEqual(settings.inputMode, .poj)

        // External writer flips the raw key (host app → extension scenario).
        defaults.set(InputMode.tl.rawValue, forKey: "inputMode")

        XCTAssertEqual(settings.inputMode, .tl, "Each read must hit UserDefaults; no in-memory cache")
    }

    // MARK: - Theme resolution (v3.6.2 PR-2a — no-migration safety; PR-2b — built-in colorScheme)

    // trace: fresh install → selectedThemeId absent → "default" → resolvedAppearance.colors == .default (all nil)
    func test_resolvedTheme_defaultUncustomized_returnsAllNil() {
        XCTAssertEqual(settings.selectedThemeId, ThemeId.default)
        XCTAssertEqual(settings.resolvedAppearance(for: .light).colors, .default)
        XCTAssertEqual(settings.resolvedAppearance(for: .light).keyShadowIntensity, 0)
    }

    // MARK: - Retired global appearance → user theme

    private static let retiredAppearanceKeys = [
        "keyHeightScale", "keyFontSizeScale", "candidateTextSizeScale", "keyCornerRadius", "keyBorderWidth", "colorSettings",
    ]

    /// A settings facade whose user themes live in a fresh temporary directory.
    private func settingsWithThemeStore() throws -> SharedSettings {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        addTeardownBlock { try? FileManager.default.removeItem(at: directory) }
        return SharedSettings(userDefaults: defaults, themesContainerURL: directory)
    }

    /// What the retired appearance screen left behind: a red background, larger keys, round corners.
    private func storeCustomizedLegacyLook() throws -> KeyboardColorSettings {
        var custom = KeyboardColorSettings()
        custom.background = .solid(CodableColor(.red))
        try defaults.set(JSONEncoder().encode(custom), forKey: "colorSettings")
        defaults.set(1.2, forKey: "keyHeightScale")
        defaults.set(10.0, forKey: "keyCornerRadius")
        return custom
    }

    private func assertRetiredKeysGone(file: StaticString = #filePath, line: UInt = #line) {
        for key in Self.retiredAppearanceKeys {
            XCTAssertNil(defaults.object(forKey: key), "\(key) must be removed", file: file, line: line)
        }
    }

    // trace: fresh install → selectedThemeId absent → "default" → the factory appearance, nothing carried
    func test_retireLegacyAppearance_nothingStored_changesNothing() throws {
        let settings = try settingsWithThemeStore()
        settings.retireLegacyAppearance(themeName: "新主題")
        XCTAssertTrue(settings.loadUserThemes().isEmpty)
        XCTAssertEqual(settings.selectedThemeId, ThemeId.default)
    }

    // trace: customized look on "default" → one user theme holding it, selected; keys removed
    func test_retireLegacyAppearance_customizedDefault_becomesTheSelectedUserTheme() throws {
        let settings = try settingsWithThemeStore()
        let custom = try storeCustomizedLegacyLook()

        settings.retireLegacyAppearance(themeName: "新主題")

        let themes = settings.loadUserThemes()
        XCTAssertEqual(themes.count, 1)
        let theme = try XCTUnwrap(themes.first)
        XCTAssertEqual(theme.name, "新主題")
        XCTAssertEqual(theme.appearance.colors.background, custom.background, "the picked color survives")
        XCTAssertEqual(theme.appearance.keyHeightScale, 1.2)
        XCTAssertEqual(theme.appearance.keyCornerRadius, 10)
        XCTAssertEqual(theme.appearance.keyShadowIntensity, 0, "the retired screen had no shadow")
        XCTAssertEqual(settings.selectedThemeId, theme.id.uuidString)
        XCTAssertEqual(settings.resolvedAppearance(for: .light), theme.appearance)
        assertRetiredKeysGone()

        // Once the keys are gone a later launch carries nothing again.
        settings.retireLegacyAppearance(themeName: "新主題")
        XCTAssertEqual(settings.loadUserThemes().count, 1)
    }

    // trace: keys stored at their factory values (an old reset wrote them) → only removed
    func test_retireLegacyAppearance_factoryValues_areOnlyRemoved() throws {
        let settings = try settingsWithThemeStore()
        defaults.set(1.0, forKey: "keyHeightScale")
        defaults.set(6.0, forKey: "keyCornerRadius")

        settings.retireLegacyAppearance(themeName: "新主題")

        XCTAssertTrue(settings.loadUserThemes().isEmpty)
        XCTAssertEqual(settings.selectedThemeId, ThemeId.default)
        assertRetiredKeysGone()
    }

    // trace: a built-in was showing → the look is kept as a theme, the selection stays
    func test_retireLegacyAppearance_builtInSelected_keepsTheSelection() throws {
        let settings = try settingsWithThemeStore()
        _ = try storeCustomizedLegacyLook()
        settings.selectedThemeId = "standardBlue"

        settings.retireLegacyAppearance(themeName: "新主題")

        XCTAssertEqual(settings.loadUserThemes().count, 1)
        XCTAssertEqual(settings.selectedThemeId, "standardBlue")
        assertRetiredKeysGone()
    }

    // trace: user themes already at the cap → the carried look is still added
    func test_retireLegacyAppearance_atTheCap_stillKeepsTheLook() throws {
        let settings = try settingsWithThemeStore()
        for index in 0 ..< UserThemeStore.maxUserThemes {
            let theme = UserTheme(id: UUID(), name: "T\(index)", appearance: .userThemeSeed, createdAt: Date(), updatedAt: Date())
            XCTAssertTrue(settings.addUserTheme(theme))
        }
        _ = try storeCustomizedLegacyLook()

        settings.retireLegacyAppearance(themeName: "新主題")

        XCTAssertEqual(settings.loadUserThemes().count, UserThemeStore.maxUserThemes + 1)
        assertRetiredKeysGone()
    }

    // trace: a saved theme list that does not decode → left byte-for-byte, keys kept
    func test_retireLegacyAppearance_unreadableThemeList_leavesItAndTheKeys() throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        addTeardownBlock { try? FileManager.default.removeItem(at: directory) }
        let file = directory.appendingPathComponent(UserThemeStore.fileName)
        let unreadable = Data("[{\"id\": \"not-a-uuid\"}]".utf8)
        try unreadable.write(to: file)
        let settings = SharedSettings(userDefaults: defaults, themesContainerURL: directory)
        _ = try storeCustomizedLegacyLook()

        settings.retireLegacyAppearance(themeName: "新主題")

        XCTAssertEqual(try Data(contentsOf: file), unreadable)
        XCTAssertEqual(settings.selectedThemeId, ThemeId.default)
        XCTAssertNotNil(defaults.object(forKey: "colorSettings"))
    }

    // trace: no theme store (the keyboard without Full Access) → keys kept for a later launch
    func test_retireLegacyAppearance_themeNotWritten_keepsTheKeys() throws {
        let settings = SharedSettings(userDefaults: defaults, themesContainerURL: nil)
        _ = try storeCustomizedLegacyLook()

        settings.retireLegacyAppearance(themeName: "新主題")

        XCTAssertEqual(settings.selectedThemeId, ThemeId.default)
        XCTAssertNotNil(defaults.object(forKey: "colorSettings"))
        XCTAssertNotNil(defaults.object(forKey: "keyHeightScale"))
    }

    // trace: selectedThemeId = built-in "standardBlue" → resolvedTheme routes through the catalog,
    // returning its soft-gradient palette (≠ .default) in both schemes.
    func test_resolvedTheme_builtIn_routesThroughCatalog() {
        settings.selectedThemeId = "standardBlue"
        let expected = BuiltInThemes.theme(id: "standardBlue")!

        XCTAssertEqual(settings.resolvedAppearance(for: .light).colors, expected.colors(for: .light))
        XCTAssertEqual(settings.resolvedAppearance(for: .dark).colors, expected.colors(for: .dark))
        XCTAssertNotNil(settings.resolvedAppearance(for: .light).colors.backgroundGradient,
                        "standardBlue routes through the catalog with a gradient palette")
    }

    // MARK: - Snapshot shadow gate (v3.6.2 PR-B — three-state shadow)

    // trace: default theme → snapshot.keyShadowIntensity == nil → render keeps KeyboardKit's
    // standard button shadow (HEAD look). Shadow is a user-theme-only feature.
    func test_snapshot_defaultTheme_shadowIsNil() {
        XCTAssertEqual(settings.selectedThemeId, ThemeId.default)
        XCTAssertNil(settings.snapshot(for: .light).keyShadowIntensity)
    }

    // trace: built-in id is not a UUID → not a user theme → shadow nil → keeps KK standard shadow.
    func test_snapshot_builtInTheme_shadowIsNil() {
        settings.selectedThemeId = "standardBlue"
        XCTAssertNil(settings.snapshot(for: .light).keyShadowIntensity)
    }

    // trace: a user-theme id (UUID) → explicit shadow semantics → non-nil (0 = flat). An orphan
    // UUID resolves to legacy appearance (shadow 0) but still carries the explicit 0, proving the
    // gate keys on id shape, not on the resolved value.
    func test_snapshot_userThemeId_shadowIsExplicit() {
        settings.selectedThemeId = UUID().uuidString
        let snap = settings.snapshot(for: .light)
        XCTAssertNotNil(snap.keyShadowIntensity)
        XCTAssertEqual(snap.keyShadowIntensity, 0)
    }

    // MARK: - Candidate display mode (Hanji–Romanization Pairing / Romanization Only) — stored vs derived split

    /// Under `.romanOnly` the derived swap / both-scripts pair reads `false`
    /// while the stored flags keep the user's `true`; leaving the mode
    /// restores the derived values without any write.
    func test_candidateDisplayMode_romanOnly_derivesFalseWithoutOverwritingStoredFlags() {
        settings.storedIsHanjiFirst = true
        settings.storedIsOutputBothScripts = true

        settings.candidateDisplayMode = .romanOnly

        XCTAssertFalse(settings.isHanjiFirst, "derived swap must read false under romanOnly")
        XCTAssertFalse(settings.isOutputBothScripts, "derived both-scripts must read false under romanOnly")
        XCTAssertTrue(settings.storedIsHanjiFirst, "stored swap must survive the mode")
        XCTAssertTrue(settings.storedIsOutputBothScripts, "stored both-scripts must survive the mode")
        XCTAssertEqual(defaults.object(forKey: "isTranslateSwapped") as? Bool, true, "raw key untouched")

        settings.candidateDisplayMode = .sideBySide

        XCTAssertTrue(settings.isHanjiFirst, "leaving romanOnly restores the derived swap")
        XCTAssertTrue(settings.isOutputBothScripts, "leaving romanOnly restores derived both-scripts")
    }

    /// The `snapshot(for:)` render path carries the DERIVED swap, so keycaps
    /// go half-width and the 文/A key reads inactive under `.romanOnly`.
    func test_snapshot_underRomanOnly_carriesDerivedSwap() {
        settings.storedIsHanjiFirst = true
        settings.candidateDisplayMode = .romanOnly

        XCTAssertFalse(settings.snapshot(for: .light).isHanjiFirst)
    }

    /// `.combined` projects the pair as swapped (cell leads with hanji, commit
    /// writes hanji) without writing the stored flag; Annotate in Brackets keeps its stored
    /// value; leaving the mode restores the stored pair.
    func test_candidateDisplayMode_combined_projectsSwappedWithoutOverwritingStoredFlags() {
        settings.storedIsHanjiFirst = false
        settings.storedIsOutputBothScripts = false

        settings.candidateDisplayMode = .combined

        XCTAssertTrue(settings.isHanjiFirst, "derived swap must read true under combined")
        XCTAssertFalse(settings.isOutputBothScripts, "derived both-scripts follows the stored false")
        XCTAssertFalse(settings.storedIsHanjiFirst, "stored swap must survive the mode")
        XCTAssertEqual(defaults.object(forKey: "isTranslateSwapped") as? Bool, false, "raw key untouched")

        // Annotate in Brackets stays as stored: the bracket form `Hanji (romanization)` applies under Hanji with Romanization.
        settings.storedIsOutputBothScripts = true
        XCTAssertTrue(settings.isOutputBothScripts, "stored both-scripts survives the projection")

        settings.candidateDisplayMode = .sideBySide

        XCTAssertFalse(settings.isHanjiFirst, "leaving combined restores the stored swap")
        XCTAssertTrue(settings.isOutputBothScripts, "leaving combined restores stored both-scripts")
    }

    /// The rules the derived pair and the UI gates read live on the enum, so
    /// they are pinned once here rather than through every consumer.
    func test_candidateDisplayMode_rules_perMode() {
        XCTAssertEqual(CandidateDisplayMode.allCases.filter(\.allowsSwapToggle), [.sideBySide, .combined])
        XCTAssertEqual(CandidateDisplayMode.allCases.filter { !$0.showsHanji }, [.romanOnly])
        XCTAssertTrue(CandidateDisplayMode.combined.effectiveHanjiFirst(stored: false))
        XCTAssertFalse(CandidateDisplayMode.romanOnly.effectiveHanjiFirst(stored: true))
        XCTAssertFalse(CandidateDisplayMode.romanOnly.effectiveOutputBothScripts(stored: true))
        XCTAssertTrue(CandidateDisplayMode.combined.effectiveOutputBothScripts(stored: true))
        // Punctuation width follows the STORED swap under Hanji–Romanization Pairing / Hanji with Romanization, never under Romanization Only.
        XCTAssertFalse(CandidateDisplayMode.combined.effectiveFullWidthPunctuation(stored: false))
        XCTAssertTrue(CandidateDisplayMode.combined.effectiveFullWidthPunctuation(stored: true))
        XCTAssertFalse(CandidateDisplayMode.romanOnly.effectiveFullWidthPunctuation(stored: true))
        XCTAssertTrue(CandidateDisplayMode.sideBySide.effectiveFullWidthPunctuation(stored: true))
    }

    /// Under `.combined` the candidate projection stays swapped while the
    /// punctuation width follows the stored flag the 文/A key toggles.
    func test_candidateDisplayMode_combined_punctuationWidthFollowsStoredSwap() {
        settings.candidateDisplayMode = .combined

        settings.storedIsHanjiFirst = false
        XCTAssertTrue(settings.isHanjiFirst, "projection stays hanji-first")
        XCTAssertFalse(settings.isFullWidthPunctuation, "half-width until the key is tapped")

        settings.storedIsHanjiFirst = true
        XCTAssertTrue(settings.isHanjiFirst)
        XCTAssertTrue(settings.isFullWidthPunctuation, "full-width after the key is tapped")

        settings.candidateDisplayMode = .romanOnly
        XCTAssertFalse(settings.isFullWidthPunctuation, "羅馬字 is always half-width")
    }

    /// TPS types Chinese: every page is full-width whatever the stored swap or
    /// display mode says, and the stored swap is untouched for the way back.
    func test_tpsLayout_isAlwaysFullWidthPunctuation() {
        settings.storedIsHanjiFirst = false
        settings.candidateDisplayMode = .romanOnly
        settings.keyboardLayoutType = .tps

        XCTAssertTrue(settings.isFullWidthPunctuation, "TPS forces full-width")
        XCTAssertFalse(settings.storedIsHanjiFirst, "stored swap untouched")

        settings.keyboardLayoutType = .phahTaigi
        XCTAssertFalse(settings.isFullWidthPunctuation, "leaving TPS restores the derived width")
    }

    func test_candidateDisplayMode_storageContract_keyAndRawValues() {
        XCTAssertEqual(settings.candidateDisplayMode, .sideBySide, "descriptor default")

        settings.candidateDisplayMode = .romanOnly
        XCTAssertEqual(defaults.string(forKey: "candidateDisplayMode"), "romanOnly", "cross-platform raw value")

        settings.candidateDisplayMode = .combined
        XCTAssertEqual(defaults.string(forKey: "candidateDisplayMode"), "combined", "cross-platform raw value")
        XCTAssertEqual(SharedSettings(userDefaults: defaults).candidateDisplayMode, .combined, "fresh reader decodes it")
    }

    /// An unknown / malformed stored raw string (a newer build's value, a
    /// hand-edited plist) falls back to `.sideBySide` — today's behaviour.
    func test_candidateDisplayMode_unknownRawString_fallsBackToSideBySide() {
        defaults.set("hanjiOnly", forKey: "candidateDisplayMode")

        XCTAssertEqual(settings.candidateDisplayMode, .sideBySide)
    }

    func test_resetToDefaults_restoresCandidateDisplayMode() {
        settings.candidateDisplayMode = .romanOnly

        settings.resetToDefaults()

        XCTAssertEqual(settings.candidateDisplayMode, .sideBySide)
    }
}
