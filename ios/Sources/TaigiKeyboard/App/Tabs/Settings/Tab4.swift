import KeyboardKit
import SwiftUI
import UIKit

/// Settings tab.
///
/// Input mode, typing options, keyboard toggles, feedback, and diagnostics.
struct Tab4: View {
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

        _selectedInputMode = State(initialValue: InputMode(rawValue: settings.inputMode.rawValue) ?? InputMode.tl)
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
                            Text(Tab4Texts.inputMode)
                            Spacer()
                            Text(inputModeDisplayName(selectedInputMode))
                                .foregroundColor(.secondary)
                        }
                    }
                }

                // Typing options
                Section {
                    Toggle(isOn: $isOutputBothScripts) {
                        HStack {
                            Text(Tab4Texts.isOutputBothScripts)
                            SettingInfoButton(description: featureSummary("hanloDesign"))
                        }
                    }
                    .onChange(of: isOutputBothScripts) { _, newValue in
                        settings.isOutputBothScripts = newValue
                    }

                    Toggle(isOn: $autoCapitalizationEnabled) {
                        HStack {
                            Text(Tab4Texts.autoCapitalization)
                            SettingInfoButton(description: featureSummary("caseSwitch"))
                        }
                    }

                    Toggle(isOn: $autoSpaceEnabled) {
                        HStack {
                            Text(Tab4Texts.autoSpace)
                            SettingInfoButton(description: featureSummary("hanloDesign"))
                        }
                    }
                    .onChange(of: autoSpaceEnabled) { _, newValue in
                        settings.isAutoSpaceEnabled = newValue
                    }
                } header: {
                    Text(Tab4Texts.typingSectionTitle)
                }

                // Keyboard settings
                Section {
                    Toggle(isOn: $toolbarAutoCollapse) {
                        HStack {
                            Label {
                                Text(Tab4Texts.toolbarAutoCollapse)
                            } icon: {
                                Image(systemName: SettingsIcons.toolbar)
                                    .foregroundColor(AppStyle.accentBlue)
                            }
                            SettingInfoButton(description: Tab4Texts.toolbarAutoCollapseInfo)
                        }
                    }
                    .onChange(of: toolbarAutoCollapse) { _, newValue in
                        settings.isToolbarAutoCollapse = newValue
                    }

                    Toggle(isOn: $isGlobeKeyEnabled) {
                        HStack {
                            Label {
                                Text(Tab4Texts.globeKey)
                            } icon: {
                                Image(systemName: SettingsIcons.globeKey)
                                    .foregroundColor(AppStyle.accentBlue)
                            }
                            SettingInfoButton(description: Tab4Texts.globeKeyInfo)
                        }
                    }
                    .onChange(of: isGlobeKeyEnabled) { _, newValue in
                        settings.isGlobeKeyEnabled = newValue
                    }
                } header: {
                    Text(Tab4Texts.keyboardSectionTitle)
                }

                // Feedback
                Section {
                    Toggle(isOn: $isAudioFeedbackEnabled) {
                        Label(Tab4Texts.soundFeedback, systemImage: SettingsIcons.soundFeedback)
                    }

                    Toggle(isOn: $isHapticFeedbackEnabled) {
                        Label(Tab4Texts.vibrationFeedback, systemImage: SettingsIcons.vibrationFeedback)
                    }
                } header: {
                    Text(Tab4Texts.feedbackSectionTitle)
                        .font(AppStyle.sectionHeaderFont)
                }

                // POJ settings
                Section {
                    Toggle(Tab4Texts.doubleTapOO, isOn: $isDoubleTapOOEnabled)
                        .onChange(of: isDoubleTapOOEnabled) { _, newValue in
                            settings.isDoubleTapOOEnabled = newValue
                        }

                    Toggle(Tab4Texts.doubleTapNN, isOn: $isDoubleTapNNEnabled)
                        .onChange(of: isDoubleTapNNEnabled) { _, newValue in
                            settings.isDoubleTapNNEnabled = newValue
                        }
                } header: {
                    Text(Tab4Texts.pojSettingsSectionTitle)
                        .font(AppStyle.sectionHeaderFont)
                }

                // TPS (方音符號) settings
                Section {
                    Toggle(isOn: $isTpsOrMappedToER) {
                        HStack {
                            Text(Tab4Texts.isTpsOrMappedToER)
                            SettingInfoButton(description: Tab4Texts.isTpsOrMappedToERInfo)
                        }
                    }
                    .onChange(of: isTpsOrMappedToER) { _, newValue in
                        settings.isTpsOrMappedToER = newValue
                    }
                } header: {
                    Text(Tab4Texts.tpsSettingsSectionTitle)
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
                                ? Tab4Texts.diagnosticCopied
                                : Tab4Texts.diagnosticCopy,
                            systemImage: diagnosticCopied ? "checkmark" : "doc.on.doc",
                        )
                        .foregroundColor(.primary)
                    }

                    ShareLink(
                        item: diagnosticText,
                        subject: Text("台語齒盤 Bug 回報"),
                        message: Text(diagnosticText),
                    ) {
                        Label(Tab4Texts.diagnosticShare, systemImage: "arrow.up.forward.square")
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
                        Label(Tab4Texts.diagnosticEmail, systemImage: "arrow.up.forward.square")
                    }
                } header: {
                    Text(Tab4Texts.diagnosticSectionTitle)
                        .font(AppStyle.sectionHeaderFont)
                }

                // Reset
                Section {
                    Button(role: .destructive) {
                        showResetSettingsAlert = true
                    } label: {
                        Text(Tab4Texts.resetSettings)
                    }
                }
            }
            .navigationTitle(Tab4Texts.tabTitle)
            .navigationBarTitleDisplayMode(.large)
            .onAppear {
                selectedInputMode = settings.inputMode
                diagnosticText = DiagnosticService.gather().formatted()
            }
        }
        .alert(Tab4Texts.resetSettings, isPresented: $showResetSettingsAlert) {
            Button(CommonTexts.cancel, role: .cancel) {}
            Button(Tab4Texts.reset, role: .destructive) {
                resetAllSettings()
            }
        } message: {
            Text(Tab4Texts.resetSettingsMessage)
        }
    }

    // MARK: - Feature Summary Lookup

    private func featureSummary(_ featureId: String) -> String {
        FeatureContentLoader.features
            .first(where: { $0.id == featureId })?
            .summary ?? ""
    }

    // MARK: - Display Name Helpers

    private func inputModeDisplayName(_ mode: InputMode) -> String {
        switch mode {
        case .poj: Tab4Texts.pojMode
        case .tl: Tab4Texts.tlMode
        case .english: Tab4Texts.englishMode
        case .tps: Tab4Texts.tpsMode
        }
    }

    // MARK: - Actions

    private func resetAllSettings() {
        settings.resetToDefaults()

        // Clear user frequency data
        do {
            try UserFrequencyService.deleteUserDatabase()
        } catch {
            DebugLogger(category: "Tab4").error("Failed to delete frequency database: \(error)")
        }

        // Clear user association data
        do {
            try NextWordService.deleteUserDatabase()
        } catch {
            DebugLogger(category: "Tab4").error("Failed to delete association database: \(error)")
        }

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

// MARK: - Input Mode Picker Subpage

private struct InputModePickerView: View {
    @Binding var selectedMode: InputMode
    var onChange: (InputMode) -> Void

    private let options: [(mode: InputMode, text: String)] = [
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
                            Text(option.text)
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
        .navigationTitle(Tab4Texts.inputMode)
        .navigationBarTitleDisplayMode(.inline)
    }
}
