// Settings persistence facade shared by the app and the keyboard extension, backed by App Group
// UserDefaults. Implements both KeyboardEnvironment (writable from the UI) and
// EngineSettings/EngineSettingsProvider (engine live-read).

import Foundation
import KeyboardKit
import SwiftUI

// setInputMode / setKeyboardLayoutType are a state machine keeping inputMode and keyboardLayoutType 1:1.
final class SharedSettings {
    private let userDefaults: UserDefaults

    static let appGroupId = "group.com.siansiansu.TaigiKeyboard"

    /// App Group shared container URL, accessible by both main app and keyboard extension.
    static var sharedContainerURL: URL? {
        FileManager.default.containerURL(forSecurityApplicationGroupIdentifier: appGroupId)
    }

    // Falls back to .standard when the suite cannot be created (misconfigured provisioning) so the
    // keyboard does not crash.
    static let sharedUserDefaults = UserDefaults(suiteName: appGroupId) ?? .standard

    // MARK: - Typed key descriptors

    //
    // Each persisted field has a `SettingsKey<T>` descriptor pairing the raw
    // UserDefaults key with its default value + codec. Key spellings are
    // frozen for binary compatibility — anything stored on disk by an older
    // build must keep round-tripping. Typos (`tpsOrMapsToER`, `khiin`,
    // `enableDoubleTapOO/NN`) are deliberate; renaming them silently abandons
    // the user's existing preference.

    // Internal (not private): the Layout tab observes these two keys through `@AppStorage`.
    static let inputModeKey: SettingsKey<InputMode> = .rawRep("inputMode", default: .tl)
    private static let inputModeBeforeTpsKey: SettingsKey<InputMode> = .rawRep("inputModeBeforeTps", default: .tl)
    static let keyboardLayoutTypeKey: SettingsKey<KeyboardLayoutType> = .rawRep("keyboardLayoutType", default: .phahTaigi)
    private static let layoutBeforeTpsKey: SettingsKey<KeyboardLayoutType> = .rawRep("layoutBeforeTps", default: .phahTaigi)
    private static let fontTypeKey: SettingsKey<FontType> = .rawRep("fontType", default: .keyboardDefault)

    private static let isDoubleTapOOEnabledKey: SettingsKey<Bool> = .bool("enableDoubleTapOO", default: true)
    private static let isDoubleTapNNEnabledKey: SettingsKey<Bool> = .bool("enableDoubleTapNN", default: true)
    /// Nasal mark in POJ capitals ships ᴺ on every platform (USER 2026-09-22).
    private static let isNasalMarkerUppercaseEnabledKey: SettingsKey<Bool> = .bool("nasalMarkerUppercaseEnabled", default: true)
    /// Hanji-first out of the box (USER 2026-09-18): the hanji is the title,
    /// the romanization the subtitle, and a commit writes the hanji. Same
    /// default on Android, macOS and Windows.
    private static let isHanjiFirstKey: SettingsKey<Bool> = .bool("isTranslateSwapped", default: true)
    private static let isOutputBothScriptsKey: SettingsKey<Bool> = .bool("outputBothScripts", default: false)
    // Raw string key shared by all four platforms; unknown / malformed → `.sideBySide`.
    private static let candidateDisplayModeKey: SettingsKey<CandidateDisplayMode> = .rawRep("candidateDisplayMode", default: .sideBySide)
    private static let isFullAccessEnabledKey: SettingsKey<Bool> = .bool("fullAccessEnabled", default: false)
    private static let isAutoSpaceEnabledKey: SettingsKey<Bool> = .bool("autoSpaceEnabled", default: false)
    private static let isLiteralRomanCandidateEnabledKey: SettingsKey<Bool> = .bool("literalRomanCandidateEnabled", default: false)
    // Raw string key shared by every platform; unknown / malformed → `.hyphen`.
    private static let syllableSeparatorKey: SettingsKey<SyllableSeparator> = .rawRep("syllableSeparator", default: .hyphen)
    // Retired 2026-10-06: the No Hyphens switch, carried over by `carryOverHyphenlessRoman()`.
    private static let retiredHyphenlessRomanEnabledKey: SettingsKey<Bool> = .bool("hyphenlessRomanEnabled", default: false)
    private static let isCustomDictEnabledKey: SettingsKey<Bool> = .bool("customDictEnabled", default: true)

    private static let isMoeDictEnabledKey: SettingsKey<Bool> = .bool("moeDictEnabled", default: true)
    private static let isNewwordDictEnabledKey: SettingsKey<Bool> = .bool("newwordDictEnabled", default: true)
    private static let isKunggeDictEnabledKey: SettingsKey<Bool> = .bool("kunggeDictEnabled", default: true)
    private static let isITaigiDictEnabledKey: SettingsKey<Bool> = .bool("iTaigiDictEnabled", default: false)
    private static let isTaiwanJapanDictEnabledKey: SettingsKey<Bool> = .bool("taiwanJapanDictEnabled", default: false)
    private static let isTaiHuaDictEnabledKey: SettingsKey<Bool> = .bool("taiHuaDictEnabled", default: false)
    private static let isTaiwanPlantDictEnabledKey: SettingsKey<Bool> = .bool("taiwanPlantDictEnabled", default: false)
    private static let isSttiDictEnabledKey: SettingsKey<Bool> = .bool("sttiDictEnabled", default: true)
    private static let isKhpooDictEnabledKey: SettingsKey<Bool> = .bool("khpooDictEnabled", default: true)
    private static let isVariantEnabledKey: SettingsKey<Bool> = .bool("variantEnabled", default: false)
    private static let isKhiinEnabledKey: SettingsKey<Bool> = .bool("khiin", default: false)
    private static let isLkkDictEnabledKey: SettingsKey<Bool> = .bool("lkkDictEnabled", default: true)
    private static let isDevDictEnabledKey: SettingsKey<Bool> = .bool("devDictEnabled", default: true)

    // kautian subcollection toggles (nested under the kautian master).
    // 10 accents default ON — opt-out model, upgrade is zero behaviour change (DD5).
    // Name appendix defaults ON — opt-out model; a user who never toggled it gets
    // surname candidates after upgrade, an explicit OFF stored value is preserved.
    // Accent key order mirrors config.yaml `dialect_columns` (= subtag bit - 1).
    private static let isKautianAccentLukangEnabledKey: SettingsKey<Bool> = .bool("kautianAccentLukangEnabled", default: true)
    private static let isKautianAccentSansiaEnabledKey: SettingsKey<Bool> = .bool("kautianAccentSansiaEnabled", default: true)
    private static let isKautianAccentTaipakEnabledKey: SettingsKey<Bool> = .bool("kautianAccentTaipakEnabled", default: true)
    private static let isKautianAccentGilanEnabledKey: SettingsKey<Bool> = .bool("kautianAccentGilanEnabled", default: true)
    private static let isKautianAccentTainanEnabledKey: SettingsKey<Bool> = .bool("kautianAccentTainanEnabled", default: true)
    private static let isKautianAccentKaohsiungEnabledKey: SettingsKey<Bool> = .bool("kautianAccentKaohsiungEnabled", default: true)
    private static let isKautianAccentKinmenEnabledKey: SettingsKey<Bool> = .bool("kautianAccentKinmenEnabled", default: true)
    private static let isKautianAccentMakungEnabledKey: SettingsKey<Bool> = .bool("kautianAccentMakungEnabled", default: true)
    private static let isKautianAccentSintikEnabledKey: SettingsKey<Bool> = .bool("kautianAccentSintikEnabled", default: true)
    private static let isKautianAccentTaichungEnabledKey: SettingsKey<Bool> = .bool("kautianAccentTaichungEnabled", default: true)
    private static let isKautianNameAppendixEnabledKey: SettingsKey<Bool> = .bool("kautianNameAppendixEnabled", default: true)

    private static let isTpsOrMappedToERKey: SettingsKey<Bool> = .bool("tpsOrMapsToER", default: true)
    private static let isToolbarAutoCollapseKey: SettingsKey<Bool> = .bool("toolbarAutoCollapse", default: true)
    // Raw strings shared with Android (`keyboard__one_handed_mode` / `keyboard__toolbar_keyboard_action`).
    private static let oneHandedModeKey: SettingsKey<OneHandedMode> = .rawRep("oneHandedMode", default: .off)
    private static let keyboardToolbarActionKey: SettingsKey<KeyboardToolbarAction> = .rawRep("keyboardToolbarAction", default: .dismiss)

    /// Globe key has a device-dependent default (`DeviceCapabilities.prefersGlobeKeyByDefault`)
    /// so the getter is hand-written; the descriptor is reused for writes
    /// and the reset-to-default `removeObject(forKey:)` path.
    private static let isGlobeKeyEnabledKey: SettingsKey<Bool> = .bool("isGlobeKeyEnabled", default: false)

    // Retired: the pre-theme global appearance, written by the appearance screen #407 removed.
    // Read once by `retireLegacyAppearance(themeName:)`, which carries a customized look into a
    // user theme and then removes them.
    private static let keyHeightScaleKey: SettingsKey<CGFloat> = .cgFloat("keyHeightScale", default: 1.0)
    private static let keyFontSizeScaleKey: SettingsKey<CGFloat> = .cgFloat("keyFontSizeScale", default: 1.0)
    private static let candidateTextSizeScaleKey: SettingsKey<CGFloat> = .cgFloat("candidateTextSizeScale", default: 1.0)
    private static let keyCornerRadiusKey: SettingsKey<CGFloat> = .cgFloat("keyCornerRadius", default: 6.0)
    private static let keyBorderWidthKey: SettingsKey<CGFloat> = .cgFloat("keyBorderWidth", default: 0)
    private static let colorSettingsKey: SettingsKey<KeyboardColorSettings> = .codable("colorSettings", default: .default)

    private static let selectedThemeIdKey: SettingsKey<String> = .string("selectedThemeId", default: ThemeId.default)
    private static let themeRevisionKey: SettingsKey<Int> = .int("themeRevision", default: 0)

    // App UI display-language tag (orthogonal to keyboard input mode) — see DisplayLanguage. Shared
    // host↔extension through the App Group; the extension picks up changes via didChangeNotification.
    // Defaults to system (Automatic), which follows the device locale on first launch. Key spelling is frozen.
    private static let displayLanguageKey: SettingsKey<String> = .string("displayLanguage", default: DisplayLanguage.defaultTag)

    static let shared = SharedSettings()

    /// Where `user_themes.json` lives: the App Group container, or a test's own directory.
    private let themesContainerURL: URL?

    private init() {
        userDefaults = Self.sharedUserDefaults
        themesContainerURL = Self.sharedContainerURL
    }

    /// Test-only seam: builds a `SharedSettings` against a caller-supplied
    /// `UserDefaults` (typically a fresh suite name) so the TPS state machine
    /// and default-value behavior can be exercised in isolation. Production
    /// code must keep using `SharedSettings.shared`.
    ///
    /// Scope caveat: `KeyboardEnvironment.settingsUserDefaults` continues to
    /// return the static App Group store regardless of the injected
    /// `userDefaults`, so this seam is **not** a fully isolated facade.
    /// Tests must not exercise the `UserDefaults.didChangeNotification`
    /// path through `settingsUserDefaults`; only stored-value reads / writes
    /// flow through the injected store. User themes go to `themesContainerURL`
    /// (nil = no store: every theme write fails).
    init(userDefaults: UserDefaults, themesContainerURL: URL? = SharedSettings.sharedContainerURL) {
        self.userDefaults = userDefaults
        self.themesContainerURL = themesContainerURL
    }

    // The setter forwards to setInputMode(_:) so the layout stays in sync.
    var inputMode: InputMode {
        get { userDefaults.value(for: Self.inputModeKey) }
        set { setInputMode(newValue) }
    }

    // A write posts didChangeNotification, so DisplayLanguageStore re-resolves in both processes
    // and the language switches live without a relaunch.
    var displayLanguage: String {
        get { userDefaults.value(for: Self.displayLanguageKey) }
        set { userDefaults.set(newValue, for: Self.displayLanguageKey) }
    }

    // Consumed by ToneConverter via PojMarkerOptions.
    var isDoubleTapOOEnabled: Bool {
        get { userDefaults.value(for: Self.isDoubleTapOOEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isDoubleTapOOEnabledKey) }
    }

    var isDoubleTapNNEnabled: Bool {
        get { userDefaults.value(for: Self.isDoubleTapNNEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isDoubleTapNNEnabledKey) }
    }

    /// Nasal mark in POJ capitals (§53): the POJ nasal marker follows the case of the letters
    /// before it (`SIÂᴺ`); off, always `ⁿ`. Reaches the engine through
    /// `pojMarkerOptions` (inverted as `AppConfig.force_lowercase_nasal_marker`)
    /// and the case ops; the `nn` key label reads it off `SettingsSnapshot`.
    // CROSS-PLATFORM INVARIANT — mirrors android/app/src/main/java/com/siansiansu/taigikeyboard/ime/settings/PrefHelper.kt:isNasalMarkerUppercaseEnabled (ON).
    // Drift causes silent divergence (one platform writes ᴺ after a capital, the other ⁿ).
    var isNasalMarkerUppercaseEnabled: Bool {
        get { userDefaults.value(for: Self.isNasalMarkerUppercaseEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isNasalMarkerUppercaseEnabledKey) }
    }

    /// Raw stored swap flag — the ONLY read-write API. Settings UI, the 文/A
    /// toggle (after its `allowsSwapToggle` guard) and `resetToDefaults` use
    /// this. Everything that *consumes* the swap reads a derived value: the
    /// candidate projection `isHanjiFirst` (`EngineSettings` conformance
    /// below) or the punctuation width `isFullWidthPunctuation`.
    var storedIsHanjiFirst: Bool {
        get { userDefaults.value(for: Self.isHanjiFirstKey) }
        set { userDefaults.set(newValue, for: Self.isHanjiFirstKey) }
    }

    /// Candidate cell rendering mode. Switching to `.romanOnly` leaves the
    /// stored swap / both-scripts flags untouched; switching back restores them.
    var candidateDisplayMode: CandidateDisplayMode {
        get { userDefaults.value(for: Self.candidateDisplayModeKey) }
        set { userDefaults.set(newValue, for: Self.candidateDisplayModeKey) }
    }

    // Global setting, not per-theme.
    var fontType: FontType {
        get { userDefaults.value(for: Self.fontTypeKey) }
        set { userDefaults.set(newValue, for: Self.fontTypeKey) }
    }

    var isFullAccessEnabled: Bool {
        get { userDefaults.value(for: Self.isFullAccessEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isFullAccessEnabledKey) }
    }

    // isAutoCapitalizationEnabled moved to KeyboardKit's KeyboardSettings
    // Access via state.keyboardContext.settings.isAutocapitalizationEnabled

    var isAutoSpaceEnabled: Bool {
        get { userDefaults.value(for: Self.isAutoSpaceEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isAutoSpaceEnabledKey) }
    }

    // The setter forwards to setKeyboardLayoutType(_:) so the input mode stays in sync.
    var keyboardLayoutType: KeyboardLayoutType {
        get { userDefaults.value(for: Self.keyboardLayoutTypeKey) }
        set { setKeyboardLayoutType(newValue) }
    }

    // MARK: - TPS state machine

    /// Writes `newMode` to `inputMode` and applies the TPS ↔ layout 1:1 sync.
    ///
    /// - Entering `.tps`: saves the current `keyboardLayoutType` into
    ///   `layoutBeforeTps` (skipped if the layout is already `.tps`) and
    ///   flips the layout to `.tps`.
    /// - Leaving `.tps`: restores the layout from `layoutBeforeTps`, but only
    ///   when the live layout is still `.tps` — a manual layout change
    ///   earlier in the same flow is preserved.
    /// - All cascading writes go through `userDefaults.set(_, for:)`, which
    ///   bypasses the property setter (no re-entry into the state machine).
    func setInputMode(_ newMode: InputMode) {
        let oldMode = inputMode
        userDefaults.set(newMode, for: Self.inputModeKey)

        if newMode == .tps, oldMode != .tps {
            let currentLayout = keyboardLayoutType
            if currentLayout != .tps {
                layoutBeforeTps = currentLayout
                userDefaults.set(.tps, for: Self.keyboardLayoutTypeKey)
            }
        } else if newMode != .tps, oldMode == .tps {
            if keyboardLayoutType == .tps {
                userDefaults.set(layoutBeforeTps, for: Self.keyboardLayoutTypeKey)
            }
        }
    }

    /// Writes `newLayout` to `keyboardLayoutType` and applies the TPS ↔
    /// input-mode 1:1 sync. Mirrors `setInputMode(_:)` with one deliberate
    /// asymmetry preserved from the pre-refactor cascade: leaving `.tps` on
    /// the layout side **unconditionally** restores `inputMode` from
    /// `inputModeBeforeTps`, whereas the input-side exit guards on
    /// `keyboardLayoutType == .tps` before restoring layout. This mirrors
    /// HEAD~1 byte-for-byte; any future symmetrization belongs in a
    /// separate slice.
    func setKeyboardLayoutType(_ newLayout: KeyboardLayoutType) {
        let oldLayout = keyboardLayoutType
        userDefaults.set(newLayout, for: Self.keyboardLayoutTypeKey)

        if newLayout == .tps, oldLayout != .tps {
            let currentMode = inputMode
            if currentMode != .tps {
                inputModeBeforeTps = currentMode
                userDefaults.set(.tps, for: Self.inputModeKey)
            }
        } else if newLayout != .tps, oldLayout == .tps {
            userDefaults.set(inputModeBeforeTps, for: Self.inputModeKey)
        }
    }

    /// Stores the inputMode before switching to TPS, so it can be restored when leaving TPS
    private var inputModeBeforeTps: InputMode {
        get { userDefaults.value(for: Self.inputModeBeforeTpsKey) }
        set { userDefaults.set(newValue, for: Self.inputModeBeforeTpsKey) }
    }

    /// Stores the layout before switching to TPS, so it can be restored when leaving TPS
    private var layoutBeforeTps: KeyboardLayoutType {
        get { userDefaults.value(for: Self.layoutBeforeTpsKey) }
        set { userDefaults.set(newValue, for: Self.layoutBeforeTpsKey) }
    }

    /// Raw stored Annotate in Brackets flag — read-write counterpart of the derived
    /// `isOutputBothScripts`; same split as `storedIsHanjiFirst`.
    var storedIsOutputBothScripts: Bool {
        get { userDefaults.value(for: Self.isOutputBothScriptsKey) }
        set { userDefaults.set(newValue, for: Self.isOutputBothScriptsKey) }
    }

    // MARK: - Literal-Roman Candidate (§34/S22, default: off)

    // Show Typed Text First: put the literal roman candidate first while composing in TL/POJ.
    var isLiteralRomanCandidateEnabled: Bool {
        get { userDefaults.value(for: Self.isLiteralRomanCandidateEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isLiteralRomanCandidateEnabledKey) }
    }

    // MARK: - Syllable Separator (§49, default: hyphen)

    /// Sent to the engine as stored: the engine exempts the TPS layout itself
    /// (`AppConfig::rendered_syllable_joiner`), so there is no effective variant.
    var syllableSeparator: SyllableSeparator {
        get { userDefaults.value(for: Self.syllableSeparatorKey) }
        set { userDefaults.set(newValue, for: Self.syllableSeparatorKey) }
    }

    /// The No Hyphens switch the Syllable Separator replaced (2026-10-06): a stored `true`
    /// becomes `.noSeparator` unless a separator is already stored, then the switch goes.
    /// Called at both launches beside `retireLegacyAppearance(themeName:)` — the keyboard can
    /// start before the host app. Idempotent: once the switch is gone it is one absent-key read.
    func carryOverHyphenlessRoman() {
        guard let stored = userDefaults.storedObject(for: Self.retiredHyphenlessRomanEnabledKey) else { return }
        if stored as? Bool == true, userDefaults.storedObject(for: Self.syllableSeparatorKey) == nil {
            syllableSeparator = .noSeparator
        }
        userDefaults.remove(Self.retiredHyphenlessRomanEnabledKey)
    }

    // MARK: - Custom Dictionary

    var isCustomDictEnabled: Bool {
        get { userDefaults.value(for: Self.isCustomDictEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isCustomDictEnabledKey) }
    }

    // MARK: - Dictionary Toggles

    var isMoeDictEnabled: Bool {
        get { userDefaults.value(for: Self.isMoeDictEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isMoeDictEnabledKey) }
    }

    var isNewwordDictEnabled: Bool {
        get { userDefaults.value(for: Self.isNewwordDictEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isNewwordDictEnabledKey) }
    }

    var isKunggeDictEnabled: Bool {
        get { userDefaults.value(for: Self.isKunggeDictEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isKunggeDictEnabledKey) }
    }

    var isITaigiDictEnabled: Bool {
        get { userDefaults.value(for: Self.isITaigiDictEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isITaigiDictEnabledKey) }
    }

    var isTaiwanJapanDictEnabled: Bool {
        get { userDefaults.value(for: Self.isTaiwanJapanDictEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isTaiwanJapanDictEnabledKey) }
    }

    var isTaiHuaDictEnabled: Bool {
        get { userDefaults.value(for: Self.isTaiHuaDictEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isTaiHuaDictEnabledKey) }
    }

    var isTaiwanPlantDictEnabled: Bool {
        get { userDefaults.value(for: Self.isTaiwanPlantDictEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isTaiwanPlantDictEnabledKey) }
    }

    var isSttiDictEnabled: Bool {
        get { userDefaults.value(for: Self.isSttiDictEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isSttiDictEnabledKey) }
    }

    var isKhpooDictEnabled: Bool {
        get { userDefaults.value(for: Self.isKhpooDictEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isKhpooDictEnabledKey) }
    }

    /// Variant characters toggle (default: off)
    // Variant-character candidates.
    var isVariantEnabled: Bool {
        get { userDefaults.value(for: Self.isVariantEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isVariantEnabledKey) }
    }

    /// Khiin supplementary data toggle (default: off)
    var isKhiinEnabled: Bool {
        get { userDefaults.value(for: Self.isKhiinEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isKhiinEnabledKey) }
    }

    /// LKK Hàn-lô mixed script suggestions (default: on)
    // LKK mixed Hanji-romanization candidates.
    var isLkkDictEnabled: Bool {
        get { userDefaults.value(for: Self.isLkkDictEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isLkkDictEnabledKey) }
    }

    /// Developer supplement dictionary (Supplementary Word List) toggle (default: on)
    var isDevDictEnabled: Bool {
        get { userDefaults.value(for: Self.isDevDictEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isDevDictEnabledKey) }
    }

    // MARK: - Kautian Subcollections (nested under the kautian master, default on)

    // Accent classification, in `config.yaml` dialect_columns order: 鹿港/三峽/臺北/金門/
    // 馬公/新竹 偏泉腔, 宜蘭/臺中 偏漳腔, 臺南/高雄 混合腔. The identifier names the locality;
    // the classification is what the identifier cannot carry.

    var isKautianAccentLukangEnabled: Bool {
        get { userDefaults.value(for: Self.isKautianAccentLukangEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isKautianAccentLukangEnabledKey) }
    }

    var isKautianAccentSansiaEnabled: Bool {
        get { userDefaults.value(for: Self.isKautianAccentSansiaEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isKautianAccentSansiaEnabledKey) }
    }

    var isKautianAccentTaipakEnabled: Bool {
        get { userDefaults.value(for: Self.isKautianAccentTaipakEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isKautianAccentTaipakEnabledKey) }
    }

    var isKautianAccentGilanEnabled: Bool {
        get { userDefaults.value(for: Self.isKautianAccentGilanEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isKautianAccentGilanEnabledKey) }
    }

    var isKautianAccentTainanEnabled: Bool {
        get { userDefaults.value(for: Self.isKautianAccentTainanEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isKautianAccentTainanEnabledKey) }
    }

    var isKautianAccentKaohsiungEnabled: Bool {
        get { userDefaults.value(for: Self.isKautianAccentKaohsiungEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isKautianAccentKaohsiungEnabledKey) }
    }

    var isKautianAccentKinmenEnabled: Bool {
        get { userDefaults.value(for: Self.isKautianAccentKinmenEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isKautianAccentKinmenEnabledKey) }
    }

    var isKautianAccentMakungEnabled: Bool {
        get { userDefaults.value(for: Self.isKautianAccentMakungEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isKautianAccentMakungEnabledKey) }
    }

    var isKautianAccentSintikEnabled: Bool {
        get { userDefaults.value(for: Self.isKautianAccentSintikEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isKautianAccentSintikEnabledKey) }
    }

    var isKautianAccentTaichungEnabled: Bool {
        get { userDefaults.value(for: Self.isKautianAccentTaichungEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isKautianAccentTaichungEnabledKey) }
    }

    // Surname Appendix (given + family names).
    var isKautianNameAppendixEnabled: Bool {
        get { userDefaults.value(for: Self.isKautianNameAppendixEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isKautianNameAppendixEnabledKey) }
    }

    // MARK: - Toolbar Settings

    /// Toolbar auto-collapse toggle (default: true = auto-collapse on composing/mode change)
    var isToolbarAutoCollapse: Bool {
        get { userDefaults.value(for: Self.isToolbarAutoCollapseKey) }
        set { userDefaults.set(newValue, for: Self.isToolbarAutoCollapseKey) }
    }

    // MARK: - One-handed mode

    /// Key area docked to one edge at 80% width (`.off` = full width).
    var oneHandedMode: OneHandedMode {
        get { userDefaults.value(for: Self.oneHandedModeKey) }
        set { userDefaults.set(newValue, for: Self.oneHandedModeKey) }
    }

    /// Most recent pick of the toolbar keyboard button's long-press callout; drives its icon + tap.
    var keyboardToolbarAction: KeyboardToolbarAction {
        get { userDefaults.value(for: Self.keyboardToolbarActionKey) }
        set { userDefaults.set(newValue, for: Self.keyboardToolbarActionKey) }
    }

    // MARK: - Globe Key

    /// Globe key toggle. Default depends on device type for backward compatibility:
    /// iPad/iPhone SE (Touch ID) = true, regular iPhone = false.
    var isGlobeKeyEnabled: Bool {
        get {
            guard let stored = userDefaults.storedObject(for: Self.isGlobeKeyEnabledKey) as? Bool else {
                return DeviceCapabilities.prefersGlobeKeyByDefault
            }
            return stored
        }
        set { userDefaults.set(newValue, for: Self.isGlobeKeyEnabledKey) }
    }

    // MARK: - TPS Settings

    /// TPS "or" maps to ㄜ (default: on). When off, "or" maps to ㄛ.
    var isTpsOrMappedToER: Bool {
        get { userDefaults.value(for: Self.isTpsOrMappedToERKey) }
        set { userDefaults.set(newValue, for: Self.isTpsOrMappedToERKey) }
    }

    // MARK: - Themes

    // "default" renders the factory appearance; any other id resolves through userThemeStore or a built-in.
    var selectedThemeId: String {
        get { userDefaults.value(for: Self.selectedThemeIdKey) }
        set { userDefaults.set(newValue, for: Self.selectedThemeIdKey) }
    }

    // User themes live in a backup-excluded JSON file in the App Group; each mutation bumps
    // themeRevision to refresh the other process, then sweeps the theme photos no saved
    // theme references any more (a replaced / deleted photo never lingers).
    private lazy var userThemeStore = UserThemeStore(
        containerURL: themesContainerURL,
        onMutated: { [weak self] in
            self?.bumpThemeRevision()
            self?.sweepThemeImages()
        },
    )

    // Writing themeRevision posts didChangeNotification so the extension re-resolves the theme.
    private func bumpThemeRevision() {
        let next = userDefaults.value(for: Self.themeRevisionKey) &+ 1
        userDefaults.set(next, for: Self.themeRevisionKey)
    }

    // Resolved appearance for the renderer. "default" takes the fast path (no file I/O). Only a UUID id
    // reads the user-theme file; a built-in id resolves from the BuiltInThemes table by colorScheme,
    // keeping the render hot path free of I/O.
    func resolvedAppearance(for colorScheme: ColorScheme) -> ThemeAppearance {
        let id = selectedThemeId
        if id == ThemeId.default {
            return .default
        }
        let userThemes = ThemeId.isUserTheme(id) ? cachedUserThemes() : []
        return ThemeResolver.resolved(themeId: id, colorScheme: colorScheme, userThemes: userThemes)
    }

    // MARK: - Retired global appearance

    /// Carries a look customized on the retired appearance screen into a user theme named
    /// `themeName` and selects it when the keyboard was showing that look (the "default" theme,
    /// or an id nothing resolves), then removes the retired keys; a factory look is only removed.
    /// Runs at every app and keyboard launch and does nothing once the keys are gone. When the
    /// theme cannot be written (no container, e.g. the keyboard without Full Access, or a saved
    /// theme list that does not read back whole) the keys stay, so the look is still there for
    /// the next launch to carry. App and keyboard are separate processes with no lock between
    /// them: a race (or a kill between the write and the removal) can carry the look twice,
    /// never lose it. Android runs the same steps in one DataStore transaction.
    func retireLegacyAppearance(themeName: String, now: Date = Date()) {
        guard let legacy = storedLegacyAppearance else { return }
        if legacy != .default {
            let id = selectedThemeId
            let wasShowingLegacy = id == ThemeId.default
                || (BuiltInThemes.theme(id: id) == nil && !loadUserThemes().contains { $0.id.uuidString == id })
            let theme = UserTheme(id: UUID(), name: themeName, appearance: legacy, createdAt: now, updatedAt: now)
            // Past the cap: a look the user made is never refused.
            guard userThemeStore.addCarriedOver(theme) else { return }
            if wasShowingLegacy {
                selectedThemeId = theme.id.uuidString
            }
        }
        userDefaults.remove(Self.keyHeightScaleKey)
        userDefaults.remove(Self.keyFontSizeScaleKey)
        userDefaults.remove(Self.candidateTextSizeScaleKey)
        userDefaults.remove(Self.keyCornerRadiusKey)
        userDefaults.remove(Self.keyBorderWidthKey)
        userDefaults.remove(Self.colorSettingsKey)
    }

    /// The look the retired keys hold, or nil when none of them is stored. Shadow is 0: the
    /// retired screen had no shadow control.
    private var storedLegacyAppearance: ThemeAppearance? {
        let isAnyStored = userDefaults.storedObject(for: Self.keyHeightScaleKey) != nil
            || userDefaults.storedObject(for: Self.keyFontSizeScaleKey) != nil
            || userDefaults.storedObject(for: Self.candidateTextSizeScaleKey) != nil
            || userDefaults.storedObject(for: Self.keyCornerRadiusKey) != nil
            || userDefaults.storedObject(for: Self.keyBorderWidthKey) != nil
            || userDefaults.storedObject(for: Self.colorSettingsKey) != nil
        guard isAnyStored else { return nil }
        return ThemeAppearance(
            colors: userDefaults.value(for: Self.colorSettingsKey),
            keyShadowIntensity: 0,
            keyHeightScale: userDefaults.value(for: Self.keyHeightScaleKey),
            keyFontSizeScale: userDefaults.value(for: Self.keyFontSizeScaleKey),
            candidateTextSizeScale: userDefaults.value(for: Self.candidateTextSizeScaleKey),
            keyCornerRadius: userDefaults.value(for: Self.keyCornerRadiusKey),
            keyBorderWidth: userDefaults.value(for: Self.keyBorderWidthKey),
        )
    }

    // Any CRUD bumps themeRevision, so a revision mismatch on the next read reloads — across processes too.
    private var userThemesCache: (revision: Int, themes: [UserTheme])?

    /// Loads user themes, cached by `themeRevision` (bumped on every mutation),
    /// so the render snapshot does not re-read `user_themes.json` per redraw.
    private func cachedUserThemes() -> [UserTheme] {
        let revision = userDefaults.value(for: Self.themeRevisionKey)
        if let cache = userThemesCache, cache.revision == revision {
            return cache.themes
        }
        let themes = userThemeStore.load()
        userThemesCache = (revision, themes)
        return themes
    }

    // MARK: - User theme CRUD

    // Public user-theme CRUD for the editor and the Custom Themes shelf, delegating to userThemeStore
    // and sharing the themeRevision cache with the render path.
    func loadUserThemes() -> [UserTheme] {
        cachedUserThemes()
    }

    /// Appends a user theme. Returns `false` at the cap or on write failure.
    @discardableResult
    func addUserTheme(_ theme: UserTheme) -> Bool {
        userThemeStore.add(theme)
    }

    func updateUserTheme(_ theme: UserTheme) {
        userThemeStore.update(theme)
    }

    func deleteUserTheme(id: UUID) {
        userThemeStore.delete(id: id)
    }

    /// Removes theme photos no saved user theme references any more. Runs after every
    /// theme mutation and when the editor closes (a photo picked in the editor is only
    /// kept once its theme is saved).
    func sweepThemeImages() {
        let referenced = Set(loadUserThemes().compactMap { $0.appearance.colors.background?.image?.file })
        ThemeImageCache.shared.store.sweep(keeping: referenced)
        ThemeImageCache.shared.removeAll()
    }

    /// Creates an immutable snapshot of render-relevant settings.
    /// Call once per render cycle to avoid repeated UserDefaults reads.
    func snapshot(for colorScheme: ColorScheme) -> SettingsSnapshot {
        let appearance = resolvedAppearance(for: colorScheme)
        // Only user themes carry explicit shadow semantics (0 = no shadow); default / built-in return nil
        // so the renderer keeps KeyboardKit's standard shadow. The shadow slider is user-theme-only.
        let isUserTheme = ThemeId.isUserTheme(selectedThemeId)
        return SettingsSnapshot(
            inputMode: inputMode,
            fontType: fontType,
            keyboardLayoutType: keyboardLayoutType,
            isHanjiFirst: isHanjiFirst,
            isTpsOrMappedToER: isTpsOrMappedToER,
            isNasalMarkerUppercaseEnabled: isNasalMarkerUppercaseEnabled,
            keyFontSizeScale: appearance.keyFontSizeScale,
            keyCornerRadius: appearance.keyCornerRadius,
            colorSettings: appearance.colors,
            candidateTextSizeScale: appearance.candidateTextSizeScale,
            keyBorderWidth: appearance.keyBorderWidth,
            keyShadowIntensity: isUserTheme ? CGFloat(appearance.keyShadowIntensity) : nil,
        )
    }

    // Resets every SharedSettings-owned field to its factory default. KeyboardKit-owned defaults are
    // not covered here — those go through SettingsResetCoordinator.resetKeyboardKitDefaults().
    func resetToDefaults() {
        inputMode = .tl
        isDoubleTapOOEnabled = true
        isDoubleTapNNEnabled = true
        isNasalMarkerUppercaseEnabled = true
        storedIsHanjiFirst = true
        storedIsOutputBothScripts = false
        candidateDisplayMode = .sideBySide
        isLiteralRomanCandidateEnabled = false
        syllableSeparator = .hyphen
        fontType = .keyboardDefault
        isAutoSpaceEnabled = false
        keyboardLayoutType = .phahTaigi
        // Dictionary toggles (iTaigi, TaiHua default off)
        isMoeDictEnabled = true
        isNewwordDictEnabled = true
        isKunggeDictEnabled = true
        isITaigiDictEnabled = false
        isTaiwanJapanDictEnabled = false
        isTaiHuaDictEnabled = false
        isTaiwanPlantDictEnabled = false
        isSttiDictEnabled = true
        isKhpooDictEnabled = true
        isVariantEnabled = false
        isKhiinEnabled = false
        isLkkDictEnabled = true
        isDevDictEnabled = true
        // Kautian subcollections (10 accents default on; name appendix default off)
        isKautianAccentLukangEnabled = true
        isKautianAccentSansiaEnabled = true
        isKautianAccentTaipakEnabled = true
        isKautianAccentGilanEnabled = true
        isKautianAccentTainanEnabled = true
        isKautianAccentKaohsiungEnabled = true
        isKautianAccentKinmenEnabled = true
        isKautianAccentMakungEnabled = true
        isKautianAccentSintikEnabled = true
        isKautianAccentTaichungEnabled = true
        isKautianNameAppendixEnabled = true
        // Toolbar
        isToolbarAutoCollapse = true
        oneHandedMode = .off
        keyboardToolbarAction = .dismiss
        // Globe key: remove stored value so device-based default takes effect
        userDefaults.remove(Self.isGlobeKeyEnabledKey)
        // TPS
        isTpsOrMappedToER = true
        // Back to the default theme (the factory appearance); stored user themes are not deleted.
        selectedThemeId = ThemeId.default
        // Writes the persisted value only. The caller (resetAllSettings) must then call
        // DisplayLanguageStore.syncFromSettings(), or the on-screen language will not follow.
        displayLanguage = DisplayLanguage.defaultTag

        // KeyboardKit-owned defaults live in a separate store; reset via
        // `SettingsResetCoordinator.resetAll()` when you need both sides.
    }
}

// MARK: - EngineSettings Conformance

/// SharedSettings bridges the engine's settings needs to its own
/// `UserDefaults` (App Group container) behind one protocol.
extension SharedSettings: EngineSettings {
    /// Live-reads the three underlying booleans per call, matching the
    /// `EngineSettingsProvider.current` live-read contract.
    var pojMarkerOptions: PojMarkerOptions {
        PojMarkerOptions(
            isDoubleTapOOEnabled: isDoubleTapOOEnabled,
            isDoubleTapNNEnabled: isDoubleTapNNEnabled,
            isNasalMarkerUppercaseEnabled: isNasalMarkerUppercaseEnabled,
        )
    }

    /// Effective swap — the rule lives on `CandidateDisplayMode`. Read-only by
    /// design: a `.toggle()` on a derived getter would overwrite the stored
    /// flag, so writers go through `storedIsHanjiFirst`.
    var isHanjiFirst: Bool {
        candidateDisplayMode.effectiveHanjiFirst(stored: storedIsHanjiFirst)
    }

    /// Effective punctuation width — same seam, read by the layout + 文/A icon.
    /// TPS types Chinese and is always full-width on every page (character,
    /// numeric, symbolic), whatever is stored; the key is hidden there.
    // CROSS-PLATFORM INVARIANT — mirrors android PrefHelper.isFullWidthPunctuation (isTpsLayout || …).
    var isFullWidthPunctuation: Bool {
        keyboardLayoutType == .tps
            || candidateDisplayMode.effectiveFullWidthPunctuation(stored: storedIsHanjiFirst)
    }

    /// Effective Annotate in Brackets — same seam, same rule owner.
    var isOutputBothScripts: Bool {
        candidateDisplayMode.effectiveOutputBothScripts(stored: storedIsOutputBothScripts)
    }
}

// MARK: - EngineSettingsProvider Conformance

extension SharedSettings: EngineSettingsProvider {
    /// Returns `self` as `EngineSettings`. Each property access on the
    /// returned value re-reads `UserDefaults`, so the engine always sees
    /// the most recent values (critical for in-app setting changes to
    /// propagate without restarting the keyboard extension).
    var current: EngineSettings {
        self
    }
}
