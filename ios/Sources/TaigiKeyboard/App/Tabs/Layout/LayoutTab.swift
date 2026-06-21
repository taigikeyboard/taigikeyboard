// 中文: Layout Tab — 鍵盤排版選擇主頁面。提供 PhahTaigi / QWERTY / MOE / TPS
// 中文: 4 種版面卡片橫向選單。外觀設定已移至「主題」tab(第 2 位)。

import SwiftUI

/// Layout tab.
///
/// Keyboard layout selection (PhahTaigi, QWERTY, MOE, TPS) with horizontal swipe cards.
// 中文: Layout Tab View — 兩段橫向卡片(羅馬字鍵盤 / 台語注音)。
struct LayoutTab: View {
    @Environment(DisplayLanguageStore.self) private var lang
    @State private var selectedLayout: KeyboardLayoutType

    private let settings = SharedSettings.shared

    // Title element is a StringKey, resolved at render via `lang` so the layout name live-switches.
    private static let tpsEntry: (KeyboardLayoutType, StringKey, String, String?, Bool) =
        (.tps, .layoutTpsLayout, "layout_tps_preview", nil, false)

    init() {
        _selectedLayout = State(initialValue: SharedSettings.shared.keyboardLayoutType)
    }

    var body: some View {
        NavigationStack {
            ScrollView(.vertical) {
                VStack(alignment: .leading, spacing: 24) {
                    // Section 1: Romanization keyboards
                    layoutSection(
                        header: .layoutRomanizationKeyboard,
                        layouts: [
                            (.phahTaigi, .layoutPhahTaigiLayout, "layout_phahtaigi_preview", nil, false),
                            (.qwerty, .layoutStandardLayout, "layout_standard_preview", nil, false),
                            (.moe1, .layoutMoe1Layout, "layout_moe1_preview", nil, false),
                            (.moe2, .layoutMoe2Layout, "layout_moe2_preview", nil, false),
                        ],
                    )

                    // Section 2: Taigi phonetic
                    layoutSection(
                        header: .layoutTaigiPhonetic,
                        layouts: [Self.tpsEntry],
                    )
                }
                .padding(.top, 20)
                .padding(.bottom)
            }
            .background(Color(.systemGroupedBackground))
            .navigationTitle(TabType.layout.title)
            .navigationBarTitleDisplayMode(.large)
        }
    }

    // MARK: - Section builder

    private func layoutSection(
        header: StringKey,
        layouts: [(KeyboardLayoutType, StringKey, String, String?, Bool)],
    ) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(lang.string(header))
                .font(AppStyle.sectionHeaderFont)
                .foregroundStyle(.secondary)
                .padding(.horizontal)

            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 12) {
                    ForEach(layouts, id: \.0) { layoutType, titleKey, imageName, subtitleText, isDisabled in
                        LayoutOptionCard(
                            title: lang.string(titleKey),
                            subtitle: subtitleText.map(\.self),
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

    // 中文: 切換選定排版,同步更新本地 state 與 SharedSettings 持久化欄位。
    private func selectLayout(_ layout: KeyboardLayoutType) {
        withAnimation(.easeInOut(duration: 0.15)) {
            selectedLayout = layout
            settings.keyboardLayoutType = layout
        }
    }
}

// MARK: - Layout option card for horizontal swipe shelf

// 中文: 單張鍵盤版面卡片(橫向卷軸用)。包含預覽圖、選中標記、停用遮罩、標題與副標。
private struct LayoutOptionCard: View {
    let title: String
    var subtitle: String?
    let previewImageName: String
    let isSelected: Bool
    var isDisabled: Bool = false
    let action: () -> Void

    /// Fixed card width for horizontal scrolling.
    /// CROSS-PAGE: matches `ThemeCardMetrics.width` (ThemePickerView.swift) so the
    /// 主題 and 佈局 keyboard previews render at the identical size. Change both.
    private let cardWidth: CGFloat = 240

    var body: some View {
        Button(action: action) {
            VStack(spacing: 6) {
                // Preview image with rounded corners (KeyboardKit theme style)
                ZStack {
                    previewImage
                        .clipShape(RoundedRectangle(cornerRadius: AppStyle.previewCornerRadius))

                    if isDisabled {
                        RoundedRectangle(cornerRadius: AppStyle.previewCornerRadius)
                            .fill(Color.black.opacity(0.5))

                        Text(subtitle ?? "")
                            .font(AppStyle.captionFont)
                            .fontWeight(.bold)
                            .foregroundColor(.white)
                            .padding(.horizontal, 12)
                            .padding(.vertical, 6)
                            .background(Color.black.opacity(0.7), in: Capsule())
                    } else if isSelected {
                        RoundedRectangle(cornerRadius: AppStyle.previewCornerRadius)
                            .fill(Color.black.opacity(0.25))

                        Circle()
                            .fill(AppStyle.accentBlue)
                            .frame(width: 36, height: 36)
                            .overlay(
                                Image(latinSystemName: "checkmark")
                                    .font(AppStyle.appFont(size: 16).bold())
                                    .foregroundColor(.white),
                            )
                    }
                }
                .overlay(
                    RoundedRectangle(cornerRadius: AppStyle.previewCornerRadius)
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
                        Image(latinSystemName: "keyboard")
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
