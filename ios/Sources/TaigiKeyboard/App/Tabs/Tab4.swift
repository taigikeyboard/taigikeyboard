import KeyboardKit
import SwiftUI
import UIKit

/// 設定 Tab
///
/// 鍵盤設定，包含輸入模式、外觀設定、開關選項。
struct Tab4: View {
    @StateObject private var languageManager = LanguageManager.shared

    private let settings = SharedSettings.shared

    @State private var selectedInputMode: InputMode
    @State private var autoSpaceEnabled: Bool
    @State private var enableDoubleTapOO: Bool
    @State private var enableDoubleTapNN: Bool
    @State private var outputBothScripts: Bool
    @State private var tpsOrMapsToER: Bool
    @State private var toolbarAutoCollapse: Bool
    @State private var isGlobeKeyEnabled: Bool
    @State private var showResetSettingsAlert = false
    @State private var diagnosticCopied = false
    @State private var diagnosticText = ""
    @Environment(\.openURL) private var openURL

    /// 使用 KeyboardKit 的持久化機制
    @AppStorage(
        "com.keyboardkit.settings.keyboard.isAutocapitalizationEnabled",
        store: UserDefaults(suiteName: SharedSettings.appGroupId),
    )
    private var autoCapitalizationEnabled = true

    @AppStorage(
        "com.keyboardkit.settings.feedback.isAudioFeedbackEnabled",
        store: UserDefaults(suiteName: SharedSettings.appGroupId),
    )
    private var isAudioFeedbackEnabled = true

    @AppStorage(
        "com.keyboardkit.settings.feedback.isHapticFeedbackEnabled",
        store: UserDefaults(suiteName: SharedSettings.appGroupId),
    )
    private var isHapticFeedbackEnabled = true

    init() {
        let settings = SharedSettings.shared

        _selectedInputMode = State(initialValue: InputMode(rawValue: settings.inputMode.rawValue) ?? InputMode.tl)
        _autoSpaceEnabled = State(initialValue: settings.isAutoSpaceEnabled)
        _enableDoubleTapOO = State(initialValue: settings.enableDoubleTapOO)
        _enableDoubleTapNN = State(initialValue: settings.enableDoubleTapNN)
        _outputBothScripts = State(initialValue: settings.outputBothScripts)
        _tpsOrMapsToER = State(initialValue: settings.tpsOrMapsToER)
        _toolbarAutoCollapse = State(initialValue: settings.isToolbarAutoCollapse)
        _isGlobeKeyEnabled = State(initialValue: settings.isGlobeKeyEnabled)
    }

    var body: some View {
        NavigationStack {
            Form {
                // 輸入模式
                Section {
                    NavigationLink {
                        InputModePickerView(
                            selectedMode: $selectedInputMode,
                            onChange: { newValue in
                                settings.inputMode = newValue
                            },
                        )
                    } label: {
                        HStack {
                            Text(languageManager.text(Tab4Texts.inputMode))
                            Spacer()
                            Text(inputModeDisplayName(selectedInputMode))
                                .foregroundColor(.secondary)
                        }
                    }
                }

                // 拍字設定
                Section {
                    Toggle(isOn: $outputBothScripts) {
                        HStack {
                            Text(languageManager.text(Tab4Texts.outputBothScripts))
                            SettingInfoButton(description: featureSummary("hanloDesign"))
                        }
                    }
                    .onChange(of: outputBothScripts) { _, newValue in
                        settings.outputBothScripts = newValue
                    }

                    Toggle(isOn: $autoCapitalizationEnabled) {
                        HStack {
                            Text(languageManager.text(Tab4Texts.autoCapitalization))
                            SettingInfoButton(description: featureSummary("caseSwitch"))
                        }
                    }

                    Toggle(isOn: $autoSpaceEnabled) {
                        HStack {
                            Text(languageManager.text(Tab4Texts.autoSpace))
                            SettingInfoButton(description: featureSummary("hanloDesign"))
                        }
                    }
                    .onChange(of: autoSpaceEnabled) { _, newValue in
                        settings.isAutoSpaceEnabled = newValue
                    }
                } header: {
                    Text(languageManager.text(Tab4Texts.typingSectionTitle))
                }

                // 齒盤設定
                Section {
                    Toggle(isOn: $toolbarAutoCollapse) {
                        HStack {
                            Label {
                                Text(languageManager.text(Tab4Texts.toolbarAutoCollapse))
                            } icon: {
                                Image(systemName: Tab4Texts.toolbarIcon)
                                    .foregroundColor(AppStyle.accentBlue)
                            }
                            SettingInfoButton(description: languageManager.text(Tab4Texts.toolbarAutoCollapseInfo))
                        }
                    }
                    .onChange(of: toolbarAutoCollapse) { _, newValue in
                        settings.isToolbarAutoCollapse = newValue
                    }

                    Toggle(isOn: $isGlobeKeyEnabled) {
                        HStack {
                            Label {
                                Text(languageManager.text(Tab4Texts.globeKey))
                            } icon: {
                                Image(systemName: Tab4Texts.globeKeyIcon)
                                    .foregroundColor(AppStyle.accentBlue)
                            }
                            SettingInfoButton(description: languageManager.text(Tab4Texts.globeKeyInfo))
                        }
                    }
                    .onChange(of: isGlobeKeyEnabled) { _, newValue in
                        settings.isGlobeKeyEnabled = newValue
                    }
                } header: {
                    Text(languageManager.text(Tab4Texts.keyboardSectionTitle))
                }

                // 回饋設定
                Section {
                    Toggle(isOn: $isAudioFeedbackEnabled) {
                        Label(languageManager.text(Tab4Texts.soundFeedback), systemImage: Tab4Texts.soundFeedbackIcon)
                    }

                    Toggle(isOn: $isHapticFeedbackEnabled) {
                        Label(languageManager.text(Tab4Texts.vibrationFeedback), systemImage: Tab4Texts.vibrationFeedbackIcon)
                    }
                } header: {
                    Text(languageManager.text(Tab4Texts.feedbackSectionTitle))
                        .font(AppStyle.sectionHeaderFont)
                }

                // 白話字設定
                Section {
                    Toggle(languageManager.text(Tab4Texts.doubleTapOO), isOn: $enableDoubleTapOO)
                        .onChange(of: enableDoubleTapOO) { _, newValue in
                            settings.enableDoubleTapOO = newValue
                        }

                    Toggle(languageManager.text(Tab4Texts.doubleTapNN), isOn: $enableDoubleTapNN)
                        .onChange(of: enableDoubleTapNN) { _, newValue in
                            settings.enableDoubleTapNN = newValue
                        }
                } header: {
                    Text(languageManager.text(Tab4Texts.pojSettingsSectionTitle))
                        .font(AppStyle.sectionHeaderFont)
                }

                // 方音符號設定
                Section {
                    Toggle(isOn: $tpsOrMapsToER) {
                        HStack {
                            Text(languageManager.text(Tab4Texts.tpsOrMapsToER))
                            SettingInfoButton(description: languageManager.text(Tab4Texts.tpsOrMapsToERInfo))
                        }
                    }
                    .onChange(of: tpsOrMapsToER) { _, newValue in
                        settings.tpsOrMapsToER = newValue
                    }
                } header: {
                    Text(languageManager.text(Tab4Texts.tpsSettingsSectionTitle))
                        .font(AppStyle.sectionHeaderFont)
                }

                // 診斷資訊
                Section {
                    Button {
                        let info = DiagnosticService.gather()
                        UIPasteboard.general.string = info.formatted()
                        diagnosticCopied = true
                        DispatchQueue.main.asyncAfter(deadline: .now() + 2) {
                            diagnosticCopied = false
                        }
                    } label: {
                        Label(
                            diagnosticCopied
                                ? languageManager.text(Tab4Texts.diagnosticCopied)
                                : languageManager.text(Tab4Texts.diagnosticCopy),
                            systemImage: diagnosticCopied ? "checkmark" : "doc.on.doc",
                        )
                        .foregroundColor(.primary)
                    }

                    ShareLink(
                        item: diagnosticText,
                        subject: Text("台語齒盤 Bug 回報"),
                        message: Text(diagnosticText),
                    ) {
                        Label(languageManager.text(Tab4Texts.diagnosticShare), systemImage: "arrow.up.forward.square")
                    }

                    Button {
                        let info = DiagnosticService.gather()
                        let subject = "台語齒盤 Bug 回報 (v\(info.appVersion))"
                            .addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? ""
                        let body = info.formatted()
                            .addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? ""
                        if let url = URL(string: "mailto:info@taigikeyboard.tw?subject=\(subject)&body=\(body)") {
                            openURL(url)
                        }
                    } label: {
                        Label(languageManager.text(Tab4Texts.diagnosticEmail), systemImage: "arrow.up.forward.square")
                    }
                } header: {
                    Text(languageManager.text(Tab4Texts.diagnosticSectionTitle))
                        .font(AppStyle.sectionHeaderFont)
                }

                // 重設按鈕
                Section {
                    Button(role: .destructive) {
                        showResetSettingsAlert = true
                    } label: {
                        Text(languageManager.text(Tab4Texts.resetSettings))
                    }
                }
            }
            .navigationTitle(languageManager.text(Tab4Texts.tabTitle))
            .navigationBarTitleDisplayMode(.large)
            .onAppear {
                selectedInputMode = settings.inputMode
                diagnosticText = DiagnosticService.gather().formatted()
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
    }

    // MARK: - Feature Summary Lookup

    private func featureSummary(_ featureId: String) -> String {
        FeatureContentLoader.features
            .first(where: { $0.id == featureId })?
            .summary?
            .asLocalizedText
            .hanji ?? ""
    }

    // MARK: - Display Name Helpers

    private func inputModeDisplayName(_ mode: InputMode) -> String {
        switch mode {
        case .poj: languageManager.text(Tab4Texts.pojMode)
        case .tl: languageManager.text(Tab4Texts.tlMode)
        case .english: languageManager.text(Tab4Texts.englishMode)
        case .tps: languageManager.text(Tab4Texts.tpsMode)
        }
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
        autoCapitalizationEnabled = true // KeyboardKit 預設值
        isAudioFeedbackEnabled = true
        isHapticFeedbackEnabled = true
        autoSpaceEnabled = settings.isAutoSpaceEnabled
        enableDoubleTapOO = settings.enableDoubleTapOO
        enableDoubleTapNN = settings.enableDoubleTapNN
        outputBothScripts = settings.outputBothScripts
        tpsOrMapsToER = settings.tpsOrMapsToER
        toolbarAutoCollapse = settings.isToolbarAutoCollapse
        isGlobeKeyEnabled = settings.isGlobeKeyEnabled

        let impactFeedback = UIImpactFeedbackGenerator(style: .medium)
        impactFeedback.impactOccurred()
    }
}

// MARK: - Input Mode Picker Subpage

private struct InputModePickerView: View {
    @StateObject private var languageManager = LanguageManager.shared
    @Binding var selectedMode: InputMode
    var onChange: (InputMode) -> Void

    private let options: [(mode: InputMode, text: LocalizedText)] = [
        (.poj, Tab4Texts.pojMode),
        (.tl, Tab4Texts.tlMode),
        (.english, Tab4Texts.englishMode),
        (.tps, Tab4Texts.tpsMode),
    ]

    var body: some View {
        Form {
            Section {
                ForEach(options, id: \.mode) { option in
                    Button {
                        selectedMode = option.mode
                        onChange(option.mode)
                    } label: {
                        HStack {
                            Text(languageManager.text(option.text))
                                .foregroundColor(.primary)
                            Spacer()
                            if selectedMode == option.mode {
                                Image(systemName: "checkmark")
                                    .foregroundColor(AppStyle.accentBlue)
                            }
                        }
                    }
                }
            }
        }
        .navigationTitle(languageManager.text(Tab4Texts.inputMode))
        .navigationBarTitleDisplayMode(.inline)
    }
}
