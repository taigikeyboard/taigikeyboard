// 從工具列叫出的「鍵盤佈局選擇」overlay。
// 把可用佈局以卡片陳列,讓使用者不必離開鍵盤就能切換 layout。

import SwiftUI

/// Layout selection overlay panel
///
/// Displays available keyboard layouts as compact cards, allowing
/// the user to switch layouts directly from the keyboard toolbar.
/// Follows the same overlay pattern as `ExpandedCandidateOverlay`.
// 鍵盤佈局選擇面板 — 與 ExpandedCandidateOverlay 採用相同的 overlay 樣式。
struct LayoutSelectionOverlay: View {
    let isExpanded: Bool
    let onDismiss: () -> Void

    @State private var selectedLayout: KeyboardLayoutType
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.candidateTheme) private var theme
    @Environment(DisplayLanguageStore.self) private var lang

    init(isExpanded: Bool, onDismiss: @escaping () -> Void) {
        self.isExpanded = isExpanded
        self.onDismiss = onDismiss
        _selectedLayout = State(initialValue: SharedSettings.shared.keyboardLayoutType)
    }

    var body: some View {
        contentView.keyboardOverlayPanel(isExpanded: isExpanded, theme: theme)
    }

    // MARK: - Content

    private var contentView: some View {
        ScrollView(.vertical, showsIndicators: false) {
            VStack(alignment: .leading, spacing: 12) {
                // Section 1: Romanization keyboards
                layoutSection(
                    header: lang.string(.layoutRomanizationKeyboard),
                    layouts: [
                        (.phahTaigi, lang.string(.layoutPhahTaigiLayout), "layout_phahtaigi_preview"),
                        (.qwerty, lang.string(.layoutStandardLayout), "layout_standard_preview"),
                        (.moe1, lang.string(.layoutMoe1Layout), "layout_moe1_preview"),
                        (.moe2, lang.string(.layoutMoe2Layout), "layout_moe2_preview"),
                    ],
                )

                // Section 2: Taigi phonetic
                layoutSection(
                    header: lang.string(.layoutTaigiPhonetic),
                    layouts: [
                        (.tps, lang.string(.layoutTpsLayout), "layout_tps_preview"),
                    ],
                )
            }
            .padding(.top, 10)
            .padding(.bottom, 4)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .onAppear {
            // 開啟 overlay 時重讀 App-Group 顯示語言 tag — 跨程序(host 改語言)可靠的重讀點。
            lang.syncFromSettings()
            selectedLayout = SharedSettings.shared.keyboardLayoutType
        }
    }

    // MARK: - Section

    private func layoutSection(
        header: String,
        layouts: [(KeyboardLayoutType, String, String?)],
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
                    ForEach(layouts, id: \.0) { layoutType, name, imageName in
                        LayoutCard(
                            name: name,
                            previewImageName: imageName,
                            isSelected: selectedLayout == layoutType,
                            action: { selectLayout(layoutType) },
                        )
                    }
                }
                .padding(.horizontal, 12)
            }
        }
    }

    // MARK: - Selection

    // 寫入 SharedSettings 並排程短暫延遲後自動收合 overlay。
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
        // Selected layout is shown only by a checkmark + accent border; announce it so VoiceOver
        // conveys which layout is active.
        .accessibilityAddTraits(isSelected ? .isSelected : [])
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
