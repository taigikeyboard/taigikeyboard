// Covers Typing / Keyboard / POJ / TPS / Feedback (the Settings tab's sections and order) / Open main app.

import KeyboardKit
import SwiftUI

/// Settings selection overlay panel
///
/// Displays keyboard behavior settings (toggles) directly from the keyboard toolbar,
/// allowing the user to change settings without leaving the keyboard context.
/// Follows the same overlay pattern as `LayoutSelectionOverlay`.
struct SettingsSelectionOverlay: View {
    let isExpanded: Bool
    let onDismiss: () -> Void
    let onOpenApp: () -> Void
    /// Routed through `KeyboardContext.candidateDisplayMode` by the host view so the strip +
    /// expanded overlay re-render immediately (same path as the 文/A toggle).
    let onCandidateDisplayModeChange: (CandidateDisplayMode) -> Void

    @State private var candidateDisplayMode: CandidateDisplayMode
    @State private var isOutputBothScripts: Bool
    @State private var literalRomanCandidateEnabled: Bool
    @State private var autoCapitalizationEnabled: Bool
    @State private var autoSpaceEnabled: Bool
    @State private var toolbarAutoCollapse: Bool
    @State private var isAudioFeedbackEnabled: Bool
    @State private var isHapticFeedbackEnabled: Bool
    @State private var isDoubleTapOOEnabled: Bool
    @State private var isDoubleTapNNEnabled: Bool
    @State private var isNasalMarkerUppercaseEnabled: Bool
    @State private var syllableSeparator: SyllableSeparator
    @State private var isTpsOrMappedToER: Bool
    @State private var isGlobeKeyEnabled: Bool

    /// Prevents auto-dismiss during initial onAppear sync
    @State private var isReady = false

    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.candidateTheme) private var theme
    @Environment(DisplayLanguageStore.self) private var lang

    private static let autoCapKey = "com.keyboardkit.settings.keyboard.isAutocapitalizationEnabled"
    private static let audioFeedbackKey = "com.keyboardkit.settings.feedback.isAudioFeedbackEnabled"
    private static let hapticFeedbackKey = "com.keyboardkit.settings.feedback.isHapticFeedbackEnabled"
    /// Body tier (`ui-style-guide.md`), the size the Settings tab and the Android overlay use.
    private static let rowFontSize: CGFloat = 17

    init(
        isExpanded: Bool,
        onDismiss: @escaping () -> Void,
        onOpenApp: @escaping () -> Void,
        onCandidateDisplayModeChange: @escaping (CandidateDisplayMode) -> Void,
    ) {
        self.isExpanded = isExpanded
        self.onDismiss = onDismiss
        self.onOpenApp = onOpenApp
        self.onCandidateDisplayModeChange = onCandidateDisplayModeChange
        let s = SharedSettings.shared
        _candidateDisplayMode = State(initialValue: s.candidateDisplayMode)
        // Toggle binds the STORED flag: it keeps showing the user's choice while disabled under Romanization Only.
        _isOutputBothScripts = State(initialValue: s.storedIsOutputBothScripts)
        _literalRomanCandidateEnabled = State(initialValue: s.isLiteralRomanCandidateEnabled)
        _autoCapitalizationEnabled = State(
            initialValue: UserDefaults.keyboardSettings.bool(forKey: Self.autoCapKey),
        )
        _autoSpaceEnabled = State(initialValue: s.isAutoSpaceEnabled)
        _toolbarAutoCollapse = State(initialValue: s.isToolbarAutoCollapse)
        _isAudioFeedbackEnabled = State(
            initialValue: UserDefaults.keyboardSettings.object(forKey: Self.audioFeedbackKey) as? Bool ?? true,
        )
        _isHapticFeedbackEnabled = State(
            initialValue: UserDefaults.keyboardSettings.object(forKey: Self.hapticFeedbackKey) as? Bool ?? true,
        )
        _isDoubleTapOOEnabled = State(initialValue: s.isDoubleTapOOEnabled)
        _isDoubleTapNNEnabled = State(initialValue: s.isDoubleTapNNEnabled)
        _isNasalMarkerUppercaseEnabled = State(initialValue: s.isNasalMarkerUppercaseEnabled)
        _syllableSeparator = State(initialValue: s.syllableSeparator)
        _isTpsOrMappedToER = State(initialValue: s.isTpsOrMappedToER)
        _isGlobeKeyEnabled = State(initialValue: s.isGlobeKeyEnabled)
    }

    var body: some View {
        contentView.keyboardOverlayPanel(isExpanded: isExpanded, theme: theme)
    }

    // MARK: - Content

    private var contentView: some View {
        VStack(spacing: 0) {
            ScrollView(.vertical, showsIndicators: false) {
                VStack(alignment: .leading, spacing: 2) {
                    sectionHeader(lang.string(.settingsTypingSectionTitle))
                    menuPickerRow(
                        lang.string(.settingsCandidateDisplayMode),
                        selection: $candidateDisplayMode,
                        options: CandidateDisplayMode.allCases.map { ($0, lang.string($0.displayNameKey)) },
                        onChange: onCandidateDisplayModeChange,
                    )
                    settingsToggle(lang.string(.settingsLiteralRomanCandidate), isOn: $literalRomanCandidateEnabled) {
                        SharedSettings.shared.isLiteralRomanCandidateEnabled = $0
                    }
                    settingsToggle(lang.string(.settingsOutputBothScripts), isOn: $isOutputBothScripts) {
                        SharedSettings.shared.storedIsOutputBothScripts = $0
                    }
                    // Annotate in Brackets is meaningless without hanji; stored value stays untouched.
                    .disabled(!candidateDisplayMode.showsHanji)
                    menuPickerRow(
                        lang.string(.settingsSyllableSeparator),
                        selection: $syllableSeparator,
                        options: SyllableSeparator.allCases.map { ($0, lang.string($0.displayNameKey)) },
                    ) { SharedSettings.shared.syllableSeparator = $0 }
                    settingsToggle(lang.string(.settingsAutoCapitalization), isOn: $autoCapitalizationEnabled) {
                        UserDefaults.keyboardSettings.set($0, forKey: Self.autoCapKey)
                    }
                    settingsToggle(lang.string(.settingsAutoSpace), isOn: $autoSpaceEnabled) {
                        SharedSettings.shared.isAutoSpaceEnabled = $0
                    }

                    sectionHeader(lang.string(.settingsKeyboardSectionTitle))
                    settingsToggle(lang.string(.settingsToolbarAutoCollapse), isOn: $toolbarAutoCollapse) {
                        SharedSettings.shared.isToolbarAutoCollapse = $0
                    }
                    settingsToggle(lang.string(.settingsGlobeKey), isOn: $isGlobeKeyEnabled) {
                        SharedSettings.shared.isGlobeKeyEnabled = $0
                    }

                    sectionHeader(lang.string(.settingsPojMode))
                    settingsToggle(lang.string(.settingsDoubleTapOO), isOn: $isDoubleTapOOEnabled) {
                        SharedSettings.shared.isDoubleTapOOEnabled = $0
                    }
                    settingsToggle(lang.string(.settingsDoubleTapNN), isOn: $isDoubleTapNNEnabled) {
                        SharedSettings.shared.isDoubleTapNNEnabled = $0
                    }
                    menuPickerRow(
                        lang.string(.settingsNasalMarkerUppercase),
                        selection: $isNasalMarkerUppercaseEnabled,
                        options: [
                            (true, lang.string(.settingsNasalMarkerUppercaseCapital)),
                            (false, lang.string(.settingsNasalMarkerUppercaseSmall)),
                        ],
                    ) { SharedSettings.shared.isNasalMarkerUppercaseEnabled = $0 }

                    sectionHeader(lang.string(.settingsTpsMode))
                    settingsToggle(lang.string(.settingsTpsOrMapsToER), isOn: $isTpsOrMappedToER) {
                        SharedSettings.shared.isTpsOrMappedToER = $0
                    }

                    sectionHeader(lang.string(.settingsFeedbackSectionTitle))
                    settingsToggle(lang.string(.settingsSoundFeedback), isOn: $isAudioFeedbackEnabled) {
                        UserDefaults.keyboardSettings.set($0, forKey: Self.audioFeedbackKey)
                    }
                    settingsToggle(lang.string(.settingsVibrationFeedback), isOn: $isHapticFeedbackEnabled) {
                        UserDefaults.keyboardSettings.set($0, forKey: Self.hapticFeedbackKey)
                    }

                    openAppButton
                }
                .padding(.horizontal, 16)
                .padding(.bottom, 8)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .onAppear {
            // Re-reads the App-Group display-language tag on overlay open — the reliable
            // re-read point for a cross-process (host-changed) language update.
            lang.syncFromSettings()
            let s = SharedSettings.shared
            candidateDisplayMode = s.candidateDisplayMode
            isOutputBothScripts = s.storedIsOutputBothScripts
            literalRomanCandidateEnabled = s.isLiteralRomanCandidateEnabled
            autoCapitalizationEnabled = UserDefaults.keyboardSettings.bool(forKey: Self.autoCapKey)
            autoSpaceEnabled = s.isAutoSpaceEnabled
            toolbarAutoCollapse = s.isToolbarAutoCollapse
            isAudioFeedbackEnabled = UserDefaults.keyboardSettings.object(forKey: Self.audioFeedbackKey) as? Bool ?? true
            isHapticFeedbackEnabled = UserDefaults.keyboardSettings.object(forKey: Self.hapticFeedbackKey) as? Bool ?? true
            isDoubleTapOOEnabled = s.isDoubleTapOOEnabled
            isDoubleTapNNEnabled = s.isDoubleTapNNEnabled
            isNasalMarkerUppercaseEnabled = s.isNasalMarkerUppercaseEnabled
            syllableSeparator = s.syllableSeparator
            isTpsOrMappedToER = s.isTpsOrMappedToER
            isGlobeKeyEnabled = s.isGlobeKeyEnabled
            isReady = true
        }
    }

    // MARK: - Components

    /// Each section opens with its header; the header's top padding is the gap above it (and the panel's top inset).
    private func sectionHeader(_ title: String) -> some View {
        KeyboardOverlaySectionHeader(title: title)
            .padding(.top, 12)
            .padding(.bottom, 2)
    }

    /// Menu-picker row shaped like the toggles (label left, current value right).
    /// The choices do not fit as segments beside the label at keyboard width.
    private func menuPickerRow<Value: Hashable>(
        _ label: String,
        selection: Binding<Value>,
        options: [(Value, String)],
        onChange: @escaping (Value) -> Void,
    ) -> some View {
        HStack(spacing: 8) {
            Text(label)
            Spacer()
            Picker(label, selection: selection) {
                ForEach(options, id: \.0) { value, name in
                    Text(name).tag(value)
                }
            }
            .pickerStyle(.menu)
            .labelsHidden()
            // `.menu` paints its label in the accent colour; match the row text instead.
            .tint(theme.primaryTextColor)
            .fixedSize()
        }
        .font(KeyboardFonts.globalFont(size: Self.rowFontSize))
        .foregroundColor(theme.primaryTextColor)
        .frame(minHeight: 44)
        .onChange(of: selection.wrappedValue) { _, newValue in
            onChange(newValue)
            autoDismissIfNeeded()
        }
    }

    private func settingsToggle(
        _ label: String,
        isOn: Binding<Bool>,
        onChange: @escaping (Bool) -> Void,
    ) -> some View {
        Toggle(isOn: isOn) {
            Text(label)
        }
        .font(KeyboardFonts.globalFont(size: Self.rowFontSize))
        .foregroundColor(theme.primaryTextColor)
        .frame(minHeight: 44)
        .onChange(of: isOn.wrappedValue) { _, newValue in
            onChange(newValue)
            autoDismissIfNeeded()
        }
    }

    private var openAppButton: some View {
        Button(action: {
            onOpenApp()
            onDismiss()
        }) {
            Text(lang.string(.settingsOpenApp))
                .font(KeyboardFonts.globalFont(size: Self.rowFontSize))
                .foregroundColor(.accentColor)
                .frame(maxWidth: .infinity)
                .frame(minHeight: 44)
        }
        .buttonStyle(.plain)
        .padding(.top, 16)
    }

    // MARK: - Auto-dismiss

    // After a setting changes, collapses the overlay after 0.3s if Collapse Toolbar After Selection is enabled.
    private func autoDismissIfNeeded() {
        guard isReady, SharedSettings.shared.isToolbarAutoCollapse else { return }
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) {
            onDismiss()
        }
    }
}
