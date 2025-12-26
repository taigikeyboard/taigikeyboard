import SwiftUI
import UIKit

/// Tab4: 設定
/// 內容：鍵盤設定（整合自 SettingsView）
struct Tab4: View {
    @StateObject private var languageManager = LanguageManager.shared
    @StateObject private var fontManager = FontManager.shared

    private let settings = SharedSettings.shared

    @State private var selectedInputMode: InputMode
    @State private var autoCapitalizationEnabled: Bool
    @State private var autoSpaceEnabled: Bool
    @State private var enableDoubleTapOO: Bool
    @State private var enableDoubleTapNN: Bool
    @State private var selectedFontType: FontType
    @State private var outputBothScripts: Bool
    @State private var showResetSettingsAlert = false
    #if DEBUG
    @State private var showDebug = false
    #endif

    init() {
        let settings = SharedSettings.shared

        _selectedInputMode = State(initialValue: InputMode(rawValue: settings.inputMode.rawValue) ?? InputMode.tl)
        _autoCapitalizationEnabled = State(initialValue: settings.isAutoCapitalizationEnabled)
        _autoSpaceEnabled = State(initialValue: settings.isAutoSpaceEnabled)
        _enableDoubleTapOO = State(initialValue: settings.enableDoubleTapOO)
        _enableDoubleTapNN = State(initialValue: settings.enableDoubleTapNN)
        _selectedFontType = State(initialValue: settings.fontType)
        _outputBothScripts = State(initialValue: settings.outputBothScripts)
    }

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(spacing: 24) {
                    inputModeSection

                    fontTypeSection

                    SettingsSection {
                        settingsSection
                    }

                    SettingsSection {
                        actionButtonsSection
                    }

                    #if DEBUG
                    SettingsSection {
                        debugSection
                    }
                    #endif
                }
                .padding(.horizontal, 20)
                .padding(.vertical, 16)
            }
            .background(Color.Theme.surfacePrimary)
            .navigationTitle(languageManager.text(Tab4Texts.tabTitle))
            .navigationBarTitleDisplayMode(.large)
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

    // MARK: - 輸入模式

    @ViewBuilder
    private var inputModeSection: some View {
        VStack(spacing: 20) {
            HStack {
                LocalizedTextView(Tab4Texts.inputMode)
                    .themeFontHeadline()
                    .foregroundColor(Color.Theme.textPrimary)

                Spacer()
            }

            Picker("", selection: $selectedInputMode) {
                LocalizedTextView(Tab4Texts.pojMode).tag(InputMode.poj)
                LocalizedTextView(Tab4Texts.tlMode).tag(InputMode.tl)
            }
            .pickerStyle(SegmentedPickerStyle())
            .id("inputMode-\(fontManager.currentFontType)")
            .onAppear {
                updateSegmentedControlAppearance()
            }
            .onChange(of: fontManager.currentFontType) { _, _ in
                updateSegmentedControlAppearance()
            }
            .onChange(of: selectedInputMode) { _, newValue in
                settings.inputMode = InputMode(rawValue: newValue.rawValue) ?? InputMode.poj
            }
        }
        .padding(.vertical, 24)
        .padding(.horizontal, 24)
        .themedCard()
    }

    // MARK: - 字體選擇

    @ViewBuilder
    private var fontTypeSection: some View {
        VStack(spacing: 20) {
            HStack {
                LocalizedTextView(Tab4Texts.customFont)
                    .themeFontHeadline()
                    .foregroundColor(Color.Theme.textPrimary)

                Spacer()
            }

            Picker("", selection: $selectedFontType) {
                LocalizedTextView(Tab4Texts.fontSystemDefault).tag(FontType.system)
                LocalizedTextView(Tab4Texts.fontOpenHuninn).tag(FontType.openHuninn)
                LocalizedTextView(Tab4Texts.fontIansui).tag(FontType.iansui)
            }
            .pickerStyle(SegmentedPickerStyle())
            .id("fontType-\(fontManager.currentFontType)")
            .onAppear {
                updateSegmentedControlAppearance()
            }
            .onChange(of: selectedFontType) { _, newValue in
                fontManager.updateFontType(newValue)
            }
        }
        .padding(.vertical, 24)
        .padding(.horizontal, 24)
        .themedCard()
    }

    // MARK: - 設定開關

    @ViewBuilder
    private var settingsSection: some View {
        SettingsToggleItem(
            titleContent: Tab4Texts.outputBothScripts,
            isOn: $outputBothScripts,
            isFirst: true,
            onChange: { newValue in
                settings.outputBothScripts = newValue
            }
        )

        SettingsToggleItem(
            titleContent: Tab4Texts.autoCapitalization,
            isOn: $autoCapitalizationEnabled,
            onChange: { newValue in
                settings.isAutoCapitalizationEnabled = newValue
            }
        )

        SettingsToggleItem(
            titleContent: Tab4Texts.autoSpace,
            isOn: $autoSpaceEnabled,
            onChange: { newValue in
                settings.isAutoSpaceEnabled = newValue
            }
        )

        SettingsToggleItem(
            titleContent: Tab4Texts.doubleTapOO,
            isOn: $enableDoubleTapOO,
            onChange: { newValue in
                settings.enableDoubleTapOO = newValue
            }
        )

        SettingsToggleItem(
            titleContent: Tab4Texts.doubleTapNN,
            isOn: $enableDoubleTapNN,
            isLast: true,
            onChange: { newValue in
                settings.enableDoubleTapNN = newValue
            }
        )
    }

    // MARK: - 重設按鈕

    @ViewBuilder
    private var actionButtonsSection: some View {
        SettingsActionButton(
            titleContent: Tab4Texts.resetSettings,
            isFirst: true,
            isLast: true,
            action: { showResetSettingsAlert = true }
        )
    }

    // MARK: - Debug 區塊

    #if DEBUG
    @ViewBuilder
    private var debugSection: some View {
        SettingsActionButton(
            titleContent: Tab4Texts.debugMode,
            isFirst: true,
            isLast: true,
            action: { showDebug = true }
        )
    }
    #endif

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

        selectedInputMode = settings.inputMode
        autoCapitalizationEnabled = settings.isAutoCapitalizationEnabled
        autoSpaceEnabled = settings.isAutoSpaceEnabled
        enableDoubleTapOO = settings.enableDoubleTapOO
        enableDoubleTapNN = settings.enableDoubleTapNN
        selectedFontType = settings.fontType
        outputBothScripts = settings.outputBothScripts

        fontManager.reloadFontType()

        let impactFeedback = UIImpactFeedbackGenerator(style: .medium)
        impactFeedback.impactOccurred()
    }

    private func updateSegmentedControlAppearance() {
        let font = KeyboardModels.Fonts.uiFont(for: fontManager.currentFontType, size: 13)
        UISegmentedControl.appearance().selectedSegmentTintColor = UIColor(Color.Theme.accent)
        UISegmentedControl.appearance().setTitleTextAttributes([
            .foregroundColor: UIColor.white,
            .font: font
        ], for: .selected)
        UISegmentedControl.appearance().setTitleTextAttributes([
            .foregroundColor: UIColor(Color.Theme.textPrimary),
            .font: font
        ], for: .normal)
    }
}
