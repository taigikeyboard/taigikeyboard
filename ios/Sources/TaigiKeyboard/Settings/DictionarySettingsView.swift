import SwiftUI
import UIKit

/// 詞庫管理頁面視圖
struct DictionarySettingsView: View {
    @Environment(\.dismiss) var dismiss
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
        SettingsPageView(
            onDismiss: { dismiss() }
        ) {
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
                SettingsSection(titleContent: AppTexts.customDictionary) {
                    customDictionaryPlaceholder
                }

                // 清除資料區塊
                SettingsSection {
                    actionButtonsSection
                }
            }
        }
        .alert(languageManager.text(AppTexts.clearCache), isPresented: $showClearCacheAlert) {
            Button(languageManager.text(AppTexts.cancel), role: .cancel) {}
            Button(languageManager.text(AppTexts.clear), role: .destructive) {
                clearUserFrequencyDatabase()
            }
        } message: {
            Text(languageManager.text(AppTexts.clearCacheMessage))
        }
    }

    // MARK: - 詞庫開關列表

    @ViewBuilder
    private var dictionaryToggles: some View {
        // 1. 教育部臺灣台語常用詞辭典
        SettingsToggleItem(
            titleContent: CopyrightTexts.moeDict,
            isOn: $moeDictEnabled,
            isFirst: true,
            onChange: { newValue in
                settings.moeDictEnabled = newValue
            }
        )

        // 2. 台語新詞辭庫
        SettingsToggleItem(
            titleContent: CopyrightTexts.newwordDict,
            isOn: $newwordDictEnabled,
            onChange: { newValue in
                settings.newwordDictEnabled = newValue
            }
        )

        // 3. 台語工藝詞庫
        SettingsToggleItem(
            titleContent: CopyrightTexts.kunggeDict,
            isOn: $kunggeDictEnabled,
            onChange: { newValue in
                settings.kunggeDictEnabled = newValue
            }
        )

        // 4. iTaigi 華台對照典
        SettingsToggleItem(
            titleContent: CopyrightTexts.iTaigiDict,
            isOn: $iTaigiDictEnabled,
            onChange: { newValue in
                settings.iTaigiDictEnabled = newValue
            }
        )

        // 5. 台日大辭典
        SettingsToggleItem(
            titleContent: CopyrightTexts.taiwanJapanDict,
            isOn: $taiwanJapanDictEnabled,
            onChange: { newValue in
                settings.taiwanJapanDictEnabled = newValue
            }
        )

        // 6. 台華線頂對照典
        SettingsToggleItem(
            titleContent: CopyrightTexts.taiHuaDict,
            isOn: $taiHuaDictEnabled,
            onChange: { newValue in
                settings.taiHuaDictEnabled = newValue
            }
        )

        // 7. 台灣植物名彙
        SettingsToggleItem(
            titleContent: CopyrightTexts.taiwanPlantDict,
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
            titleContent: AppTexts.variantDictionary,
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
            LocalizedTextView(AppTexts.comingSoon)
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
            titleContent: AppTexts.clearCache,
            isFirst: true,
            isLast: true,
            action: { showClearCacheAlert = true }
        )
    }

    /// 清除使用者頻率資料庫
    private func clearUserFrequencyDatabase() {
        do {
            try UserFrequencyService.deleteUserDatabase()

            let impactFeedback = UIImpactFeedbackGenerator(style: .medium)
            impactFeedback.impactOccurred()
        } catch {}
    }
}
