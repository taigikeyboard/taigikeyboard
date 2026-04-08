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

    @State private var outputBothScripts: Bool
    @State private var autoCapitalizationEnabled: Bool
    @State private var autoSpaceEnabled: Bool
    @State private var toolbarAutoCollapse: Bool
    @State private var isAudioFeedbackEnabled: Bool
    @State private var isHapticFeedbackEnabled: Bool
    @State private var enableDoubleTapOO: Bool
    @State private var enableDoubleTapNN: Bool
    @State private var tpsOrMapsToER: Bool
    @State private var isGlobeKeyEnabled: Bool

    // Prevents auto-dismiss during initial onAppear sync
    @State private var isReady = false

    @Environment(\.colorScheme) private var colorScheme

    private static let autoCapKey = "com.keyboardkit.settings.keyboard.isAutocapitalizationEnabled"
    private static let audioFeedbackKey = "com.keyboardkit.settings.feedback.isAudioFeedbackEnabled"
    private static let hapticFeedbackKey = "com.keyboardkit.settings.feedback.isHapticFeedbackEnabled"

    init(isExpanded: Bool, onDismiss: @escaping () -> Void, onOpenApp: @escaping () -> Void) {
        self.isExpanded = isExpanded
        self.onDismiss = onDismiss
        self.onOpenApp = onOpenApp
        let s = SharedSettings.shared
        _outputBothScripts = State(initialValue: s.outputBothScripts)
        _autoCapitalizationEnabled = State(
            initialValue: KeyboardSettings.store.bool(forKey: Self.autoCapKey)
        )
        _autoSpaceEnabled = State(initialValue: s.isAutoSpaceEnabled)
        _toolbarAutoCollapse = State(initialValue: s.isToolbarAutoCollapse)
        _isAudioFeedbackEnabled = State(
            initialValue: KeyboardSettings.store.object(forKey: Self.audioFeedbackKey) as? Bool ?? true
        )
        _isHapticFeedbackEnabled = State(
            initialValue: KeyboardSettings.store.object(forKey: Self.hapticFeedbackKey) as? Bool ?? true
        )
        _enableDoubleTapOO = State(initialValue: s.enableDoubleTapOO)
        _enableDoubleTapNN = State(initialValue: s.enableDoubleTapNN)
        _tpsOrMapsToER = State(initialValue: s.tpsOrMapsToER)
        _isGlobeKeyEnabled = State(initialValue: s.isGlobeKeyEnabled)
    }

    var body: some View {
        Group {
            if isExpanded {
                GeometryReader { geometry in
                    let toolbarHeight = CandidateViewModels.UI.height
                    contentView
                        .frame(maxWidth: .infinity)
                        .frame(height: geometry.size.height - toolbarHeight)
                }
            } else {
                EmptyView()
            }
        }
    }

    // MARK: - Content

    private var contentView: some View {
        VStack(spacing: 0) {
            ScrollView(.vertical, showsIndicators: false) {
                VStack(alignment: .leading, spacing: 2) {
                    // General settings
                    settingsToggle(Tab4Texts.outputBothScripts.hanji, isOn: $outputBothScripts, icon: Tab4Texts.outputBothScriptsIcon) {
                        SharedSettings.shared.outputBothScripts = $0
                    }
                    settingsToggle(Tab4Texts.autoCapitalization.hanji, isOn: $autoCapitalizationEnabled, icon: Tab4Texts.autoCapitalizationIcon) {
                        KeyboardSettings.store.set($0, forKey: Self.autoCapKey)
                    }
                    settingsToggle(Tab4Texts.autoSpace.hanji, isOn: $autoSpaceEnabled, icon: Tab4Texts.autoSpaceIcon) {
                        SharedSettings.shared.isAutoSpaceEnabled = $0
                    }
                    settingsToggle(Tab4Texts.toolbarAutoCollapse.hanji, isOn: $toolbarAutoCollapse, icon: Tab4Texts.toolbarIcon) {
                        SharedSettings.shared.isToolbarAutoCollapse = $0
                    }
                    settingsToggle(Tab4Texts.globeKey.hanji, isOn: $isGlobeKeyEnabled, icon: Tab4Texts.globeKeyIcon) {
                        SharedSettings.shared.isGlobeKeyEnabled = $0
                    }

                    // Feedback settings
                    settingsToggle(Tab4Texts.soundFeedback.hanji, isOn: $isAudioFeedbackEnabled, icon: Tab4Texts.soundFeedbackIcon) {
                        KeyboardSettings.store.set($0, forKey: Self.audioFeedbackKey)
                    }
                    settingsToggle(Tab4Texts.vibrationFeedback.hanji, isOn: $isHapticFeedbackEnabled, icon: Tab4Texts.vibrationFeedbackIcon) {
                        KeyboardSettings.store.set($0, forKey: Self.hapticFeedbackKey)
                    }

                    // POJ settings
                    settingsToggle(Tab4Texts.doubleTapOO.hanji, isOn: $enableDoubleTapOO) {
                        SharedSettings.shared.enableDoubleTapOO = $0
                    }
                    settingsToggle(Tab4Texts.doubleTapNN.hanji, isOn: $enableDoubleTapNN) {
                        SharedSettings.shared.enableDoubleTapNN = $0
                    }

                    // TPS settings
                    settingsToggle(Tab4Texts.tpsOrMapsToER.hanji, isOn: $tpsOrMapsToER) {
                        SharedSettings.shared.tpsOrMapsToER = $0
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
            outputBothScripts = s.outputBothScripts
            autoCapitalizationEnabled = KeyboardSettings.store.bool(forKey: Self.autoCapKey)
            autoSpaceEnabled = s.isAutoSpaceEnabled
            toolbarAutoCollapse = s.isToolbarAutoCollapse
            isAudioFeedbackEnabled = KeyboardSettings.store.object(forKey: Self.audioFeedbackKey) as? Bool ?? true
            isHapticFeedbackEnabled = KeyboardSettings.store.object(forKey: Self.hapticFeedbackKey) as? Bool ?? true
            enableDoubleTapOO = s.enableDoubleTapOO
            enableDoubleTapNN = s.enableDoubleTapNN
            tpsOrMapsToER = s.tpsOrMapsToER
            isGlobeKeyEnabled = s.isGlobeKeyEnabled
            isReady = true
        }
    }

    // MARK: - Components

    private func settingsToggle(
        _ label: String,
        isOn: Binding<Bool>,
        icon: String? = nil,
        onChange: @escaping (Bool) -> Void
    ) -> some View {
        Toggle(isOn: isOn) {
            if let icon {
                HStack(spacing: 8) {
                    Image(systemName: icon)
                        .font(.system(size: 16))
                        .frame(width: 20)
                    Text(label)
                }
            } else {
                Text(label)
            }
        }
        .font(KeyboardModels.Fonts.globalFont(size: 15))
        .foregroundColor(CandidateViewModels.Colors.primaryTextColor)
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
            Text(Tab4Texts.openApp.hanji)
                .font(KeyboardModels.Fonts.globalFont(size: 15))
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
