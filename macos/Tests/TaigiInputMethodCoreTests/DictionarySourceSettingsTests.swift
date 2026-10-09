// The 25 dictionary toggles: their stored names and their fresh-install
// values.

@testable import TaigiInputMethodCore
import XCTest

/// The keys and defaults are copied from iOS on purpose, so the same
/// `defaults read` answers the same way on both platforms and a settings
/// transfer would have one name per setting rather than two.
final class DictionarySourceSettingsTests: XCTestCase {
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
            "kautianNameAppendixEnabled", "kautianAltReadingEnabled",
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
            SettingsStore.Keys.isKautianAltReadingEnabled.name,
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
        typealias Keys = SettingsStore.Keys
        let on = [
            Keys.isKautianEnabled, Keys.isTaigitvEnabled, Keys.isKunggeEnabled, Keys.isSttiEnabled,
            Keys.isKhpooEnabled, Keys.isLkkEnabled, Keys.isDevEnabled,
        ]
        let off = [
            Keys.isItaigiEnabled, Keys.isSitbutEnabled, Keys.isTaihoaEnabled, Keys.isTaijitEnabled,
            Keys.isVariantEnabled, Keys.isKhiinEnabled,
        ]

        for key in on {
            XCTAssertTrue(key.defaultValue, key.name)
        }
        for key in off {
            XCTAssertFalse(key.defaultValue, key.name)
        }
    }

    /// Every subcollection ships on: accents are opt-out, not opt-in.
    func testFreshInstall_hasEveryKautianSubcollectionOn() {
        typealias Keys = SettingsStore.Keys
        let subcollections = [
            Keys.isKautianAccentLukangEnabled, Keys.isKautianAccentSansiaEnabled,
            Keys.isKautianAccentTaipakEnabled, Keys.isKautianAccentGilanEnabled,
            Keys.isKautianAccentTainanEnabled, Keys.isKautianAccentKaohsiungEnabled,
            Keys.isKautianAccentKinmenEnabled, Keys.isKautianAccentMakungEnabled,
            Keys.isKautianAccentSintikEnabled, Keys.isKautianAccentTaichungEnabled,
            Keys.isKautianNameAppendixEnabled, Keys.isKautianAltReadingEnabled,
        ]

        for key in subcollections {
            XCTAssertTrue(key.defaultValue, key.name)
        }
    }

    func testFreshInstall_hasTheCustomDictionaryOn() {
        XCTAssertTrue(SettingsStore.Keys.isCustomDictEnabled.defaultValue)
    }
}
