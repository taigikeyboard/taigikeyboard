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

        /// Which chrome generation the candidate window draws — `auto` follows
        /// the OS. Presentation-only like `candidateLayout`.
        static let candidateWindowStyle = SettingsKey(
            name: "candidateWindowStyle",
            defaultValue: CandidateWindowStyleChoice.auto,
        )

        /// The candidate highlight's accent colour — `auto` follows the
        /// system (and the host app under Multicolour). Presentation-only
        /// like `candidateLayout`.
        static let candidateAccentColor = SettingsKey(
            name: "candidateAccentColor",
            defaultValue: CandidateAccentChoice.auto,
        )

        /// The candidate window's light/dark choice — `auto` follows the
        /// system. Presentation-only like `candidateLayout`.
        static let candidateAppearanceMode = SettingsKey(
            name: "candidateAppearanceMode",
            defaultValue: CandidateAppearanceMode.auto,
        )
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

    /// The candidate window's layout, read fresh on every access like the rest
    /// of the store so a change in the settings window applies to the very
    /// next keystroke's window. Unknown stored values read as the default.
    var candidateLayout: CandidateLayout {
        userDefaults.string(forKey: Keys.candidateLayout.name)
            .flatMap(CandidateLayout.init(rawValue:))
            ?? Keys.candidateLayout.defaultValue
    }

    /// The candidate window's chrome choice, live-read like `candidateLayout`.
    var candidateWindowStyle: CandidateWindowStyleChoice {
        userDefaults.string(forKey: Keys.candidateWindowStyle.name)
            .flatMap(CandidateWindowStyleChoice.init(rawValue:))
            ?? Keys.candidateWindowStyle.defaultValue
    }

    /// The candidate highlight's accent choice, live-read like the two above.
    var candidateAccentColor: CandidateAccentChoice {
        userDefaults.string(forKey: Keys.candidateAccentColor.name)
            .flatMap(CandidateAccentChoice.init(rawValue:))
            ?? Keys.candidateAccentColor.defaultValue
    }

    /// The candidate window's light/dark choice, live-read like the rest.
    var candidateAppearanceMode: CandidateAppearanceMode {
        userDefaults.string(forKey: Keys.candidateAppearanceMode.name)
            .flatMap(CandidateAppearanceMode.init(rawValue:))
            ?? Keys.candidateAppearanceMode.defaultValue
    }

    /// The romanization being typed. A stored value that names no mode — a
    /// hand-edited `defaults write`, or a mode a future version removes — reads
    /// as the default rather than as a mode the engine cannot render.
    var inputMode: InputMode {
        get {
            userDefaults.string(forKey: Keys.inputMode.name)
                .flatMap(InputMode.init(rawValue:))
                ?? Keys.inputMode.defaultValue
        }
        set { userDefaults.set(newValue.rawValue, forKey: Keys.inputMode.name) }
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

    /// The app UI display language tag. Read as a raw tag rather than a `DisplayLanguage` so a value
    /// naming no language — a hand-edited `defaults write`, or a language a future version removes —
    /// stays readable here and is clamped once, by `DisplayLanguage.fromTag`, at the display boundary.
    var displayLanguage: String {
        get { userDefaults.string(forKey: Keys.displayLanguage.name) ?? Keys.displayLanguage.defaultValue }
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
