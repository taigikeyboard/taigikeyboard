import KeyboardKit
import SwiftUI
import UIKit

/// Settings tab.
///
/// Input mode, typing options, keyboard toggles, feedback, and diagnostics.
/// Some toggles are shared with the keyboard extension through KeyboardKit `@AppStorage` in the App Group.
struct SettingsTab: View {
    @Environment(DisplayLanguageStore.self) private var lang
    private let settings = SharedSettings.shared

    @State private var selectedInputMode: InputMode
    @State private var selectedFontType: FontType
    @State private var autoSpaceEnabled: Bool
    @State private var isDoubleTapOOEnabled: Bool
    @State private var isDoubleTapNNEnabled: Bool
    @State private var isNasalMarkerUppercaseEnabled: Bool
    @State private var candidateDisplayMode: CandidateDisplayMode
    @State private var isOutputBothScripts: Bool
    @State private var literalRomanCandidateEnabled: Bool
    @State private var syllableSeparator: SyllableSeparator
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
        _isNasalMarkerUppercaseEnabled = State(initialValue: settings.isNasalMarkerUppercaseEnabled)
        _candidateDisplayMode = State(initialValue: settings.candidateDisplayMode)
        // Toggle binds the STORED flag: it keeps showing the user's choice while disabled under Romanization Only.
        _isOutputBothScripts = State(initialValue: settings.storedIsOutputBothScripts)
        _literalRomanCandidateEnabled = State(initialValue: settings.isLiteralRomanCandidateEnabled)
        _syllableSeparator = State(initialValue: settings.syllableSeparator)
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

                // Typing options: candidates, then output format, then automatic behaviour.
                Section {
                    // Navigation-link picker = gray value + selection subpage, the same shape as the
                    // Input Mode / Font rows. Explicit: a Form's automatic style is a menu on iOS 27.
                    Picker(selection: $candidateDisplayMode) {
                        ForEach(CandidateDisplayMode.allCases, id: \.self) { mode in
                            Text(lang.string(mode.displayNameKey)).tag(mode)
                        }
                    } label: {
                        Text(lang.string(.settingsCandidateDisplayMode))
                    }
                    .pickerStyle(.navigationLink)
                    .onChange(of: candidateDisplayMode) { _, newValue in
                        settings.candidateDisplayMode = newValue
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

                    Toggle(isOn: $isOutputBothScripts) {
                        HStack {
                            Text(lang.string(.settingsOutputBothScripts))
                            SettingInfoButton(description: featureSummary("typingSettings"))
                        }
                    }
                    // Annotate in Brackets is meaningless without hanji; stored value stays untouched.
                    .disabled(!candidateDisplayMode.showsHanji)
                    .onChange(of: isOutputBothScripts) { _, newValue in
                        settings.storedIsOutputBothScripts = newValue
                    }

                    // Same shape as Candidate Display: a menu picker would draw the value in the accent colour.
                    Picker(selection: $syllableSeparator) {
                        ForEach(SyllableSeparator.allCases, id: \.self) { separator in
                            Text(lang.string(separator.displayNameKey)).tag(separator)
                        }
                    } label: {
                        HStack {
                            Text(lang.string(.settingsSyllableSeparator))
                            SettingInfoButton(description: lang.string(.settingsSyllableSeparatorInfo))
                        }
                    }
                    .pickerStyle(.navigationLink)
                    .onChange(of: syllableSeparator) { _, newValue in
                        settings.syllableSeparator = newValue
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
                            SettingInfoButton(description: featureSummary("typingSettings"))
                        }
                    }
                    .onChange(of: autoSpaceEnabled) { _, newValue in
                        settings.isAutoSpaceEnabled = newValue
                    }
                } header: {
                    Text(lang.string(.settingsTypingSectionTitle))
                        .font(AppStyle.sectionHeaderFont)
                }

                // Keyboard settings. Font applies to every theme (font is NOT per-theme).
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

                    Toggle(isOn: $toolbarAutoCollapse) {
                        HStack {
                            Text(lang.string(.settingsToolbarAutoCollapse))
                            SettingInfoButton(description: lang.string(.settingsToolbarAutoCollapseInfo))
                        }
                    }
                    .onChange(of: toolbarAutoCollapse) { _, newValue in
                        settings.isToolbarAutoCollapse = newValue
                    }

                    Toggle(isOn: $isGlobeKeyEnabled) {
                        HStack {
                            Text(lang.string(.settingsGlobeKey))
                            SettingInfoButton(description: lang.string(.settingsGlobeKeyInfo))
                        }
                    }
                    .onChange(of: isGlobeKeyEnabled) { _, newValue in
                        settings.isGlobeKeyEnabled = newValue
                    }
                } header: {
                    Text(lang.string(.settingsKeyboardSectionTitle))
                        .font(AppStyle.sectionHeaderFont)
                }

                // Mode-specific settings follow Typing / Keyboard (default mode is TL): POJ, then TPS. Feedback closes the settings.
                Section {
                    Toggle(lang.string(.settingsDoubleTapOO), isOn: $isDoubleTapOOEnabled)
                        .onChange(of: isDoubleTapOOEnabled) { _, newValue in
                            settings.isDoubleTapOOEnabled = newValue
                        }

                    Toggle(lang.string(.settingsDoubleTapNN), isOn: $isDoubleTapNNEnabled)
                        .onChange(of: isDoubleTapNNEnabled) { _, newValue in
                            settings.isDoubleTapNNEnabled = newValue
                        }

                    // Nasal mark in POJ capitals (§53) — the case rule of the marker the row above
                    // composes: a picker over the stored switch, ᴺ or ⁿ, on a selection sub-page.
                    Picker(selection: $isNasalMarkerUppercaseEnabled) {
                        Text(lang.string(.settingsNasalMarkerUppercaseCapital)).tag(true)
                        Text(lang.string(.settingsNasalMarkerUppercaseSmall)).tag(false)
                    } label: {
                        Text(lang.string(.settingsNasalMarkerUppercase))
                    }
                    // Explicit: a Form's automatic picker style is context-dependent (Apple
                    // `PickerStyle`), and this row is a selection sub-page on purpose.
                    .pickerStyle(.navigationLink)
                    .onChange(of: isNasalMarkerUppercaseEnabled) { _, newValue in
                        settings.isNasalMarkerUppercaseEnabled = newValue
                    }
                } header: {
                    Text(lang.string(.settingsPojMode))
                        .font(AppStyle.sectionHeaderFont)
                }

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
                    Text(lang.string(.settingsTpsMode))
                        .font(AppStyle.sectionHeaderFont)
                }

                // Feedback
                Section {
                    Toggle(lang.string(.settingsSoundFeedback), isOn: $isAudioFeedbackEnabled)

                    Toggle(lang.string(.settingsVibrationFeedback), isOn: $isHapticFeedbackEnabled)
                } header: {
                    Text(lang.string(.settingsFeedbackSectionTitle))
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
                        // In-app action: primary text, accent icon (the share / email rows below leave
                        // the app and keep the link colour).
                        Label {
                            Text(diagnosticCopied
                                ? lang.string(.settingsDiagnosticCopied)
                                : lang.string(.settingsDiagnosticCopy))
                                .foregroundColor(.primary)
                        } icon: {
                            Image(latinSystemName: diagnosticCopied ? "checkmark" : "doc.on.doc")
                                .foregroundColor(AppStyle.accentBlue)
                        }
                    }

                    ShareLink(
                        item: diagnosticText,
                        subject: Text(verbatim: "TaigiKeyboard Bug Report"),
                        message: Text(diagnosticText),
                    ) {
                        Label(lang.string(.settingsDiagnosticShare), systemImage: "arrow.up.forward.square")
                    }

                    Button {
                        let info = DiagnosticService.gather()
                        let subject = "TaigiKeyboard Bug Report (v\(info.appVersion))"
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
            .navigationTitle(lang.string(TabType.settings.titleKey))
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

    private func featureSummary(_ featureId: String) -> String {
        FeatureContentLoader.features
            .first(where: { $0.id == featureId })?
            .summary?.resolve(for: lang.language) ?? ""
    }

    // MARK: - Actions

    private func resetAllSettings() {
        SettingsResetCoordinator.resetAll()

        // Sync local state
        selectedInputMode = settings.inputMode
        selectedFontType = settings.fontType
        autoCapitalizationEnabled = true // KeyboardKit default
        isAudioFeedbackEnabled = true
        isHapticFeedbackEnabled = true
        autoSpaceEnabled = settings.isAutoSpaceEnabled
        isDoubleTapOOEnabled = settings.isDoubleTapOOEnabled
        isDoubleTapNNEnabled = settings.isDoubleTapNNEnabled
        isNasalMarkerUppercaseEnabled = settings.isNasalMarkerUppercaseEnabled
        candidateDisplayMode = settings.candidateDisplayMode
        isOutputBothScripts = settings.storedIsOutputBothScripts
        literalRomanCandidateEnabled = settings.isLiteralRomanCandidateEnabled
        syllableSeparator = settings.syllableSeparator
        isTpsOrMappedToER = settings.isTpsOrMappedToER
        toolbarAutoCollapse = settings.isToolbarAutoCollapse
        isGlobeKeyEnabled = settings.isGlobeKeyEnabled
        // Reset persists displayLanguage back to system (Automatic), but the live store is injected — sync it
        // back so the UI language reverts immediately.
        lang.syncFromSettings()

        let impactFeedback = UIImpactFeedbackGenerator(style: .medium)
        impactFeedback.impactOccurred()
    }
}
