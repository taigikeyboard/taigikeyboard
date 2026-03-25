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
    @State private var enableDoubleTapOO: Bool
    @State private var enableDoubleTapNN: Bool
    @State private var tpsOrMapsToER: Bool

    @Environment(\.colorScheme) private var colorScheme

    private static let autoCapKey = "com.keyboardkit.settings.keyboard.isAutocapitalizationEnabled"

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
        _enableDoubleTapOO = State(initialValue: s.enableDoubleTapOO)
        _enableDoubleTapNN = State(initialValue: s.enableDoubleTapNN)
        _tpsOrMapsToER = State(initialValue: s.tpsOrMapsToER)
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
                VStack(alignment: .leading, spacing: 0) {
                    // General section
                    sectionHeader(Tab4Texts.tabTitle.hanji)

                    settingsToggle(Tab4Texts.outputBothScripts.hanji, isOn: $outputBothScripts) {
                        SharedSettings.shared.outputBothScripts = $0
                    }
                    settingsToggle(Tab4Texts.autoCapitalization.hanji, isOn: $autoCapitalizationEnabled) {
                        KeyboardSettings.store.set($0, forKey: Self.autoCapKey)
                    }
                    settingsToggle(Tab4Texts.autoSpace.hanji, isOn: $autoSpaceEnabled) {
                        SharedSettings.shared.isAutoSpaceEnabled = $0
                    }
                    settingsToggle(Tab4Texts.toolbarAutoCollapse.hanji, isOn: $toolbarAutoCollapse) {
                        SharedSettings.shared.isToolbarAutoCollapse = $0
                    }

                    // POJ section
                    settingsDivider()
                        .padding(.top, 16)
                    sectionHeader(Tab4Texts.pojSettingsSectionTitle.hanji)

                    settingsToggle(Tab4Texts.doubleTapOO.hanji, isOn: $enableDoubleTapOO) {
                        SharedSettings.shared.enableDoubleTapOO = $0
                    }
                    settingsToggle(Tab4Texts.doubleTapNN.hanji, isOn: $enableDoubleTapNN) {
                        SharedSettings.shared.enableDoubleTapNN = $0
                    }

                    // TPS section
                    settingsDivider()
                        .padding(.top, 16)
                    sectionHeader(Tab4Texts.tpsSettingsSectionTitle.hanji)

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
            enableDoubleTapOO = s.enableDoubleTapOO
            enableDoubleTapNN = s.enableDoubleTapNN
            tpsOrMapsToER = s.tpsOrMapsToER
        }
    }

    // MARK: - Components

    private func sectionHeader(_ title: String) -> some View {
        Text(title)
            .font(.caption)
            .fontWeight(.semibold)
            .foregroundColor(CandidateViewModels.Colors.secondaryTextColor)
            .textCase(.uppercase)
            .padding(.bottom, 4)
    }

    private func settingsToggle(
        _ label: String,
        isOn: Binding<Bool>,
        onChange: @escaping (Bool) -> Void
    ) -> some View {
        Toggle(label, isOn: isOn)
            .font(.system(size: 15))
            .foregroundColor(CandidateViewModels.Colors.primaryTextColor)
            .tint(.accentColor)
            .frame(height: 44)
            .onChange(of: isOn.wrappedValue) { _, newValue in
                onChange(newValue)
            }
    }

    private func settingsDivider() -> some View {
        Rectangle()
            .fill(CandidateViewModels.Colors.separatorColor)
            .frame(height: 0.5)
    }

    private var openAppButton: some View {
        Button(action: {
            onOpenApp()
            onDismiss()
        }) {
            Text(Tab4Texts.openApp.hanji)
                .font(.system(size: 15))
                .foregroundColor(.accentColor)
                .frame(maxWidth: .infinity)
                .frame(height: 44)
        }
        .buttonStyle(.plain)
        .padding(.top, 16)
    }
}
