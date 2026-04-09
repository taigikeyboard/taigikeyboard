import SwiftUI
import UIKit

/// Dictionary tab.
///
/// Manage dictionary toggles, custom dictionary, frequency/association data, and search.
struct Tab3: View {
    @StateObject private var languageManager = LanguageManager.shared
    @StateObject private var searchVM = DictionarySearchViewModel()

    private let settings = SharedSettings.shared

    // Dictionary toggle states
    @State private var moeDictEnabled: Bool
    @State private var newwordDictEnabled: Bool
    @State private var kunggeDictEnabled: Bool
    @State private var iTaigiDictEnabled: Bool
    @State private var taiwanJapanDictEnabled: Bool
    @State private var taiHuaDictEnabled: Bool
    @State private var taiwanPlantDictEnabled: Bool
    @State private var sttiDictEnabled: Bool
    @State private var khpooDictEnabled: Bool
    @State private var variantEnabled: Bool
    @State private var khiin: Bool
    @State private var lkkDictEnabled: Bool

    /// Search focus
    @FocusState private var isSearchFocused: Bool

    // Dictionary lookup selection
    @State private var selectedResult: DictionarySearchResult?
    @State private var showLookupDialog = false

    init() {
        let settings = SharedSettings.shared
        _moeDictEnabled = State(initialValue: settings.moeDictEnabled)
        _newwordDictEnabled = State(initialValue: settings.newwordDictEnabled)
        _kunggeDictEnabled = State(initialValue: settings.kunggeDictEnabled)
        _iTaigiDictEnabled = State(initialValue: settings.iTaigiDictEnabled)
        _taiwanJapanDictEnabled = State(initialValue: settings.taiwanJapanDictEnabled)
        _taiHuaDictEnabled = State(initialValue: settings.taiHuaDictEnabled)
        _taiwanPlantDictEnabled = State(initialValue: settings.taiwanPlantDictEnabled)
        _sttiDictEnabled = State(initialValue: settings.sttiDictEnabled)
        _khpooDictEnabled = State(initialValue: settings.khpooDictEnabled)
        _variantEnabled = State(initialValue: settings.variantEnabled)
        _khiin = State(initialValue: settings.khiin)
        _lkkDictEnabled = State(initialValue: settings.lkkDictEnabled)
    }

    var body: some View {
        NavigationStack {
            Form {
                // Data management
                Section {
                    NavigationLink(destination: CustomDictionaryView()) {
                        Text(languageManager.text(Tab3Texts.customDictionary))
                    }
                    NavigationLink(destination: FrequencyDataView()) {
                        Text(languageManager.text(Tab3Texts.frequencyManagement))
                    }
                    NavigationLink(destination: AssociationDataView()) {
                        Text(languageManager.text(Tab3Texts.associationManagement))
                    }
                    NavigationLink(destination: DataManagementView()) {
                        Text(languageManager.text(Tab3Texts.backupRestore))
                    }
                } header: {
                    Text(languageManager.text(Tab3Texts.dataManagement))
                        .font(AppStyle.sectionHeaderFont)
                }

                // MOE dictionaries (教育部)
                Section {
                    dictToggleWithDescription(
                        title: Tab3Texts.moeDict,
                        url: "https://sutian.moe.edu.tw/",
                        isOn: $moeDictEnabled,
                        description: "提供臺灣台語搜尋及華語搜尋，可聆聽詞目和例句發音，方便學習。附有分類索引、部首筆劃索引及附錄。",
                    ) { settings.moeDictEnabled = $0 }
                    dictToggleWithDescription(
                        title: Tab3Texts.newwordDict,
                        url: "https://www.taigitv.org.tw/taigi-words",
                        isOn: $newwordDictEnabled,
                        description: "台語台邀請專家學者，定期召開會議，討論新興詞彙的適當台語講法，建立詞庫予民眾查詢使用。",
                    ) { settings.newwordDictEnabled = $0 }
                    dictToggleWithDescription(
                        title: Tab3Texts.sttiDict,
                        url: "https://stti.moe.edu.tw/index.html?lang=sutgi",
                        isOn: $sttiDictEnabled,
                        description: "於106 年起進行語文、數學、社會、自然科學、藝術、綜合活動、科技、健康與體育等8大領域學科術語之台語編譯。",
                    ) { settings.sttiDictEnabled = $0 }
                    dictToggleWithDescription(
                        title: Tab3Texts.kunggeDict,
                        url: "https://kanggesu.ntcri.org.tw",
                        isOn: $kunggeDictEnabled,
                        description: "收錄多達一千兩百組關鍵台語工藝詞彙，涵蓋陶瓷、木藝、金工、竹藤、纖維、玻璃、漆藝、石藝、皮革、紙藝等十一項工藝類別。",
                    ) { settings.kunggeDictEnabled = $0 }
                } header: {
                    Text(languageManager.text(Tab3Texts.moeSectionTitle))
                        .font(AppStyle.sectionHeaderFont)
                }

                // Other dictionaries
                Section {
                    dictionaryToggle(Tab3Texts.iTaigiDict, isOn: $iTaigiDictEnabled, info: .iTaigi) {
                        settings.iTaigiDictEnabled = $0
                    }
                    dictionaryToggle(Tab3Texts.taiwanJapanDict, isOn: $taiwanJapanDictEnabled, info: .taiwanJapan) {
                        settings.taiwanJapanDictEnabled = $0
                    }
                    dictionaryToggle(Tab3Texts.taiHuaDict, isOn: $taiHuaDictEnabled, info: .taiHua) {
                        settings.taiHuaDictEnabled = $0
                    }
                    dictionaryToggle(Tab3Texts.taiwanPlantDict, isOn: $taiwanPlantDictEnabled, info: .taiwanPlant) {
                        settings.taiwanPlantDictEnabled = $0
                    }
                } header: {
                    Text(languageManager.text(Tab3Texts.otherSectionTitle))
                        .font(AppStyle.sectionHeaderFont)
                }

                // Variant characters / legacy characters / accent data
                Section {
                    dictionaryToggle(Tab3Texts.variantDictionary, isOn: $variantEnabled, info: .variant) {
                        settings.variantEnabled = $0
                    }

                    dictionaryToggle(Tab3Texts.khiin, isOn: $khiin, info: .khiin) {
                        settings.khiin = $0
                    }

                    dictionaryToggle(Tab3Texts.khpooDict, isOn: $khpooDictEnabled, info: .khpoo) {
                        settings.khpooDictEnabled = $0
                    }

                    dictToggleWithDescription(
                        title: Tab3Texts.lkkDict,
                        url: "https://docs.google.com/spreadsheets/d/1ICPcP3PuEdLirax-HBLtewiOz53KzAfpme9sjmoIO-w/edit?usp=sharing",
                        isOn: $lkkDictEnabled,
                        description: "李江却台語文教基金會漢羅合用建議用字。",
                    ) { settings.lkkDictEnabled = $0 }
                } header: {
                    Text(languageManager.text(Tab3Texts.supplementSectionTitle))
                        .font(AppStyle.sectionHeaderFont)
                }
            }
            .navigationTitle(languageManager.text(Tab3Texts.tabTitle))
            .navigationBarTitleDisplayMode(.large)
            .scrollDismissesKeyboard(.interactively)
            .safeAreaInset(edge: .bottom) {
                VStack(spacing: 0) {
                    Divider()
                    // Search results above search bar
                    if !searchVM.searchText.isEmpty {
                        if searchVM.results.isEmpty, !searchVM.isSearching {
                            Text(languageManager.text(Tab3Texts.noResults))
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
                            languageManager.text(Tab3Texts.searchPlaceholder),
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
                    Button(languageManager.text(Tab3Texts.lookupMoe)) {
                        UIApplication.shared.open(moeURL)
                    }
                }
                if let chhoeURL = result.chhoeURL {
                    Button(languageManager.text(Tab3Texts.lookupChhoe)) {
                        UIApplication.shared.open(chhoeURL)
                    }
                }
            }
            Button(languageManager.text(Tab3Texts.cancel), role: .cancel) {}
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
                let uniqueTags: [String] = {
                    var seen = Set<String>()
                    return result.sources.compactMap { source in
                        let name = source.displayName
                        guard !name.isEmpty, seen.insert(name).inserted else { return nil }
                        return name
                    }
                }()
                ForEach(uniqueTags, id: \.self) { tag in
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
        title: LocalizedText,
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
                        Text(languageManager.text(title))
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
        _ text: LocalizedText,
        isOn: Binding<Bool>,
        info: DictionaryInfo,
        onChange: @escaping (Bool) -> Void,
    ) -> some View {
        Toggle(isOn: isOn) {
            HStack {
                Text(languageManager.text(text))
                SettingInfoButton(description: info.description)
            }
        }
        .onChange(of: isOn.wrappedValue) { _, newValue in
            onChange(newValue)
        }
    }
}
