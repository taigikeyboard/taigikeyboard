import SwiftUI
import UIKit
import KeyboardKit

/// 設定 Tab
///
/// 鍵盤設定，包含輸入模式、字體、開關選項。
struct Tab4: View {
    @StateObject private var languageManager = LanguageManager.shared
    @StateObject private var fontManager = FontManager.shared

    private let settings = SharedSettings.shared

    @State private var selectedInputMode: InputMode
    @State private var autoSpaceEnabled: Bool
    @State private var enableDoubleTapOO: Bool
    @State private var enableDoubleTapNN: Bool
    @State private var selectedFontType: FontType
    @State private var outputBothScripts: Bool
    @State private var showResetSettingsAlert = false

    // 使用 KeyboardKit 的持久化機制
    @AppStorage(
        "com.keyboardkit.settings.keyboard.isAutocapitalizationEnabled",
        store: UserDefaults(suiteName: SharedSettings.appGroupId)
    )
    private var autoCapitalizationEnabled = true

    #if DEBUG
    @State private var showDebug = false
    #endif

    init() {
        let settings = SharedSettings.shared

        _selectedInputMode = State(initialValue: InputMode(rawValue: settings.inputMode.rawValue) ?? InputMode.tl)
        _autoSpaceEnabled = State(initialValue: settings.isAutoSpaceEnabled)
        _enableDoubleTapOO = State(initialValue: settings.enableDoubleTapOO)
        _enableDoubleTapNN = State(initialValue: settings.enableDoubleTapNN)
        _selectedFontType = State(initialValue: settings.fontType)
        _outputBothScripts = State(initialValue: settings.outputBothScripts)
    }

    var body: some View {
        NavigationStack {
            Form {
                // 輸入模式
                Section(languageManager.text(Tab4Texts.inputMode)) {
                    Picker("", selection: $selectedInputMode) {
                        Text(languageManager.text(Tab4Texts.pojMode)).tag(InputMode.poj)
                        Text(languageManager.text(Tab4Texts.tlMode)).tag(InputMode.tl)
                        Text(languageManager.text(Tab4Texts.englishMode)).tag(InputMode.english)
                    }
                    .pickerStyle(.segmented)
                    .onChange(of: selectedInputMode) { _, newValue in
                        settings.inputMode = newValue
                    }
                }

                // 字體選擇
                Section(languageManager.text(Tab4Texts.customFont)) {
                    Picker("", selection: $selectedFontType) {
                        Text(languageManager.text(Tab4Texts.fontSystemDefault)).tag(FontType.system)
                        Text(languageManager.text(Tab4Texts.fontOpenHuninn)).tag(FontType.openHuninn)
                        Text(languageManager.text(Tab4Texts.fontIansui)).tag(FontType.iansui)
                    }
                    .pickerStyle(.segmented)
                    .onChange(of: selectedFontType) { _, newValue in
                        fontManager.updateFontType(newValue)
                    }
                }

                // 設定開關
                Section {
                    Toggle(languageManager.text(Tab4Texts.outputBothScripts), isOn: $outputBothScripts)
                        .onChange(of: outputBothScripts) { _, newValue in
                            settings.outputBothScripts = newValue
                        }

                    Toggle(languageManager.text(Tab4Texts.autoCapitalization), isOn: $autoCapitalizationEnabled)

                    Toggle(languageManager.text(Tab4Texts.autoSpace), isOn: $autoSpaceEnabled)
                        .onChange(of: autoSpaceEnabled) { _, newValue in
                            settings.isAutoSpaceEnabled = newValue
                        }

                    Toggle(languageManager.text(Tab4Texts.doubleTapOO), isOn: $enableDoubleTapOO)
                        .onChange(of: enableDoubleTapOO) { _, newValue in
                            settings.enableDoubleTapOO = newValue
                        }

                    Toggle(languageManager.text(Tab4Texts.doubleTapNN), isOn: $enableDoubleTapNN)
                        .onChange(of: enableDoubleTapNN) { _, newValue in
                            settings.enableDoubleTapNN = newValue
                        }
                }

                // 重設按鈕
                Section {
                    Button(role: .destructive) {
                        showResetSettingsAlert = true
                    } label: {
                        Text(languageManager.text(Tab4Texts.resetSettings))
                    }
                }

                #if DEBUG
                // Debug 區塊
                Section {
                    Button {
                        showDebug = true
                    } label: {
                        Text(languageManager.text(Tab4Texts.debugMode))
                    }
                }
                #endif
            }
            .navigationTitle(languageManager.text(Tab4Texts.tabTitle))
            .navigationBarTitleDisplayMode(.large)
            .onAppear {
                // 同步 Keyboard Extension 更改的 inputMode
                selectedInputMode = settings.inputMode
            }
        }
        .alert(languageManager.text(Tab4Texts.resetSettings), isPresented: $showResetSettingsAlert) {
            Button(languageManager.text(Tab4Texts.cancel), role: .cancel) {}
            Button(languageManager.text(Tab4Texts.reset), role: .destructive) {
                resetAllSettings()
            }
        } message: {
            Text(languageManager.text(Tab4Texts.resetSettingsMessage))
        }
        #if DEBUG
        .sheet(isPresented: $showDebug) {
            DebugView()
        }
        #endif
    }

    // MARK: - 功能函數

    private func resetAllSettings() {
        settings.resetToDefaults()

        // 清除使用者詞頻資料
        do {
            try UserFrequencyService.deleteUserDatabase()
        } catch {}

        // 清除 NextWord 使用者關聯資料
        do {
            try NextWordService.deleteUserDatabase()
        } catch {}

        // 更新本地狀態
        selectedInputMode = settings.inputMode
        autoCapitalizationEnabled = true  // KeyboardKit 預設值
        autoSpaceEnabled = settings.isAutoSpaceEnabled
        enableDoubleTapOO = settings.enableDoubleTapOO
        enableDoubleTapNN = settings.enableDoubleTapNN
        selectedFontType = settings.fontType
        outputBothScripts = settings.outputBothScripts

        fontManager.reloadFontType()

        let impactFeedback = UIImpactFeedbackGenerator(style: .medium)
        impactFeedback.impactOccurred()
    }
}
