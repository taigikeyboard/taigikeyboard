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

    /// Presentation-only like `displayLanguage`: a fresh install shows the
    /// expandable window — MacishType's own default — and a stored value from
    /// a build that removed a case, or a hand-edited `defaults write`, reads
    /// as that default rather than as a layout the router cannot build.
    func testCandidateLayout_withNothingStored_isExpandable() {
        XCTAssertEqual(makeStore().candidateLayout, .expandable)
    }

    func testCandidateLayout_readsWhatTheSettingsFormWrites() {
        // The form writes through `@AppStorage`, which stores the raw string.
        userDefaults.set(
            CandidateLayout.vertical.rawValue,
            forKey: SettingsStore.Keys.candidateLayout.name,
        )
        XCTAssertEqual(makeStore().candidateLayout, .vertical)
    }

    func testCandidateLayout_withAnUnknownStoredValue_fallsBackToTheDefault() {
        userDefaults.set("diagonal", forKey: SettingsStore.Keys.candidateLayout.name)
        XCTAssertEqual(makeStore().candidateLayout, .expandable)
    }

    /// The 外觀 row's contract: 自動 forces nothing (the panel resolves
    /// against the system), and the two explicit modes force the matching
    /// appearance — a mode that resolved to nil would silently behave as 自動.
    func testAppearanceMode_withNothingStored_followsTheSystem() {
        XCTAssertEqual(makeStore().appearanceMode, .auto)
        XCTAssertNil(AppearanceMode.auto.forcedAppearance)
        XCTAssertEqual(AppearanceMode.light.forcedAppearance?.name, .aqua)
        XCTAssertEqual(AppearanceMode.dark.forcedAppearance?.name, .darkAqua)
    }

    func testAppearanceMode_readsWhatTheThumbnailsWrite() {
        userDefaults.set(
            AppearanceMode.dark.rawValue,
            forKey: SettingsStore.Keys.appearanceMode.name,
        )
        XCTAssertEqual(makeStore().appearanceMode, .dark)

        userDefaults.set("sepia", forKey: SettingsStore.Keys.appearanceMode.name)
        XCTAssertEqual(makeStore().appearanceMode, .auto, "unknown values fall back to 自動")
    }

    /// The two size rows default one step above the metrics the window
    /// originally rendered at (USER 2026-08-21), so an install that never
    /// touched them gets the larger window — 細 is the way back.
    func testCandidateSizes_withNothingStored_areTheEnlargedDefaults() {
        let store = makeStore()

        XCTAssertEqual(store.candidateTextSize, .medium)
        XCTAssertEqual(store.candidateWindowSize, .medium)
        XCTAssertEqual(
            store.candidateMetrics,
            CandidateMetrics(textSize: .medium, windowSize: .medium),
        )
    }

    func testCandidateSizes_readWhatTheSizePickersWrite() {
        userDefaults.set(
            CandidateTextSizeChoice.large.rawValue,
            forKey: SettingsStore.Keys.candidateTextSize.name,
        )
        userDefaults.set(
            CandidateWindowSizeChoice.small.rawValue,
            forKey: SettingsStore.Keys.candidateWindowSize.name,
        )
        XCTAssertEqual(makeStore().candidateTextSize, .large)
        XCTAssertEqual(makeStore().candidateWindowSize, .small)
    }

    /// The 特大 tier was removed (USER 2026-08-21): an install that stored it
    /// reads back as the default rather than crashing or pinning a ghost size.
    func testCandidateTextSize_storedRetiredExtraLarge_fallsBackToTheDefault() {
        userDefaults.set("extraLarge", forKey: SettingsStore.Keys.candidateTextSize.name)

        XCTAssertEqual(makeStore().candidateTextSize, .medium)
    }

    func testCandidateSizes_withUnknownStoredValues_fallBackToTheDefaults() {
        userDefaults.set("gigantic", forKey: SettingsStore.Keys.candidateTextSize.name)
        userDefaults.set("gigantic", forKey: SettingsStore.Keys.candidateWindowSize.name)

        XCTAssertEqual(makeStore().candidateTextSize, .medium)
        XCTAssertEqual(makeStore().candidateWindowSize, .medium)
    }

    /// A fresh Mac renders in the system font (USER 2026-08-23) — the key
    /// spelling is iOS's, the default is not.
    func testCandidateFont_withNothingStored_isTheSystemFont() {
        let store = makeStore()

        XCTAssertEqual(store.candidateFontChoice, .system)
        XCTAssertEqual(store.candidateMetrics.fontChoice, .system)
    }

    func testCandidateFont_readsWhatTheFontPickerWrites() {
        userDefaults.set(
            CandidateFontChoice.openHuninn.rawValue,
            forKey: SettingsStore.Keys.fontType.name,
        )
        let store = makeStore()

        XCTAssertEqual(store.candidateFontChoice, .openHuninn)
        XCTAssertEqual(store.candidateMetrics.fontChoice, .openHuninn)
    }

    /// A face a later version drops — or an iOS value this build does not name
    /// — reads back as the system font rather than leaving the window with a
    /// typeface nothing can resolve.
    func testCandidateFont_withAnUnknownStoredValue_fallsBackToTheSystemFont() {
        userDefaults.set("comicSans", forKey: SettingsStore.Keys.fontType.name)

        XCTAssertEqual(makeStore().candidateFontChoice, .system)
    }

    // MARK: - Composing key bindings

    func testComposingKeyBindings_withNothingStored_areTheShippedContract() {
        XCTAssertEqual(makeStore().composingKeyBindings, .default)
    }

    func testComposingChords_roundTripThroughTheSuite() throws {
        let store = makeStore()
        // A chord no default holds, so the resolver has no duplicate to drop
        // and the round trip is the only thing under test.
        let chord = try TestFixtures.chordNoDefaultHolds()

        store.setComposingChord(chord, for: .commitHanji)

        XCTAssertEqual(makeStore().composingKeyBindings.chord(for: .commitHanji), chord)
        XCTAssertEqual(
            userDefaults.string(forKey: ComposingAction.commitHanji.settingsKeyName),
            chord.rawValue,
            "the stored form is what a later build has to keep reading",
        )
    }

    /// The request that freed the eight non-syllable letters: 直接輸出漢字 on a
    /// bare `z` must survive a relaunch, stored in the same raw form every
    /// modifier chord uses.
    func testABareNonSyllableLetter_roundTripsThroughTheSuite() throws {
        let store = makeStore()
        let bareZ = try ComposingKeyChord.make(key: "z", modifiers: []).get()

        store.setComposingChord(bareZ, for: .commitHanji)

        XCTAssertEqual(makeStore().composingKeyBindings.chord(for: .commitHanji), bareZ)
        XCTAssertEqual(
            userDefaults.string(forKey: ComposingAction.commitHanji.settingsKeyName),
            "|007A",
        )
    }

    /// Clearing a row is a stored empty string, not an absent key: an absent
    /// key means "never touched" and reads as the action's default, so the two
    /// cannot be collapsed without undoing the user's clearing on next launch.
    func testClearedComposingChord_staysClearedAcrossReads() {
        let store = makeStore()

        store.setComposingChord(nil, for: .pageForward)

        XCTAssertNil(makeStore().composingKeyBindings.chord(for: .pageForward))
        XCTAssertEqual(
            userDefaults.string(forKey: ComposingAction.pageForward.settingsKeyName),
            "",
        )
    }

    /// A stored chord the build cannot parse reads as an empty row rather than
    /// as the default: restoring the default would undo a deliberate clearing,
    /// and anything that must stay reachable is put back by the resolver.
    func testComposingChords_withAnUnparsableStoredValue_readAsCleared() {
        userDefaults.set("nonsense", forKey: ComposingAction.pageForward.settingsKeyName)

        XCTAssertNil(makeStore().composingKeyBindings.chord(for: .pageForward))
    }

    func testCandidateSlotModifier_withAnUnknownStoredValue_fallsBackToControl() {
        userDefaults.set("nonsense", forKey: SettingsStore.Keys.candidateSlotModifier.name)

        XCTAssertEqual(makeStore().composingKeyBindings.slotModifier, .control)
    }
}
