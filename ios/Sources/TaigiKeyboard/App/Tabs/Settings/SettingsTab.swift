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
    private let settings = SharedSettings.shared

    @State private var selectedInputMode: InputMode
    @State private var autoSpaceEnabled: Bool
    @State private var isDoubleTapOOEnabled: Bool
    @State private var isDoubleTapNNEnabled: Bool
    @State private var isOutputBothScripts: Bool
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
        _autoSpaceEnabled = State(initialValue: settings.isAutoSpaceEnabled)
        _isDoubleTapOOEnabled = State(initialValue: settings.isDoubleTapOOEnabled)
        _isDoubleTapNNEnabled = State(initialValue: settings.isDoubleTapNNEnabled)
        _isOutputBothScripts = State(initialValue: settings.isOutputBothScripts)
        _isTpsOrMappedToER = State(initialValue: settings.isTpsOrMappedToER)
        _toolbarAutoCollapse = State(initialValue: settings.isToolbarAutoCollapse)
        _isGlobeKeyEnabled = State(initialValue: settings.isGlobeKeyEnabled)
    }

    var body: some View {
        NavigationStack {
            Form {
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
                            Text(SettingsTexts.inputMode)
                            Spacer()
                            Text(selectedInputMode.displayName)
                                .foregroundColor(.secondary)
                        }
                    }
                }

                // Typing options
                Section {
                    Toggle(isOn: $isOutputBothScripts) {
                        HStack {
                            Text(SettingsTexts.isOutputBothScripts)
                            SettingInfoButton(description: featureSummary("hanloDesign"))
                        }
                    }
                    .onChange(of: isOutputBothScripts) { _, newValue in
                        settings.isOutputBothScripts = newValue
                    }

                    Toggle(isOn: $autoCapitalizationEnabled) {
                        HStack {
                            Text(SettingsTexts.autoCapitalization)
                            SettingInfoButton(description: featureSummary("caseSwitch"))
                        }
                    }

                    Toggle(isOn: $autoSpaceEnabled) {
                        HStack {
                            Text(SettingsTexts.autoSpace)
                            SettingInfoButton(description: featureSummary("hanloDesign"))
                        }
                    }
                    .onChange(of: autoSpaceEnabled) { _, newValue in
                        settings.isAutoSpaceEnabled = newValue
                    }
                } header: {
                    Text(SettingsTexts.typingSectionTitle)
                }

                // Keyboard settings
                Section {
                    Toggle(isOn: $toolbarAutoCollapse) {
                        HStack {
                            Label {
                                Text(SettingsTexts.toolbarAutoCollapse)
                            } icon: {
                                Image(latinSystemName: SettingsIcons.toolbar)
                                    .foregroundColor(AppStyle.accentBlue)
                            }
                            SettingInfoButton(description: SettingsTexts.toolbarAutoCollapseInfo)
                        }
                    }
                    .onChange(of: toolbarAutoCollapse) { _, newValue in
                        settings.isToolbarAutoCollapse = newValue
                    }

                    Toggle(isOn: $isGlobeKeyEnabled) {
                        HStack {
                            Label {
                                Text(SettingsTexts.globeKey)
                            } icon: {
                                Image(latinSystemName: SettingsIcons.globeKey)
                                    .foregroundColor(AppStyle.accentBlue)
                            }
                            SettingInfoButton(description: SettingsTexts.globeKeyInfo)
                        }
                    }
                    .onChange(of: isGlobeKeyEnabled) { _, newValue in
                        settings.isGlobeKeyEnabled = newValue
                    }
                } header: {
                    Text(SettingsTexts.keyboardSectionTitle)
                }

                // Feedback
                Section {
                    Toggle(isOn: $isAudioFeedbackEnabled) {
                        Label(SettingsTexts.soundFeedback, systemImage: SettingsIcons.soundFeedback)
                    }

                    Toggle(isOn: $isHapticFeedbackEnabled) {
                        Label(SettingsTexts.vibrationFeedback, systemImage: SettingsIcons.vibrationFeedback)
                    }
                } header: {
                    Text(SettingsTexts.feedbackSectionTitle)
                        .font(AppStyle.sectionHeaderFont)
                }

                // POJ settings
                Section {
                    Toggle(SettingsTexts.doubleTapOO, isOn: $isDoubleTapOOEnabled)
                        .onChange(of: isDoubleTapOOEnabled) { _, newValue in
                            settings.isDoubleTapOOEnabled = newValue
                        }

                    Toggle(SettingsTexts.doubleTapNN, isOn: $isDoubleTapNNEnabled)
                        .onChange(of: isDoubleTapNNEnabled) { _, newValue in
                            settings.isDoubleTapNNEnabled = newValue
                        }
                } header: {
                    Text(SettingsTexts.pojSettingsSectionTitle)
                        .font(AppStyle.sectionHeaderFont)
                }

                // TPS (方音符號) settings
                Section {
                    Toggle(isOn: $isTpsOrMappedToER) {
                        HStack {
                            Text(SettingsTexts.isTpsOrMappedToER)
                            SettingInfoButton(description: SettingsTexts.isTpsOrMappedToERInfo)
                        }
                    }
                    .onChange(of: isTpsOrMappedToER) { _, newValue in
                        settings.isTpsOrMappedToER = newValue
                    }
                } header: {
                    Text(SettingsTexts.tpsSettingsSectionTitle)
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
                                ? SettingsTexts.diagnosticCopied
                                : SettingsTexts.diagnosticCopy,
                            systemImage: diagnosticCopied ? "checkmark" : "doc.on.doc",
                        )
                        .foregroundColor(.primary)
                    }

                    ShareLink(
                        item: diagnosticText,
                        subject: Text("台語齒盤 Bug 回報"),
                        message: Text(diagnosticText),
                    ) {
                        Label(SettingsTexts.diagnosticShare, systemImage: "arrow.up.forward.square")
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
                        Label(SettingsTexts.diagnosticEmail, systemImage: "arrow.up.forward.square")
                    }
                } header: {
                    Text(SettingsTexts.diagnosticSectionTitle)
                        .font(AppStyle.sectionHeaderFont)
                }

                // Reset
                Section {
                    Button(role: .destructive) {
                        showResetSettingsAlert = true
                    } label: {
                        Text(SettingsTexts.resetSettings)
                    }
                }
            }
            .navigationTitle(SettingsTexts.tabTitle)
            .navigationBarTitleDisplayMode(.large)
            .onAppear {
                selectedInputMode = settings.inputMode
                diagnosticText = DiagnosticService.gather().formatted()
            }
        }
        .alert(SettingsTexts.resetSettings, isPresented: $showResetSettingsAlert) {
            Button(CommonTexts.cancel, role: .cancel) {}
            Button(SettingsTexts.reset, role: .destructive) {
                resetAllSettings()
            }
        } message: {
            Text(SettingsTexts.resetSettingsMessage)
        }
    }

    // MARK: - Feature Summary Lookup

    // 中文: 依 featureId 從 FeatureContentLoader 找對應的功能摘要,供 SettingInfoButton 顯示。
    private func featureSummary(_ featureId: String) -> String {
        FeatureContentLoader.features
            .first(where: { $0.id == featureId })?
            .summary ?? ""
    }

    // MARK: - Actions

    // 中文: 觸發 SettingsResetCoordinator 全量重置(設定 + 使用者資料),並把本地 @State
    // 中文: 同步回預設,最後給一次 medium 觸覺回饋。
    private func resetAllSettings() {
        SettingsResetCoordinator.resetAll()
        SettingsResetCoordinator.resetAllUserData()

        // Sync local state
        selectedInputMode = settings.inputMode
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

        let impactFeedback = UIImpactFeedbackGenerator(style: .medium)
        impactFeedback.impactOccurred()
    }
}
