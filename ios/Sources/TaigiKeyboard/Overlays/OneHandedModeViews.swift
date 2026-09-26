// One-handed mode UI: the toolbar keyboard button's long-press callout and the side panel in the docked gap.

import KeyboardKit
import SwiftUI

/// Long-press callout of the toolbar keyboard button: Dismiss / Left / Right. The docked side is
/// highlighted (none at full width); Dismiss never is. No Normal item: a tap on the toolbar button
/// or the side panel's restore button already returns to full width.
/// Mirrors Android `OneHandedMenuContent`.
struct OneHandedModeCallout: View {
    let currentMode: OneHandedMode
    /// The key long-press callout style (theme key fill + key text), so both callouts look alike.
    let style: KeyboardCalloutStyle
    let onSelectMode: (OneHandedMode) -> Void
    let onSelectDismiss: () -> Void

    @Environment(DisplayLanguageStore.self) private var lang

    var body: some View {
        HStack(spacing: 4) {
            cell(
                systemName: KeyboardToolbarAction.dismiss.systemImageName,
                label: lang.string(.keyboardDismissKeyboard),
                isSelected: false,
                action: onSelectDismiss,
            )
            modeCell(.left, systemName: KeyboardToolbarAction.left.systemImageName, labelKey: .keyboardOneHandedLeft)
            modeCell(.right, systemName: KeyboardToolbarAction.right.systemImageName, labelKey: .keyboardOneHandedRight)
        }
        .padding(6)
        .background(style.backgroundColor, in: RoundedRectangle(cornerRadius: 12))
        .shadow(color: .black.opacity(0.2), radius: 6, y: 2)
        .accessibilityElement(children: .contain)
        .accessibilityLabel(lang.string(.keyboardOneHandedMode))
    }

    private func modeCell(_ mode: OneHandedMode, systemName: String, labelKey: StringKey) -> some View {
        cell(
            systemName: systemName,
            label: lang.string(labelKey),
            isSelected: currentMode == mode,
            action: { onSelectMode(mode) },
        )
    }

    private func cell(systemName: String, label: String, isSelected: Bool, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            VStack(spacing: 4) {
                Image(latinSystemName: systemName)
                    .font(KeyboardFonts.globalFont(size: 22))
                    .fontWeight(.light)
                Text(label)
                    .font(KeyboardFonts.globalFont(size: 12))
                    .lineLimit(1)
                    .fixedSize()
            }
            .foregroundColor(isSelected ? .white : style.foregroundColor)
            .frame(minWidth: 56, minHeight: 56)
            .padding(.horizontal, 4)
            .background(
                RoundedRectangle(cornerRadius: 8)
                    .fill(isSelected ? Color.accentColor : Color.clear),
            )
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityAddTraits(isSelected ? .isSelected : [])
    }
}

/// Buttons in the gap beside the docked keys: move the keys to the other edge, or restore full width.
/// Mirrors Android `OneHandedSidePanel`.
struct OneHandedSidePanel: View {
    let mode: OneHandedMode
    let onSwapSide: () -> Void
    let onRestore: () -> Void

    @Environment(\.candidateTheme) private var theme
    @Environment(DisplayLanguageStore.self) private var lang

    var body: some View {
        VStack(spacing: 24) {
            // The chevron points at the gap: that is where the keys move to.
            panelButton(
                systemName: mode == .left ? "chevron.right" : "chevron.left",
                label: lang.string(.keyboardOneHandedSwapSide),
                action: onSwapSide,
            )
            panelButton(
                systemName: "arrow.up.left.and.arrow.down.right",
                label: lang.string(.keyboardOneHandedOff),
                action: onRestore,
            )
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private func panelButton(systemName: String, label: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Image(latinSystemName: systemName)
                .font(KeyboardFonts.globalFont(size: 22))
                .foregroundColor(theme.primaryTextColor)
                .frame(width: 48, height: 48)
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityLabel(label)
    }
}
