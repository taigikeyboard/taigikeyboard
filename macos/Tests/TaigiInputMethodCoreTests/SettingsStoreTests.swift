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
        userDefaults.set(false, forKey: SettingsStore.Keys.isFrequencyRecordingEnabled.name)
        userDefaults.set(false, forKey: SettingsStore.Keys.isAssociationRecordingEnabled.name)
        // §34/S22 — the default moved ON on 2026-09-03, so a stored `false`
        // has to keep winning: someone who turned 顯示當咧拍的字 off stays off.
        userDefaults.set(false, forKey: SettingsStore.Keys.isLiteralRomanCandidateEnabled.name)

        XCTAssertEqual(
            makeStore().current,
            EngineSettings(
                inputMode: .poj,
                isTranslateSwapped: true,
                isOutputBothScripts: true,
                candidateDisplayMode: .sideBySide,
                isLiteralRomanCandidateEnabled: false,
                isFrequencyRecordingEnabled: false,
                isAssociationRecordingEnabled: false,
                isCustomDictEnabled: EngineSettings.defaults.isCustomDictEnabled,
                dictionarySources: EngineSettings.defaults.dictionarySources,
            ),
        )
    }

    // MARK: - Candidate display mode

    /// The one platform-side rule of the romanization-only display: the
    /// engine and every gate read a swap pair that is `(false, false)` under
    /// it — there is no Hanji to lead with or to bracket — while what the user
    /// STORED stays put, so leaving the mode gives their swap straight back.
    func testCurrent_underRomanOnly_derivesTheSwapPairFalse_andLeavesTheStoredValuesAlone() {
        let store = makeStore()
        store.storedIsTranslateSwapped = true
        store.storedIsOutputBothScripts = true

        store.candidateDisplayMode = .romanOnly

        XCTAssertEqual(store.current.candidateDisplayMode, .romanOnly)
        XCTAssertFalse(store.current.isTranslateSwapped)
        XCTAssertFalse(store.current.isOutputBothScripts)
        XCTAssertTrue(store.storedIsTranslateSwapped, "the stored swap must survive the mode")
        XCTAssertTrue(store.storedIsOutputBothScripts, "the stored bracket setting must survive the mode")
        XCTAssertEqual(userDefaults.object(forKey: SettingsStore.Keys.isTranslateSwapped.name) as? Bool, true)
        XCTAssertEqual(userDefaults.object(forKey: SettingsStore.Keys.isOutputBothScripts.name) as? Bool, true)

        store.candidateDisplayMode = .sideBySide

        XCTAssertTrue(store.current.isTranslateSwapped, "side by side must read the stored swap again")
        XCTAssertTrue(store.current.isOutputBothScripts)
    }

    /// The one platform-side rule of the combined display: the swap reads
    /// `true` whatever is stored — the Hanji cell comes first and its commit
    /// writes the Hanji — while the bracket setting is read as stored, so
    /// 括號標註 still commits `漢字 (羅馬字)`. The stored swap survives the
    /// mode, so leaving it gives the user their own swap straight back.
    func testCurrent_underCombined_forcesTheSwapOn_readsTheBracketAsStored_andLeavesTheStoredValuesAlone() {
        let store = makeStore()
        store.storedIsTranslateSwapped = false
        store.storedIsOutputBothScripts = false

        store.candidateDisplayMode = .combined

        XCTAssertEqual(store.current.candidateDisplayMode, .combined)
        XCTAssertTrue(store.current.isTranslateSwapped, "combined must lead with — and commit — the Hanji")
        XCTAssertFalse(store.current.isOutputBothScripts, "combined must not invent a bracket setting")
        XCTAssertFalse(store.storedIsTranslateSwapped, "the stored swap must survive the mode")

        store.storedIsOutputBothScripts = true

        XCTAssertTrue(store.current.isTranslateSwapped)
        XCTAssertTrue(store.current.isOutputBothScripts, "括號標註 stays in force under combined")

        store.candidateDisplayMode = .romanOnly

        XCTAssertFalse(store.current.isTranslateSwapped, "romanization-only is unchanged by the third mode")
        XCTAssertFalse(store.current.isOutputBothScripts)

        store.candidateDisplayMode = .sideBySide

        XCTAssertFalse(store.current.isTranslateSwapped, "side by side must read the stored swap again")
        XCTAssertTrue(store.current.isOutputBothScripts)
    }

    func testCandidateDisplayMode_withNothingStored_isSideBySide() {
        XCTAssertEqual(makeStore().candidateDisplayMode, .sideBySide)
        XCTAssertEqual(makeStore().current.candidateDisplayMode, .sideBySide)
    }

    func testCandidateDisplayMode_readsWhatTheSettingsFormWrites() {
        // The form writes through `@AppStorage`, which stores the raw string.
        userDefaults.set(
            CandidateDisplayMode.romanOnly.rawValue,
            forKey: SettingsStore.Keys.candidateDisplayMode.name,
        )

        XCTAssertEqual(makeStore().candidateDisplayMode, .romanOnly)

        userDefaults.set(
            CandidateDisplayMode.combined.rawValue,
            forKey: SettingsStore.Keys.candidateDisplayMode.name,
        )

        XCTAssertEqual(makeStore().candidateDisplayMode, .combined)
        XCTAssertEqual(makeStore().current.candidateDisplayMode, .combined)
    }

    /// A hand-edited `defaults write`, or a mode a later version adds and an
    /// older build reads, must not leave the window with cells it cannot draw.
    func testCandidateDisplayMode_withAnUnknownStoredValue_fallsBackToSideBySide() {
        userDefaults.set("stacked", forKey: SettingsStore.Keys.candidateDisplayMode.name)

        XCTAssertEqual(makeStore().candidateDisplayMode, .sideBySide)
        XCTAssertEqual(makeStore().current.candidateDisplayMode, .sideBySide)
    }

    /// Storage contract shared by all four platforms (research doc §12): the
    /// key and all three raw values are spelled the same everywhere, so a future
    /// settings transfer carries one vocabulary.
    func testCandidateDisplayMode_storageSpellings_matchTheOtherPlatforms() {
        XCTAssertEqual(SettingsStore.Keys.candidateDisplayMode.name, "candidateDisplayMode")
        XCTAssertEqual(CandidateDisplayMode.sideBySide.rawValue, "sideBySide")
        XCTAssertEqual(CandidateDisplayMode.combined.rawValue, "combined")
        XCTAssertEqual(CandidateDisplayMode.romanOnly.rawValue, "romanOnly")
    }

    /// What the cycle shortcut steps through: the picker's order, and back to
    /// the start after as many presses as there are modes.
    func testCandidateDisplayMode_next_walksThePickerOrderAndComesBackRound() {
        XCTAssertEqual(CandidateDisplayMode.sideBySide.next, .combined)
        XCTAssertEqual(CandidateDisplayMode.combined.next, .romanOnly)
        XCTAssertEqual(CandidateDisplayMode.romanOnly.next, .sideBySide)
        XCTAssertEqual(CandidateDisplayMode.sideBySide.next.next.next, .sideBySide)
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
    /// expandable window (USER 2026-08-28; vertical between 2026-08-26 and
    /// then), and a stored value from a build that removed a case, or a
    /// hand-edited `defaults write`, reads as that default rather than as a
    /// layout the router cannot build.
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

    /// The 外觀 pane's reset button, which has to reach every key that pane
    /// owns — a button that restored some of them would leave rows it visibly
    /// did not touch.
    @MainActor
    func testResetAppearanceSettings_putsEveryRowBack() {
        let store = makeStore()
        userDefaults.set(AppearanceMode.dark.rawValue, forKey: SettingsStore.Keys.appearanceMode.name)
        userDefaults.set(CandidateLayout.horizontal.rawValue, forKey: SettingsStore.Keys.candidateLayout.name)
        userDefaults.set(
            CandidateWindowSizeChoice.large.rawValue,
            forKey: SettingsStore.Keys.candidateWindowSize.name,
        )
        userDefaults.set(CandidateTextSizeChoice.small.rawValue, forKey: SettingsStore.Keys.candidateTextSize.name)
        userDefaults.set(
            CandidateDisplayMode.romanOnly.rawValue,
            forKey: SettingsStore.Keys.candidateDisplayMode.name,
        )

        store.resetAppearanceSettings()

        XCTAssertEqual(store.appearanceMode, SettingsStore.Keys.appearanceMode.defaultValue)
        XCTAssertEqual(store.candidateLayout, SettingsStore.Keys.candidateLayout.defaultValue)
        XCTAssertEqual(store.candidateDisplayMode, SettingsStore.Keys.candidateDisplayMode.defaultValue)
        XCTAssertEqual(store.candidateWindowSize, SettingsStore.Keys.candidateWindowSize.defaultValue)
        XCTAssertEqual(store.candidateTextSize, SettingsStore.Keys.candidateTextSize.defaultValue)
    }

    /// The typeface is 字型管理's, not 外觀's: a pane's reset restores the rows
    /// that pane shows, and 外觀 no longer shows the typeface.
    @MainActor
    func testResetAppearanceSettings_leavesTheTypefaceAlone() {
        userDefaults.set(CandidateFontChoice.genYoMin.rawValue, forKey: SettingsStore.Keys.fontType.name)

        makeStore().resetAppearanceSettings()

        XCTAssertEqual(makeStore().candidateFontSelection, .builtIn(.genYoMin))
    }

    /// 字型管理's own reset, which owns BOTH halves of the selection: `fontType`
    /// alone would leave a file name pointing at nothing.
    @MainActor
    func testResetFontSettings_clearsBothHalvesOfTheSelection() {
        userDefaults.set(CandidateFontSelection.customRawValue, forKey: SettingsStore.Keys.fontType.name)
        userDefaults.set("something.ttf", forKey: SettingsStore.Keys.customFontFile.name)

        makeStore().resetFontSettings()

        XCTAssertNil(userDefaults.string(forKey: SettingsStore.Keys.fontType.name))
        XCTAssertNil(userDefaults.string(forKey: SettingsStore.Keys.customFontFile.name))
        XCTAssertEqual(makeStore().candidateFontSelection, .builtIn(.system))
    }

    /// Removed, not written over — the rule `resetComposingShortcuts` states:
    /// a stored default is indistinguishable from a value the user chose, and
    /// would pin this version's default onto an install a later version means
    /// to move.
    func testResetAppearanceSettings_leavesNothingStored() {
        userDefaults.set(CandidateLayout.horizontal.rawValue, forKey: SettingsStore.Keys.candidateLayout.name)

        makeStore().resetAppearanceSettings()

        XCTAssertNil(userDefaults.object(forKey: SettingsStore.Keys.candidateLayout.name))
    }

    /// The 辭典管理 pane's reset button, which owns the master sources and the
    /// 腔口 subcollections alike: a 腔口 left switched off would be a source
    /// the user had visibly restored still missing candidates.
    func testResetDictionarySources_putsEverySourceAndAccentBack() {
        let store = makeStore()
        userDefaults.set(false, forKey: SettingsStore.Keys.isKautianEnabled.name)
        userDefaults.set(false, forKey: SettingsStore.Keys.isKautianAccentTainanEnabled.name)
        userDefaults.set(true, forKey: SettingsStore.Keys.isDevEnabled.name)

        store.resetDictionarySources()

        XCTAssertEqual(store.current.dictionarySources, EngineSettings.defaults.dictionarySources)
        XCTAssertNil(userDefaults.object(forKey: SettingsStore.Keys.isKautianEnabled.name))
        XCTAssertNil(userDefaults.object(forKey: SettingsStore.Keys.isKautianAccentTainanEnabled.name))
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
    @MainActor
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
    @MainActor
    func testCandidateFont_withNothingStored_isTheSystemFont() {
        let store = makeStore()

        XCTAssertEqual(store.candidateFontSelection, .builtIn(.system))
        XCTAssertEqual(store.candidateMetrics.fontSelection, .builtIn(.system))
    }

    @MainActor
    func testCandidateFont_readsWhatTheFontPickerWrites() {
        userDefaults.set(
            CandidateFontChoice.openHuninn.rawValue,
            forKey: SettingsStore.Keys.fontType.name,
        )
        let store = makeStore()

        XCTAssertEqual(store.candidateFontSelection, .builtIn(.openHuninn))
        XCTAssertEqual(store.candidateMetrics.fontSelection, .builtIn(.openHuninn))
    }

    /// A face a later version drops — or an iOS value this build does not name
    /// — reads back as the system font rather than leaving the window with a
    /// typeface nothing can resolve.
    @MainActor
    func testCandidateFont_withAnUnknownStoredValue_fallsBackToTheSystemFont() {
        userDefaults.set("comicSans", forKey: SettingsStore.Keys.fontType.name)

        XCTAssertEqual(makeStore().candidateFontSelection, .builtIn(.system))
    }

    /// `custom` is deliberately not a `CandidateFontChoice` raw value, so the
    /// roster's own unknown-value fallback answers for it: an install whose
    /// typeface file is gone renders in the system font.
    @MainActor
    func testStoredCustom_withNoSuchFile_readsAsTheSystemFont() {
        userDefaults.set(CandidateFontSelection.customRawValue, forKey: SettingsStore.Keys.fontType.name)
        userDefaults.set("gone.ttf", forKey: SettingsStore.Keys.customFontFile.name)

        XCTAssertEqual(makeStore().candidateFontSelection, .builtIn(.system))
    }

    /// Reading must not rewrite: the file may be back — an external volume, a
    /// restore — before the user next opens the pane, and a preference silently
    /// replaced by the system font is a choice they never made.
    @MainActor
    func testStoredCustom_withNoSuchFile_leavesThePreferenceAlone() {
        userDefaults.set(CandidateFontSelection.customRawValue, forKey: SettingsStore.Keys.fontType.name)
        userDefaults.set("gone.ttf", forKey: SettingsStore.Keys.customFontFile.name)

        _ = makeStore().candidateFontSelection

        XCTAssertEqual(
            userDefaults.string(forKey: SettingsStore.Keys.fontType.name),
            CandidateFontSelection.customRawValue,
        )
        XCTAssertEqual(userDefaults.string(forKey: SettingsStore.Keys.customFontFile.name), "gone.ttf")
    }

    /// Two keys are two writes, so every combination of them has to resolve to
    /// something drawable — including the half-written one.
    @MainActor
    func testStoredCustom_withNoFileNameBesideIt_readsAsTheSystemFont() {
        userDefaults.set(CandidateFontSelection.customRawValue, forKey: SettingsStore.Keys.fontType.name)

        XCTAssertEqual(makeStore().candidateFontSelection, .builtIn(.system))
    }

    /// A file name left over from a custom selection does not make a built-in
    /// one custom.
    @MainActor
    func testStoredBuiltIn_withAStaleFileNameBesideIt_readsAsTheBuiltIn() {
        userDefaults.set(CandidateFontChoice.genYoMin.rawValue, forKey: SettingsStore.Keys.fontType.name)
        userDefaults.set("left-over.ttf", forKey: SettingsStore.Keys.customFontFile.name)

        XCTAssertEqual(makeStore().candidateFontSelection, .builtIn(.genYoMin))
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

        store.setComposingChord(chord, for: .pageForward)

        XCTAssertEqual(makeStore().composingKeyBindings.chord(for: .pageForward), chord)
        XCTAssertEqual(
            userDefaults.string(forKey: ComposingAction.pageForward.settingsKeyName),
            chord.rawValue,
            "the stored form is what a later build has to keep reading",
        )
    }

    /// The request that freed the eight non-syllable letters: an action on a
    /// bare `z` must survive a relaunch, stored in the same raw form every
    /// modifier chord uses.
    func testABareNonSyllableLetter_roundTripsThroughTheSuite() throws {
        let store = makeStore()
        // Under the digits: the shipped slot key set holds every free letter,
        // and a read through it resolves the row empty.
        userDefaults.set(CandidateSlotKeySet.control.rawValue, forKey: SettingsStore.Keys.candidateSlotModifier.name)
        let bareZ = try ComposingKeyChord.make(key: "z", modifiers: []).get()

        store.setComposingChord(bareZ, for: .pageBackward)

        XCTAssertEqual(makeStore().composingKeyBindings.chord(for: .pageBackward), bareZ)
        XCTAssertEqual(
            userDefaults.string(forKey: ComposingAction.pageBackward.settingsKeyName),
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

    func testCandidateSlotKeySet_withAnUnknownStoredValue_fallsBackToTheLetters() {
        userDefaults.set("nonsense", forKey: SettingsStore.Keys.candidateSlotModifier.name)

        XCTAssertEqual(makeStore().composingKeyBindings.slotKeySet, .bareKeys)
    }

    /// The rules `current` and the swap shortcut read live on the enum — pinned once.
    func testCandidateDisplayMode_rules_perMode() {
        XCTAssertEqual(CandidateDisplayMode.allCases.filter(\.allowsSwapToggle), [.sideBySide])
        XCTAssertEqual(CandidateDisplayMode.allCases.filter { !$0.showsHanji }, [.romanOnly])
        XCTAssertTrue(CandidateDisplayMode.combined.effectiveTranslateSwapped(stored: false))
        XCTAssertFalse(CandidateDisplayMode.romanOnly.effectiveTranslateSwapped(stored: true))
        XCTAssertFalse(CandidateDisplayMode.romanOnly.effectiveOutputBothScripts(stored: true))
        XCTAssertTrue(CandidateDisplayMode.combined.effectiveOutputBothScripts(stored: true))
    }
}
