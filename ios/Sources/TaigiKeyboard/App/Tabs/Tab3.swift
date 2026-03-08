import SwiftUI
import UIKit

/// 詞庫 Tab
///
/// 管理詞庫開關和清除使用者學習資料。
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
    @State private var sttiDictEnabled: Bool
    @State private var khpooDictEnabled: Bool
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
        _sttiDictEnabled = State(initialValue: settings.sttiDictEnabled)
        _khpooDictEnabled = State(initialValue: settings.khpooDictEnabled)
        _variantEnabled = State(initialValue: settings.variantEnabled)
    }

    var body: some View {
        NavigationStack {
            Form {
                // 自訂詞庫區塊
                Section {
                    NavigationLink(destination: CustomDictionaryView()) {
                        Text(languageManager.text(Tab3Texts.customDictionary))
                    }
                }

                // 詞庫列表
                Section {
                    Toggle(languageManager.text(Tab3Texts.moeDict), isOn: $moeDictEnabled)
                        .onChange(of: moeDictEnabled) { _, newValue in
                            settings.moeDictEnabled = newValue
                        }

                    Toggle(languageManager.text(Tab3Texts.sttiDict), isOn: $sttiDictEnabled)
                        .onChange(of: sttiDictEnabled) { _, newValue in
                            settings.sttiDictEnabled = newValue
                        }

                    Toggle(languageManager.text(Tab3Texts.newwordDict), isOn: $newwordDictEnabled)
                        .onChange(of: newwordDictEnabled) { _, newValue in
                            settings.newwordDictEnabled = newValue
                        }

                    Toggle(languageManager.text(Tab3Texts.kunggeDict), isOn: $kunggeDictEnabled)
                        .onChange(of: kunggeDictEnabled) { _, newValue in
                            settings.kunggeDictEnabled = newValue
                        }

                    Toggle(languageManager.text(Tab3Texts.iTaigiDict), isOn: $iTaigiDictEnabled)
                        .onChange(of: iTaigiDictEnabled) { _, newValue in
                            settings.iTaigiDictEnabled = newValue
                        }

                    Toggle(languageManager.text(Tab3Texts.taiwanJapanDict), isOn: $taiwanJapanDictEnabled)
                        .onChange(of: taiwanJapanDictEnabled) { _, newValue in
                            settings.taiwanJapanDictEnabled = newValue
                        }

                    Toggle(languageManager.text(Tab3Texts.taiHuaDict), isOn: $taiHuaDictEnabled)
                        .onChange(of: taiHuaDictEnabled) { _, newValue in
                            settings.taiHuaDictEnabled = newValue
                        }

                    Toggle(languageManager.text(Tab3Texts.taiwanPlantDict), isOn: $taiwanPlantDictEnabled)
                        .onChange(of: taiwanPlantDictEnabled) { _, newValue in
                            settings.taiwanPlantDictEnabled = newValue
                        }

                    Toggle(languageManager.text(Tab3Texts.khpooDict), isOn: $khpooDictEnabled)
                        .onChange(of: khpooDictEnabled) { _, newValue in
                            settings.khpooDictEnabled = newValue
                        }
                }

                // 異用字開關
                Section {
                    Toggle(languageManager.text(Tab3Texts.variantDictionary), isOn: $variantEnabled)
                        .onChange(of: variantEnabled) { _, newValue in
                            settings.variantEnabled = newValue
                        }
                }

                // 清除資料區塊
                Section {
                    Button(role: .destructive) {
                        showClearCacheAlert = true
                    } label: {
                        Text(languageManager.text(Tab3Texts.clearCache))
                    }
                }
            }
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
