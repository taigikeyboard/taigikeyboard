import SwiftUI
import UIKit

/// 設定頁面視圖
struct SettingsView: View {
    @Environment(\.dismiss) var dismiss
    @State private var selectedInputMode: InputMode = .tl
    @State private var showHanjiMode = true

    private let settings = SharedSettings.shared

    @State private var autoCapitalizationEnabled: Bool
    @State private var autoSpaceEnabled: Bool
    @State private var enableDoubleTapOO: Bool
    @State private var enableDoubleTapNN: Bool
    @State private var phahTaigiLayoutEnabled: Bool
    @State private var customFontEnabled: Bool
    @State private var outputBothScripts: Bool
    @State private var showClearCacheAlert = false
    @State private var showResetSettingsAlert = false

    @StateObject private var languageManager = LanguageManager.shared

    init() {
        let settings = SharedSettings.shared

        _selectedInputMode = State(initialValue: InputMode(rawValue: settings.inputMode.rawValue) ?? InputMode.tl)
        _autoCapitalizationEnabled = State(initialValue: settings.isAutoCapitalizationEnabled)
        _autoSpaceEnabled = State(initialValue: settings.isAutoSpaceEnabled)
        _showHanjiMode = State(initialValue: settings.showHanjiMode)
        _enableDoubleTapOO = State(initialValue: settings.enableDoubleTapOO)
        _enableDoubleTapNN = State(initialValue: settings.enableDoubleTapNN)
        _phahTaigiLayoutEnabled = State(initialValue: settings.phahTaigiLayoutEnabled)
        _customFontEnabled = State(initialValue: settings.isCustomFontEnabled)
        _outputBothScripts = State(initialValue: settings.outputBothScripts)

        LanguageManager.shared.updateDisplayLanguage()
    }

    var body: some View {
        SettingsPageView(
            onDismiss: { dismiss() }
        ) {
            VStack(spacing: 24) {
                inputModeSection

                SettingsSection(titleContent: AppTexts.inputSettings) {
                    hanjiSettingsSection
                }

                SettingsSection(titleContent: AppTexts.layoutSettings) {
                    basicSettingsSection
                }

                SettingsSection(titleContent: AppTexts.doubleTapCombination) {
                    doubleTapSection
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
                    .font(Font.Theme.headline)
                    .foregroundColor(Color.Theme.textPrimary)

                Spacer()
            }

            Picker("", selection: $selectedInputMode) {
                LocalizedTextView(AppTexts.pojMode).tag(InputMode.poj)
                LocalizedTextView(AppTexts.tlMode).tag(InputMode.tl)
            }
            .pickerStyle(SegmentedPickerStyle())
            .onAppear {
                UISegmentedControl.appearance().selectedSegmentTintColor = UIColor(Color.Theme.accent)
                UISegmentedControl.appearance().setTitleTextAttributes([.foregroundColor: UIColor.white], for: .selected)
                UISegmentedControl.appearance().setTitleTextAttributes([.foregroundColor: UIColor.black], for: .normal)
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
    private var hanjiSettingsSection: some View {
        SettingsToggleItem(
            titleContent: AppTexts.showHanji,
            isOn: $showHanjiMode,
            isFirst: true,
            onChange: { newValue in
                settings.showHanjiMode = newValue
                languageManager.updateDisplayLanguage()
            }
        )

        SettingsToggleItem(
            titleContent: AppTexts.outputBothScripts,
            isOn: $outputBothScripts,
            isLast: true,
            onChange: { newValue in
                settings.outputBothScripts = newValue
            }
        )
        .disabled(!showHanjiMode)
        .opacity(showHanjiMode ? 1.0 : 0.4)
    }

    @ViewBuilder
    private var basicSettingsSection: some View {
        SettingsToggleItem(
            titleContent: AppTexts.autoCapitalization,
            isOn: $autoCapitalizationEnabled,
            isFirst: true,
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
            titleContent: AppTexts.customFont,
            isOn: $customFontEnabled,
            onChange: { newValue in
                settings.isCustomFontEnabled = newValue
                languageManager.updateDisplayLanguage()
            }
        )

        SettingsToggleItem(
            titleContent: AppTexts.phahTaigiLayout,
            isOn: $phahTaigiLayoutEnabled,
            isLast: true,
            onChange: { newValue in
                settings.phahTaigiLayoutEnabled = newValue
            }
        )
    }

    @ViewBuilder
    private var doubleTapSection: some View {
        SettingsToggleItem(
            titleContent: AppTexts.doubleTapOO,
            isOn: $enableDoubleTapOO,
            isFirst: true,
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
        showHanjiMode = settings.showHanjiMode
        enableDoubleTapOO = settings.enableDoubleTapOO
        enableDoubleTapNN = settings.enableDoubleTapNN
        phahTaigiLayoutEnabled = settings.phahTaigiLayoutEnabled
        customFontEnabled = settings.isCustomFontEnabled
        outputBothScripts = settings.outputBothScripts

        languageManager.updateDisplayLanguage()

        let impactFeedback = UIImpactFeedbackGenerator(style: .medium)
        impactFeedback.impactOccurred()
    }
}

