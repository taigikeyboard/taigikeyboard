import SwiftUI

/// 佈局 Tab
///
/// 鍵盤佈局選擇，支援 PhahTaigi、標準 QWERTY 和 Flick 聲調佈局。
struct Tab2: View {
    @StateObject private var languageManager = LanguageManager.shared
    @State private var selectedLayout: KeyboardLayoutType
    @Namespace private var namespace

    private let settings = SharedSettings.shared

    init() {
        _selectedLayout = State(initialValue: SharedSettings.shared.keyboardLayoutType)
    }

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    // PhahTaigi 佈局
                    LayoutOptionCard(
                        title: languageManager.text(Tab2Texts.phahTaigiLayout),
                        previewImageName: "layout_phahtaigi_preview",
                        isSelected: selectedLayout == .phahTaigi,
                        namespace: namespace,
                        checkmarkId: "layout_checkmark",
                        action: {
                            selectLayout(.phahTaigi)
                        }
                    )

                    // 標準 QWERTY 佈局
                    LayoutOptionCard(
                        title: languageManager.text(Tab2Texts.standardLayout),
                        previewImageName: "layout_standard_preview",
                        isSelected: selectedLayout == .qwerty,
                        namespace: namespace,
                        checkmarkId: "layout_checkmark",
                        action: {
                            selectLayout(.qwerty)
                        }
                    )

                    #if DEBUG
                    // 台灣注音（方音符號）佈局（開發中，僅 Debug 模式顯示）
                    LayoutOptionCard(
                        title: languageManager.text(Tab2Texts.tpsLayout),
                        subtitle: languageManager.text(Tab2Texts.tpsLayoutDescription),
                        previewImageName: "layout_tps_preview",
                        isSelected: selectedLayout == .tps,
                        namespace: namespace,
                        checkmarkId: "layout_checkmark",
                        action: {
                            selectLayout(.tps)
                        }
                    )
                    #endif

                    #if DEBUG
                    // Flick 聲調佈局（開發中，僅 Debug 模式顯示）
                    LayoutOptionCard(
                        title: languageManager.text(Tab2Texts.flickLayout),
                        subtitle: languageManager.text(Tab2Texts.flickLayoutDescription),
                        previewImageName: "layout_flick_preview",
                        isSelected: selectedLayout == .flick,
                        namespace: namespace,
                        checkmarkId: "layout_checkmark",
                        action: {
                            selectLayout(.flick)
                        }
                    )
                    #endif
                } header: {
                    Text(languageManager.text(Tab2Texts.layoutDescription))
                }
            }
            .navigationTitle(languageManager.text(Tab2Texts.tabTitle))
            .navigationBarTitleDisplayMode(.large)
        }
    }

    private func selectLayout(_ layout: KeyboardLayoutType) {
        withAnimation(.easeInOut(duration: 0.15)) {
            selectedLayout = layout
            settings.keyboardLayoutType = layout
        }
    }
}

// MARK: - 佈局選項卡片（參考 azooKey ThemeTab）

private struct LayoutOptionCard: View {
    let title: String
    var subtitle: String? = nil
    let previewImageName: String
    let isSelected: Bool
    let namespace: Namespace.ID
    let checkmarkId: String
    let action: () -> Void

    /// 預覽圖縮放比例
    private let previewScale: CGFloat = 0.9

    var body: some View {
        Button(action: action) {
            ZStack {
                // 預覽圖（縮小並維持原圖比例）
                previewImage
                    .scaleEffect(previewScale)

                // 選中時的遮罩
                if isSelected {
                    Color.black.opacity(0.3)
                }

                // 勾選圓圈（置中）
                if isSelected {
                    Circle()
                        .fill(Color.accentColor)
                        .frame(width: 60, height: 60)
                        .overlay(
                            Image(systemName: "checkmark")
                                .font(.system(size: 28, weight: .bold))
                                .foregroundColor(.white)
                        )
                        .matchedGeometryEffect(id: checkmarkId, in: namespace)
                }

                // 標題（底部，毛玻璃背景）
                VStack {
                    Spacer()
                    VStack(spacing: 2) {
                        Text(title)
                            .font(.caption)
                            .bold()
                            .foregroundColor(.secondary)
                        if let subtitle = subtitle {
                            Text(subtitle)
                                .font(.caption2)
                                .foregroundColor(.secondary.opacity(0.8))
                        }
                    }
                    .padding(8)
                    .background(
                        Capsule()
                            .fill(.regularMaterial)
                            .shadow(radius: 1.5)
                    )
                    .padding(.bottom, 8)
                }
            }
        }
        .buttonStyle(.plain)
        .padding(.vertical, 4)
    }

    /// 預覽圖（維持原圖比例）
    @ViewBuilder
    private var previewImage: some View {
        if let uiImage = UIImage(named: previewImageName) {
            Image(uiImage: uiImage)
                .resizable()
                .aspectRatio(contentMode: .fit)
        } else {
            Rectangle()
                .fill(Color(.tertiarySystemBackground))
                .aspectRatio(16/9, contentMode: .fit)
                .overlay(
                    VStack(spacing: 8) {
                        Image(systemName: keyboardIconName)
                            .font(.system(size: 40))
                            .foregroundColor(.secondary)
                        Text(title)
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                )
        }
    }

    /// 根據標題選擇對應的圖示
    private var keyboardIconName: String {
        if title.contains("Flick") {
            return "hand.draw"
        } else {
            return "keyboard"
        }
    }
}
