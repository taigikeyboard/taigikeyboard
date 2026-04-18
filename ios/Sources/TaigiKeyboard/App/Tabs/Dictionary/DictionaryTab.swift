import SwiftUI
import UIKit

/// Dictionary tab.
///
/// Manage dictionary toggles, custom dictionary, frequency/association data, and search.
struct DictionaryTab: View {
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
                        title: CommonTexts.moeDict,
                        url: "https://sutian.moe.edu.tw/",
                        isOn: $isMoeDictEnabled,
                        description: "提供臺灣台語搜尋及華語搜尋，可聆聽詞目和例句發音，方便學習。附有分類索引、部首筆劃索引及附錄。",
                    ) { settings.isMoeDictEnabled = $0 }
                    dictToggleWithDescription(
                        title: CommonTexts.newwordDict,
                        url: "https://www.taigitv.org.tw/taigi-words",
                        isOn: $isNewwordDictEnabled,
                        description: "台語台邀請專家學者，定期召開會議，討論新興詞彙的適當台語講法，建立詞庫予民眾查詢使用。",
                    ) { settings.isNewwordDictEnabled = $0 }
                    dictToggleWithDescription(
                        title: CommonTexts.sttiDict,
                        url: "https://stti.moe.edu.tw/index.html?lang=sutgi",
                        isOn: $isSttiDictEnabled,
                        description: "於106 年起進行語文、數學、社會、自然科學、藝術、綜合活動、科技、健康與體育等8大領域學科術語之台語編譯。",
                    ) { settings.isSttiDictEnabled = $0 }
                    dictToggleWithDescription(
                        title: CommonTexts.kunggeDict,
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
                    dictionaryToggle(CommonTexts.iTaigiDict, isOn: $isITaigiDictEnabled, info: .iTaigi) {
                        settings.isITaigiDictEnabled = $0
                    }
                    dictionaryToggle(CommonTexts.taiwanJapanDict, isOn: $isTaiwanJapanDictEnabled, info: .taiwanJapan) {
                        settings.isTaiwanJapanDictEnabled = $0
                    }
                    dictionaryToggle(CommonTexts.taiHuaDict, isOn: $isTaiHuaDictEnabled, info: .taiHua) {
                        settings.isTaiHuaDictEnabled = $0
                    }
                    dictionaryToggle(CommonTexts.taiwanPlantDict, isOn: $isTaiwanPlantDictEnabled, info: .taiwanPlant) {
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

                    dictionaryToggle(CommonTexts.accentDict, isOn: $isKhpooDictEnabled, info: .khpoo) {
                        settings.isKhpooDictEnabled = $0
                    }

                    dictToggleWithDescription(
                        title: DictionaryTexts.lkkDict,
                        url: "https://docs.google.com/spreadsheets/d/1ICPcP3PuEdLirax-HBLtewiOz53KzAfpme9sjmoIO-w/edit?usp=sharing",
                        isOn: $isLkkDictEnabled,
                        description: "李江却台語文教基金會漢羅合用建議用字。",
                    ) { settings.isLkkDictEnabled = $0 }
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
                        Image(systemName: "magnifyingglass")
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
                                Image(systemName: "xmark.circle.fill")
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
            Button(CommonTexts.cancel, role: .cancel) {}
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
                Image(systemName: "arrow.up.right")
                    .font(AppStyle.captionFont)
                    .foregroundStyle(.secondary)
            }
            .padding(.horizontal, AppStyle.horizontalPadding)
            .padding(.vertical, 10)
        }
        .buttonStyle(.plain)
    }

    // MARK: - Dictionary Toggle with Description + Link

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
                        Image(systemName: "arrow.up.forward.square")
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
}
