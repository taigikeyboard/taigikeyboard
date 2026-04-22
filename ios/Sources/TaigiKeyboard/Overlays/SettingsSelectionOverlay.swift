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

    @State private var isOutputBothScripts: Bool
    @State private var autoCapitalizationEnabled: Bool
    @State private var autoSpaceEnabled: Bool
    @State private var toolbarAutoCollapse: Bool
    @State private var isAudioFeedbackEnabled: Bool
    @State private var isHapticFeedbackEnabled: Bool
    @State private var isDoubleTapOOEnabled: Bool
    @State private var isDoubleTapNNEnabled: Bool
    @State private var isTpsOrMappedToER: Bool
    @State private var isGlobeKeyEnabled: Bool

    /// Prevents auto-dismiss during initial onAppear sync
    @State private var isReady = false

    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.candidateTheme) private var theme

    private static let autoCapKey = "com.keyboardkit.settings.keyboard.isAutocapitalizationEnabled"
    private static let audioFeedbackKey = "com.keyboardkit.settings.feedback.isAudioFeedbackEnabled"
    private static let hapticFeedbackKey = "com.keyboardkit.settings.feedback.isHapticFeedbackEnabled"

    init(isExpanded: Bool, onDismiss: @escaping () -> Void, onOpenApp: @escaping () -> Void) {
        self.isExpanded = isExpanded
        self.onDismiss = onDismiss
        self.onOpenApp = onOpenApp
        let s = SharedSettings.shared
        _isOutputBothScripts = State(initialValue: s.isOutputBothScripts)
        _autoCapitalizationEnabled = State(
            initialValue: KeyboardSettings.store.bool(forKey: Self.autoCapKey),
        )
        _autoSpaceEnabled = State(initialValue: s.isAutoSpaceEnabled)
        _toolbarAutoCollapse = State(initialValue: s.isToolbarAutoCollapse)
        _isAudioFeedbackEnabled = State(
            initialValue: KeyboardSettings.store.object(forKey: Self.audioFeedbackKey) as? Bool ?? true,
        )
        _isHapticFeedbackEnabled = State(
            initialValue: KeyboardSettings.store.object(forKey: Self.hapticFeedbackKey) as? Bool ?? true,
        )
        _isDoubleTapOOEnabled = State(initialValue: s.isDoubleTapOOEnabled)
        _isDoubleTapNNEnabled = State(initialValue: s.isDoubleTapNNEnabled)
        _isTpsOrMappedToER = State(initialValue: s.isTpsOrMappedToER)
        _isGlobeKeyEnabled = State(initialValue: s.isGlobeKeyEnabled)
    }

    var body: some View {
        Group {
            if isExpanded {
                GeometryReader { geometry in
                    let toolbarHeight = theme.height
                    contentView
                        .frame(maxWidth: .infinity)
                        .frame(height: geometry.size.height - toolbarHeight)
                }
            }
        }
    }

    // MARK: - Content

    private var contentView: some View {
        VStack(spacing: 0) {
            ScrollView(.vertical, showsIndicators: false) {
                VStack(alignment: .leading, spacing: 2) {
                    // General settings
                    settingsToggle(SettingsTexts.isOutputBothScripts, isOn: $isOutputBothScripts, icon: SettingsIcons.isOutputBothScripts) {
                        SharedSettings.shared.isOutputBothScripts = $0
                    }
                    settingsToggle(SettingsTexts.autoCapitalization, isOn: $autoCapitalizationEnabled, icon: SettingsIcons.autoCapitalization) {
                        KeyboardSettings.store.set($0, forKey: Self.autoCapKey)
                    }
                    settingsToggle(SettingsTexts.autoSpace, isOn: $autoSpaceEnabled, icon: SettingsIcons.autoSpace) {
                        SharedSettings.shared.isAutoSpaceEnabled = $0
                    }
                    settingsToggle(SettingsTexts.toolbarAutoCollapse, isOn: $toolbarAutoCollapse, icon: SettingsIcons.toolbar) {
                        SharedSettings.shared.isToolbarAutoCollapse = $0
                    }
                    settingsToggle(SettingsTexts.globeKey, isOn: $isGlobeKeyEnabled, icon: SettingsIcons.globeKey) {
                        SharedSettings.shared.isGlobeKeyEnabled = $0
                    }

                    // Feedback settings
                    settingsToggle(SettingsTexts.soundFeedback, isOn: $isAudioFeedbackEnabled, icon: SettingsIcons.soundFeedback) {
                        KeyboardSettings.store.set($0, forKey: Self.audioFeedbackKey)
                    }
                    settingsToggle(SettingsTexts.vibrationFeedback, isOn: $isHapticFeedbackEnabled, icon: SettingsIcons.vibrationFeedback) {
                        KeyboardSettings.store.set($0, forKey: Self.hapticFeedbackKey)
                    }

                    // POJ settings
                    settingsToggle(SettingsTexts.doubleTapOO, isOn: $isDoubleTapOOEnabled) {
                        SharedSettings.shared.isDoubleTapOOEnabled = $0
                    }
                    settingsToggle(SettingsTexts.doubleTapNN, isOn: $isDoubleTapNNEnabled) {
                        SharedSettings.shared.isDoubleTapNNEnabled = $0
                    }

                    // TPS settings
                    settingsToggle(SettingsTexts.isTpsOrMappedToER, isOn: $isTpsOrMappedToER) {
                        SharedSettings.shared.isTpsOrMappedToER = $0
                    }

                    openAppButton
                }
                .padding(.top, 12)
                .padding(.horizontal, 16)
                .padding(.bottom, 8)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .background(Color.keyboardBackground)
        .onAppear {
            let s = SharedSettings.shared
            isOutputBothScripts = s.isOutputBothScripts
            autoCapitalizationEnabled = KeyboardSettings.store.bool(forKey: Self.autoCapKey)
            autoSpaceEnabled = s.isAutoSpaceEnabled
            toolbarAutoCollapse = s.isToolbarAutoCollapse
            isAudioFeedbackEnabled = KeyboardSettings.store.object(forKey: Self.audioFeedbackKey) as? Bool ?? true
            isHapticFeedbackEnabled = KeyboardSettings.store.object(forKey: Self.hapticFeedbackKey) as? Bool ?? true
            isDoubleTapOOEnabled = s.isDoubleTapOOEnabled
            isDoubleTapNNEnabled = s.isDoubleTapNNEnabled
            isTpsOrMappedToER = s.isTpsOrMappedToER
            isGlobeKeyEnabled = s.isGlobeKeyEnabled
            isReady = true
        }
    }

    // MARK: - Components

    private func settingsToggle(
        _ label: String,
        isOn: Binding<Bool>,
        icon: String? = nil,
        onChange: @escaping (Bool) -> Void,
    ) -> some View {
        Toggle(isOn: isOn) {
            if let icon {
                HStack(spacing: 8) {
                    Image(latinSystemName: icon)
                        .font(.system(size: 16))
                        .frame(width: 20)
                    Text(label)
                }
            } else {
                Text(label)
            }
        }
        .font(KeyboardFonts.globalFont(size: 15))
        .foregroundColor(theme.primaryTextColor)
        .tint(.accentColor)
        .frame(height: 44)
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
            Text(SettingsTexts.openApp)
                .font(KeyboardFonts.globalFont(size: 15))
                .foregroundColor(.accentColor)
                .frame(maxWidth: .infinity)
                .frame(height: 44)
        }
        .buttonStyle(.plain)
        .padding(.top, 16)
    }

    // MARK: - Auto-dismiss

    private func autoDismissIfNeeded() {
        guard isReady, SharedSettings.shared.isToolbarAutoCollapse else { return }
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) {
            onDismiss()
        }
    }
}
