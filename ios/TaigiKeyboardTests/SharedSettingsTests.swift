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

    // trace: default theme threads ALL customized appearance (5 size scalars + font) through
    // snapshot, not just colors — the renderer-switch must not drop any global appearance key.
    func test_snapshot_defaultTheme_carriesCustomizedSizesAndFont() {
        settings.keyFontSizeScale = 1.1
        settings.candidateTextSizeScale = 0.9
        settings.keyCornerRadius = 10
        settings.keyBorderWidth = 2
        settings.fontType = .iansui

        let snap = settings.snapshot(for: .light)

        XCTAssertEqual(snap.keyFontSizeScale, 1.1)
        XCTAssertEqual(snap.candidateTextSizeScale, 0.9)
        XCTAssertEqual(snap.keyCornerRadius, 10)
        XCTAssertEqual(snap.keyBorderWidth, 2)
        XCTAssertEqual(snap.fontType, .iansui)
    }

    // trace: a customized user stays on "default" → resolvedTheme preserves their colorSettings verbatim,
    // which is the entire "no migration needed" safety argument for the renderer switch.
    func test_resolvedTheme_defaultTheme_preservesCustomizedColorSettings() {
        var custom = KeyboardColorSettings()
        custom.backgroundColor = CodableColor(.red)
        settings.colorSettings = custom

        XCTAssertEqual(settings.selectedThemeId, ThemeId.default)
        XCTAssertEqual(settings.resolvedAppearance(for: .light).colors, custom)
        XCTAssertEqual(settings.resolvedAppearance(for: .light).keyShadowIntensity, 0)
    }

    // trace: selectedThemeId = built-in "standardBlue" → resolvedTheme routes through the catalog,
    // returning its soft-gradient palette (≠ .default) in both schemes.
    func test_resolvedTheme_builtIn_routesThroughCatalog() {
        settings.selectedThemeId = "standardBlue"
        let expected = BuiltInThemes.theme(id: "standardBlue")!

        XCTAssertEqual(settings.resolvedAppearance(for: .light).colors, expected.colors(for: .light))
        XCTAssertEqual(settings.resolvedAppearance(for: .dark).colors, expected.colors(for: .dark))
        XCTAssertTrue(settings.resolvedAppearance(for: .light).colors.hasBackgroundGradient,
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

    // MARK: - Candidate display mode (漢羅對應 / 羅馬字) — stored vs derived split

    /// Under `.romanOnly` the derived swap / both-scripts pair reads `false`
    /// while the stored flags keep the user's `true`; leaving the mode
    /// restores the derived values without any write.
    func test_candidateDisplayMode_romanOnly_derivesFalseWithoutOverwritingStoredFlags() {
        settings.storedIsTranslateSwapped = true
        settings.storedIsOutputBothScripts = true

        settings.candidateDisplayMode = .romanOnly

        XCTAssertFalse(settings.isTranslateSwapped, "derived swap must read false under romanOnly")
        XCTAssertFalse(settings.isOutputBothScripts, "derived both-scripts must read false under romanOnly")
        XCTAssertTrue(settings.storedIsTranslateSwapped, "stored swap must survive the mode")
        XCTAssertTrue(settings.storedIsOutputBothScripts, "stored both-scripts must survive the mode")
        XCTAssertEqual(defaults.object(forKey: "isTranslateSwapped") as? Bool, true, "raw key untouched")

        settings.candidateDisplayMode = .sideBySide

        XCTAssertTrue(settings.isTranslateSwapped, "leaving romanOnly restores the derived swap")
        XCTAssertTrue(settings.isOutputBothScripts, "leaving romanOnly restores derived both-scripts")
    }

    /// The `snapshot(for:)` render path carries the DERIVED swap, so keycaps
    /// go half-width and the 文/A key reads inactive under `.romanOnly`.
    func test_snapshot_underRomanOnly_carriesDerivedSwap() {
        settings.storedIsTranslateSwapped = true
        settings.candidateDisplayMode = .romanOnly

        XCTAssertFalse(settings.snapshot(for: .light).isTranslateSwapped)
    }

    /// `.combined` projects the pair as swapped (cell leads with hanji, commit
    /// writes hanji) without writing the stored flag; 括號標註 keeps its stored
    /// value; leaving the mode restores the stored pair.
    func test_candidateDisplayMode_combined_projectsSwappedWithoutOverwritingStoredFlags() {
        settings.storedIsTranslateSwapped = false
        settings.storedIsOutputBothScripts = false

        settings.candidateDisplayMode = .combined

        XCTAssertTrue(settings.isTranslateSwapped, "derived swap must read true under combined")
        XCTAssertFalse(settings.isOutputBothScripts, "derived both-scripts follows the stored false")
        XCTAssertFalse(settings.storedIsTranslateSwapped, "stored swap must survive the mode")
        XCTAssertEqual(defaults.object(forKey: "isTranslateSwapped") as? Bool, false, "raw key untouched")

        // 括號標註 stays as stored: the bracket form `漢字 (羅馬字)` applies under 合用.
        settings.storedIsOutputBothScripts = true
        XCTAssertTrue(settings.isOutputBothScripts, "stored both-scripts survives the projection")

        settings.candidateDisplayMode = .sideBySide

        XCTAssertFalse(settings.isTranslateSwapped, "leaving combined restores the stored swap")
        XCTAssertTrue(settings.isOutputBothScripts, "leaving combined restores stored both-scripts")
    }

    /// The rules the derived pair and the UI gates read live on the enum, so
    /// they are pinned once here rather than through every consumer.
    func test_candidateDisplayMode_rules_perMode() {
        XCTAssertEqual(CandidateDisplayMode.allCases.filter(\.allowsSwapToggle), [.sideBySide])
        XCTAssertEqual(CandidateDisplayMode.allCases.filter { !$0.showsHanji }, [.romanOnly])
        XCTAssertTrue(CandidateDisplayMode.combined.effectiveTranslateSwapped(stored: false))
        XCTAssertFalse(CandidateDisplayMode.romanOnly.effectiveTranslateSwapped(stored: true))
        XCTAssertFalse(CandidateDisplayMode.romanOnly.effectiveOutputBothScripts(stored: true))
        XCTAssertTrue(CandidateDisplayMode.combined.effectiveOutputBothScripts(stored: true))
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
