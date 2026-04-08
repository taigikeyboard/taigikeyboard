import SwiftUI

/// Layout tab.
///
/// Keyboard layout selection (PhahTaigi, QWERTY, MOE, TPS) with horizontal swipe cards.
struct Tab2: View {
    @StateObject private var languageManager = LanguageManager.shared
    @State private var selectedLayout: KeyboardLayoutType

    private let settings = SharedSettings.shared

    private static let tpsEntry: (KeyboardLayoutType, LocalizedText, String, LocalizedText?, Bool) =
        (.tps, Tab2Texts.tpsLayout, "layout_tps_preview", nil, false)

    init() {
        _selectedLayout = State(initialValue: SharedSettings.shared.keyboardLayoutType)
    }

    var body: some View {
        NavigationStack {
            ScrollView(.vertical) {
                VStack(alignment: .leading, spacing: 24) {
                    // Appearance settings link
                    NavigationLink {
                        AppearanceSettingsView()
                    } label: {
                        HStack {
                            Text(languageManager.text(Tab2Texts.appearanceSettings))
                            Spacer()
                            Image(systemName: "chevron.right")
                                .font(AppStyle.captionFont)
                                .foregroundColor(.secondary)
                        }
                        .padding()
                        .background(Color(.secondarySystemGroupedBackground))
                        .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
                    }
                    .buttonStyle(.plain)
                    .padding(.horizontal)

                    // Section 1: Romanization keyboards
                    layoutSection(
                        header: languageManager.text(Tab2Texts.romanizationKeyboard),
                        layouts: [
                            (.phahTaigi, Tab2Texts.phahTaigiLayout, "layout_phahtaigi_preview", nil, false),
                            (.qwerty, Tab2Texts.standardLayout, "layout_standard_preview", nil, false),
                            (.moe1, Tab2Texts.moe1Layout, "layout_moe1_preview", nil, false),
                            (.moe2, Tab2Texts.moe2Layout, "layout_moe2_preview", nil, false),
                        ],
                    )

                    // Section 2: Taigi phonetic
                    layoutSection(
                        header: languageManager.text(Tab2Texts.taigiPhonetic),
                        layouts: [Self.tpsEntry],
                    )
                }
                .padding(.top, 20)
                .padding(.bottom)
            }
            .background(Color(.systemGroupedBackground))
            .navigationTitle(languageManager.text(Tab2Texts.tabTitle))
            .navigationBarTitleDisplayMode(.large)
        }
    }

    // MARK: - Section builder

    private func layoutSection(
        header: String,
        layouts: [(KeyboardLayoutType, LocalizedText, String, LocalizedText?, Bool)],
    ) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(header)
                .font(AppStyle.sectionHeaderFont)
                .foregroundStyle(.secondary)
                .padding(.horizontal)

            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 12) {
                    ForEach(layouts, id: \.0) { layoutType, titleText, imageName, subtitleText, isDisabled in
                        LayoutOptionCard(
                            title: languageManager.text(titleText),
                            subtitle: subtitleText.map { languageManager.text($0) },
                            previewImageName: imageName,
                            isSelected: selectedLayout == layoutType,
                            isDisabled: isDisabled,
                            action: {
                                selectLayout(layoutType)
                            },
                        )
                    }
                }
                .padding(.horizontal)
            }
        }
    }

    private func selectLayout(_ layout: KeyboardLayoutType) {
        withAnimation(.easeInOut(duration: 0.15)) {
            selectedLayout = layout
            settings.keyboardLayoutType = layout
        }
    }
}

// MARK: - Layout option card for horizontal swipe shelf

private struct LayoutOptionCard: View {
    let title: String
    var subtitle: String?
    let previewImageName: String
    let isSelected: Bool
    var isDisabled: Bool = false
    let action: () -> Void

    /// Fixed card width for horizontal scrolling
    private let cardWidth: CGFloat = 200

    var body: some View {
        Button(action: action) {
            VStack(spacing: 6) {
                // Preview image with rounded corners (KeyboardKit theme style)
                ZStack {
                    previewImage
                        .clipShape(RoundedRectangle(cornerRadius: 10))

                    if isDisabled {
                        RoundedRectangle(cornerRadius: 10)
                            .fill(Color.black.opacity(0.5))

                        Text(subtitle ?? "")
                            .font(AppStyle.captionFont)
                            .fontWeight(.bold)
                            .foregroundColor(.white)
                            .padding(.horizontal, 12)
                            .padding(.vertical, 6)
                            .background(Color.black.opacity(0.7), in: Capsule())
                    } else if isSelected {
                        RoundedRectangle(cornerRadius: 10)
                            .fill(Color.black.opacity(0.25))

                        Circle()
                            .fill(AppStyle.accentBlue)
                            .frame(width: 36, height: 36)
                            .overlay(
                                Image(systemName: "checkmark")
                                    .font(AppStyle.appFont(size: 16).bold())
                                    .foregroundColor(.white),
                            )
                    }
                }
                .overlay(
                    RoundedRectangle(cornerRadius: 10)
                        .stroke(isSelected && !isDisabled ? AppStyle.accentBlue : Color.clear, lineWidth: 2.5),
                )
                .frame(width: cardWidth)

                // Title label
                VStack(spacing: 2) {
                    Text(title)
                        .font(AppStyle.captionFont)
                        .fontWeight(.semibold)
                        .foregroundColor(isDisabled ? .secondary : .primary)
                        .lineLimit(1)
                    if let subtitle, !isDisabled {
                        Text(subtitle)
                            .font(AppStyle.captionFont)
                            .foregroundColor(.secondary)
                            .lineLimit(1)
                    }
                }
            }
        }
        .buttonStyle(.plain)
        .disabled(isDisabled)
    }

    /// Preview image (maintains aspect ratio within card bounds)
    @ViewBuilder
    private var previewImage: some View {
        if let uiImage = UIImage(named: previewImageName) {
            Image(uiImage: uiImage)
                .resizable()
                .aspectRatio(contentMode: .fit)
        } else {
            Rectangle()
                .fill(Color(.tertiarySystemBackground))
                .overlay(
                    VStack(spacing: 6) {
                        Image(systemName: "keyboard")
                            .font(AppStyle.appFont(size: 28))
                            .foregroundColor(.secondary)
                        Text(title)
                            .font(AppStyle.captionFont)
                            .foregroundColor(.secondary)
                    },
                )
        }
    }
}
