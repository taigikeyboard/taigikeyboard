@testable import TaigiInputMethodCore
import XCTest

/// The reactive display-language state the settings window and menus read. Every case runs against
/// its own `UserDefaults` suite and pins the device locale explicitly, so nothing here depends on the
/// language of the machine running the tests.
@MainActor
final class DisplayLanguageStoreTests: XCTestCase {
    private var suiteName = ""
    private var userDefaults = UserDefaults.standard

    override func setUpWithError() throws {
        try super.setUpWithError()
        suiteName = "DisplayLanguageStoreTests.\(UUID().uuidString)"
        userDefaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
    }

    override func tearDown() {
        userDefaults.removePersistentDomain(forName: suiteName)
        super.tearDown()
    }

    private func makeStore(deviceSubtag: String = "zh") -> DisplayLanguageStore {
        DisplayLanguageStore(
            settings: SettingsStore(userDefaults: userDefaults),
            deviceLanguageSubtag: { deviceSubtag },
        )
    }

    func testInit_withNothingStored_followsTheDeviceLocale() {
        // A fresh install is Automatic, so the effective language is whatever the device reports —
        // and the selection stays `.system` so the picker can show it as such.
        let store = makeStore(deviceSubtag: "ja")
        XCTAssertEqual(store.selected, .system)
        XCTAssertEqual(store.language, .japanese)
    }

    func testInit_withUnknownStoredTag_fallsBackToHanji() {
        userDefaults.set("xx", forKey: SettingsStore.Keys.displayLanguage.name)
        let store = makeStore(deviceSubtag: "en")
        XCTAssertEqual(store.selected, .hanji)
        XCTAssertEqual(store.language, .hanji)
    }

    func testSetLanguage_persistsToTheSameSuiteItReadsFrom() {
        let store = makeStore()
        store.setLanguage(.poj)

        XCTAssertEqual(store.selected, .poj)
        XCTAssertEqual(store.language, .poj)
        XCTAssertEqual(userDefaults.string(forKey: SettingsStore.Keys.displayLanguage.name), "poj")
    }

    func testSetLanguage_toSystem_persistsTheAutomaticTagAndResolvesTheDeviceLocale() {
        let store = makeStore(deviceSubtag: "en")
        store.setLanguage(.hanji)
        store.setLanguage(.system)

        XCTAssertEqual(userDefaults.string(forKey: SettingsStore.Keys.displayLanguage.name), "system")
        XCTAssertEqual(store.selected, .system)
        XCTAssertEqual(store.language, .english, "`.system` must resolve away before reaching the resolver")
    }

    /// Selecting the language Automatic already resolves to leaves the resolver alone but must still
    /// move the picker: `selected` and the effective language are separate pieces of state.
    func testSetLanguage_whenEffectiveLanguageIsUnchanged_stillUpdatesTheSelection() {
        let store = makeStore(deviceSubtag: "ja")
        XCTAssertEqual(store.selected, .system)
        XCTAssertEqual(store.language, .japanese)

        store.setLanguage(.japanese)

        XCTAssertEqual(store.selected, .japanese)
        XCTAssertEqual(store.language, .japanese)
    }

    /// A language written straight to the defaults domain rather than through `setLanguage` — the
    /// picker's own `@AppStorage` binding does exactly this — must still reach the store, because it
    /// observes the key instead of relying on every writer to route through it.
    ///
    /// The write here is in-process; the same observation is what carries an out-of-process one
    /// (`defaults write`), which a unit test cannot stage. Delivery is asynchronous: the observation
    /// fires on the writing thread and the store hops to the main actor before touching its state.
    func testStore_whenTheDefaultsKeyIsWrittenDirectly_picksUpTheNewLanguage() {
        let store = makeStore(deviceSubtag: "zh")
        XCTAssertEqual(store.language, .hanji)

        userDefaults.set(DisplayLanguage.tailo.tag, forKey: SettingsStore.Keys.displayLanguage.name)

        XCTAssertTrue(TestFixtures.spinRunLoop(until: { store.selected == .tailo }))
        XCTAssertEqual(store.language, .tailo)
    }

    func testSelectionLabel_usesEndonymsExceptForAutomatic() {
        let store = makeStore()
        store.setLanguage(.english)

        XCTAssertEqual(store.selectionLabel(for: .system), "Automatic")
        XCTAssertEqual(store.selectionLabel(for: .tailo), "Tâi-lô")
        XCTAssertEqual(store.selectionLabel(for: .japanese), "日本語")
    }

    func testString_readsUnderTheActiveLanguage() {
        let store = makeStore()
        store.setLanguage(.japanese)
        XCTAssertEqual(store.string(.settingsDisplayLanguage), "表示言語")

        store.setLanguage(.english)
        XCTAssertEqual(store.string(.settingsDisplayLanguage), "Display Language")
    }
}
