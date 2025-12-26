import SwiftUI
import UIKit

/// Tab3: 詞庫
/// 內容：詞庫管理（整合自 DictionarySettingsView）
struct Tab3: View {
    @StateObject private var languageManager = LanguageManager.shared

    private let settings = SharedSettings.shared

    // 詞庫開關狀態
    @State private var moeDictEnabled: Bool
    @State private var newwordDictEnabled: Bool
    @State private var kunggeDictEnabled: Bool
    @State private var iTaigiDictEnabled: Bool
    @State private var taiwanJapanDictEnabled: Bool
    @State private var taiHuaDictEnabled: Bool
    @State private var taiwanPlantDictEnabled: Bool
    @State private var variantEnabled: Bool

    // 清除資料 Alert
    @State private var showClearCacheAlert = false

    init() {
        let settings = SharedSettings.shared
        _moeDictEnabled = State(initialValue: settings.moeDictEnabled)
        _newwordDictEnabled = State(initialValue: settings.newwordDictEnabled)
        _kunggeDictEnabled = State(initialValue: settings.kunggeDictEnabled)
        _iTaigiDictEnabled = State(initialValue: settings.iTaigiDictEnabled)
        _taiwanJapanDictEnabled = State(initialValue: settings.taiwanJapanDictEnabled)
        _taiHuaDictEnabled = State(initialValue: settings.taiHuaDictEnabled)
        _taiwanPlantDictEnabled = State(initialValue: settings.taiwanPlantDictEnabled)
        _variantEnabled = State(initialValue: settings.variantEnabled)
    }

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(spacing: 24) {
                    // 詞庫列表
                    SettingsSection {
                        dictionaryToggles
                    }

                    // 異用字開關
                    SettingsSection {
                        variantToggle
                    }

                    // 自訂詞庫區塊
                    SettingsSection(titleContent: Tab3Texts.customDictionary) {
                        customDictionaryPlaceholder
                    }

                    // 清除資料區塊
                    SettingsSection {
                        actionButtonsSection
                    }
                }
                .padding(.horizontal, 20)
                .padding(.vertical, 16)
            }
            .background(Color.Theme.surfacePrimary)
            .navigationTitle(languageManager.text(Tab3Texts.tabTitle))
            .navigationBarTitleDisplayMode(.large)
        }
        .alert(languageManager.text(Tab3Texts.clearCache), isPresented: $showClearCacheAlert) {
            Button(languageManager.text(Tab3Texts.cancel), role: .cancel) {}
            Button(languageManager.text(Tab3Texts.clear), role: .destructive) {
                clearUserFrequencyDatabase()
            }
        } message: {
            Text(languageManager.text(Tab3Texts.clearCacheMessage))
        }
    }

    // MARK: - 詞庫開關列表

    @ViewBuilder
    private var dictionaryToggles: some View {
        // 1. 教育部臺灣台語常用詞辭典
        SettingsToggleItem(
            titleContent: Tab3Texts.moeDict,
            isOn: $moeDictEnabled,
            isFirst: true,
            onChange: { newValue in
                settings.moeDictEnabled = newValue
            }
        )

        // 2. 台語新詞辭庫
        SettingsToggleItem(
            titleContent: Tab3Texts.newwordDict,
            isOn: $newwordDictEnabled,
            onChange: { newValue in
                settings.newwordDictEnabled = newValue
            }
        )

        // 3. 台語工藝詞庫
        SettingsToggleItem(
            titleContent: Tab3Texts.kunggeDict,
            isOn: $kunggeDictEnabled,
            onChange: { newValue in
                settings.kunggeDictEnabled = newValue
            }
        )

        // 4. iTaigi 華台對照典
        SettingsToggleItem(
            titleContent: Tab3Texts.iTaigiDict,
            isOn: $iTaigiDictEnabled,
            onChange: { newValue in
                settings.iTaigiDictEnabled = newValue
            }
        )

        // 5. 台日大辭典
        SettingsToggleItem(
            titleContent: Tab3Texts.taiwanJapanDict,
            isOn: $taiwanJapanDictEnabled,
            onChange: { newValue in
                settings.taiwanJapanDictEnabled = newValue
            }
        )

        // 6. 台華線頂對照典
        SettingsToggleItem(
            titleContent: Tab3Texts.taiHuaDict,
            isOn: $taiHuaDictEnabled,
            onChange: { newValue in
                settings.taiHuaDictEnabled = newValue
            }
        )

        // 7. 台灣植物名彙
        SettingsToggleItem(
            titleContent: Tab3Texts.taiwanPlantDict,
            isOn: $taiwanPlantDictEnabled,
            isLast: true,
            onChange: { newValue in
                settings.taiwanPlantDictEnabled = newValue
            }
        )
    }

    // MARK: - 異用字開關

    @ViewBuilder
    private var variantToggle: some View {
        SettingsToggleItem(
            titleContent: Tab3Texts.variantDictionary,
            isOn: $variantEnabled,
            isFirst: true,
            isLast: true,
            onChange: { newValue in
                settings.variantEnabled = newValue
            }
        )
    }

    // MARK: - 自訂詞庫區塊

    @ViewBuilder
    private var customDictionaryPlaceholder: some View {
        HStack {
            LocalizedTextView(Tab3Texts.comingSoon)
                .themeFontBody()
                .foregroundColor(Color.Theme.textSecondary)
                .frame(maxWidth: .infinity, alignment: .center)
        }
        .padding(.horizontal, 20)
        .padding(.vertical, 24)
    }

    // MARK: - 清除資料區塊

    @ViewBuilder
    private var actionButtonsSection: some View {
        SettingsActionButton(
            titleContent: Tab3Texts.clearCache,
            isFirst: true,
            isLast: true,
            action: { showClearCacheAlert = true }
        )
    }

    /// 清除使用者學習資料（詞頻 + 詞關聯）
    private func clearUserFrequencyDatabase() {
        do {
            // 刪除 User Frequency
            try UserFrequencyService.deleteUserDatabase()
            // 刪除 User Association（比照 Android）
            try NextWordService.deleteUserDatabase()

            let impactFeedback = UIImpactFeedbackGenerator(style: .medium)
            impactFeedback.impactOccurred()
        } catch {}
    }
}
