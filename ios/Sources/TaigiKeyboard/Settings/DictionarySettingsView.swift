import SwiftUI

/// 詞庫管理頁面視圖
struct DictionarySettingsView: View {
    @Environment(\.dismiss) var dismiss
    @StateObject private var languageManager = LanguageManager.shared

    private let settings = SharedSettings.shared

    // 異用字搜尋（實際功能）
    @State private var variantSearchEnabled: Bool

    // 詞庫開關狀態（僅 UI，不實作功能）
    @State private var moeDictEnabled = true
    @State private var newwordDictEnabled = true
    @State private var iTaigiDictEnabled = true
    @State private var taiwanPlantDictEnabled = true
    @State private var taiHuaDictEnabled = true
    @State private var taiwanJapanDictEnabled = true

    init() {
        let settings = SharedSettings.shared
        _variantSearchEnabled = State(initialValue: settings.variantSearchEnabled)
    }

    var body: some View {
        SettingsPageView(
            onDismiss: { dismiss() }
        ) {
            VStack(spacing: 24) {
                // 詞庫列表
                SettingsSection {
                    dictionaryToggles
                }

                // 搜尋設定區塊
                SettingsSection {
                    searchSettingsSection
                }

                // 自訂詞庫區塊
                SettingsSection(titleContent: AppTexts.customDictionary) {
                    customDictionaryPlaceholder
                }
            }
        }
    }

    // MARK: - 搜尋設定區塊

    @ViewBuilder
    private var searchSettingsSection: some View {
        // 異用字搜尋
        SettingsToggleItem(
            titleContent: AppTexts.variantSearch,
            isOn: $variantSearchEnabled,
            isFirst: true,
            isLast: true,
            onChange: { newValue in
                settings.variantSearchEnabled = newValue
            }
        )
    }

    // MARK: - 詞庫開關列表

    @ViewBuilder
    private var dictionaryToggles: some View {
        // 教育部臺灣台語常用詞辭典
        SettingsToggleItem(
            titleContent: CopyrightTexts.moeDict,
            isOn: $moeDictEnabled,
            isFirst: true,
            onChange: { _ in
                // TODO: 實作詞庫開關功能
            }
        )

        // 台語新詞辭庫
        SettingsToggleItem(
            titleContent: CopyrightTexts.newwordDict,
            isOn: $newwordDictEnabled,
            onChange: { _ in
                // TODO: 實作詞庫開關功能
            }
        )

        // iTaigi 華台對照典
        SettingsToggleItem(
            titleContent: CopyrightTexts.iTaigiDict,
            isOn: $iTaigiDictEnabled,
            onChange: { _ in
                // TODO: 實作詞庫開關功能
            }
        )

        // 台灣植物名彙
        SettingsToggleItem(
            titleContent: CopyrightTexts.taiwanPlantDict,
            isOn: $taiwanPlantDictEnabled,
            onChange: { _ in
                // TODO: 實作詞庫開關功能
            }
        )

        // 台華線頂對照典
        SettingsToggleItem(
            titleContent: CopyrightTexts.taiHuaDict,
            isOn: $taiHuaDictEnabled,
            onChange: { _ in
                // TODO: 實作詞庫開關功能
            }
        )

        // 台日大辭典
        SettingsToggleItem(
            titleContent: CopyrightTexts.taiwanJapanDict,
            isOn: $taiwanJapanDictEnabled,
            isLast: true,
            onChange: { _ in
                // TODO: 實作詞庫開關功能
            }
        )
    }

    // MARK: - 自訂詞庫區塊

    @ViewBuilder
    private var customDictionaryPlaceholder: some View {
        HStack {
            LocalizedTextView(AppTexts.comingSoon)
                .themeFontBody()
                .foregroundColor(Color.Theme.textSecondary)
                .frame(maxWidth: .infinity, alignment: .center)
        }
        .padding(.horizontal, 20)
        .padding(.vertical, 24)
    }
}
