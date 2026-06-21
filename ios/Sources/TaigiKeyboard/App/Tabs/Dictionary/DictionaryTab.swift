// 中文: Dictionary 分頁主畫面 — 詞典開關、自訂詞庫、頻率/聯想資料管理、跨詞典搜尋。
// 中文: 各 NavigationLink 子畫面分別由 CustomDictionaryView / FrequencyDataView /
// 中文: AssociationDataView / DataManagementView 處理。

import SwiftUI
import UIKit

/// Dictionary tab.
///
/// Manage dictionary toggles, custom dictionary, frequency/association data, and search.
// 中文: Dictionary 分頁的根 View,組裝詞典開關區塊、底部搜尋列與查詢結果浮層。
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
                        Text(DictionaryTexts.customDictionary)
                    }
                    NavigationLink(destination: FrequencyDataView()) {
                        Text(DictionaryTexts.frequencyManagement)
                    }
                    NavigationLink(destination: AssociationDataView()) {
                        Text(DictionaryTexts.associationManagement)
                    }
                    NavigationLink(destination: DataManagementView()) {
                        Text(DictionaryTexts.backupRestore)
                    }
                } header: {
                    Text(DictionaryTexts.dataManagement)
                        .font(AppStyle.sectionHeaderFont)
                }

                // MOE dictionaries (教育部)
                Section {
                    dictToggleWithDescription(
                        title: lang.string(.commonMoeDict),
                        url: "https://sutian.moe.edu.tw/",
                        isOn: $isMoeDictEnabled,
                        description: "提供臺灣台語搜尋及華語搜尋，可聆聽詞目和例句發音，方便學習。附有分類索引、部首筆劃索引及附錄。",
                    ) { settings.isMoeDictEnabled = $0 }
                    // Kautian subcollections — nested under the master row,
                    // greyed when the MOE/kautian master is off (DD7).
                    kautianSubcollToggle(DictionaryTexts.kautianAccentLukang, isOn: $isKautianAccentLukangEnabled) {
                        settings.isKautianAccentLukangEnabled = $0
                    }
                    kautianSubcollToggle(DictionaryTexts.kautianAccentSansia, isOn: $isKautianAccentSansiaEnabled) {
                        settings.isKautianAccentSansiaEnabled = $0
                    }
                    kautianSubcollToggle(DictionaryTexts.kautianAccentTaipak, isOn: $isKautianAccentTaipakEnabled) {
                        settings.isKautianAccentTaipakEnabled = $0
                    }
                    kautianSubcollToggle(DictionaryTexts.kautianAccentGilan, isOn: $isKautianAccentGilanEnabled) {
                        settings.isKautianAccentGilanEnabled = $0
                    }
                    kautianSubcollToggle(DictionaryTexts.kautianAccentTainan, isOn: $isKautianAccentTainanEnabled) {
                        settings.isKautianAccentTainanEnabled = $0
                    }
                    kautianSubcollToggle(DictionaryTexts.kautianAccentKaohsiung, isOn: $isKautianAccentKaohsiungEnabled) {
                        settings.isKautianAccentKaohsiungEnabled = $0
                    }
                    kautianSubcollToggle(DictionaryTexts.kautianAccentKinmen, isOn: $isKautianAccentKinmenEnabled) {
                        settings.isKautianAccentKinmenEnabled = $0
                    }
                    kautianSubcollToggle(DictionaryTexts.kautianAccentMakung, isOn: $isKautianAccentMakungEnabled) {
                        settings.isKautianAccentMakungEnabled = $0
                    }
                    kautianSubcollToggle(DictionaryTexts.kautianAccentSintik, isOn: $isKautianAccentSintikEnabled) {
                        settings.isKautianAccentSintikEnabled = $0
                    }
                    kautianSubcollToggle(DictionaryTexts.kautianAccentTaichung, isOn: $isKautianAccentTaichungEnabled) {
                        settings.isKautianAccentTaichungEnabled = $0
                    }
                    kautianSubcollToggle(DictionaryTexts.kautianNameAppendix, isOn: $isKautianNameAppendixEnabled) {
                        settings.isKautianNameAppendixEnabled = $0
                    }
                    dictToggleWithDescription(
                        title: lang.string(.commonNewwordDict),
                        url: "https://www.taigitv.org.tw/taigi-words",
                        isOn: $isNewwordDictEnabled,
                        description: "台語台邀請專家學者，定期召開會議，討論新興詞彙的適當台語講法，建立詞庫予民眾查詢使用。",
                    ) { settings.isNewwordDictEnabled = $0 }
                    dictToggleWithDescription(
                        title: lang.string(.commonSttiDict),
                        url: "https://stti.moe.edu.tw/index.html?lang=sutgi",
                        isOn: $isSttiDictEnabled,
                        description: "於106 年起進行語文、數學、社會、自然科學、藝術、綜合活動、科技、健康與體育等8大領域學科術語之台語編譯。",
                    ) { settings.isSttiDictEnabled = $0 }
                    dictToggleWithDescription(
                        title: lang.string(.commonKunggeDict),
                        url: "https://kanggesu.ntcri.org.tw",
                        isOn: $isKunggeDictEnabled,
                        description: "收錄多達一千兩百組關鍵台語工藝詞彙，涵蓋陶瓷、木藝、金工、竹藤、纖維、玻璃、漆藝、石藝、皮革、紙藝等十一項工藝類別。",
                    ) { settings.isKunggeDictEnabled = $0 }
                } header: {
                    Text(DictionaryTexts.moeSectionTitle)
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
                    Text(DictionaryTexts.otherSectionTitle)
                        .font(AppStyle.sectionHeaderFont)
                }

                // Variant characters / legacy characters / accent data
                Section {
                    dictionaryToggle(DictionaryTexts.variantDictionary, isOn: $isVariantEnabled, info: .variant) {
                        settings.isVariantEnabled = $0
                    }

                    dictionaryToggle(DictionaryTexts.khiin, isOn: $isKhiinEnabled, info: .khiin) {
                        settings.isKhiinEnabled = $0
                    }

                    dictionaryToggle(lang.string(.commonAccentDict), isOn: $isKhpooDictEnabled, info: .khpoo) {
                        settings.isKhpooDictEnabled = $0
                    }

                    dictToggleWithDescription(
                        title: DictionaryTexts.lkkDict,
                        url: "https://docs.google.com/spreadsheets/d/1ICPcP3PuEdLirax-HBLtewiOz53KzAfpme9sjmoIO-w/edit?usp=sharing",
                        isOn: $isLkkDictEnabled,
                        description: "李江却台語文教基金會漢羅合用建議用字。",
                    ) { settings.isLkkDictEnabled = $0 }

                    dictToggleWithDescription(
                        title: DictionaryTexts.devSupplementDict,
                        url: "https://github.com/luke871016/Taigi-Input-method-dictionary-supplement",
                        isOn: $isDevDictEnabled,
                        description: "一府五院、菜市仔名、台/臺、教典僻智識、數字時間日期、行政區。",
                    ) { settings.isDevDictEnabled = $0 }
                } header: {
                    Text(DictionaryTexts.supplementSectionTitle)
                        .font(AppStyle.sectionHeaderFont)
                }
            }
            .navigationTitle(DictionaryTexts.tabTitle)
            .navigationBarTitleDisplayMode(.large)
            .scrollDismissesKeyboard(.interactively)
            .safeAreaInset(edge: .bottom) {
                VStack(spacing: 0) {
                    Divider()
                    // Search results above search bar
                    if !searchVM.searchText.isEmpty {
                        if searchVM.results.isEmpty, !searchVM.isSearching {
                            Text(DictionaryTexts.noResults)
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
                            DictionaryTexts.searchPlaceholder,
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
                    Button(DictionaryTexts.lookupMoe) {
                        UIApplication.shared.open(moeURL)
                    }
                }
                if let chhoeURL = result.chhoeURL {
                    Button(DictionaryTexts.lookupChhoe) {
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
                ForEach(result.uniqueTagNames, id: \.self) { tag in
                    Text(tag)
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

    // MARK: - Dictionary Toggle with Description + Link

    // 中文: 帶外部連結 + 說明文字的詞典開關列(MOE / 補充類詞典使用)。
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

    // 中文: 帶資訊按鈕的詞典開關列(顯示 DictionaryInfo.description 彈窗)。
    private func dictionaryToggle(
        _ text: String,
        isOn: Binding<Bool>,
        info: DictionaryInfo,
        onChange: @escaping (Bool) -> Void,
    ) -> some View {
        Toggle(isOn: isOn) {
            HStack {
                Text(text)
                SettingInfoButton(description: info.description)
            }
        }
        .onChange(of: isOn.wrappedValue) { _, newValue in
            onChange(newValue)
        }
    }

    // MARK: - Kautian Subcollection Toggle (nested, dependent on master)

    // 中文: 教育部辭典底下的巢狀子集開關 — 縮排顯示;父開關 (isMoeDictEnabled) 關閉時整組變灰停用 (DD7)。
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
