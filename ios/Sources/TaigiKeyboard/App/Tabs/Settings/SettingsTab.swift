import KeyboardKit
import SwiftUI
import UIKit

/// Settings tab.
///
/// Input mode, typing options, keyboard toggles, feedback, and diagnostics.
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

    private func featureSummary(_ featureId: String) -> String {
        FeatureContentLoader.features
            .first(where: { $0.id == featureId })?
            .summary ?? ""
    }

    // MARK: - Actions

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
