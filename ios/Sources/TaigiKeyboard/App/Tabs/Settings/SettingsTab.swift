// 中文: Settings Tab — App 主要設定頁。內含輸入模式、輸入行為、鍵盤、回饋、
// 中文: POJ / TPS 切換、診斷複製/分享/Email、重置等 Section。
// 中文: 部分開關透過 KeyboardKit @AppStorage 與 keyboard extension 共用 App Group。

import KeyboardKit
import SwiftUI
import UIKit

/// Settings tab.
///
/// Input mode, typing options, keyboard toggles, feedback, and diagnostics.
// 中文: Settings Tab View。集中所有設定 row,並透過 SharedSettings / KeyboardKit
// 中文: App Group UserDefaults 雙路徑落盤。
struct SettingsTab: View {
    @Environment(DisplayLanguageStore.self) private var lang
    private let settings = SharedSettings.shared

    @State private var selectedInputMode: InputMode
    @State private var selectedFontType: FontType
    @State private var autoSpaceEnabled: Bool
    @State private var isDoubleTapOOEnabled: Bool
    @State private var isDoubleTapNNEnabled: Bool
    @State private var isOutputBothScripts: Bool
    @State private var literalRomanCandidateEnabled: Bool
    @State private var isTpsOrMappedToER: Bool
    @State private var toolbarAutoCollapse: Bool
    @State private var isGlobeKeyEnabled: Bool
    @State private var showResetSettingsAlert = false
    @State private var diagnosticCopied = false
    @State private var diagnosticText = ""
    @Environment(\.openURL) private var openURL

    /// KeyboardKit persisted settings via App Group
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

        _selectedInputMode = State(initialValue: settings.inputMode)
        _selectedFontType = State(initialValue: settings.fontType)
        _autoSpaceEnabled = State(initialValue: settings.isAutoSpaceEnabled)
        _isDoubleTapOOEnabled = State(initialValue: settings.isDoubleTapOOEnabled)
        _isDoubleTapNNEnabled = State(initialValue: settings.isDoubleTapNNEnabled)
        _isOutputBothScripts = State(initialValue: settings.isOutputBothScripts)
        _literalRomanCandidateEnabled = State(initialValue: settings.isLiteralRomanCandidateEnabled)
        _isTpsOrMappedToER = State(initialValue: settings.isTpsOrMappedToER)
        _toolbarAutoCollapse = State(initialValue: settings.isToolbarAutoCollapse)
        _isGlobeKeyEnabled = State(initialValue: settings.isGlobeKeyEnabled)
    }

    var body: some View {
        NavigationStack {
            Form {
                // App UI display language — its own Section (separate card), kept distinct from the
                // input-mode row below so the two "language / mode" pickers don't read as related.
                Section {
                    NavigationLink {
                        DisplayLanguagePickerView()
                    } label: {
                        HStack {
                            Text(lang.string(.settingsDisplayLanguage))
                            Spacer()
                            Text(lang.selectionLabel(for: lang.selected))
                                .foregroundColor(.secondary)
                        }
                    }
                }

                // Input mode
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
                            Text(lang.string(.settingsInputMode))
                            Spacer()
                            Text(lang.string(selectedInputMode.displayNameKey))
                                .foregroundColor(.secondary)
                        }
                    }
                }

                // Global keyboard font — its own Section (separate card) below 輸入模式.
                // Applies to every theme (font is NOT per-theme); native Form grouped
                // container, no hand-rolled card.
                // 中文: 全域鍵盤字型獨立 Section(自成一卡),放輸入模式下方;非 per-theme,改一次=全部主題。
                Section {
                    NavigationLink {
                        ThemeFontPickerView(
                            selectedFont: $selectedFontType,
                            onChange: { settings.fontType = $0 },
                        )
                    } label: {
                        HStack {
                            Text(lang.string(.themeCustomFont))
                            Spacer()
                            Text(lang.string(selectedFontType.displayNameKey))
                                .foregroundColor(.secondary)
                        }
                    }
                }

                // Typing options
                Section {
                    Toggle(isOn: $isOutputBothScripts) {
                        HStack {
                            Text(lang.string(.settingsOutputBothScripts))
                            SettingInfoButton(description: featureSummary("hanloDesign"))
                        }
                    }
                    .onChange(of: isOutputBothScripts) { _, newValue in
                        settings.isOutputBothScripts = newValue
                    }

                    Toggle(isOn: $literalRomanCandidateEnabled) {
                        HStack {
                            Text(lang.string(.settingsLiteralRomanCandidate))
                            SettingInfoButton(description: lang.string(.settingsLiteralRomanCandidateInfo))
                        }
                    }
                    .onChange(of: literalRomanCandidateEnabled) { _, newValue in
                        settings.isLiteralRomanCandidateEnabled = newValue
                    }

                    Toggle(isOn: $autoCapitalizationEnabled) {
                        HStack {
                            Text(lang.string(.settingsAutoCapitalization))
                            SettingInfoButton(description: featureSummary("caseSwitch"))
                        }
                    }

                    Toggle(isOn: $autoSpaceEnabled) {
                        HStack {
                            Text(lang.string(.settingsAutoSpace))
                            SettingInfoButton(description: featureSummary("hanloDesign"))
                        }
                    }
                    .onChange(of: autoSpaceEnabled) { _, newValue in
                        settings.isAutoSpaceEnabled = newValue
                    }
                } header: {
                    Text(lang.string(.settingsTypingSectionTitle))
                }

                // Keyboard settings
                Section {
                    Toggle(isOn: $toolbarAutoCollapse) {
                        HStack {
                            Label {
                                Text(lang.string(.settingsToolbarAutoCollapse))
                            } icon: {
                                Image(latinSystemName: SettingsIcons.toolbar)
                                    .foregroundColor(AppStyle.accentBlue)
                            }
                            SettingInfoButton(description: lang.string(.settingsToolbarAutoCollapseInfo))
                        }
                    }
                    .onChange(of: toolbarAutoCollapse) { _, newValue in
                        settings.isToolbarAutoCollapse = newValue
                    }

                    Toggle(isOn: $isGlobeKeyEnabled) {
                        HStack {
                            Label {
                                Text(lang.string(.settingsGlobeKey))
                            } icon: {
                                Image(latinSystemName: SettingsIcons.globeKey)
                                    .foregroundColor(AppStyle.accentBlue)
                            }
                            SettingInfoButton(description: lang.string(.settingsGlobeKeyInfo))
                        }
                    }
                    .onChange(of: isGlobeKeyEnabled) { _, newValue in
                        settings.isGlobeKeyEnabled = newValue
                    }
                } header: {
                    Text(lang.string(.settingsKeyboardSectionTitle))
                }

                // Feedback
                Section {
                    Toggle(isOn: $isAudioFeedbackEnabled) {
                        Label(lang.string(.settingsSoundFeedback), systemImage: SettingsIcons.soundFeedback)
                    }

                    Toggle(isOn: $isHapticFeedbackEnabled) {
                        Label(lang.string(.settingsVibrationFeedback), systemImage: SettingsIcons.vibrationFeedback)
                    }
                } header: {
                    Text(lang.string(.settingsFeedbackSectionTitle))
                        .font(AppStyle.sectionHeaderFont)
                }

                // POJ settings
                Section {
                    Toggle(lang.string(.settingsDoubleTapOO), isOn: $isDoubleTapOOEnabled)
                        .onChange(of: isDoubleTapOOEnabled) { _, newValue in
                            settings.isDoubleTapOOEnabled = newValue
                        }

                    Toggle(lang.string(.settingsDoubleTapNN), isOn: $isDoubleTapNNEnabled)
                        .onChange(of: isDoubleTapNNEnabled) { _, newValue in
                            settings.isDoubleTapNNEnabled = newValue
                        }
                } header: {
                    Text(lang.string(.settingsPojSettingsSectionTitle))
                        .font(AppStyle.sectionHeaderFont)
                }

                // TPS (方音符號) settings
                Section {
                    Toggle(isOn: $isTpsOrMappedToER) {
                        HStack {
                            Text(lang.string(.settingsTpsOrMapsToER))
                            SettingInfoButton(description: lang.string(.settingsTpsOrMapsToERInfo))
                        }
                    }
                    .onChange(of: isTpsOrMappedToER) { _, newValue in
                        settings.isTpsOrMappedToER = newValue
                    }
                } header: {
                    Text(lang.string(.settingsTpsSettingsSectionTitle))
                        .font(AppStyle.sectionHeaderFont)
                }

                // Diagnostics
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
                                ? lang.string(.settingsDiagnosticCopied)
                                : lang.string(.settingsDiagnosticCopy),
                            systemImage: diagnosticCopied ? "checkmark" : "doc.on.doc",
                        )
                        .foregroundColor(.primary)
                    }

                    ShareLink(
                        item: diagnosticText,
                        subject: Text("台語齒盤 Bug 回報"),
                        message: Text(diagnosticText),
                    ) {
                        Label(lang.string(.settingsDiagnosticShare), systemImage: "arrow.up.forward.square")
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
                        Label(lang.string(.settingsDiagnosticEmail), systemImage: "arrow.up.forward.square")
                    }
                } header: {
                    Text(lang.string(.settingsDiagnosticSectionTitle))
                        .font(AppStyle.sectionHeaderFont)
                }

                // Reset
                Section {
                    Button(role: .destructive) {
                        showResetSettingsAlert = true
                    } label: {
                        Text(lang.string(.settingsResetSettings))
                    }
                }
            }
            .navigationTitle(TabType.settings.title)
            .navigationBarTitleDisplayMode(.large)
            .onAppear {
                selectedInputMode = settings.inputMode
                selectedFontType = settings.fontType
                diagnosticText = DiagnosticService.gather().formatted()
            }
        }
        .alert(lang.string(.settingsResetSettings), isPresented: $showResetSettingsAlert) {
            Button(lang.string(.commonCancel), role: .cancel) {}
            Button(lang.string(.settingsReset), role: .destructive) {
                resetAllSettings()
            }
        } message: {
            Text(lang.string(.settingsResetSettingsMessage))
        }
    }

    // MARK: - Feature Summary Lookup

    // 中文: 依 featureId 從 FeatureContentLoader 找對應的功能摘要,供 SettingInfoButton 顯示。
    private func featureSummary(_ featureId: String) -> String {
        FeatureContentLoader.features
            .first(where: { $0.id == featureId })?
            .summary?.resolve(for: lang.language) ?? ""
    }

    // MARK: - Actions

    // 中文: 觸發 SettingsResetCoordinator 全量重置(設定 + 使用者資料),並把本地 @State
    // 中文: 同步回預設,最後給一次 medium 觸覺回饋。
    private func resetAllSettings() {
        SettingsResetCoordinator.resetAll()
        SettingsResetCoordinator.resetAllUserData()

        // Sync local state
        selectedInputMode = settings.inputMode
        selectedFontType = settings.fontType
        autoCapitalizationEnabled = true // KeyboardKit default
        isAudioFeedbackEnabled = true
        isHapticFeedbackEnabled = true
        autoSpaceEnabled = settings.isAutoSpaceEnabled
        isDoubleTapOOEnabled = settings.isDoubleTapOOEnabled
        isDoubleTapNNEnabled = settings.isDoubleTapNNEnabled
        isOutputBothScripts = settings.isOutputBothScripts
        isTpsOrMappedToER = settings.isTpsOrMappedToER
        toolbarAutoCollapse = settings.isToolbarAutoCollapse
        isGlobeKeyEnabled = settings.isGlobeKeyEnabled
        // 中文: reset 把 persisted displayLanguage 寫回 hanji,但 live store 是注入的;同步回來才會即時還原畫面語言。
        lang.syncFromSettings()

        let impactFeedback = UIImpactFeedbackGenerator(style: .medium)
        impactFeedback.impactOccurred()
    }
}
