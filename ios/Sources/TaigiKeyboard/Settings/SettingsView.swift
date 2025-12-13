import SwiftUI
import UIKit

/// 設定頁面視圖
struct SettingsView: View {
    @Environment(\.dismiss) var dismiss
    @State private var selectedInputMode: InputMode = .tl

    private let settings = SharedSettings.shared

    @State private var autoCapitalizationEnabled: Bool
    @State private var autoSpaceEnabled: Bool
    @State private var enableDoubleTapOO: Bool
    @State private var enableDoubleTapNN: Bool
    @State private var phahTaigiLayoutEnabled: Bool
    @State private var selectedFontType: FontType
    @State private var outputBothScripts: Bool
    @State private var showClearCacheAlert = false
    @State private var showResetSettingsAlert = false

    @StateObject private var languageManager = LanguageManager.shared
    @StateObject private var fontManager = FontManager.shared

    init() {
        let settings = SharedSettings.shared

        _selectedInputMode = State(initialValue: InputMode(rawValue: settings.inputMode.rawValue) ?? InputMode.tl)
        _autoCapitalizationEnabled = State(initialValue: settings.isAutoCapitalizationEnabled)
        _autoSpaceEnabled = State(initialValue: settings.isAutoSpaceEnabled)
        _enableDoubleTapOO = State(initialValue: settings.enableDoubleTapOO)
        _enableDoubleTapNN = State(initialValue: settings.enableDoubleTapNN)
        _phahTaigiLayoutEnabled = State(initialValue: settings.phahTaigiLayoutEnabled)
        _selectedFontType = State(initialValue: settings.fontType)
        _outputBothScripts = State(initialValue: settings.outputBothScripts)

        LanguageManager.shared.updateDisplayLanguage()
    }

    var body: some View {
        SettingsPageView(
            onDismiss: { dismiss() }
        ) {
            VStack(spacing: 24) {
                inputModeSection

                fontTypeSection

                SettingsSection {
                    settingsSection
                }

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
        .alert(languageManager.text(AppTexts.resetSettings), isPresented: $showResetSettingsAlert) {
            Button(languageManager.text(AppTexts.cancel), role: .cancel) {}
            Button(languageManager.text(AppTexts.reset), role: .destructive) {
                resetAllSettings()
            }
        } message: {
            Text(languageManager.text(AppTexts.resetSettingsMessage))
        }
    }

    @ViewBuilder
    private var inputModeSection: some View {
        VStack(spacing: 20) {
            HStack {
                LocalizedTextView(AppTexts.inputMode)
                    .themeFontHeadline()
                    .foregroundColor(Color.Theme.textPrimary)

                Spacer()
            }

            Picker("", selection: $selectedInputMode) {
                LocalizedTextView(AppTexts.pojMode).tag(InputMode.poj)
                LocalizedTextView(AppTexts.tlMode).tag(InputMode.tl)
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
                languageManager.updateDisplayLanguage()
            }
        }
        .padding(.vertical, 24)
        .padding(.horizontal, 24)
        .themedCard()
    }

    @ViewBuilder
    private var fontTypeSection: some View {
        VStack(spacing: 20) {
            HStack {
                LocalizedTextView(AppTexts.customFont)
                    .themeFontHeadline()
                    .foregroundColor(Color.Theme.textPrimary)

                Spacer()
            }

            Picker("", selection: $selectedFontType) {
                LocalizedTextView(AppTexts.fontSystemDefault).tag(FontType.system)
                LocalizedTextView(AppTexts.fontOpenHuninn).tag(FontType.openHuninn)
                LocalizedTextView(AppTexts.fontIansui).tag(FontType.iansui)
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

    @ViewBuilder
    private var settingsSection: some View {
        // 注意：outputBothScripts 依賴 showHanjiMode，預設為開啟狀態
        SettingsToggleItem(
            titleContent: AppTexts.outputBothScripts,
            isOn: $outputBothScripts,
            isFirst: true,
            onChange: { newValue in
                settings.outputBothScripts = newValue
            }
        )
        // showHanjiMode 預設為 true，因此 outputBothScripts 永遠可用

        SettingsToggleItem(
            titleContent: AppTexts.autoCapitalization,
            isOn: $autoCapitalizationEnabled,
            onChange: { newValue in
                settings.isAutoCapitalizationEnabled = newValue
            }
        )

        SettingsToggleItem(
            titleContent: AppTexts.autoSpace,
            isOn: $autoSpaceEnabled,
            onChange: { newValue in
                settings.isAutoSpaceEnabled = newValue
            }
        )

        SettingsToggleItem(
            titleContent: AppTexts.phahTaigiLayout,
            isOn: $phahTaigiLayoutEnabled,
            onChange: { newValue in
                settings.phahTaigiLayoutEnabled = newValue
            }
        )

        SettingsToggleItem(
            titleContent: AppTexts.doubleTapOO,
            isOn: $enableDoubleTapOO,
            onChange: { newValue in
                settings.enableDoubleTapOO = newValue
            }
        )

        SettingsToggleItem(
            titleContent: AppTexts.doubleTapNN,
            isOn: $enableDoubleTapNN,
            isLast: true,
            onChange: { newValue in
                settings.enableDoubleTapNN = newValue
            }
        )
    }

    @ViewBuilder
    private var actionButtonsSection: some View {
        SettingsActionButton(
            titleContent: AppTexts.clearCache,
            isFirst: true,
            action: { showClearCacheAlert = true }
        )

        SettingsActionButton(
            titleContent: AppTexts.resetSettings,
            isLast: true,
            action: { showResetSettingsAlert = true }
        )
    }

    private func clearUserFrequencyDatabase() {
        do {
            try UserFrequencyService.deleteUserDatabase()

            let impactFeedback = UIImpactFeedbackGenerator(style: .medium)
            impactFeedback.impactOccurred()
        } catch {}
    }

    private func resetAllSettings() {
        settings.resetToDefaults()

        do {
            try UserFrequencyService.deleteUserDatabase()
        } catch {}

        selectedInputMode = settings.inputMode
        autoCapitalizationEnabled = settings.isAutoCapitalizationEnabled
        autoSpaceEnabled = settings.isAutoSpaceEnabled
        enableDoubleTapOO = settings.enableDoubleTapOO
        enableDoubleTapNN = settings.enableDoubleTapNN
        phahTaigiLayoutEnabled = settings.phahTaigiLayoutEnabled
        selectedFontType = settings.fontType
        outputBothScripts = settings.outputBothScripts

        languageManager.updateDisplayLanguage()
        fontManager.reloadFontType()

        let impactFeedback = UIImpactFeedbackGenerator(style: .medium)
        impactFeedback.impactOccurred()
    }

    /// 更新 SegmentedControl 的外觀（包含字體）
    private func updateSegmentedControlAppearance() {
        let font = KeyboardModels.Fonts.uiFont(for: fontManager.currentFontType, size: 13)
        UISegmentedControl.appearance().selectedSegmentTintColor = UIColor(Color.Theme.accent)
        UISegmentedControl.appearance().setTitleTextAttributes([
            .foregroundColor: UIColor.white,
            .font: font
        ], for: .selected)
        UISegmentedControl.appearance().setTitleTextAttributes([
            .foregroundColor: UIColor.black,
            .font: font
        ], for: .normal)
    }
}

