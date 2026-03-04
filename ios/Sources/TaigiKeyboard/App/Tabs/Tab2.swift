import SwiftUI

/// 佈局 Tab
///
/// 鍵盤佈局選擇，支援 PhahTaigi、標準 QWERTY、教育部及方音符號佈局。
/// Uses horizontal swipe cards grouped into sections.
struct Tab2: View {
    @StateObject private var languageManager = LanguageManager.shared
    @State private var selectedLayout: KeyboardLayoutType

    private let settings = SharedSettings.shared

    #if DEBUG
    private static let tpsEntry: (KeyboardLayoutType, LocalizedText, String, LocalizedText?, Bool) =
        (.tps, Tab2Texts.tpsLayout, "layout_tps_preview", nil, false)
    #else
    private static let tpsEntry: (KeyboardLayoutType, LocalizedText, String, LocalizedText?, Bool) =
        (.tps, Tab2Texts.tpsLayout, "layout_tps_preview", Tab2Texts.comingSoon, true)
    #endif

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
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                        .padding()
                        .background(Color(.secondarySystemGroupedBackground))
                        .cornerRadius(10)
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
                        ]
                    )

                    // Section 2: Taigi phonetic
                    layoutSection(
                        header: languageManager.text(Tab2Texts.taigiPhonetic),
                        layouts: [Self.tpsEntry]
                    )
                }
                .padding(.vertical)
            }
            .background(Color(.systemGroupedBackground))
            .navigationTitle(languageManager.text(Tab2Texts.tabTitle))
            .navigationBarTitleDisplayMode(.large)
        }
    }

    // MARK: - Section builder

    @ViewBuilder
    private func layoutSection(
        header: String,
        layouts: [(KeyboardLayoutType, LocalizedText, String, LocalizedText?, Bool)]
    ) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(header)
                .font(.subheadline)
                .fontWeight(.semibold)
                .foregroundColor(.secondary)
                .textCase(.uppercase)
                .padding(.horizontal)

            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 12) {
                    ForEach(layouts, id: \.0) { (layoutType, titleText, imageName, subtitleText, isDisabled) in
                        LayoutOptionCard(
                            title: languageManager.text(titleText),
                            subtitle: subtitleText.map { languageManager.text($0) },
                            previewImageName: imageName,
                            isSelected: selectedLayout == layoutType,
                            isDisabled: isDisabled,
                            action: {
                                selectLayout(layoutType)
                            }
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
    var subtitle: String? = nil
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
                            .font(.footnote)
                            .fontWeight(.bold)
                            .foregroundColor(.white)
                            .padding(.horizontal, 12)
                            .padding(.vertical, 6)
                            .background(Color.black.opacity(0.7), in: Capsule())
                    } else if isSelected {
                        RoundedRectangle(cornerRadius: 10)
                            .fill(Color.black.opacity(0.25))

                        Circle()
                            .fill(Color.accentColor)
                            .frame(width: 36, height: 36)
                            .overlay(
                                Image(systemName: "checkmark")
                                    .font(.system(size: 16, weight: .bold))
                                    .foregroundColor(.white)
                            )
                    }
                }
                .overlay(
                    RoundedRectangle(cornerRadius: 10)
                        .stroke(isSelected && !isDisabled ? Color.accentColor : Color.clear, lineWidth: 2.5)
                )
                .frame(width: cardWidth)

                // Title label
                VStack(spacing: 2) {
                    Text(title)
                        .font(.caption)
                        .fontWeight(.semibold)
                        .foregroundColor(isDisabled ? .secondary : .primary)
                        .lineLimit(1)
                    if let subtitle = subtitle, !isDisabled {
                        Text(subtitle)
                            .font(.caption2)
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
                            .font(.system(size: 28))
                            .foregroundColor(.secondary)
                        Text(title)
                            .font(.caption2)
                            .foregroundColor(.secondary)
                    }
                )
        }
    }
}
