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

    #if DEBUG
    private static let tpsDisabled = false
    #else
    private static let tpsDisabled = true
    #endif

    init(isExpanded: Bool, onDismiss: @escaping () -> Void) {
        self.isExpanded = isExpanded
        self.onDismiss = onDismiss
        _selectedLayout = State(initialValue: SharedSettings.shared.keyboardLayoutType)
    }

    var body: some View {
        Group {
            if isExpanded {
                GeometryReader { geometry in
                    let toolbarHeight = CandidateViewModels.UI.height
                    contentView
                        .frame(
                            maxWidth: .infinity,
                            minHeight: geometry.size.height - toolbarHeight,
                            maxHeight: .infinity
                        )
                }
            } else {
                EmptyView()
            }
        }
    }

    // MARK: - Content

    private var contentView: some View {
        VStack(alignment: .leading, spacing: 12) {
            // Section 1: Romanization keyboards
            layoutSection(
                header: Tab2Texts.romanizationKeyboard.hanji,
                layouts: [
                    (.phahTaigi, Tab2Texts.phahTaigiLayout.hanji, "layout_phahtaigi_preview", false),
                    (.qwerty, Tab2Texts.standardLayout.hanji, "layout_standard_preview", false),
                    (.moe1, Tab2Texts.moe1Layout.hanji, "layout_moe1_preview", false),
                    (.moe2, Tab2Texts.moe2Layout.hanji, "layout_moe2_preview", false),
                ]
            )

            // Section 2: Taigi phonetic
            layoutSection(
                header: Tab2Texts.taigiPhonetic.hanji,
                layouts: [
                    (.tps, Tab2Texts.tpsLayout.hanji, "layout_tps_preview", Self.tpsDisabled),
                ]
            )
        }
        .padding(.top, 10)
        .padding(.bottom, 4)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .background(Color.keyboardBackground)
        .onAppear {
            selectedLayout = SharedSettings.shared.keyboardLayoutType
        }
    }

    // MARK: - Section

    @ViewBuilder
    private func layoutSection(
        header: String,
        layouts: [(KeyboardLayoutType, String, String?, Bool)]
    ) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(header)
                .font(.caption)
                .fontWeight(.semibold)
                .foregroundColor(CandidateViewModels.Colors.secondaryTextColor)
                .textCase(.uppercase)
                .padding(.horizontal, 12)

            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 8) {
                    ForEach(layouts, id: \.0) { (layoutType, name, imageName, isDisabled) in
                        LayoutCard(
                            name: name,
                            previewImageName: imageName,
                            isSelected: selectedLayout == layoutType,
                            isDisabled: isDisabled,
                            action: { selectLayout(layoutType) }
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

                        Text(Tab2Texts.comingSoon.hanji)
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
                                    .foregroundColor(.white)
                            )
                    }
                }
                .overlay(
                    RoundedRectangle(cornerRadius: 8)
                        .stroke(isSelected && !isDisabled ? Color.accentColor : Color.clear, lineWidth: 2)
                )
                .frame(width: cardWidth)

                // Title label
                Text(name)
                    .font(.caption2)
                    .fontWeight(.medium)
                    .foregroundColor(isDisabled ? CandidateViewModels.Colors.secondaryTextColor : CandidateViewModels.Colors.primaryTextColor)
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
            // Fallback: keyboard icon + name (matches Tab2's fallback)
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
                    }
                )
        }
    }
}
