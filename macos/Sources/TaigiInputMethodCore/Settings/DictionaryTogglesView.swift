// Which dictionaries the keyboard draws candidates from.

import SwiftUI

/// The dictionary source toggles, in the sections iOS groups them into
/// (`ios/.../App/Tabs/Dictionary/DictionaryTab.swift`).
///
/// `@AppStorage` per toggle rather than one snapshot object: each row is an
/// independent setting the engine live-reads, and binding them individually is
/// what makes a `defaults write` — or a future settings import — show up in the
/// form without anything having to be told.
struct DictionaryTogglesView: View {
    @Environment(DisplayLanguageStore.self) private var language

    @AppStorage(SettingsStore.Keys.isKautianEnabled.name)
    private var isKautianEnabled = SettingsStore.Keys.isKautianEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isTaigitvEnabled.name)
    private var isTaigitvEnabled = SettingsStore.Keys.isTaigitvEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isKunggeEnabled.name)
    private var isKunggeEnabled = SettingsStore.Keys.isKunggeEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isSttiEnabled.name)
    private var isSttiEnabled = SettingsStore.Keys.isSttiEnabled.defaultValue

    @AppStorage(SettingsStore.Keys.isItaigiEnabled.name)
    private var isItaigiEnabled = SettingsStore.Keys.isItaigiEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isTaijitEnabled.name)
    private var isTaijitEnabled = SettingsStore.Keys.isTaijitEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isTaihoaEnabled.name)
    private var isTaihoaEnabled = SettingsStore.Keys.isTaihoaEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isSitbutEnabled.name)
    private var isSitbutEnabled = SettingsStore.Keys.isSitbutEnabled.defaultValue

    @AppStorage(SettingsStore.Keys.isVariantEnabled.name)
    private var isVariantEnabled = SettingsStore.Keys.isVariantEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isKhiinEnabled.name)
    private var isKhiinEnabled = SettingsStore.Keys.isKhiinEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isKhpooEnabled.name)
    private var isKhpooEnabled = SettingsStore.Keys.isKhpooEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isLkkEnabled.name)
    private var isLkkEnabled = SettingsStore.Keys.isLkkEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isDevEnabled.name)
    private var isDevEnabled = SettingsStore.Keys.isDevEnabled.defaultValue

    @AppStorage(SettingsStore.Keys.isKautianAccentLukangEnabled.name)
    private var isLukangEnabled = SettingsStore.Keys.isKautianAccentLukangEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isKautianAccentSansiaEnabled.name)
    private var isSansiaEnabled = SettingsStore.Keys.isKautianAccentSansiaEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isKautianAccentTaipakEnabled.name)
    private var isTaipakEnabled = SettingsStore.Keys.isKautianAccentTaipakEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isKautianAccentGilanEnabled.name)
    private var isGilanEnabled = SettingsStore.Keys.isKautianAccentGilanEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isKautianAccentTainanEnabled.name)
    private var isTainanEnabled = SettingsStore.Keys.isKautianAccentTainanEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isKautianAccentKaohsiungEnabled.name)
    private var isKaohsiungEnabled = SettingsStore.Keys.isKautianAccentKaohsiungEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isKautianAccentKinmenEnabled.name)
    private var isKinmenEnabled = SettingsStore.Keys.isKautianAccentKinmenEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isKautianAccentMakungEnabled.name)
    private var isMakungEnabled = SettingsStore.Keys.isKautianAccentMakungEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isKautianAccentSintikEnabled.name)
    private var isSintikEnabled = SettingsStore.Keys.isKautianAccentSintikEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isKautianAccentTaichungEnabled.name)
    private var isTaichungEnabled = SettingsStore.Keys.isKautianAccentTaichungEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isKautianNameAppendixEnabled.name)
    private var isNameAppendixEnabled = SettingsStore.Keys.isKautianNameAppendixEnabled.defaultValue

    var body: some View {
        Form {
            Section {
                Toggle(language.string(.commonMoeDict), isOn: $isKautianEnabled)
                Group {
                    Toggle(language.string(.dictionaryKautianAccentLukang), isOn: $isLukangEnabled)
                    Toggle(language.string(.dictionaryKautianAccentSansia), isOn: $isSansiaEnabled)
                    Toggle(language.string(.dictionaryKautianAccentTaipak), isOn: $isTaipakEnabled)
                    Toggle(language.string(.dictionaryKautianAccentGilan), isOn: $isGilanEnabled)
                    Toggle(language.string(.dictionaryKautianAccentTainan), isOn: $isTainanEnabled)
                    Toggle(language.string(.dictionaryKautianAccentKaohsiung), isOn: $isKaohsiungEnabled)
                    Toggle(language.string(.dictionaryKautianAccentKinmen), isOn: $isKinmenEnabled)
                    Toggle(language.string(.dictionaryKautianAccentMakung), isOn: $isMakungEnabled)
                    Toggle(language.string(.dictionaryKautianAccentSintik), isOn: $isSintikEnabled)
                    Toggle(language.string(.dictionaryKautianAccentTaichung), isOn: $isTaichungEnabled)
                    Toggle(language.string(.dictionaryKautianNameAppendix), isOn: $isNameAppendixEnabled)
                }
                .padding(.leading, Metrics.subcollectionIndent)
                // Disabled, not cleared: the master switch says whether this
                // dictionary is searched at all, and turning it back on has to
                // return the 腔口 the user had chosen rather than all of them.
                .disabled(!isKautianEnabled)

                Toggle(language.string(.commonNewwordDict), isOn: $isTaigitvEnabled)
                Toggle(language.string(.commonKunggeDict), isOn: $isKunggeEnabled)
                Toggle(language.string(.commonSttiDict), isOn: $isSttiEnabled)
            } header: {
                Text(language.string(.dictionaryMoeSectionTitle))
            }

            Section {
                Toggle(language.string(.commonITaigiDict), isOn: $isItaigiEnabled)
                Toggle(language.string(.commonTaiwanJapanDict), isOn: $isTaijitEnabled)
                Toggle(language.string(.commonTaiHuaDict), isOn: $isTaihoaEnabled)
                Toggle(language.string(.commonTaiwanPlantDict), isOn: $isSitbutEnabled)
            } header: {
                Text(language.string(.dictionaryOtherSectionTitle))
            }

            Section {
                Toggle(language.string(.dictionaryVariantDictionary), isOn: $isVariantEnabled)
                Toggle(language.string(.dictionaryKhiin), isOn: $isKhiinEnabled)
                Toggle(language.string(.commonAccentDict), isOn: $isKhpooEnabled)
                Toggle(language.string(.dictionaryLkkDict), isOn: $isLkkEnabled)
                Toggle(language.string(.dictionaryDevSupplementDict), isOn: $isDevEnabled)
            } header: {
                Text(language.string(.dictionarySupplementSectionTitle))
            }

            // Its own section, at the end, drawn the way the 快捷鍵 and 外觀
            // panes draw theirs: it acts on every toggle above it rather than
            // on any one of them.
            Section {
                WideActionRow(titleKey: .themeEditorResetAll, action: restoreDefaults)
            }
        }
        .formStyle(.grouped)
    }

    /// Puts every source and 腔口 toggle back to what a fresh install searches.
    ///
    /// The `@AppStorage` bindings above repaint on their own: removing a key is
    /// a `UserDefaults` change like any other, and each binding falls back to
    /// the default it was declared with.
    private func restoreDefaults() {
        SettingsStore().resetDictionarySources()
    }

    private enum Metrics {
        static let subcollectionIndent: CGFloat = 16
    }
}
