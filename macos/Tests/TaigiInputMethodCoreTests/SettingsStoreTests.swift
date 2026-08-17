@testable import TaigiInputMethodCore
import XCTest

/// The persistence layer the engine and the settings form share. Every case
/// runs against its own `UserDefaults` suite, so nothing here can see — or
/// disturb — the settings of the machine running the tests.
final class SettingsStoreTests: XCTestCase {
    private var suiteName = ""
    private var userDefaults = UserDefaults.standard

    override func setUpWithError() throws {
        try super.setUpWithError()
        suiteName = "SettingsStoreTests.\(UUID().uuidString)"
        userDefaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
    }

    override func tearDown() {
        userDefaults.removePersistentDomain(forName: suiteName)
        super.tearDown()
    }

    private func makeStore() -> SettingsStore {
        SettingsStore(userDefaults: userDefaults)
    }

    /// The single most important property of the whole file: what a fresh
    /// install types with is what `EngineSettings.defaults` says, which is
    /// itself kept aligned with iOS and Android. A store that answered its own
    /// defaults would let a macOS install drift away from the other two.
    ///
    /// Also the regression test for reading a `Bool` through
    /// `UserDefaults.bool(forKey:)`, which answers `false` for an absent key:
    /// the settings that ship ON — the two learning switches — would be off for
    /// every new user, and only this case would notice.
    func testCurrent_withNothingStored_matchesTheShippedDefaults() {
        XCTAssertEqual(makeStore().current, EngineSettings.defaults)
    }

    func testCurrent_readsEveryStoredValue() {
        userDefaults.set(InputMode.poj.rawValue, forKey: SettingsStore.Keys.inputMode.name)
        userDefaults.set(true, forKey: SettingsStore.Keys.isTranslateSwapped.name)
        userDefaults.set(true, forKey: SettingsStore.Keys.isOutputBothScripts.name)
        userDefaults.set(true, forKey: SettingsStore.Keys.isLiteralRomanCandidateEnabled.name)
        userDefaults.set(false, forKey: SettingsStore.Keys.isFrequencyRecordingEnabled.name)
        userDefaults.set(false, forKey: SettingsStore.Keys.isAssociationRecordingEnabled.name)

        XCTAssertEqual(
            makeStore().current,
            EngineSettings(
                inputMode: .poj,
                isTranslateSwapped: true,
                isOutputBothScripts: true,
                isLiteralRomanCandidateEnabled: true,
                isFrequencyRecordingEnabled: false,
                isAssociationRecordingEnabled: false,
                isCustomDictEnabled: EngineSettings.defaults.isCustomDictEnabled,
                dictionarySources: EngineSettings.defaults.dictionarySources,
            ),
        )
    }

    /// The store is read on every operation rather than snapshotted, so a mode
    /// changed from the menu applies to the next keystroke. A cached read would
    /// pass every other case in this file and fail only here.
    ///
    /// The write goes through a SECOND store rather than through the one being
    /// read: the input-source menu and the composing engine each hold their own
    /// instance, so a store that cached `current` and refreshed the cache in its
    /// own setter would still leave the engine composing under the old mode —
    /// and a single-instance write/read would not notice.
    func testCurrent_isReadLive_evenForAChangeMadeThroughAnotherStore() {
        let engineStore = makeStore()
        XCTAssertEqual(engineStore.current.inputMode, .tl)

        makeStore().inputMode = .poj

        XCTAssertEqual(engineStore.current.inputMode, .poj)
    }

    /// The same property for a setting nothing else in the process writes: a
    /// value changed underneath the store — `defaults write`, or a settings
    /// window bound straight to `UserDefaults` through `@AppStorage` — has to
    /// reach the engine without the store being told.
    func testCurrent_isReadLive_forAValueWrittenBehindTheStore() {
        let store = makeStore()
        XCTAssertFalse(store.current.isOutputBothScripts)

        userDefaults.set(true, forKey: SettingsStore.Keys.isOutputBothScripts.name)

        XCTAssertTrue(store.current.isOutputBothScripts)
    }

    func testInputMode_writesTheRawValueOthersCanRead() {
        makeStore().inputMode = .poj

        XCTAssertEqual(
            userDefaults.string(forKey: SettingsStore.Keys.inputMode.name),
            InputMode.poj.rawValue,
        )
    }

    /// A hand-written `defaults write`, or a mode a later version drops, must
    /// not leave the engine with a romanization it cannot render.
    func testInputMode_withAnUnknownStoredValue_fallsBackToTheDefault() {
        userDefaults.set("bopomofo", forKey: SettingsStore.Keys.inputMode.name)

        XCTAssertEqual(makeStore().inputMode, .tl)
    }

    /// Key spellings are the iOS ones on purpose
    /// (`ios/Sources/TaigiKeyboard/Settings/SharedSettings.swift:36-50`), so
    /// both platforms name the same setting the same way. Pinned because the
    /// alignment is invisible from this file alone.
    func testKeys_matchTheIOSSpellings() {
        XCTAssertEqual(SettingsStore.Keys.inputMode.name, "inputMode")
        XCTAssertEqual(SettingsStore.Keys.isTranslateSwapped.name, "isTranslateSwapped")
        XCTAssertEqual(SettingsStore.Keys.isOutputBothScripts.name, "outputBothScripts")
        XCTAssertEqual(
            SettingsStore.Keys.isLiteralRomanCandidateEnabled.name,
            "literalRomanCandidateEnabled",
        )
        XCTAssertEqual(SettingsStore.Keys.displayLanguage.name, "displayLanguage")
    }

    /// The app UI language is the one setting whose default comes from the
    /// display-language roster rather than `EngineSettings.defaults` — the
    /// engine never reads it. A fresh install is Automatic.
    func testDisplayLanguage_withNothingStored_isTheAutomaticTag() {
        XCTAssertEqual(makeStore().displayLanguage, "system")
    }

    func testDisplayLanguage_roundTripsThroughTheSuite() {
        let store = makeStore()
        store.displayLanguage = DisplayLanguage.poj.tag

        XCTAssertEqual(store.displayLanguage, "poj")
        XCTAssertEqual(userDefaults.string(forKey: SettingsStore.Keys.displayLanguage.name), "poj")
    }
}
