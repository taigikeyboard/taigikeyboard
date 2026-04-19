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
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.candidateTheme) private var theme

    private static let tpsDisabled = false

    init(isExpanded: Bool, onDismiss: @escaping () -> Void) {
        self.isExpanded = isExpanded
        self.onDismiss = onDismiss
        _selectedLayout = State(initialValue: SharedSettings.shared.keyboardLayoutType)
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
        ScrollView(.vertical, showsIndicators: false) {
            VStack(alignment: .leading, spacing: 12) {
                // Section 1: Romanization keyboards
                layoutSection(
                    header: LayoutTexts.romanizationKeyboard,
                    layouts: [
                        (.phahTaigi, LayoutTexts.phahTaigiLayout, "layout_phahtaigi_preview", false),
                        (.qwerty, LayoutTexts.standardLayout, "layout_standard_preview", false),
                        (.moe1, LayoutTexts.moe1Layout, "layout_moe1_preview", false),
                        (.moe2, LayoutTexts.moe2Layout, "layout_moe2_preview", false),
                    ],
                )

                // Section 2: Taigi phonetic
                layoutSection(
                    header: LayoutTexts.taigiPhonetic,
                    layouts: [
                        (.tps, LayoutTexts.tpsLayout, "layout_tps_preview", Self.tpsDisabled),
                    ],
                )
            }
            .padding(.top, 10)
            .padding(.bottom, 4)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .background(Color.keyboardBackground)
        .onAppear {
            selectedLayout = SharedSettings.shared.keyboardLayoutType
        }
    }

    // MARK: - Section

    private func layoutSection(
        header: String,
        layouts: [(KeyboardLayoutType, String, String?, Bool)],
    ) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(header)
                .font(.caption)
                .fontWeight(.semibold)
                .foregroundColor(theme.secondaryTextColor)
                .textCase(.uppercase)
                .padding(.horizontal, 12)

            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 8) {
                    ForEach(layouts, id: \.0) { layoutType, name, imageName, isDisabled in
                        LayoutCard(
                            name: name,
                            previewImageName: imageName,
                            isSelected: selectedLayout == layoutType,
                            isDisabled: isDisabled,
                            action: { selectLayout(layoutType) },
                        )
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
        }
        SharedSettings.shared.keyboardLayoutType = layout

        // Auto-collapse after short delay
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) {
            onDismiss()
        }
    }
}

// MARK: - Layout Card

private struct LayoutCard: View {
    let name: String
    let previewImageName: String?
    let isSelected: Bool
    var isDisabled: Bool = false
    let action: () -> Void

    private let cardWidth: CGFloat = 120

    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.candidateTheme) private var theme

    var body: some View {
        Button(action: action) {
            VStack(spacing: 4) {
                // Preview image with selected/disabled overlay
                ZStack {
                    previewImage
                        .clipShape(RoundedRectangle(cornerRadius: 8))

                    if isDisabled {
                        RoundedRectangle(cornerRadius: 8)
                            .fill(Color.black.opacity(0.5))

                        Text(LayoutTexts.comingSoon)
                            .font(.caption2)
                            .fontWeight(.semibold)
                            .foregroundColor(.white)
                            .padding(.horizontal, 8)
                            .padding(.vertical, 4)
                            .background(Color.black.opacity(0.7), in: Capsule())
                    } else if isSelected {
                        RoundedRectangle(cornerRadius: 8)
                            .fill(Color.black.opacity(0.25))

                        Circle()
                            .fill(Color.accentColor)
                            .frame(width: 24, height: 24)
                            .overlay(
                                Image(systemName: "checkmark")
                                    .font(.system(size: 12, weight: .bold))
                                    .foregroundColor(.white),
                            )
                    }
                }
                .overlay(
                    RoundedRectangle(cornerRadius: 8)
                        .stroke(isSelected && !isDisabled ? Color.accentColor : Color.clear, lineWidth: 2),
                )
                .frame(width: cardWidth)

                // Title label
                Text(name)
                    .font(.caption2)
                    .fontWeight(.medium)
                    .foregroundColor(isDisabled ? theme.secondaryTextColor : theme.primaryTextColor)
                    .lineLimit(1)
            }
        }
        .buttonStyle(.plain)
        .disabled(isDisabled)
    }

    /// Preview image with fallback for layouts without screenshots
    @ViewBuilder
    private var previewImage: some View {
        if let imageName = previewImageName, let uiImage = UIImage(named: imageName) {
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
                        Image(systemName: "keyboard")
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
