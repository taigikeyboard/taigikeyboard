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

    /// `object(forKey:)` rather than `bool(forKey:)`: the latter answers `false`
    /// for a key that was never written, which would silently turn every
    /// default-on setting off on a fresh install.
    private func bool(_ key: SettingsKey<Bool>) -> Bool {
        userDefaults.object(forKey: key.name) as? Bool ?? key.defaultValue
    }
}
