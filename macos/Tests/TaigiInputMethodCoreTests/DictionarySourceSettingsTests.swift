// The 24 dictionary toggles: their stored names, their fresh-install values,
// and what the engine resolves them to.

@testable import TaigiInputMethodCore
import XCTest

/// The keys and defaults are copied from iOS on purpose, so the same
/// `defaults read` answers the same way on both platforms and a settings
/// transfer would have one name per setting rather than two.
final class DictionarySourceSettingsTests: XCTestCase {
    private var suiteName = ""
    private var userDefaults = UserDefaults.standard

    override func setUpWithError() throws {
        try super.setUpWithError()
        suiteName = "DictionarySourceSettingsTests.\(UUID().uuidString)"
        userDefaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
    }

    override func tearDown() {
        userDefaults.removePersistentDomain(forName: suiteName)
        super.tearDown()
    }

    private func makeStore() -> SettingsStore {
        SettingsStore(userDefaults: userDefaults)
    }

    /// Every key spelling, checked as a set against the iOS ones. A typo here
    /// is invisible at runtime — the store just reads the default forever.
    func testKeyNames_matchTheIosSpellings() {
        let expected: Set = [
            "customDictEnabled",
            "moeDictEnabled", "newwordDictEnabled", "iTaigiDictEnabled",
            "taiwanPlantDictEnabled", "taiHuaDictEnabled", "taiwanJapanDictEnabled",
            "kunggeDictEnabled", "sttiDictEnabled", "khpooDictEnabled",
            "variantEnabled", "khiin", "lkkDictEnabled", "devDictEnabled",
            "kautianAccentLukangEnabled", "kautianAccentSansiaEnabled",
            "kautianAccentTaipakEnabled", "kautianAccentGilanEnabled",
            "kautianAccentTainanEnabled", "kautianAccentKaohsiungEnabled",
            "kautianAccentKinmenEnabled", "kautianAccentMakungEnabled",
            "kautianAccentSintikEnabled", "kautianAccentTaichungEnabled",
            "kautianNameAppendixEnabled",
        ]

        let actual: Set<String> = [
            SettingsStore.Keys.isCustomDictEnabled.name,
            SettingsStore.Keys.isKautianEnabled.name,
            SettingsStore.Keys.isTaigitvEnabled.name,
            SettingsStore.Keys.isItaigiEnabled.name,
            SettingsStore.Keys.isSitbutEnabled.name,
            SettingsStore.Keys.isTaihoaEnabled.name,
            SettingsStore.Keys.isTaijitEnabled.name,
            SettingsStore.Keys.isKunggeEnabled.name,
            SettingsStore.Keys.isSttiEnabled.name,
            SettingsStore.Keys.isKhpooEnabled.name,
            SettingsStore.Keys.isVariantEnabled.name,
            SettingsStore.Keys.isKhiinEnabled.name,
            SettingsStore.Keys.isLkkEnabled.name,
            SettingsStore.Keys.isDevEnabled.name,
            SettingsStore.Keys.isKautianAccentLukangEnabled.name,
            SettingsStore.Keys.isKautianAccentSansiaEnabled.name,
            SettingsStore.Keys.isKautianAccentTaipakEnabled.name,
            SettingsStore.Keys.isKautianAccentGilanEnabled.name,
            SettingsStore.Keys.isKautianAccentTainanEnabled.name,
            SettingsStore.Keys.isKautianAccentKaohsiungEnabled.name,
            SettingsStore.Keys.isKautianAccentKinmenEnabled.name,
            SettingsStore.Keys.isKautianAccentMakungEnabled.name,
            SettingsStore.Keys.isKautianAccentSintikEnabled.name,
            SettingsStore.Keys.isKautianAccentTaichungEnabled.name,
            SettingsStore.Keys.isKautianNameAppendixEnabled.name,
        ]

        XCTAssertEqual(actual, expected)
    }

    /// `khiin` really is spelled without the suffix every other source key
    /// carries. Called out on its own so a future tidy-up cannot "fix" it into
    /// a key iOS does not write.
    func testKhiinKey_hasNoEnabledSuffix() {
        XCTAssertEqual(SettingsStore.Keys.isKhiinEnabled.name, "khiin")
    }

    func testFreshInstall_readsTheIosDefaultSourceSet() {
        let sources = makeStore().current.dictionarySources

        XCTAssertTrue(sources.kautian)
        XCTAssertTrue(sources.taigitv)
        XCTAssertTrue(sources.kungge)
        XCTAssertTrue(sources.stti)
        XCTAssertTrue(sources.khpoo)
        XCTAssertTrue(sources.lkk)
        XCTAssertTrue(sources.dev)
        XCTAssertFalse(sources.itaigi)
        XCTAssertFalse(sources.sitbut)
        XCTAssertFalse(sources.taihoa)
        XCTAssertFalse(sources.taijit)
        XCTAssertFalse(sources.variant)
        XCTAssertFalse(sources.khiin)
    }

    /// Every subcollection ships on: 腔調 are opt-out, not opt-in.
    func testFreshInstall_hasEveryKautianSubcollectionOn() {
        let subcollections = makeStore().current.dictionarySources.kautianSubcollections

        XCTAssertEqual(subcollections, .defaults)
        XCTAssertTrue(subcollections.accentLukang)
        XCTAssertTrue(subcollections.nameAppendix)
    }

    func testFreshInstall_hasTheCustomDictionaryOn() {
        XCTAssertTrue(makeStore().current.isCustomDictEnabled)
    }

    /// A default-on toggle switched off has to read as off. `UserDefaults`
    /// answers `false` for a key nobody wrote, so a store reading through
    /// `bool(forKey:)` could not tell "off" from "untouched".
    func testStoredOverrides_areRead() {
        userDefaults.set(false, forKey: SettingsStore.Keys.isKautianEnabled.name)
        userDefaults.set(true, forKey: SettingsStore.Keys.isKhiinEnabled.name)
        userDefaults.set(false, forKey: SettingsStore.Keys.isKautianAccentGilanEnabled.name)
        userDefaults.set(false, forKey: SettingsStore.Keys.isCustomDictEnabled.name)

        let settings = makeStore().current

        XCTAssertFalse(settings.dictionarySources.kautian)
        XCTAssertTrue(settings.dictionarySources.khiin)
        XCTAssertFalse(settings.dictionarySources.kautianSubcollections.accentGilan)
        XCTAssertFalse(settings.isCustomDictEnabled)
    }

    /// The store caches nothing: a toggle changed from the settings window (or
    /// `defaults write`) has to reach the very next keystroke.
    func testAToggleWrittenElsewhere_isSeenByTheNextRead() {
        let store = makeStore()
        XCTAssertTrue(store.current.dictionarySources.stti)

        userDefaults.set(false, forKey: SettingsStore.Keys.isSttiEnabled.name)

        XCTAssertFalse(store.current.dictionarySources.stti)
    }
}
