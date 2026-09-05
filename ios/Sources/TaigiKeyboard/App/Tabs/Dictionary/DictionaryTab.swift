// Dictionary 分頁主畫面 — 詞典開關、自訂詞庫、頻率/聯想資料管理、跨詞典搜尋。
// 各 NavigationLink 子畫面分別由 CustomDictionaryView / FrequencyDataView /
// AssociationDataView / DataManagementView 處理。

import SwiftUI
import UIKit

/// Dictionary tab.
///
/// Manage dictionary toggles, custom dictionary, frequency/association data, and search.
// Dictionary 分頁的根 View,組裝詞典開關區塊、底部搜尋列與查詢結果浮層。
struct DictionaryTab: View {
    @Environment(DisplayLanguageStore.self) private var lang
    @StateObject private var searchVM = DictionarySearchViewModel()

    private let settings = SharedSettings.shared

    // Dictionary toggle states
    @State private var isMoeDictEnabled: Bool
    @State private var isNewwordDictEnabled: Bool
    @State private var isKunggeDictEnabled: Bool
    @State private var isITaigiDictEnabled: Bool
    @State private var isTaiwanJapanDictEnabled: Bool
    @State private var isTaiHuaDictEnabled: Bool
    @State private var isTaiwanPlantDictEnabled: Bool
    @State private var isSttiDictEnabled: Bool
    @State private var isKhpooDictEnabled: Bool
    @State private var isVariantEnabled: Bool
    @State private var isKhiinEnabled: Bool
    @State private var isLkkDictEnabled: Bool
    @State private var isDevDictEnabled: Bool

    // Kautian subcollection toggles (nested under the MOE/kautian master row)
    @State private var isKautianAccentLukangEnabled: Bool
    @State private var isKautianAccentSansiaEnabled: Bool
    @State private var isKautianAccentTaipakEnabled: Bool
    @State private var isKautianAccentGilanEnabled: Bool
    @State private var isKautianAccentTainanEnabled: Bool
    @State private var isKautianAccentKaohsiungEnabled: Bool
    @State private var isKautianAccentKinmenEnabled: Bool
    @State private var isKautianAccentMakungEnabled: Bool
    @State private var isKautianAccentSintikEnabled: Bool
    @State private var isKautianAccentTaichungEnabled: Bool
    @State private var isKautianNameAppendixEnabled: Bool

    /// Search focus
    @FocusState private var isSearchFocused: Bool

    // Dictionary lookup selection
    @State private var selectedResult: DictionarySearchResult?
    @State private var showLookupDialog = false

    init() {
        let settings = SharedSettings.shared
        _isMoeDictEnabled = State(initialValue: settings.isMoeDictEnabled)
        _isNewwordDictEnabled = State(initialValue: settings.isNewwordDictEnabled)
        _isKunggeDictEnabled = State(initialValue: settings.isKunggeDictEnabled)
        _isITaigiDictEnabled = State(initialValue: settings.isITaigiDictEnabled)
        _isTaiwanJapanDictEnabled = State(initialValue: settings.isTaiwanJapanDictEnabled)
        _isTaiHuaDictEnabled = State(initialValue: settings.isTaiHuaDictEnabled)
        _isTaiwanPlantDictEnabled = State(initialValue: settings.isTaiwanPlantDictEnabled)
        _isSttiDictEnabled = State(initialValue: settings.isSttiDictEnabled)
        _isKhpooDictEnabled = State(initialValue: settings.isKhpooDictEnabled)
        _isVariantEnabled = State(initialValue: settings.isVariantEnabled)
        _isKhiinEnabled = State(initialValue: settings.isKhiinEnabled)
        _isLkkDictEnabled = State(initialValue: settings.isLkkDictEnabled)
        _isDevDictEnabled = State(initialValue: settings.isDevDictEnabled)
        _isKautianAccentLukangEnabled = State(initialValue: settings.isKautianAccentLukangEnabled)
        _isKautianAccentSansiaEnabled = State(initialValue: settings.isKautianAccentSansiaEnabled)
        _isKautianAccentTaipakEnabled = State(initialValue: settings.isKautianAccentTaipakEnabled)
        _isKautianAccentGilanEnabled = State(initialValue: settings.isKautianAccentGilanEnabled)
        _isKautianAccentTainanEnabled = State(initialValue: settings.isKautianAccentTainanEnabled)
        _isKautianAccentKaohsiungEnabled = State(initialValue: settings.isKautianAccentKaohsiungEnabled)
        _isKautianAccentKinmenEnabled = State(initialValue: settings.isKautianAccentKinmenEnabled)
        _isKautianAccentMakungEnabled = State(initialValue: settings.isKautianAccentMakungEnabled)
        _isKautianAccentSintikEnabled = State(initialValue: settings.isKautianAccentSintikEnabled)
        _isKautianAccentTaichungEnabled = State(initialValue: settings.isKautianAccentTaichungEnabled)
        _isKautianNameAppendixEnabled = State(initialValue: settings.isKautianNameAppendixEnabled)
    }

    var body: some View {
        NavigationStack {
            Form {
                // Data management
                Section {
                    NavigationLink(destination: CustomDictionaryView()) {
                        Text(lang.string(.dictionaryCustomDictionary))
                    }
                    NavigationLink(destination: FrequencyDataView()) {
                        Text(lang.string(.dictionaryFrequencyManagement))
                    }
                    NavigationLink(destination: AssociationDataView()) {
                        Text(lang.string(.dictionaryAssociationManagement))
                    }
                    NavigationLink(destination: DataManagementView()) {
                        Text(lang.string(.dictionaryBackupRestore))
                    }
                } header: {
                    Text(lang.string(.dictionaryDataManagement))
                        .font(AppStyle.sectionHeaderFont)
                }

                // MOE dictionaries (教育部)
                Section {
                    dictToggleWithDescription(
                        title: lang.string(.commonMoeDict),
                        url: "https://sutian.moe.edu.tw/",
                        isOn: $isMoeDictEnabled,
                        description: lang.string(.dictionaryMoeDescription),
                    ) { settings.isMoeDictEnabled = $0 }
                    // Kautian subcollections — nested under the master row,
                    // greyed when the MOE/kautian master is off (DD7).
                    kautianSubcollToggle(lang.string(.dictionaryKautianAccentLukang), isOn: $isKautianAccentLukangEnabled) {
                        settings.isKautianAccentLukangEnabled = $0
                    }
                    kautianSubcollToggle(lang.string(.dictionaryKautianAccentSansia), isOn: $isKautianAccentSansiaEnabled) {
                        settings.isKautianAccentSansiaEnabled = $0
                    }
                    kautianSubcollToggle(lang.string(.dictionaryKautianAccentTaipak), isOn: $isKautianAccentTaipakEnabled) {
                        settings.isKautianAccentTaipakEnabled = $0
                    }
                    kautianSubcollToggle(lang.string(.dictionaryKautianAccentGilan), isOn: $isKautianAccentGilanEnabled) {
                        settings.isKautianAccentGilanEnabled = $0
                    }
                    kautianSubcollToggle(lang.string(.dictionaryKautianAccentTainan), isOn: $isKautianAccentTainanEnabled) {
                        settings.isKautianAccentTainanEnabled = $0
                    }
                    kautianSubcollToggle(lang.string(.dictionaryKautianAccentKaohsiung), isOn: $isKautianAccentKaohsiungEnabled) {
                        settings.isKautianAccentKaohsiungEnabled = $0
                    }
                    kautianSubcollToggle(lang.string(.dictionaryKautianAccentKinmen), isOn: $isKautianAccentKinmenEnabled) {
                        settings.isKautianAccentKinmenEnabled = $0
                    }
                    kautianSubcollToggle(lang.string(.dictionaryKautianAccentMakung), isOn: $isKautianAccentMakungEnabled) {
                        settings.isKautianAccentMakungEnabled = $0
                    }
                    kautianSubcollToggle(lang.string(.dictionaryKautianAccentSintik), isOn: $isKautianAccentSintikEnabled) {
                        settings.isKautianAccentSintikEnabled = $0
                    }
                    kautianSubcollToggle(lang.string(.dictionaryKautianAccentTaichung), isOn: $isKautianAccentTaichungEnabled) {
                        settings.isKautianAccentTaichungEnabled = $0
                    }
                    kautianSubcollToggle(lang.string(.dictionaryKautianNameAppendix), isOn: $isKautianNameAppendixEnabled) {
                        settings.isKautianNameAppendixEnabled = $0
                    }
                    dictToggleWithDescription(
                        title: lang.string(.commonNewwordDict),
                        url: "https://www.taigitv.org.tw/taigi-words",
                        isOn: $isNewwordDictEnabled,
                        description: lang.string(.dictionaryNewwordDescription),
                    ) { settings.isNewwordDictEnabled = $0 }
                    dictToggleWithDescription(
                        title: lang.string(.commonSttiDict),
                        url: "https://stti.moe.edu.tw/index.html?lang=sutgi",
                        isOn: $isSttiDictEnabled,
                        description: lang.string(.dictionarySttiDescription),
                    ) { settings.isSttiDictEnabled = $0 }
                    dictToggleWithDescription(
                        title: lang.string(.commonKunggeDict),
                        url: "https://kanggesu.ntcri.org.tw",
                        isOn: $isKunggeDictEnabled,
                        description: lang.string(.dictionaryKunggeDescription),
                    ) { settings.isKunggeDictEnabled = $0 }
                } header: {
                    Text(lang.string(.dictionaryMoeSectionTitle))
                        .font(AppStyle.sectionHeaderFont)
                }

                // Other dictionaries
                Section {
                    dictionaryToggle(lang.string(.commonITaigiDict), isOn: $isITaigiDictEnabled, info: .iTaigi) {
                        settings.isITaigiDictEnabled = $0
                    }
                    dictionaryToggle(lang.string(.commonTaiwanJapanDict), isOn: $isTaiwanJapanDictEnabled, info: .taiwanJapan) {
                        settings.isTaiwanJapanDictEnabled = $0
                    }
                    dictionaryToggle(lang.string(.commonTaiHuaDict), isOn: $isTaiHuaDictEnabled, info: .taiHua) {
                        settings.isTaiHuaDictEnabled = $0
                    }
                    dictionaryToggle(lang.string(.commonTaiwanPlantDict), isOn: $isTaiwanPlantDictEnabled, info: .taiwanPlant) {
                        settings.isTaiwanPlantDictEnabled = $0
                    }
                } header: {
                    Text(lang.string(.dictionaryOtherSectionTitle))
                        .font(AppStyle.sectionHeaderFont)
                }

                // Variant characters / legacy characters / accent data
                Section {
                    dictionaryToggle(lang.string(.dictionaryVariantDictionary), isOn: $isVariantEnabled, info: .variant) {
                        settings.isVariantEnabled = $0
                    }

                    dictionaryToggle(lang.string(.dictionaryKhiin), isOn: $isKhiinEnabled, info: .khiin) {
                        settings.isKhiinEnabled = $0
                    }

                    dictionaryToggle(lang.string(.commonAccentDict), isOn: $isKhpooDictEnabled, info: .khpoo) {
                        settings.isKhpooDictEnabled = $0
                    }

                    dictToggleWithDescription(
                        title: lang.string(.dictionaryLkkDict),
                        url: "https://docs.google.com/spreadsheets/d/1ICPcP3PuEdLirax-HBLtewiOz53KzAfpme9sjmoIO-w/edit?usp=sharing",
                        isOn: $isLkkDictEnabled,
                        description: lang.string(.dictionaryLkkDescription),
                    ) { settings.isLkkDictEnabled = $0 }

                    dictToggleWithDescription(
                        title: lang.string(.dictionaryDevSupplementDict),
                        url: "https://github.com/luke871016/Taigi-Input-method-dictionary-supplement",
                        isOn: $isDevDictEnabled,
                        description: lang.string(.dictionaryDevDescription),
                    ) { settings.isDevDictEnabled = $0 }
                } header: {
                    Text(lang.string(.dictionarySupplementSectionTitle))
                        .font(AppStyle.sectionHeaderFont)
                }
            }
            .navigationTitle(lang.string(TabType.dictionary.titleKey))
            .navigationBarTitleDisplayMode(.large)
            .scrollDismissesKeyboard(.interactively)
            .safeAreaInset(edge: .bottom) {
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
            }
        }
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
                if let hanzi = result.hanzi {
                    Text(hanzi)
                        .foregroundStyle(.primary)
                }
                ForEach(uniqueTagKeys(for: result), id: \.self) { tagKey in
                    Text(lang.string(tagKey))
                        .font(AppStyle.captionFont)
                        .foregroundStyle(.secondary)
                        .padding(.horizontal, 5)
                        .padding(.vertical, 2)
                        .background(Color(.systemGray5))
                        .clipShape(RoundedRectangle(cornerRadius: 4))
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
    // 來源 badge 短標籤的 i18n key — 在此 App 層映射,讓 shared-core enum 不依賴 i18n 型別。
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
        // khpoo/khiin/dev/custom collapse to one "補充資料" badge (reuses the section-title key).
        case .khpoo, .khiin, .dev, .custom: .dictionarySupplementSectionTitle
        }
    }

    /// Deduplicated badge keys for a result, preserving `sources` order. Dedup by
    /// key (not resolved string) so the 4 supplementary sources collapse to one
    /// badge regardless of the active display language.
    // 去重後的 badge key — 以 key 去重(非解析後字串),保留 sources 原排序。
    private func uniqueTagKeys(for result: DictionarySearchResult) -> [StringKey] {
        var seen = Set<StringKey>()
        return result.sources.compactMap { source in
            let key = tagKey(for: source)
            return seen.insert(key).inserted ? key : nil
        }
    }

    // MARK: - Dictionary Toggle with Description + Link

    // 帶外部連結 + 說明文字的詞典開關列(MOE / 補充類詞典使用)。
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

    // 帶資訊按鈕的詞典開關列(顯示 DictionaryInfo.descriptionKey 解析後的彈窗)。
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

    // 教育部辭典底下的巢狀子集開關 — 縮排顯示;父開關 (isMoeDictEnabled) 關閉時整組變灰停用 (DD7)。
    private func kautianSubcollToggle(
        _ title: String,
        isOn: Binding<Bool>,
        onChange: @escaping (Bool) -> Void,
    ) -> some View {
        Toggle(isOn: isOn) {
            Text(title)
                .padding(.leading, 16)
        }
        .disabled(!isMoeDictEnabled)
        .onChange(of: isOn.wrappedValue) { _, newValue in
            onChange(newValue)
        }
    }
}
