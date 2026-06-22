@testable import TaigiKeyboard
import SwiftUI
import XCTest

/// Tests for the `SettingsKey<T>` typed descriptor introduced by B8a iOS
/// PR-2 and its `UserDefaults` typed-read/write/remove extensions. Coverage
/// pillars:
/// - Bool / Double / CGFloat / RawRepresentable / Codable codec round-trips.
/// - "absent or malformed → defaultValue" fallback per codec.
/// - `UserDefaults.remove(_:)` returns the slot to "absent → defaultValue".
/// - `SharedSettings` public properties still preserve their default value
///   when the backing key is absent (descriptor-wired defaults are
///   observable through the facade exactly as before).
/// - `isGlobeKeyEnabled` device-aware default survives the wrapper: absent
///   key → `DeviceCapabilities.prefersGlobeKeyByDefault`; `resetToDefaults()`
///   removes any stored value so the device default re-takes effect.
/// - `isFullAccessEnabled` semantics (Fork 5(s)): switched from
///   `bool(forKey:)` to descriptor-backed `object as? Bool ?? false`;
///   absent → false; stored `true` → true.
///
/// Each test builds a fresh `UserDefaults` suite to isolate from the App
/// Group store and from sibling tests.
final class SettingsKeyTests: XCTestCase {
    private var suiteName: String!
    private var defaults: UserDefaults!

    override func setUp() {
        super.setUp()
        suiteName = "SettingsKeyTests.\(UUID().uuidString)"
        defaults = UserDefaults(suiteName: suiteName)!
    }

    override func tearDown() {
        defaults.removePersistentDomain(forName: suiteName)
        defaults = nil
        suiteName = nil
        super.tearDown()
    }

    // MARK: - Bool codec

    func test_boolKey_absent_returnsDefault() {
        let key = SettingsKey<Bool>.bool("absentBool", default: true)
        XCTAssertTrue(defaults.value(for: key))
    }

    func test_boolKey_stored_returnsStoredValue() {
        let key = SettingsKey<Bool>.bool("storedBool", default: true)
        defaults.set(false, for: key)
        XCTAssertFalse(defaults.value(for: key))
    }

    func test_boolKey_remove_restoresDefault() {
        let key = SettingsKey<Bool>.bool("removableBool", default: true)
        defaults.set(false, for: key)
        defaults.remove(key)
        XCTAssertTrue(defaults.value(for: key), "After remove, descriptor default should re-apply")
    }

    func test_boolKey_storedNonBool_fallsBackToDefault() {
        let key = SettingsKey<Bool>.bool("boolMalformed", default: true)
        defaults.set("not-a-bool", forKey: "boolMalformed")
        XCTAssertTrue(defaults.value(for: key), "Non-Bool stored value falls back to descriptor default")
    }

    // MARK: - Double / CGFloat codec

    func test_doubleKey_roundTrip_andAbsentDefault() {
        let key = SettingsKey<Double>.double("doubleKey", default: 3.14)
        XCTAssertEqual(defaults.value(for: key), 3.14)
        defaults.set(2.5, for: key)
        XCTAssertEqual(defaults.value(for: key), 2.5)
    }

    func test_doubleKey_storedNonNumeric_fallsBackToDefault() {
        let key = SettingsKey<Double>.double("doubleMalformed", default: 1.0)
        defaults.set("not-a-number", forKey: "doubleMalformed")
        XCTAssertEqual(defaults.value(for: key), 1.0, "Non-numeric stored value falls back to descriptor default")
    }

    func test_cgFloatKey_roundTripThroughDoubleStorage() {
        let key = SettingsKey<CGFloat>.cgFloat("cgFloatKey", default: 1.0)
        XCTAssertEqual(defaults.value(for: key), CGFloat(1.0))
        defaults.set(CGFloat(2.5), for: key)
        // On-disk representation is Double; descriptor reads it back as CGFloat.
        XCTAssertEqual(defaults.object(forKey: "cgFloatKey") as? Double, 2.5)
        XCTAssertEqual(defaults.value(for: key), CGFloat(2.5))
    }

    // MARK: - RawRepresentable<String> codec

    func test_rawRepKey_roundTripAndUnknownRawFallsBackToDefault() {
        let key: SettingsKey<InputMode> = .rawRep("rawRepKey", default: .tl)
        XCTAssertEqual(defaults.value(for: key), .tl)
        defaults.set(InputMode.poj, for: key)
        XCTAssertEqual(defaults.value(for: key), .poj)
        // Garbage stored raw → default.
        defaults.set("not-a-mode", forKey: "rawRepKey")
        XCTAssertEqual(defaults.value(for: key), .tl, "Unknown raw string falls back to descriptor default")
    }

    // MARK: - Codable codec

    func test_codableKey_roundTripAndCorruptedDataFallsBackToDefault() {
        let key: SettingsKey<KeyboardColorSettings> = .codable("codableKey", default: .default)
        XCTAssertEqual(defaults.value(for: key), .default)

        // Writing a non-default value round-trips through JSON.
        var custom = KeyboardColorSettings.default
        custom.backgroundColor = CodableColor(.red)
        defaults.set(custom, for: key)
        XCTAssertEqual(defaults.value(for: key).backgroundColor, custom.backgroundColor)

        // Corrupted blob (random bytes) → default.
        defaults.set(Data([0xDE, 0xAD, 0xBE, 0xEF]), forKey: "codableKey")
        XCTAssertEqual(defaults.value(for: key), .default, "Undecodable data falls back to descriptor default")
    }

    // MARK: - SharedSettings facade integration

    func test_sharedSettings_absentKeys_returnDescriptorDefaults() {
        let settings = SharedSettings(userDefaults: defaults)

        XCTAssertEqual(settings.inputMode, .tl)
        XCTAssertEqual(settings.keyboardLayoutType, .phahTaigi)
        XCTAssertEqual(settings.fontType, .openHuninn)
        XCTAssertTrue(settings.isDoubleTapOOEnabled)
        XCTAssertTrue(settings.isDoubleTapNNEnabled)
        XCTAssertFalse(settings.isTranslateSwapped)
        XCTAssertFalse(settings.isOutputBothScripts)
        XCTAssertTrue(settings.isMoeDictEnabled)
        XCTAssertFalse(settings.isITaigiDictEnabled)
        XCTAssertEqual(settings.keyHeightScale, 1.0)
        XCTAssertEqual(settings.keyCornerRadius, 6.0)
        XCTAssertEqual(settings.keyBorderWidth, 0)
        XCTAssertEqual(settings.colorSettings, .default)
    }

    func test_isFullAccessEnabled_absentReturnsFalse_andStoredTrueReturnsTrue() {
        let settings = SharedSettings(userDefaults: defaults)

        XCTAssertFalse(settings.isFullAccessEnabled, "Descriptor default (false) applies to absent key")
        settings.isFullAccessEnabled = true
        XCTAssertTrue(settings.isFullAccessEnabled)
    }

    // MARK: - Globe key (device-aware default)

    func test_isGlobeKeyEnabled_absent_returnsDeviceDefault() {
        let settings = SharedSettings(userDefaults: defaults)
        XCTAssertEqual(
            settings.isGlobeKeyEnabled,
            DeviceCapabilities.prefersGlobeKeyByDefault,
            "Absent globe key falls back to device-aware default, not descriptor default",
        )
    }

    func test_isGlobeKeyEnabled_storedValueOverridesDeviceDefault() {
        let settings = SharedSettings(userDefaults: defaults)
        let opposite = !DeviceCapabilities.prefersGlobeKeyByDefault
        settings.isGlobeKeyEnabled = opposite
        XCTAssertEqual(settings.isGlobeKeyEnabled, opposite)
    }

    func test_resetToDefaults_removesGlobeKey_soDeviceDefaultReturns() {
        let settings = SharedSettings(userDefaults: defaults)
        let opposite = !DeviceCapabilities.prefersGlobeKeyByDefault
        settings.isGlobeKeyEnabled = opposite
        XCTAssertEqual(settings.isGlobeKeyEnabled, opposite)

        settings.resetToDefaults()

        XCTAssertNil(defaults.object(forKey: "isGlobeKeyEnabled"), "Reset must remove the stored globe key")
        XCTAssertEqual(
            settings.isGlobeKeyEnabled,
            DeviceCapabilities.prefersGlobeKeyByDefault,
            "After reset, device-aware default re-applies",
        )
    }

    func test_isGlobeKeyEnabled_writeProducesOnDiskBoolFormat() {
        // 中文: 確保 descriptor 寫盤的 Bool 表示與 pre-wrapper 完全一致 — 舊版升上來不會讀到亂掉的格子。
        let settings = SharedSettings(userDefaults: defaults)
        settings.isGlobeKeyEnabled = true
        XCTAssertEqual(defaults.object(forKey: "isGlobeKeyEnabled") as? Bool, true)
        settings.isGlobeKeyEnabled = false
        XCTAssertEqual(defaults.object(forKey: "isGlobeKeyEnabled") as? Bool, false)
    }

    // MARK: - resetToDefaults covers every descriptor

    /// Catches the foot-gun where a new persisted property gets added to
    /// `SharedSettings` but the maintainer forgets to wire it into
    /// `resetToDefaults()`. Every descriptor-backed field is touched with
    /// a non-default value, then `resetToDefaults()` must restore the
    /// descriptor default for every one.
    // 中文: 防呆 — 將每個欄位設為非預設值,呼叫 resetToDefaults() 後逐欄位驗證已還原。
    func test_resetToDefaults_restoresEveryDescriptorBackedProperty() {
        let settings = SharedSettings(userDefaults: defaults)

        // Flip every Bool to the opposite of its descriptor default.
        settings.isDoubleTapOOEnabled = false
        settings.isDoubleTapNNEnabled = false
        settings.isTranslateSwapped = true
        settings.isOutputBothScripts = true
        settings.isAutoSpaceEnabled = true
        settings.isMoeDictEnabled = false
        settings.isNewwordDictEnabled = false
        settings.isKunggeDictEnabled = false
        settings.isITaigiDictEnabled = true
        settings.isTaiwanJapanDictEnabled = true
        settings.isTaiHuaDictEnabled = true
        settings.isTaiwanPlantDictEnabled = true
        settings.isSttiDictEnabled = false
        settings.isKhpooDictEnabled = false
        settings.isVariantEnabled = true
        settings.isKhiinEnabled = true
        settings.isLkkDictEnabled = false
        settings.isToolbarAutoCollapse = false
        settings.isTpsOrMappedToER = false
        settings.isGlobeKeyEnabled = !DeviceCapabilities.prefersGlobeKeyByDefault

        // RawRep enums + font.
        settings.setInputMode(.poj)
        settings.setKeyboardLayoutType(.qwerty)
        settings.fontType = .system

        // App UI display language (String descriptor).
        settings.displayLanguage = DisplayLanguage.tailo.tag

        // CGFloat appearance.
        settings.keyHeightScale = 2.0
        settings.keyFontSizeScale = 2.0
        settings.candidateTextSizeScale = 2.0
        settings.keyCornerRadius = 12.0
        settings.keyBorderWidth = 3.0

        // Codable colour blob.
        var custom = KeyboardColorSettings.default
        custom.backgroundColor = CodableColor(.red)
        settings.colorSettings = custom

        settings.resetToDefaults()

        XCTAssertEqual(settings.inputMode, .tl)
        XCTAssertEqual(settings.keyboardLayoutType, .phahTaigi)
        XCTAssertEqual(settings.fontType, .openHuninn)
        XCTAssertEqual(settings.displayLanguage, DisplayLanguage.defaultTag)
        XCTAssertTrue(settings.isDoubleTapOOEnabled)
        XCTAssertTrue(settings.isDoubleTapNNEnabled)
        XCTAssertFalse(settings.isTranslateSwapped)
        XCTAssertFalse(settings.isOutputBothScripts)
        XCTAssertFalse(settings.isAutoSpaceEnabled)
        XCTAssertTrue(settings.isMoeDictEnabled)
        XCTAssertTrue(settings.isNewwordDictEnabled)
        XCTAssertTrue(settings.isKunggeDictEnabled)
        XCTAssertFalse(settings.isITaigiDictEnabled)
        XCTAssertFalse(settings.isTaiwanJapanDictEnabled)
        XCTAssertFalse(settings.isTaiHuaDictEnabled)
        XCTAssertFalse(settings.isTaiwanPlantDictEnabled)
        XCTAssertTrue(settings.isSttiDictEnabled)
        XCTAssertTrue(settings.isKhpooDictEnabled)
        XCTAssertFalse(settings.isVariantEnabled)
        XCTAssertFalse(settings.isKhiinEnabled)
        XCTAssertTrue(settings.isLkkDictEnabled)
        XCTAssertTrue(settings.isToolbarAutoCollapse)
        XCTAssertTrue(settings.isTpsOrMappedToER)
        XCTAssertEqual(settings.isGlobeKeyEnabled, DeviceCapabilities.prefersGlobeKeyByDefault)
        XCTAssertEqual(settings.keyHeightScale, 1.0)
        XCTAssertEqual(settings.keyFontSizeScale, 1.0)
        XCTAssertEqual(settings.candidateTextSizeScale, 1.0)
        XCTAssertEqual(settings.keyCornerRadius, 6.0)
        XCTAssertEqual(settings.keyBorderWidth, 0)
        XCTAssertEqual(settings.colorSettings, .default)
    }

    // MARK: - DisplayLanguage roster + clamp

    func test_INVARIANT_DISPLAY_LANGUAGE_PRODUCTION_ROSTER_pinsHanjiEnglishAndJapanese() {
        // CROSS-PLATFORM INVARIANT — must equal the Android productionLanguages roster (behavioral-invariants.md §37).
        XCTAssertEqual(DisplayLanguage.productionLanguages.map(\.tag), ["hanji", "en", "ja"])
    }

    func test_INVARIANT_DISPLAY_LANGUAGE_PRODUCTION_ROSTER_fromTagClampsToSelectable() {
        // Production languages resolve to themselves in every build.
        XCTAssertEqual(DisplayLanguage.fromTag("hanji"), .hanji)
        XCTAssertEqual(DisplayLanguage.fromTag("en"), .english)
        XCTAssertEqual(DisplayLanguage.fromTag("ja"), .japanese)
        // Not-yet-selectable identities (tl/poj) + unknown tags clamp to Hanji so the picker selection
        // always matches the rendered language; the persisted tag is left untouched (see migration tests).
        XCTAssertEqual(DisplayLanguage.fromTag("tailo"), .hanji)
        XCTAssertEqual(DisplayLanguage.fromTag("poj"), .hanji)
        XCTAssertEqual(DisplayLanguage.fromTag("xx"), .hanji)
    }

    func test_INVARIANT_DISPLAY_LANGUAGE_PRODUCTION_ROSTER_systemLeadsSelectableButNotRoster() {
        // System (Automatic) is a selection policy, not an authored language: it leads the picker but
        // never joins the production (authored-catalog) roster.
        XCTAssertEqual(DisplayLanguage.productionLanguages.map(\.tag), ["hanji", "en", "ja"])
        XCTAssertEqual(DisplayLanguage.selectableLanguages.first, .system)
        XCTAssertFalse(DisplayLanguage.productionLanguages.contains(.system))
        // "system" is selectable, so it round-trips (not clamped to Hanji like tl/poj).
        XCTAssertEqual(DisplayLanguage.fromTag("system"), .system)
    }

    // MARK: - Automatic (system) resolution

    func test_INVARIANT_DISPLAY_LANGUAGE_AUTOMATIC_RESOLUTION_mapsDeviceSubtagToConcreteLanguage() {
        // resolveAutomatic maps the device OS language subtag to a concrete authored language:
        // ja* → Japanese, zh* → Hanji, anything else (incl. empty) → English.
        XCTAssertEqual(DisplayLanguage.resolveAutomatic("ja"), .japanese)
        XCTAssertEqual(DisplayLanguage.resolveAutomatic("zh"), .hanji)
        XCTAssertEqual(DisplayLanguage.resolveAutomatic("en"), .english)
        XCTAssertEqual(DisplayLanguage.resolveAutomatic("fr"), .english)
        XCTAssertEqual(DisplayLanguage.resolveAutomatic(""), .english)
    }

    func test_INVARIANT_DISPLAY_LANGUAGE_AUTOMATIC_RESOLUTION_effectiveLanguageDefersOnlyForSystem() {
        // An explicit language resolves to itself regardless of the device subtag.
        XCTAssertEqual(DisplayLanguage.hanji.effectiveLanguage("ja"), .hanji)
        // .system defers to the device subtag via resolveAutomatic.
        XCTAssertEqual(DisplayLanguage.system.effectiveLanguage("ja"), .japanese)
        XCTAssertEqual(DisplayLanguage.system.effectiveLanguage("zh"), .hanji)
        XCTAssertEqual(DisplayLanguage.system.effectiveLanguage("de"), .english)
    }

    // MARK: - Raw-key migration parity

    /// Locks in the exact persisted `UserDefaults` key strings per
    /// property. If a future refactor renames a descriptor key, existing
    /// users' stored values would be silently abandoned — this test
    /// catches that by writing through the legacy raw key and reading
    /// back through the facade.
    // 中文: 拿舊版 raw key 寫盤,再用 facade 讀回 — 若哪個 descriptor key 被改名,測試就會炸。
    func test_legacyRawKeys_roundTripThroughFacade() {
        let settings = SharedSettings(userDefaults: defaults)

        // Bool fields — write opposite of descriptor default via raw key.
        defaults.set(false, forKey: "enableDoubleTapOO")
        defaults.set(false, forKey: "enableDoubleTapNN")
        defaults.set(true, forKey: "isTranslateSwapped")
        defaults.set(true, forKey: "outputBothScripts")
        defaults.set(true, forKey: "autoSpaceEnabled")
        defaults.set(false, forKey: "frequencyRecordingEnabled")
        defaults.set(false, forKey: "associationRecordingEnabled")
        defaults.set(false, forKey: "customDictEnabled")
        defaults.set(false, forKey: "moeDictEnabled")
        defaults.set(false, forKey: "newwordDictEnabled")
        defaults.set(false, forKey: "kunggeDictEnabled")
        defaults.set(true, forKey: "iTaigiDictEnabled")
        defaults.set(true, forKey: "taiwanJapanDictEnabled")
        defaults.set(true, forKey: "taiHuaDictEnabled")
        defaults.set(true, forKey: "taiwanPlantDictEnabled")
        defaults.set(false, forKey: "sttiDictEnabled")
        defaults.set(false, forKey: "khpooDictEnabled")
        defaults.set(true, forKey: "variantEnabled")
        defaults.set(true, forKey: "khiin")
        defaults.set(false, forKey: "lkkDictEnabled")
        defaults.set(false, forKey: "toolbarAutoCollapse")
        defaults.set(false, forKey: "tpsOrMapsToER")
        defaults.set(true, forKey: "fullAccessEnabled")
        defaults.set(true, forKey: "isGlobeKeyEnabled")

        // RawRep enums.
        defaults.set(InputMode.poj.rawValue, forKey: "inputMode")
        defaults.set(KeyboardLayoutType.qwerty.rawValue, forKey: "keyboardLayoutType")
        defaults.set(FontType.system.rawValue, forKey: "fontType")

        // App UI display language (frozen key "displayLanguage").
        defaults.set(DisplayLanguage.tailo.tag, forKey: "displayLanguage")

        // CGFloat / Double appearance.
        defaults.set(1.5, forKey: "keyHeightScale")
        defaults.set(1.5, forKey: "keyFontSizeScale")
        defaults.set(1.5, forKey: "candidateTextSizeScale")
        defaults.set(8.0, forKey: "keyCornerRadius")
        defaults.set(2.0, forKey: "keyBorderWidth")

        // Facade reads through the descriptors.
        XCTAssertFalse(settings.isDoubleTapOOEnabled)
        XCTAssertFalse(settings.isDoubleTapNNEnabled)
        XCTAssertTrue(settings.isTranslateSwapped)
        XCTAssertTrue(settings.isOutputBothScripts)
        XCTAssertTrue(settings.isAutoSpaceEnabled)
        XCTAssertFalse(settings.isFrequencyRecordingEnabled)
        XCTAssertFalse(settings.isAssociationRecordingEnabled)
        XCTAssertFalse(settings.isCustomDictEnabled)
        XCTAssertFalse(settings.isMoeDictEnabled)
        XCTAssertFalse(settings.isNewwordDictEnabled)
        XCTAssertFalse(settings.isKunggeDictEnabled)
        XCTAssertTrue(settings.isITaigiDictEnabled)
        XCTAssertTrue(settings.isTaiwanJapanDictEnabled)
        XCTAssertTrue(settings.isTaiHuaDictEnabled)
        XCTAssertTrue(settings.isTaiwanPlantDictEnabled)
        XCTAssertFalse(settings.isSttiDictEnabled)
        XCTAssertFalse(settings.isKhpooDictEnabled)
        XCTAssertTrue(settings.isVariantEnabled)
        XCTAssertTrue(settings.isKhiinEnabled)
        XCTAssertFalse(settings.isLkkDictEnabled)
        XCTAssertFalse(settings.isToolbarAutoCollapse)
        XCTAssertFalse(settings.isTpsOrMappedToER)
        XCTAssertTrue(settings.isFullAccessEnabled)
        XCTAssertTrue(settings.isGlobeKeyEnabled)
        XCTAssertEqual(settings.inputMode, .poj)
        XCTAssertEqual(settings.keyboardLayoutType, .qwerty)
        XCTAssertEqual(settings.fontType, .system)
        XCTAssertEqual(settings.displayLanguage, DisplayLanguage.tailo.tag)
        XCTAssertEqual(settings.keyHeightScale, 1.5)
        XCTAssertEqual(settings.keyFontSizeScale, 1.5)
        XCTAssertEqual(settings.candidateTextSizeScale, 1.5)
        XCTAssertEqual(settings.keyCornerRadius, 8.0)
        XCTAssertEqual(settings.keyBorderWidth, 2.0)
    }
}
