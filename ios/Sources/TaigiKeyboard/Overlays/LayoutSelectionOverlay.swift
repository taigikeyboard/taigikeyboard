import SwiftUI

/// Layout selection overlay panel
///
/// Displays available keyboard layouts as compact cards, allowing
/// the user to switch layouts directly from the keyboard toolbar.
/// Follows the same overlay pattern as `ExpandedCandidateOverlay`.
struct LayoutSelectionOverlay: View {
    let isExpanded: Bool
    let onDismiss: () -> Void

    @State private var selectedLayout: KeyboardLayoutType
    /// The active custom layout's id; nil while a built-in layout shows.
    @State private var selectedCustomLayoutId: UUID?
    @State private var customLayouts: [CustomLayout]
    /// The mode the romanization cards render in once picked, so a POJ user sees the POJ keys.
    /// A snapshot is enough: the toolbar's mode switch closes this panel.
    @State private var previewInputMode: InputMode
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.candidateTheme) private var theme
    @Environment(DisplayLanguageStore.self) private var lang

    init(isExpanded: Bool, onDismiss: @escaping () -> Void) {
        self.isExpanded = isExpanded
        self.onDismiss = onDismiss
        let settings = SharedSettings.shared
        _selectedLayout = State(initialValue: settings.keyboardLayoutType)
        _selectedCustomLayoutId = State(initialValue: settings.activeCustomLayout?.id)
        _customLayouts = State(initialValue: settings.loadCustomLayouts())
        _previewInputMode = State(initialValue: settings.romanizationInputMode)
    }

    var body: some View {
        contentView.keyboardOverlayPanel(isExpanded: isExpanded, theme: theme)
    }

    // MARK: - Content

    private var contentView: some View {
        ScrollView(.vertical, showsIndicators: false) {
            VStack(alignment: .leading, spacing: 12) {
                if !customLayouts.isEmpty {
                    customLayoutSection
                }

                // Romanization keyboards
                layoutSection(
                    header: lang.string(.layoutRomanizationKeyboard),
                    layouts: [
                        (.phahTaigi, lang.string(.layoutPhahTaigiLayout)),
                        (.qwerty, lang.string(.layoutStandardLayout)),
                        (.moe1, lang.string(.layoutMoe1Layout)),
                        (.moe2, lang.string(.layoutMoe2Layout)),
                    ],
                )

                // Taigi phonetic
                layoutSection(
                    header: lang.string(.settingsTpsMode),
                    layouts: [
                        (.tps, lang.string(.layoutTpsLayout)),
                    ],
                )
            }
            .padding(.top, 10)
            .padding(.bottom, 4)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .onAppear {
            // Reliable re-sync point for the App-Group display-language tag (host may change it).
            lang.syncFromSettings()
            let settings = SharedSettings.shared
            selectedLayout = settings.keyboardLayoutType
            selectedCustomLayoutId = settings.activeCustomLayout?.id
            customLayouts = settings.loadCustomLayouts()
            previewInputMode = settings.romanizationInputMode
        }
    }

    // MARK: - Section

    private var customLayoutSection: some View {
        VStack(alignment: .leading, spacing: 6) {
            KeyboardOverlaySectionHeader(title: lang.string(.layoutCustomLayoutsSection))
                .padding(.horizontal, 12)

            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 8) {
                    ForEach(customLayouts) { layout in
                        LayoutCard(
                            name: layout.name,
                            isSelected: selectedCustomLayoutId == layout.id,
                            action: { selectCustomLayout(layout) },
                        ) {
                            LayoutKeyGrid(rows: layout.rows(withGlobe: false))
                                .aspectRatio(LayoutKeyGrid.cardAspectRatio, contentMode: .fit)
                        }
                    }
                }
                .padding(.horizontal, 12)
            }
        }
    }

    private func layoutSection(
        header: String,
        layouts: [(KeyboardLayoutType, String)],
    ) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            KeyboardOverlaySectionHeader(title: header)
                .padding(.horizontal, 12)

            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 8) {
                    ForEach(layouts, id: \.0) { layoutType, name in
                        LayoutCard(
                            name: name,
                            isSelected: selectedCustomLayoutId == nil && selectedLayout == layoutType,
                            action: { selectLayout(layoutType) },
                        ) {
                            LayoutScreenshot(imageName: layoutType.previewImageName(for: previewInputMode), name: name)
                        }
                    }
                }
                .padding(.horizontal, 12)
            }
        }
    }

    // MARK: - Selection

    private func selectLayout(_ layout: KeyboardLayoutType) {
        withAnimation(.easeInOut(duration: 0.15)) {
            selectedLayout = layout
            selectedCustomLayoutId = nil
        }
        SharedSettings.shared.keyboardLayoutType = layout
        collapseSoon()
    }

    private func selectCustomLayout(_ layout: CustomLayout) {
        withAnimation(.easeInOut(duration: 0.15)) {
            selectedCustomLayoutId = layout.id
        }
        SharedSettings.shared.applyCustomLayout(layout)
        collapseSoon()
    }

    private func collapseSoon() {
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) {
            onDismiss()
        }
    }
}

// MARK: - Layout Card

private struct LayoutCard<Preview: View>: View {
    let name: String
    let isSelected: Bool
    let action: () -> Void
    @ViewBuilder let preview: Preview

    private let cardWidth: CGFloat = 120

    @Environment(\.candidateTheme) private var theme

    var body: some View {
        Button(action: action) {
            VStack(spacing: 4) {
                // Preview with selected overlay
                ZStack {
                    preview
                        .clipShape(RoundedRectangle(cornerRadius: 8))

                    if isSelected {
                        RoundedRectangle(cornerRadius: 8)
                            .fill(Color.black.opacity(0.25))

                        Circle()
                            .fill(Color.accentColor)
                            .frame(width: 24, height: 24)
                            .overlay(
                                Image(latinSystemName: "checkmark")
                                    .font(.system(size: 12, weight: .bold))
                                    .foregroundColor(.white),
                            )
                    }
                }
                .overlay(
                    RoundedRectangle(cornerRadius: 8)
                        .stroke(isSelected ? Color.accentColor : Color.clear, lineWidth: 2),
                )
                .frame(width: cardWidth)

                // Title label
                Text(name)
                    .font(.caption2)
                    .fontWeight(.medium)
                    .foregroundColor(theme.primaryTextColor)
                    .lineLimit(1)
            }
        }
        .buttonStyle(.plain)
        // VoiceOver reads the name, not the preview's keys. The selected layout is shown only by a
        // checkmark + accent border; announce it so VoiceOver conveys which layout is active.
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(name)
        .accessibilityAddTraits(isSelected ? [.isButton, .isSelected] : .isButton)
    }
}

/// A built-in layout's card screenshot, with a fallback for a missing asset.
private struct LayoutScreenshot: View {
    let imageName: String
    let name: String

    @Environment(\.colorScheme) private var colorScheme

    var body: some View {
        if let uiImage = UIImage(named: imageName) {
            Image(uiImage: uiImage)
                .resizable()
                .aspectRatio(contentMode: .fit)
        } else {
            // Fallback: keyboard icon + name (matches LayoutTab's fallback)
            Rectangle()
                .fill(colorScheme == .dark ? Color(.systemGray5) : Color(.systemGray6))
                .aspectRatio(1.8, contentMode: .fit)
                .overlay(
                    VStack(spacing: 4) {
                        Image(latinSystemName: "keyboard")
                            .font(.system(size: 24))
                            .foregroundColor(.secondary)
                        Text(name)
                            .font(.caption2)
                            .foregroundColor(.secondary)
                    },
                )
        }
    }
}
