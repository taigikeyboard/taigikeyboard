// Where the user settings are persisted, and where every reader of them —
// the engine, the settings form, the input-source menu — agrees on the keys.

import Foundation

/// The `UserDefaults` key of one setting, paired with the value used when the
/// user has never touched it.
///
/// The default is not written here: it is read from `EngineSettings.defaults`,
/// which is the domain model's own statement of what a fresh install types
/// with and is kept aligned with iOS and Android on purpose. A literal repeated
/// here would be a second place for that alignment to drift.
struct SettingsKey<Value: Sendable>: Sendable {
    let name: String
    let defaultValue: Value
}

/// Reads and writes the settings the composing engine is driven by.
///
/// Live-read by construction: `current` goes to `UserDefaults` on every access
/// and caches nothing, so a mode switched from the input-source menu applies to
/// the very next keystroke without anything having to be told about it
/// (`.claude/rules/ios-settings-injection.md` §3 — a snapshot taken at
/// construction is exactly what breaks mid-session TL↔POJ switching).
///
/// Instances cache nothing, so there is no shared one: the input-source menu
/// and the composition root each hold their own against the same defaults
/// domain, and a mode written through either is read by both.
///
/// `@unchecked Sendable` because `UserDefaults` carries no `Sendable`
/// annotation in Foundation while Apple documents the class itself as
/// thread-safe. Nothing else is stored here, so that one reference is the whole
/// of what the unchecked promise covers.
final class SettingsStore: EngineSettingsProvider, @unchecked Sendable {
    /// Key spellings are the iOS ones (`ios/Sources/TaigiKeyboard/Settings/SharedSettings.swift:36-50`).
    /// macOS has its own defaults domain, so this is not shared storage and
    /// carries no binary-compatibility obligation — it is a deliberate
    /// cross-platform schema alignment, so that `defaults read` speaks the same
    /// vocabulary on both platforms and a future settings-transfer feature has
    /// one name per setting rather than two.
    enum Keys {
        static let inputMode = SettingsKey(
            name: "inputMode",
            defaultValue: EngineSettings.defaults.inputMode,
        )
        static let isTranslateSwapped = SettingsKey(
            name: "isTranslateSwapped",
            defaultValue: EngineSettings.defaults.isTranslateSwapped,
        )
        static let isOutputBothScripts = SettingsKey(
            name: "outputBothScripts",
            defaultValue: EngineSettings.defaults.isOutputBothScripts,
        )
        static let isLiteralRomanCandidateEnabled = SettingsKey(
            name: "literalRomanCandidateEnabled",
            defaultValue: EngineSettings.defaults.isLiteralRomanCandidateEnabled,
        )
        static let isFrequencyRecordingEnabled = SettingsKey(
            name: "frequencyRecordingEnabled",
            defaultValue: EngineSettings.defaults.isFrequencyRecordingEnabled,
        )
        static let isAssociationRecordingEnabled = SettingsKey(
            name: "associationRecordingEnabled",
            defaultValue: EngineSettings.defaults.isAssociationRecordingEnabled,
        )
        static let isCustomDictEnabled = SettingsKey(
            name: "customDictEnabled",
            defaultValue: EngineSettings.defaults.isCustomDictEnabled,
        )

        /// Auto-insert a trailing space after committing a word. Platform-side
        /// on every platform — the engine never reads it — so like
        /// `displayLanguage` the default is owned here rather than by
        /// `EngineSettings.defaults`. The key spelling is iOS's
        /// (`SharedSettings.swift:47`); the DEFAULT is macOS's own: a Mac
        /// starts with auto-space ON (USER 2026-08-23) where the phones start
        /// OFF, so a future settings transfer must carry only values a user
        /// explicitly stored.
        static let isAutoSpaceEnabled = SettingsKey(
            name: "autoSpaceEnabled",
            defaultValue: true,
        )

        /// Whether typed punctuation becomes full-width while the 漢羅對調
        /// swap has Hanji coming first (`FullWidthPunctuation`). macOS-only
        /// and platform-side — the engine never sees the mapped character
        /// before it is document text — so like `isAutoSpaceEnabled` the
        /// default is owned here. ON to match what every macOS Chinese input
        /// method does for CJK output; roman-first output is untouched either
        /// way, so a fresh install only notices this after swapping to Hanji.
        static let isFullWidthPunctuationEnabled = SettingsKey(
            name: "fullWidthPunctuationEnabled",
            defaultValue: true,
        )

        /// Whether a solo Shift tap toggles 英數 passthrough
        /// (`ShiftTapDetector`). macOS-only — the phones have no Shift key to
        /// tap — so the default is owned here. ON because the gesture is the
        /// one MOE 輸入法 users arrive with; the preference stores only the
        /// permission, never the current mode, which is per-session and
        /// volatile.
        static let isShiftToggleAlphanumericEnabled = SettingsKey(
            name: "shiftTogglesAlphanumericEnabled",
            defaultValue: true,
        )

        // The dictionary sources. Key spellings are the iOS ones verbatim
        // (`SharedSettings.swift:53-66`) — including `khiin`, which is the one
        // key with no `Enabled` suffix. The name on the left is the engine's
        // vocabulary, the string on the right is the settings vocabulary; they
        // differ (`moeDictEnabled` ↔ `kautian`) and that is the iOS mapping.
        static let isKautianEnabled = SettingsKey(
            name: "moeDictEnabled",
            defaultValue: EngineSettings.defaults.dictionarySources.kautian,
        )
        static let isTaigitvEnabled = SettingsKey(
            name: "newwordDictEnabled",
            defaultValue: EngineSettings.defaults.dictionarySources.taigitv,
        )
        static let isItaigiEnabled = SettingsKey(
            name: "iTaigiDictEnabled",
            defaultValue: EngineSettings.defaults.dictionarySources.itaigi,
        )
        static let isSitbutEnabled = SettingsKey(
            name: "taiwanPlantDictEnabled",
            defaultValue: EngineSettings.defaults.dictionarySources.sitbut,
        )
        static let isTaihoaEnabled = SettingsKey(
            name: "taiHuaDictEnabled",
            defaultValue: EngineSettings.defaults.dictionarySources.taihoa,
        )
        static let isTaijitEnabled = SettingsKey(
            name: "taiwanJapanDictEnabled",
            defaultValue: EngineSettings.defaults.dictionarySources.taijit,
        )
        static let isKunggeEnabled = SettingsKey(
            name: "kunggeDictEnabled",
            defaultValue: EngineSettings.defaults.dictionarySources.kungge,
        )
        static let isSttiEnabled = SettingsKey(
            name: "sttiDictEnabled",
            defaultValue: EngineSettings.defaults.dictionarySources.stti,
        )
        static let isKhpooEnabled = SettingsKey(
            name: "khpooDictEnabled",
            defaultValue: EngineSettings.defaults.dictionarySources.khpoo,
        )
        static let isVariantEnabled = SettingsKey(
            name: "variantEnabled",
            defaultValue: EngineSettings.defaults.dictionarySources.variant,
        )
        static let isKhiinEnabled = SettingsKey(
            name: "khiin",
            defaultValue: EngineSettings.defaults.dictionarySources.khiin,
        )
        static let isLkkEnabled = SettingsKey(
            name: "lkkDictEnabled",
            defaultValue: EngineSettings.defaults.dictionarySources.lkk,
        )
        static let isDevEnabled = SettingsKey(
            name: "devDictEnabled",
            defaultValue: EngineSettings.defaults.dictionarySources.dev,
        )

        /// Kautian subcollections (`SharedSettings.swift:74-84`), all default on.
        static let isKautianAccentLukangEnabled = SettingsKey(
            name: "kautianAccentLukangEnabled",
            defaultValue: EngineSettings.defaults.dictionarySources.kautianSubcollections.accentLukang,
        )
        static let isKautianAccentSansiaEnabled = SettingsKey(
            name: "kautianAccentSansiaEnabled",
            defaultValue: EngineSettings.defaults.dictionarySources.kautianSubcollections.accentSansia,
        )
        static let isKautianAccentTaipakEnabled = SettingsKey(
            name: "kautianAccentTaipakEnabled",
            defaultValue: EngineSettings.defaults.dictionarySources.kautianSubcollections.accentTaipak,
        )
        static let isKautianAccentGilanEnabled = SettingsKey(
            name: "kautianAccentGilanEnabled",
            defaultValue: EngineSettings.defaults.dictionarySources.kautianSubcollections.accentGilan,
        )
        static let isKautianAccentTainanEnabled = SettingsKey(
            name: "kautianAccentTainanEnabled",
            defaultValue: EngineSettings.defaults.dictionarySources.kautianSubcollections.accentTainan,
        )
        static let isKautianAccentKaohsiungEnabled = SettingsKey(
            name: "kautianAccentKaohsiungEnabled",
            defaultValue: EngineSettings.defaults.dictionarySources.kautianSubcollections.accentKaohsiung,
        )
        static let isKautianAccentKinmenEnabled = SettingsKey(
            name: "kautianAccentKinmenEnabled",
            defaultValue: EngineSettings.defaults.dictionarySources.kautianSubcollections.accentKinmen,
        )
        static let isKautianAccentMakungEnabled = SettingsKey(
            name: "kautianAccentMakungEnabled",
            defaultValue: EngineSettings.defaults.dictionarySources.kautianSubcollections.accentMakung,
        )
        static let isKautianAccentSintikEnabled = SettingsKey(
            name: "kautianAccentSintikEnabled",
            defaultValue: EngineSettings.defaults.dictionarySources.kautianSubcollections.accentSintik,
        )
        static let isKautianAccentTaichungEnabled = SettingsKey(
            name: "kautianAccentTaichungEnabled",
            defaultValue: EngineSettings.defaults.dictionarySources.kautianSubcollections.accentTaichung,
        )
        static let isKautianNameAppendixEnabled = SettingsKey(
            name: "kautianNameAppendixEnabled",
            defaultValue: EngineSettings.defaults.dictionarySources.kautianSubcollections.nameAppendix,
        )

        /// The app UI display language, as a `DisplayLanguage` tag. The only key here whose default
        /// does not come from `EngineSettings.defaults`: which language the UI is written in is not
        /// something the composing engine reads, so the roster owns the default instead.
        static let displayLanguage = SettingsKey(
            name: "displayLanguage",
            defaultValue: DisplayLanguage.defaultTag,
        )

        /// `UpdateChecker` bookkeeping, named here per the every-key-in-one-place
        /// rule: when the next automatic check may connect, which version was
        /// already announced (announced once, never repeated), the newer version
        /// waiting to be installed, and whether the user has been asked about
        /// notifications yet.
        static let updateNextCheckDate = SettingsKey(
            name: "updateNextCheckDate",
            defaultValue: Date.distantPast,
        )
        static let updateLastNotifiedVersion = SettingsKey(
            name: "updateLastNotifiedVersion",
            defaultValue: "",
        )
        /// The pending update, stored as the manifest's own wire JSON: version
        /// and download page are one fact, and two keys could be read torn —
        /// a new version beside the previous one's URL.
        static let updatePendingManifest = SettingsKey<Data?>(
            name: "updatePendingManifest",
            defaultValue: nil,
        )
        /// Whether the notification offer has been put to the user. One-time:
        /// declining is an answer, and asking again would be nagging.
        static let hasOfferedUpdateNotifications = SettingsKey(
            name: "hasOfferedUpdateNotifications",
            defaultValue: false,
        )

        /// The settings-window pane the sidebar reopens on. UI-only like
        /// `displayLanguage` — the engine never reads it — but registered here
        /// so every defaults key this app writes is named in one place.
        static let selectedSettingsPane = SettingsKey(
            name: "selectedSettingsPane",
            defaultValue: SettingsPane.general,
        )

        /// The candidate window's layout. Presentation-only — the engine never
        /// reads it — and macOS-only, so like `displayLanguage` its default is
        /// owned by its own type rather than `EngineSettings.defaults`.
        /// Expandable is MacishType's own default, and the port keeps it.
        static let candidateLayout = SettingsKey(
            name: "candidateLayout",
            defaultValue: CandidateLayout.expandable,
        )

        /// The candidate window's light/dark choice — `auto` follows the
        /// system. Presentation-only like `candidateLayout`.
        static let candidateAppearanceMode = SettingsKey(
            name: "candidateAppearanceMode",
            defaultValue: CandidateAppearanceMode.auto,
        )

        /// How big the candidate text renders. Presentation-only like
        /// `candidateLayout`. The default is one step above the size the
        /// window originally rendered at (USER 2026-08-21) — `small`
        /// reproduces the original exactly.
        static let candidateTextSize = SettingsKey(
            name: "candidateTextSize",
            defaultValue: CandidateTextSizeChoice.medium,
        )

        /// How much air the candidate window puts around its text, as a
        /// multiplier over the cell paddings. Default enlarged like
        /// `candidateTextSize`.
        static let candidateWindowSize = SettingsKey(
            name: "candidateWindowSize",
            defaultValue: CandidateWindowSizeChoice.medium,
        )

        /// Which typeface the candidate window draws in. The key spelling is
        /// iOS's (`SharedSettings.swift`) like the rest of this enum, but the
        /// DEFAULT is macOS's own: a Mac starts on the system font
        /// (USER 2026-08-23) where iOS starts on Open Huninn. A future settings
        /// transfer must therefore carry the value a user explicitly stored,
        /// never a source platform's resolved default — materializing iOS's
        /// would silently put Open Huninn on a Mac.
        static let fontType = SettingsKey(
            name: "fontType",
            defaultValue: CandidateFontChoice.system,
        )

        /// Which modifier the candidate-slot chords use. macOS-only: the phone
        /// keyboards have no modifier keys to chord with, so the default is
        /// owned by `ComposingKeyBindings` rather than by the shared
        /// `EngineSettings.defaults`.
        static let candidateSlotModifier = SettingsKey(
            name: "candidateSlotModifier",
            defaultValue: ComposingKeyBindings.default.slotModifier,
        )

        /// The per-action composing chords are keyed by
        /// `ComposingAction.settingsKeyName` rather than named here one by one:
        /// the roster is the source of truth for which of them exist, and a
        /// second list would be one an action could be added to only one of.
        ///
        /// An absent key means "never touched" and reads as the action's
        /// default; a stored empty string means the user cleared the row, which
        /// is why the two cannot be collapsed.
        static let clearedComposingChord = ""
    }

    private let userDefaults: UserDefaults

    /// The suite is injectable so a test can run against its own domain rather
    /// than the one the user's real settings live in.
    init(userDefaults: UserDefaults = .standard) {
        self.userDefaults = userDefaults
    }

    var current: EngineSettings {
        EngineSettings(
            inputMode: inputMode,
            isTranslateSwapped: bool(Keys.isTranslateSwapped),
            isOutputBothScripts: bool(Keys.isOutputBothScripts),
            isLiteralRomanCandidateEnabled: bool(Keys.isLiteralRomanCandidateEnabled),
            isFrequencyRecordingEnabled: bool(Keys.isFrequencyRecordingEnabled),
            isAssociationRecordingEnabled: bool(Keys.isAssociationRecordingEnabled),
            isCustomDictEnabled: bool(Keys.isCustomDictEnabled),
            dictionarySources: dictionarySources,
        )
    }

    /// Read as part of `current` rather than on its own, so the toggles the
    /// engine filters candidates by and the settings it composes under always
    /// come from the same instant.
    private var dictionarySources: DictionarySourceToggles {
        DictionarySourceToggles(
            kautian: bool(Keys.isKautianEnabled),
            taigitv: bool(Keys.isTaigitvEnabled),
            itaigi: bool(Keys.isItaigiEnabled),
            sitbut: bool(Keys.isSitbutEnabled),
            taihoa: bool(Keys.isTaihoaEnabled),
            taijit: bool(Keys.isTaijitEnabled),
            kungge: bool(Keys.isKunggeEnabled),
            stti: bool(Keys.isSttiEnabled),
            khpoo: bool(Keys.isKhpooEnabled),
            variant: bool(Keys.isVariantEnabled),
            khiin: bool(Keys.isKhiinEnabled),
            lkk: bool(Keys.isLkkEnabled),
            dev: bool(Keys.isDevEnabled),
            kautianSubcollections: DictionarySourceToggles.KautianSubcollections(
                accentLukang: bool(Keys.isKautianAccentLukangEnabled),
                accentSansia: bool(Keys.isKautianAccentSansiaEnabled),
                accentTaipak: bool(Keys.isKautianAccentTaipakEnabled),
                accentGilan: bool(Keys.isKautianAccentGilanEnabled),
                accentTainan: bool(Keys.isKautianAccentTainanEnabled),
                accentKaohsiung: bool(Keys.isKautianAccentKaohsiungEnabled),
                accentKinmen: bool(Keys.isKautianAccentKinmenEnabled),
                accentMakung: bool(Keys.isKautianAccentMakungEnabled),
                accentSintik: bool(Keys.isKautianAccentSintikEnabled),
                accentTaichung: bool(Keys.isKautianAccentTaichungEnabled),
                nameAppendix: bool(Keys.isKautianNameAppendixEnabled),
            ),
        )
    }

    /// A setting stored as the raw value of a `String`-backed enum, read fresh
    /// on every access like the rest of the store so a change in the settings
    /// window applies to the very next keystroke.
    ///
    /// A stored value the type does not name — a hand-edited `defaults write`,
    /// or a choice a future version removes — reads as the default rather than
    /// leaving the reader with nothing.
    private func choice<Value: RawRepresentable>(_ key: SettingsKey<Value>) -> Value
        where Value.RawValue == String
    {
        userDefaults.string(forKey: key.name).flatMap(Value.init(rawValue:)) ?? key.defaultValue
    }

    /// The candidate window's layout.
    var candidateLayout: CandidateLayout {
        choice(Keys.candidateLayout)
    }

    /// The candidate window's light/dark choice.
    var candidateAppearanceMode: CandidateAppearanceMode {
        choice(Keys.candidateAppearanceMode)
    }

    /// The candidate text size.
    var candidateTextSize: CandidateTextSizeChoice {
        choice(Keys.candidateTextSize)
    }

    /// The candidate window's chrome size.
    var candidateWindowSize: CandidateWindowSizeChoice {
        choice(Keys.candidateWindowSize)
    }

    /// The candidate window's typeface.
    var candidateFontChoice: CandidateFontChoice {
        choice(Keys.fontType)
    }

    /// The metrics the candidate window renders at. The one place the three
    /// presentation choices are resolved together, so no caller has to know
    /// that the window is drawn from three settings rather than one.
    var candidateMetrics: CandidateMetrics {
        CandidateMetrics(
            textSize: candidateTextSize,
            windowSize: candidateWindowSize,
            fontChoice: candidateFontChoice,
        )
    }

    /// The user's composing key contract, as one snapshot.
    ///
    /// Assembled here rather than read piecemeal inside the classifier: one
    /// keystroke is classified against one assembled value, so nothing goes
    /// back to `UserDefaults` part-way through deciding what a key meant.
    var composingKeyBindings: ComposingKeyBindings {
        var chords: [ComposingAction: ComposingKeyChord?] = [:]
        for action in ComposingAction.allCases {
            guard let stored = userDefaults.string(forKey: action.settingsKeyName) else { continue }
            // A chord the current build cannot parse — a hand-edited value, or
            // one a later version wrote — reads as an empty row rather than as
            // "never touched": silently restoring the default would undo a
            // deliberate clearing, and the resolver puts back anything that
            // must stay reachable.
            chords[action] = ComposingKeyChord(rawValue: stored)
        }
        return ComposingKeyBindings(chords: chords, slotModifier: choice(Keys.candidateSlotModifier))
    }

    /// Records `chord` on `action`, or clears the row when it is nil.
    func setComposingChord(_ chord: ComposingKeyChord?, for action: ComposingAction) {
        userDefaults.set(
            chord?.rawValue ?? Keys.clearedComposingChord,
            forKey: action.settingsKeyName,
        )
    }

    /// The romanization being typed. Written as well as read, which is what
    /// keeps it out of the run of read-only choices above.
    var inputMode: InputMode {
        get { choice(Keys.inputMode) }
        set { userDefaults.set(newValue.rawValue, forKey: Keys.inputMode.name) }
    }

    /// The pane the settings window shows. Written as well as read, because
    /// opening the window ON a pane is how the input-source menu's rows lead to
    /// the place their key is set — the window binds this key with
    /// `@AppStorage`, so a write moves it even while it is already open.
    var selectedSettingsPane: SettingsPane {
        get { choice(Keys.selectedSettingsPane) }
        set { userDefaults.set(newValue.rawValue, forKey: Keys.selectedSettingsPane.name) }
    }

    /// The update-check bookkeeping. Engine-invisible like `displayLanguage`,
    /// and typed here for the same reason: this store is where every defaults
    /// key this app writes gets its one typed accessor.
    var updateNextCheckDate: Date {
        get { date(Keys.updateNextCheckDate) }
        set { userDefaults.set(newValue, forKey: Keys.updateNextCheckDate.name) }
    }

    var updateLastNotifiedVersion: String {
        get { string(Keys.updateLastNotifiedVersion) }
        set { userDefaults.set(newValue, forKey: Keys.updateLastNotifiedVersion.name) }
    }

    var updatePendingManifest: Data? {
        get { userDefaults.data(forKey: Keys.updatePendingManifest.name) }
        set { userDefaults.set(newValue, forKey: Keys.updatePendingManifest.name) }
    }

    var hasOfferedUpdateNotifications: Bool {
        get { bool(Keys.hasOfferedUpdateNotifications) }
        set { userDefaults.set(newValue, forKey: Keys.hasOfferedUpdateNotifications.name) }
    }

    /// The candidate settings a shortcut can flip. Typed properties rather than
    /// a raw key write at the call site, so a toggle always goes through the
    /// same never-written-reads-as-default rule its readers use.
    var isTranslateSwapped: Bool {
        get { bool(Keys.isTranslateSwapped) }
        set { userDefaults.set(newValue, forKey: Keys.isTranslateSwapped.name) }
    }

    var isOutputBothScripts: Bool {
        get { bool(Keys.isOutputBothScripts) }
        set { userDefaults.set(newValue, forKey: Keys.isOutputBothScripts.name) }
    }

    var isLiteralRomanCandidateEnabled: Bool {
        get { bool(Keys.isLiteralRomanCandidateEnabled) }
        set { userDefaults.set(newValue, forKey: Keys.isLiteralRomanCandidateEnabled.name) }
    }

    /// Whether committing a word auto-inserts a trailing space. Read by the
    /// controller's commit paths, never by the engine.
    var isAutoSpaceEnabled: Bool {
        get { bool(Keys.isAutoSpaceEnabled) }
        set { userDefaults.set(newValue, forKey: Keys.isAutoSpaceEnabled.name) }
    }

    /// Whether typed punctuation becomes full-width in hanji-first output.
    /// Read by the controller's two punctuation-insertion sites, never by the
    /// engine.
    var isFullWidthPunctuationEnabled: Bool {
        get { bool(Keys.isFullWidthPunctuationEnabled) }
        set { userDefaults.set(newValue, forKey: Keys.isFullWidthPunctuationEnabled.name) }
    }

    /// Whether a solo Shift tap toggles 英數 passthrough. Read on each
    /// completed tap, never by the engine.
    var isShiftToggleAlphanumericEnabled: Bool {
        get { bool(Keys.isShiftToggleAlphanumericEnabled) }
        set { userDefaults.set(newValue, forKey: Keys.isShiftToggleAlphanumericEnabled.name) }
    }

    /// The app UI display language tag. Read as a raw tag rather than a `DisplayLanguage` so a value
    /// naming no language — a hand-edited `defaults write`, or a language a future version removes —
    /// stays readable here and is clamped once, by `DisplayLanguage.fromTag`, at the display boundary.
    var displayLanguage: String {
        get { string(Keys.displayLanguage) }
        set { userDefaults.set(newValue, forKey: Keys.displayLanguage.name) }
    }

    /// Calls `onChange` whenever `key` changes in this store's domain, and answers the observation —
    /// which stays live only as long as the caller holds it.
    ///
    /// Key-value observing rather than `UserDefaults.didChangeNotification`, which only fires for
    /// writes made in this process: the point is to catch the ones made outside it, `defaults write`
    /// included. Live here rather than at the reader so the key and the domain — the two halves that
    /// have to agree — stay in the one type that knows both.
    ///
    /// `onChange` carries no value: it fires on whatever thread performed the write, so a reader has
    /// to hop to its own isolation and re-read anyway, and reading current state beats applying a
    /// value that may already be stale.
    func observeChanges(of key: SettingsKey<some Any>, onChange: @escaping @Sendable () -> Void) -> AnyObject {
        DefaultsKeyObserver(userDefaults: userDefaults, key: key.name, onChange: onChange)
    }

    /// `object(forKey:)` rather than `bool(forKey:)`: the latter answers `false`
    /// for a key that was never written, which would silently turn every
    /// default-on setting off on a fresh install.
    private func bool(_ key: SettingsKey<Bool>) -> Bool {
        userDefaults.object(forKey: key.name) as? Bool ?? key.defaultValue
    }

    /// `object(forKey:)` like `bool(_:)`, and for the same reason: a key that
    /// was never written must read as its default, not as a reference date.
    private func date(_ key: SettingsKey<Date>) -> Date {
        userDefaults.object(forKey: key.name) as? Date ?? key.defaultValue
    }

    private func string(_ key: SettingsKey<String>) -> String {
        userDefaults.string(forKey: key.name) ?? key.defaultValue
    }
}

/// One `UserDefaults` key observation, ended by letting go of the object.
///
/// KVO on `UserDefaults` predates typed observation, so this is a classic `NSObject` observer rather
/// than something `SettingsStore` can do itself.
private final class DefaultsKeyObserver: NSObject {
    /// This observer's own address, used as the KVO context: a callback for an observation registered
    /// by anyone else — a superclass, or another observer of the same key — is forwarded on rather than
    /// mistaken for ours. Per instance rather than one shared token, so the filter is exact.
    private var observationContext: UnsafeMutableRawPointer {
        Unmanaged.passUnretained(self).toOpaque()
    }

    private let userDefaults: UserDefaults
    private let key: String
    private let onChange: @Sendable () -> Void

    init(userDefaults: UserDefaults, key: String, onChange: @escaping @Sendable () -> Void) {
        self.userDefaults = userDefaults
        self.key = key
        self.onChange = onChange
        super.init()
        userDefaults.addObserver(self, forKeyPath: key, options: [], context: observationContext)
    }

    deinit {
        userDefaults.removeObserver(self, forKeyPath: key, context: observationContext)
    }

    override func observeValue(
        forKeyPath keyPath: String?,
        of object: Any?,
        change: [NSKeyValueChangeKey: Any]?,
        context: UnsafeMutableRawPointer?,
    ) {
        guard context == observationContext, keyPath == key else {
            super.observeValue(forKeyPath: keyPath, of: object, change: change, context: context)
            return
        }
        onChange()
    }
}
