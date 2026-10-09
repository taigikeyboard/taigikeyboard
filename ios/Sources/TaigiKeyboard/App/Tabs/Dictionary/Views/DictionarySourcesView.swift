import SwiftUI
import UIKit

/// Manage Dictionaries subpage of the Dictionary tab — the dictionary-source
/// toggles (MOE characters with nested kautian subcollections, other
/// dictionaries, supplementary data) plus dictionary search pinned at the
/// bottom. Desktop twin: the Manage Dictionaries settings pane.
struct DictionarySourcesView: View {
    @Environment(DisplayLanguageStore.self) private var lang

    private let settings = SharedSettings.shared

    /// Every toggle's value, read from `SharedSettings` as one snapshot — on init and again
    /// on every appearance, so a Settings → Reset made while this page sat in the tab's
    /// stack (or before the tab built its link) shows here.
    private struct Toggles {
        // Dictionary toggle states
        var isMoeDictEnabled: Bool
        var isNewwordDictEnabled: Bool
        var isKunggeDictEnabled: Bool
        var isITaigiDictEnabled: Bool
        var isTaiwanJapanDictEnabled: Bool
        var isTaiHuaDictEnabled: Bool
        var isTaiwanPlantDictEnabled: Bool
        var isSttiDictEnabled: Bool
        var isKhpooDictEnabled: Bool
        var isVariantEnabled: Bool
        var isKhiinEnabled: Bool
        var isLkkDictEnabled: Bool
        var isDevDictEnabled: Bool

        // Kautian subcollection toggles (nested under the MOE/kautian master row)
        var isKautianAccentLukangEnabled: Bool
        var isKautianAccentSansiaEnabled: Bool
        var isKautianAccentTaipakEnabled: Bool
        var isKautianAccentGilanEnabled: Bool
        var isKautianAccentTainanEnabled: Bool
        var isKautianAccentKaohsiungEnabled: Bool
        var isKautianAccentKinmenEnabled: Bool
        var isKautianAccentMakungEnabled: Bool
        var isKautianAccentSintikEnabled: Bool
        var isKautianAccentTaichungEnabled: Bool
        var isKautianNameAppendixEnabled: Bool
        var isKautianAltReadingEnabled: Bool

        init(_ settings: SharedSettings) {
            isMoeDictEnabled = settings.isMoeDictEnabled
            isNewwordDictEnabled = settings.isNewwordDictEnabled
            isKunggeDictEnabled = settings.isKunggeDictEnabled
            isITaigiDictEnabled = settings.isITaigiDictEnabled
            isTaiwanJapanDictEnabled = settings.isTaiwanJapanDictEnabled
            isTaiHuaDictEnabled = settings.isTaiHuaDictEnabled
            isTaiwanPlantDictEnabled = settings.isTaiwanPlantDictEnabled
            isSttiDictEnabled = settings.isSttiDictEnabled
            isKhpooDictEnabled = settings.isKhpooDictEnabled
            isVariantEnabled = settings.isVariantEnabled
            isKhiinEnabled = settings.isKhiinEnabled
            isLkkDictEnabled = settings.isLkkDictEnabled
            isDevDictEnabled = settings.isDevDictEnabled
            isKautianAccentLukangEnabled = settings.isKautianAccentLukangEnabled
            isKautianAccentSansiaEnabled = settings.isKautianAccentSansiaEnabled
            isKautianAccentTaipakEnabled = settings.isKautianAccentTaipakEnabled
            isKautianAccentGilanEnabled = settings.isKautianAccentGilanEnabled
            isKautianAccentTainanEnabled = settings.isKautianAccentTainanEnabled
            isKautianAccentKaohsiungEnabled = settings.isKautianAccentKaohsiungEnabled
            isKautianAccentKinmenEnabled = settings.isKautianAccentKinmenEnabled
            isKautianAccentMakungEnabled = settings.isKautianAccentMakungEnabled
            isKautianAccentSintikEnabled = settings.isKautianAccentSintikEnabled
            isKautianAccentTaichungEnabled = settings.isKautianAccentTaichungEnabled
            isKautianNameAppendixEnabled = settings.isKautianNameAppendixEnabled
            isKautianAltReadingEnabled = settings.isKautianAltReadingEnabled
        }
    }

    @State private var toggles = Toggles(SharedSettings.shared)

    var body: some View {
        Form {
            // MOE dictionaries
            Section {
                dictToggleWithDescription(
                    title: lang.string(.commonMoeDict),
                    url: "https://sutian.moe.edu.tw/",
                    isOn: $toggles.isMoeDictEnabled,
                    description: lang.string(.dictionaryMoeDescription),
                ) { settings.isMoeDictEnabled = $0 }
                // Kautian subcollections — nested under the master row,
                // greyed when the MOE/kautian master is off (DD7).
                kautianSubcollToggle(lang.string(.dictionaryKautianAltReading), isOn: $toggles.isKautianAltReadingEnabled) {
                    settings.isKautianAltReadingEnabled = $0
                }
                kautianSubcollToggle(lang.string(.dictionaryKautianAccentLukang), isOn: $toggles.isKautianAccentLukangEnabled) {
                    settings.isKautianAccentLukangEnabled = $0
                }
                kautianSubcollToggle(lang.string(.dictionaryKautianAccentSansia), isOn: $toggles.isKautianAccentSansiaEnabled) {
                    settings.isKautianAccentSansiaEnabled = $0
                }
                kautianSubcollToggle(lang.string(.dictionaryKautianAccentTaipak), isOn: $toggles.isKautianAccentTaipakEnabled) {
                    settings.isKautianAccentTaipakEnabled = $0
                }
                kautianSubcollToggle(lang.string(.dictionaryKautianAccentGilan), isOn: $toggles.isKautianAccentGilanEnabled) {
                    settings.isKautianAccentGilanEnabled = $0
                }
                kautianSubcollToggle(lang.string(.dictionaryKautianAccentTainan), isOn: $toggles.isKautianAccentTainanEnabled) {
                    settings.isKautianAccentTainanEnabled = $0
                }
                kautianSubcollToggle(lang.string(.dictionaryKautianAccentKaohsiung), isOn: $toggles.isKautianAccentKaohsiungEnabled) {
                    settings.isKautianAccentKaohsiungEnabled = $0
                }
                kautianSubcollToggle(lang.string(.dictionaryKautianAccentKinmen), isOn: $toggles.isKautianAccentKinmenEnabled) {
                    settings.isKautianAccentKinmenEnabled = $0
                }
                kautianSubcollToggle(lang.string(.dictionaryKautianAccentMakung), isOn: $toggles.isKautianAccentMakungEnabled) {
                    settings.isKautianAccentMakungEnabled = $0
                }
                kautianSubcollToggle(lang.string(.dictionaryKautianAccentSintik), isOn: $toggles.isKautianAccentSintikEnabled) {
                    settings.isKautianAccentSintikEnabled = $0
                }
                kautianSubcollToggle(lang.string(.dictionaryKautianAccentTaichung), isOn: $toggles.isKautianAccentTaichungEnabled) {
                    settings.isKautianAccentTaichungEnabled = $0
                }
                kautianSubcollToggle(lang.string(.dictionaryKautianNameAppendix), isOn: $toggles.isKautianNameAppendixEnabled) {
                    settings.isKautianNameAppendixEnabled = $0
                }
                dictToggleWithDescription(
                    title: lang.string(.commonNewwordDict),
                    url: "https://www.taigitv.org.tw/taigi-words",
                    isOn: $toggles.isNewwordDictEnabled,
                    description: lang.string(.dictionaryNewwordDescription),
                ) { settings.isNewwordDictEnabled = $0 }
                dictToggleWithDescription(
                    title: lang.string(.commonSttiDict),
                    url: "https://stti.moe.edu.tw/index.html?lang=sutgi",
                    isOn: $toggles.isSttiDictEnabled,
                    description: lang.string(.dictionarySttiDescription),
                ) { settings.isSttiDictEnabled = $0 }
                dictToggleWithDescription(
                    title: lang.string(.commonKunggeDict),
                    url: "https://kanggesu.ntcri.gov.tw",
                    isOn: $toggles.isKunggeDictEnabled,
                    description: lang.string(.dictionaryKunggeDescription),
                ) { settings.isKunggeDictEnabled = $0 }
            } header: {
                Text(lang.string(.dictionaryMoeSectionTitle))
                    .font(AppStyle.sectionHeaderFont)
            }

            // Other dictionaries
            Section {
                dictionaryToggle(lang.string(.commonITaigiDict), isOn: $toggles.isITaigiDictEnabled, info: .iTaigi) {
                    settings.isITaigiDictEnabled = $0
                }
                dictionaryToggle(lang.string(.commonTaiwanJapanDict), isOn: $toggles.isTaiwanJapanDictEnabled, info: .taiwanJapan) {
                    settings.isTaiwanJapanDictEnabled = $0
                }
                dictionaryToggle(lang.string(.commonTaiHuaDict), isOn: $toggles.isTaiHuaDictEnabled, info: .taiHua) {
                    settings.isTaiHuaDictEnabled = $0
                }
                dictionaryToggle(lang.string(.commonTaiwanPlantDict), isOn: $toggles.isTaiwanPlantDictEnabled, info: .taiwanPlant) {
                    settings.isTaiwanPlantDictEnabled = $0
                }
            } header: {
                Text(lang.string(.dictionaryOtherSectionTitle))
                    .font(AppStyle.sectionHeaderFont)
            }

            // Variant characters / legacy characters / accent data
            Section {
                dictionaryToggle(lang.string(.dictionaryVariantDictionary), isOn: $toggles.isVariantEnabled, info: .variant) {
                    settings.isVariantEnabled = $0
                }

                dictionaryToggle(lang.string(.dictionaryKhiin), isOn: $toggles.isKhiinEnabled, info: .khiin) {
                    settings.isKhiinEnabled = $0
                }

                dictionaryToggle(lang.string(.commonAccentDict), isOn: $toggles.isKhpooDictEnabled, info: .khpoo) {
                    settings.isKhpooDictEnabled = $0
                }

                dictToggleWithDescription(
                    title: lang.string(.dictionaryLkkDict),
                    url: "https://docs.google.com/spreadsheets/d/1ICPcP3PuEdLirax-HBLtewiOz53KzAfpme9sjmoIO-w/edit?usp=sharing",
                    isOn: $toggles.isLkkDictEnabled,
                    description: lang.string(.dictionaryLkkDescription),
                ) { settings.isLkkDictEnabled = $0 }

                dictToggleWithDescription(
                    title: lang.string(.dictionaryDevSupplementDict),
                    url: "https://github.com/luke871016/Taigi-Input-method-dictionary-supplement",
                    isOn: $toggles.isDevDictEnabled,
                    description: lang.string(.dictionaryDevDescription),
                ) { settings.isDevDictEnabled = $0 }
            } header: {
                Text(lang.string(.dictionarySupplementSectionTitle))
                    .font(AppStyle.sectionHeaderFont)
            }
        }
        .navigationTitle(lang.string(.desktopDictionarySourcesLink))
        .navigationBarTitleDisplayMode(.large)
        .onAppear { toggles = Toggles(settings) }
        .scrollDismissesKeyboard(.interactively)
        .safeAreaInset(edge: .bottom) { DictionarySearchPanel() }
    }

    // MARK: - Dictionary Toggle with Description + Link

    // Dictionary toggle row with an external link + description (MOE / supplementary dictionaries).
    private func dictToggleWithDescription(
        title: String,
        url: String,
        isOn: Binding<Bool>,
        description: String,
        onChange: @escaping (Bool) -> Void,
    ) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Button {
                    if let link = URL(string: url) {
                        UIApplication.shared.open(link)
                    }
                } label: {
                    HStack(spacing: 4) {
                        Image(latinSystemName: "arrow.up.forward.square")
                            .font(.subheadline)
                        Text(title)
                    }
                    .foregroundColor(AppStyle.accentBlue)
                }
                .buttonStyle(.plain)
                Spacer()
                Toggle("", isOn: isOn)
                    .labelsHidden()
            }
            .onChange(of: isOn.wrappedValue) { _, newValue in
                onChange(newValue)
            }
            Text(description)
                .font(AppStyle.bodyFont)
                .foregroundColor(.primary)
        }
    }

    // MARK: - Dictionary Toggle with Info Button

    // Dictionary toggle row with an info button (popup from DictionaryInfo.descriptionKey).
    private func dictionaryToggle(
        _ text: String,
        isOn: Binding<Bool>,
        info: DictionaryInfo,
        onChange: @escaping (Bool) -> Void,
    ) -> some View {
        Toggle(isOn: isOn) {
            HStack {
                Text(text)
                SettingInfoButton(description: lang.string(info.descriptionKey))
            }
        }
        .onChange(of: isOn.wrappedValue) { _, newValue in
            onChange(newValue)
        }
    }

    // MARK: - Kautian Subcollection Toggle (nested, dependent on master)

    // Nested MOE subcollection toggle, indented; greys out as a group when
    // toggles.isMoeDictEnabled is off (DD7). Mirrors Android DictionarySubToggleRow.
    private func kautianSubcollToggle(
        _ title: String,
        isOn: Binding<Bool>,
        onChange: @escaping (Bool) -> Void,
    ) -> some View {
        Toggle(isOn: isOn) {
            Text(title)
                .padding(.leading, 16)
        }
        .disabled(!toggles.isMoeDictEnabled)
        .onChange(of: isOn.wrappedValue) { _, newValue in
            onChange(newValue)
        }
    }
}

/// Dictionary search pinned at the bottom of Manage Dictionaries; results (at most 5) pop up
/// above the bar. Its own view so a keystroke re-renders only the panel, not the toggle Form.
private struct DictionarySearchPanel: View {
    @Environment(DisplayLanguageStore.self) private var lang

    @StateObject private var searchVM = DictionarySearchViewModel()

    /// Search focus
    @FocusState private var isSearchFocused: Bool

    // Dictionary lookup selection
    @State private var selectedResult: DictionarySearchResult?
    @State private var showLookupDialog = false

    var body: some View {
        VStack(spacing: 0) {
            Divider()
            // Search results above search bar
            if !searchVM.searchText.isEmpty {
                if searchVM.results.isEmpty, !searchVM.isSearching {
                    Text(lang.string(.dictionaryNoResults))
                        .font(AppStyle.captionFont)
                        .foregroundStyle(.secondary)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(.horizontal, AppStyle.horizontalPadding)
                        .padding(.vertical, AppStyle.verticalPadding)
                } else if !searchVM.results.isEmpty {
                    Divider()
                    ScrollView {
                        LazyVStack(spacing: 0) {
                            let visible = Array(searchVM.results.prefix(5))
                            ForEach(Array(visible.enumerated()), id: \.offset) { index, result in
                                searchResultRow(result)
                                if index < visible.count - 1 {
                                    Divider()
                                        .padding(.leading, 16)
                                }
                            }
                        }
                    }
                    .frame(maxHeight: 200)
                }
            }

            // Search bar
            HStack {
                Image(latinSystemName: "magnifyingglass")
                    .foregroundStyle(.secondary)
                TextField(
                    lang.string(.dictionarySearchPlaceholder),
                    text: $searchVM.searchText,
                )
                .focused($isSearchFocused)
                .onChange(of: searchVM.searchText) { _, _ in
                    searchVM.onSearchTextChanged()
                }
                if !searchVM.searchText.isEmpty {
                    Button {
                        searchVM.searchText = ""
                        isSearchFocused = false
                    } label: {
                        Image(latinSystemName: "xmark.circle.fill")
                            .foregroundStyle(.secondary)
                    }
                    .buttonStyle(.plain)
                }
            }
            .padding(.horizontal, AppStyle.innerHorizontalPadding)
            .padding(.vertical, AppStyle.verticalPadding)
            .background(Color(.tertiarySystemFill))
            .clipShape(RoundedRectangle(cornerRadius: AppStyle.cardCornerRadius, style: .continuous))
            .padding(.horizontal, AppStyle.horizontalPadding)
            .padding(.vertical, AppStyle.verticalPadding)
        }
        .background(Color(.systemBackground))
        .padding(.bottom, 8)
        .confirmationDialog("", isPresented: $showLookupDialog) {
            if let result = selectedResult {
                if let moeURL = result.moeURL {
                    Button(lang.string(.dictionaryLookupMoe)) {
                        UIApplication.shared.open(moeURL)
                    }
                }
                if let chhoeURL = result.chhoeURL {
                    Button(lang.string(.dictionaryLookupChhoe)) {
                        UIApplication.shared.open(chhoeURL)
                    }
                }
            }
            Button(lang.string(.commonCancel), role: .cancel) {}
        }
    }

    // MARK: - Search Result Row

    private func searchResultRow(_ result: DictionarySearchResult) -> some View {
        Button {
            selectedResult = result
            showLookupDialog = true
        } label: {
            HStack {
                Text(result.roman)
                    .foregroundStyle(.primary)
                if let hanji = result.hanji {
                    Text(hanji)
                        .foregroundStyle(.primary)
                }
                ForEach(uniqueTagKeys(for: result), id: \.self) { tagKey in
                    TagBadge(text: lang.string(tagKey))
                }
                Spacer()
                Image(latinSystemName: "arrow.up.right")
                    .font(AppStyle.captionFont)
                    .foregroundStyle(.secondary)
            }
            .padding(.horizontal, AppStyle.horizontalPadding)
            .padding(.vertical, 10)
        }
        .buttonStyle(.plain)
    }

    // MARK: - Source Badge Tags

    /// i18n key for a dictionary source's compact badge label. Resolved at the
    /// call site via the active display language, so the shared-core
    /// `DictionarySource` enum stays free of App-layer i18n types.
    private func tagKey(for source: DictionarySource) -> StringKey {
        switch source {
        case .kautian: .dictionaryKautianTag
        case .taigitv: .dictionaryTaigitvTag
        case .itaigi: .dictionaryITaigiTag
        case .sitbut: .dictionarySitbutTag
        case .taihoa: .dictionaryTaihoaTag
        case .taijit: .dictionaryTaijitTag
        case .kungge: .dictionaryKunggeTag
        case .stti: .dictionarySttiTag
        case .lkk: .dictionaryLkkTag
        // khpoo/khiin/dev/custom collapse to one "Supplementary Data" badge (reuses the section-title key).
        case .khpoo, .khiin, .dev, .custom: .dictionarySupplementSectionTitle
        }
    }

    /// Deduplicated badge keys for a result, preserving `sources` order. Dedup by
    /// key (not resolved string) so the 4 supplementary sources collapse to one
    /// badge regardless of the active display language.
    private func uniqueTagKeys(for result: DictionarySearchResult) -> [StringKey] {
        var seen = Set<StringKey>()
        return result.sources.compactMap { source in
            let key = tagKey(for: source)
            return seen.insert(key).inserted ? key : nil
        }
    }
}
