import KeyboardKit
import SwiftUI

/// Tool-shortcut toolbar above the candidate row: an always-visible `+` toggle plus, when expanded,
/// nine equal-width buttons (4 input modes + symbol / layout / globe / dismiss / settings).
struct ToolShortcutsToolbar: View {
    @Binding var isExpanded: Bool
    let currentInputMode: InputMode
    let onInputModeChange: (InputMode) -> Void
    let onSymbolTap: () -> Void
    let onLayoutTap: () -> Void
    let onDismissKeyboard: () -> Void
    let onSettingsTap: () -> Void

    @Environment(\.candidateTheme) private var theme
    // Resolves smartbar a11y labels under the picker's display language (mirrors Android
    // InputView.applyAccessibilityStrings). Reading lang.string(_:) in body registers the live-switch.
    @Environment(DisplayLanguageStore.self) private var lang

    var body: some View {
        HStack(spacing: 0) {
            toggleButton

            if isExpanded {
                expandedButtons
                    .transition(.move(edge: .bottom))
            }
        }
    }

    /// `+` expand/collapse button; rotates 45° into a `×` when expanded.
    private var toggleButton: some View {
        Button(action: {
            withAnimation(.easeInOut(duration: 0.2)) {
                isExpanded.toggle()
            }
        }) {
            Image(latinSystemName: "plus")
                .font(KeyboardFonts.globalFont(size: 16))
                .fontWeight(.light)
                .foregroundColor(theme.primaryTextColor)
                .rotationEffect(.degrees(isExpanded ? 45 : 0))
                .animation(.easeInOut(duration: 0.2), value: isExpanded)
                .frame(width: 36, height: theme.height)
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .offset(y: 7)
        .accessibilityLabel(lang.string(.keyboardToggleToolbar))
    }

    private var expandedButtons: some View {
        HStack(spacing: 0) {
            inputModeButton(mode: .poj, label: "POJ")
            inputModeButton(mode: .tl, label: "TL")
            inputModeButton(mode: .english, label: "EN")
            inputModeButton(mode: .tps, label: "TPS")

            ToolShortcutButton(
                systemName: "number",
                accessibilityLabel: lang.string(.keyboardSymbolPanel),
                action: onSymbolTap,
            )

            ToolShortcutButton(
                systemName: "photo",
                accessibilityLabel: lang.string(.keyboardSwitchLayout),
                action: onLayoutTap,
            )

            globeButton

            ToolShortcutButton(
                systemName: "keyboard.chevron.compact.down",
                accessibilityLabel: lang.string(.keyboardDismissKeyboard),
                action: onDismissKeyboard,
            )

            ToolShortcutButton(
                systemName: "gearshape",
                accessibilityLabel: lang.string(.keyboardSettings),
                action: onSettingsTap,
            )
        }
        .offset(y: 7)
    }

    private func inputModeButton(mode: InputMode, label: String) -> some View {
        let isSelected = currentInputMode == mode
        return Button(action: { onInputModeChange(mode) }) {
            Text(label)
                .font(KeyboardFonts.globalFont(size: 16))
                .fontWeight(isSelected ? .semibold : .regular)
                .foregroundColor(isSelected ? .white : theme.primaryTextColor)
                .lineLimit(1)
                .fixedSize(horizontal: true, vertical: false)
                .padding(.horizontal, 6)
                .padding(.vertical, 6)
                .background(
                    RoundedRectangle(cornerRadius: 6)
                        .fill(isSelected ? Color.accentColor : Color.clear),
                )
                .animation(nil, value: isSelected)
        }
        .buttonStyle(.plain)
        .frame(maxWidth: .infinity)
        // VoiceOver reads the full mode name (台羅/白話字/…) via the existing InputMode.displayNameKey
        // while the visible chip stays the short code (TL/POJ/…). Selected state is conveyed by the
        // .isSelected trait, not baked into the label, so VoiceOver announces "selected" itself.
        .accessibilityLabel(lang.string(mode.displayNameKey))
        .accessibilityAddTraits(isSelected ? .isSelected : [])
    }

    /// Globe button — tap: next keyboard, long-press: keyboard picker.
    private var globeButton: some View {
        Keyboard.NextKeyboardButton {
            ToolShortcutIcon(systemName: "globe")
        }
        .frame(maxWidth: .infinity)
        .accessibilityLabel(lang.string(.keyboardSwitchInputMethod))
        .accessibilityHint(lang.string(.keyboardSwitchInputMethodHint))
    }
}

// MARK: - Icon shortcut button

/// Shared template for the symbol / layout / dismiss-keyboard / settings icon buttons.
private struct ToolShortcutButton: View {
    let systemName: String
    let accessibilityLabel: String
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            ToolShortcutIcon(systemName: systemName)
        }
        .buttonStyle(ToolShortcutButtonStyle())
        .frame(maxWidth: .infinity)
        // No accessibilityHint: the label already names the action and VoiceOver appends
        // "button, double tap to activate" — an explicit hint would just restate it.
        .accessibilityLabel(accessibilityLabel)
    }
}

/// Shared icon layout so `ToolShortcutButton` and the globe `NextKeyboardButton` look identical.
private struct ToolShortcutIcon: View {
    let systemName: String

    @Environment(\.candidateTheme) private var theme

    var body: some View {
        Image(latinSystemName: systemName)
            .font(KeyboardFonts.globalFont(size: 18))
            .fontWeight(.light)
            .foregroundColor(theme.primaryTextColor)
            .scaleEffect(1.2)
            .frame(width: 30, height: theme.height)
            .contentShape(Rectangle())
    }
}

/// Press feedback for tool shortcut buttons (scale + opacity).
private struct ToolShortcutButtonStyle: SwiftUI.ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .opacity(configuration.isPressed ? 0.5 : 1.0)
            .scaleEffect(configuration.isPressed ? 0.85 : 1.0)
            .animation(.easeInOut(duration: 0.1), value: configuration.isPressed)
    }
}
