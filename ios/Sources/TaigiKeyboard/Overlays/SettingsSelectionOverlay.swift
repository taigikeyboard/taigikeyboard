// 中文: 從工具列叫出的「設定」overlay — 不必離開鍵盤就能切換常用設定。
// 中文: 涵蓋通用 / 回饋 / POJ 雙擊 / TPS or→ㄜ 對應 / 開啟主 App。

import KeyboardKit
import SwiftUI

/// Settings selection overlay panel
///
/// Displays keyboard behavior settings (toggles) directly from the keyboard toolbar,
/// allowing the user to change settings without leaving the keyboard context.
/// Follows the same overlay pattern as `LayoutSelectionOverlay`.
// 中文: 鍵盤設定選擇面板 — 與 LayoutSelectionOverlay 採用相同的 overlay 樣式。
struct SettingsSelectionOverlay: View {
    let isExpanded: Bool
    let onDismiss: () -> Void
    let onOpenApp: () -> Void

    @State private var isOutputBothScripts: Bool
    @State private var literalRomanCandidateEnabled: Bool
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
    // 中文: onAppear 同步狀態時暫時擋住 auto-dismiss,避免一開啟就被收合。
    @State private var isReady = false

    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.candidateTheme) private var theme
    @Environment(DisplayLanguageStore.self) private var lang

    private static let autoCapKey = "com.keyboardkit.settings.keyboard.isAutocapitalizationEnabled"
    private static let audioFeedbackKey = "com.keyboardkit.settings.feedback.isAudioFeedbackEnabled"
    private static let hapticFeedbackKey = "com.keyboardkit.settings.feedback.isHapticFeedbackEnabled"

    init(isExpanded: Bool, onDismiss: @escaping () -> Void, onOpenApp: @escaping () -> Void) {
        self.isExpanded = isExpanded
        self.onDismiss = onDismiss
        self.onOpenApp = onOpenApp
        let s = SharedSettings.shared
        _isOutputBothScripts = State(initialValue: s.isOutputBothScripts)
        _literalRomanCandidateEnabled = State(initialValue: s.isLiteralRomanCandidateEnabled)
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
        contentView.keyboardOverlayPanel(isExpanded: isExpanded, theme: theme)
    }

    // MARK: - Content

    private var contentView: some View {
        VStack(spacing: 0) {
            ScrollView(.vertical, showsIndicators: false) {
                VStack(alignment: .leading, spacing: 2) {
                    // General settings
                    settingsToggle(lang.string(.settingsOutputBothScripts), isOn: $isOutputBothScripts, icon: SettingsIcons.isOutputBothScripts) {
                        SharedSettings.shared.isOutputBothScripts = $0
                    }
                    settingsToggle(lang.string(.settingsLiteralRomanCandidate), isOn: $literalRomanCandidateEnabled, icon: SettingsIcons.literalRomanCandidate) {
                        SharedSettings.shared.isLiteralRomanCandidateEnabled = $0
                    }
                    settingsToggle(lang.string(.settingsAutoCapitalization), isOn: $autoCapitalizationEnabled, icon: SettingsIcons.autoCapitalization) {
                        KeyboardSettings.store.set($0, forKey: Self.autoCapKey)
                    }
                    settingsToggle(lang.string(.settingsAutoSpace), isOn: $autoSpaceEnabled, icon: SettingsIcons.autoSpace) {
                        SharedSettings.shared.isAutoSpaceEnabled = $0
                    }
                    settingsToggle(lang.string(.settingsToolbarAutoCollapse), isOn: $toolbarAutoCollapse, icon: SettingsIcons.toolbar) {
                        SharedSettings.shared.isToolbarAutoCollapse = $0
                    }
                    settingsToggle(lang.string(.settingsGlobeKey), isOn: $isGlobeKeyEnabled, icon: SettingsIcons.globeKey) {
                        SharedSettings.shared.isGlobeKeyEnabled = $0
                    }

                    // Feedback settings
                    settingsToggle(lang.string(.settingsSoundFeedback), isOn: $isAudioFeedbackEnabled, icon: SettingsIcons.soundFeedback) {
                        KeyboardSettings.store.set($0, forKey: Self.audioFeedbackKey)
                    }
                    settingsToggle(lang.string(.settingsVibrationFeedback), isOn: $isHapticFeedbackEnabled, icon: SettingsIcons.vibrationFeedback) {
                        KeyboardSettings.store.set($0, forKey: Self.hapticFeedbackKey)
                    }

                    // POJ settings
                    settingsToggle(lang.string(.settingsDoubleTapOO), isOn: $isDoubleTapOOEnabled) {
                        SharedSettings.shared.isDoubleTapOOEnabled = $0
                    }
                    settingsToggle(lang.string(.settingsDoubleTapNN), isOn: $isDoubleTapNNEnabled) {
                        SharedSettings.shared.isDoubleTapNNEnabled = $0
                    }

                    // TPS settings
                    settingsToggle(lang.string(.settingsTpsOrMapsToER), isOn: $isTpsOrMappedToER) {
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
        .onAppear {
            // 中文: 開啟 overlay 時重讀 App-Group 顯示語言 tag — 跨程序(host 改語言)可靠的重讀點。
            lang.syncFromSettings()
            let s = SharedSettings.shared
            isOutputBothScripts = s.isOutputBothScripts
            literalRomanCandidateEnabled = s.isLiteralRomanCandidateEnabled
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
            Text(lang.string(.settingsOpenApp))
                .font(KeyboardFonts.globalFont(size: 15))
                .foregroundColor(.accentColor)
                .frame(maxWidth: .infinity)
                .frame(height: 44)
        }
        .buttonStyle(.plain)
        .padding(.top, 16)
    }

    // MARK: - Auto-dismiss

    // 中文: 切換設定後若使用者啟用「自動收合工具列」則延遲 0.3 秒收合 overlay。
    private func autoDismissIfNeeded() {
        guard isReady, SharedSettings.shared.isToolbarAutoCollapse else { return }
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) {
            onDismiss()
        }
    }
}
