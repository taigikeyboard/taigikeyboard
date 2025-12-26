import SwiftUI

/// Tab2: 佈局
/// 內容：鍵盤佈局選擇（標準 / PhahTaigi）
/// 主題風格：與 Tab1 一致
/// 排版參考：azooKey ThemeTab（水平列表式）
struct Tab2: View {
    @StateObject private var languageManager = LanguageManager.shared
    @State private var phahTaigiLayoutEnabled: Bool
    @Namespace private var namespace

    private let settings = SharedSettings.shared

    init() {
        _phahTaigiLayoutEnabled = State(initialValue: SharedSettings.shared.phahTaigiLayoutEnabled)
    }

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(spacing: 32) {
                    // 佈局選擇區塊
                    layoutSelectionSection
                }
                .padding(.horizontal, 20)
                .padding(.top, 24)
                .padding(.bottom, 40)
            }
            .background(Color.Theme.surfacePrimary)
            .navigationTitle(languageManager.text(Tab2Texts.tabTitle))
            .navigationBarTitleDisplayMode(.large)
        }
    }

    // MARK: - 佈局選擇區塊

    @ViewBuilder
    private var layoutSelectionSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            // 說明文字
            LocalizedTextView(Tab2Texts.layoutDescription)
                .themeFontBody()
                .foregroundColor(Color.Theme.textSecondary)

            // 佈局選項卡片
            VStack(spacing: 0) {
                // PhahTaigi 佈局（預設）
                LayoutOptionRow(
                    title: Tab2Texts.phahTaigiLayout,
                    previewImageName: "layout_phahtaigi_preview",
                    isSelected: phahTaigiLayoutEnabled,
                    namespace: namespace,
                    checkmarkId: "layout_checkmark",
                    isLast: false,
                    action: {
                        withAnimation(.easeInOut(duration: 0.15)) {
                            phahTaigiLayoutEnabled = true
                            settings.phahTaigiLayoutEnabled = true
                        }
                    }
                )

                // 標準佈局
                LayoutOptionRow(
                    title: Tab2Texts.standardLayout,
                    previewImageName: "layout_standard_preview",
                    isSelected: !phahTaigiLayoutEnabled,
                    namespace: namespace,
                    checkmarkId: "layout_checkmark",
                    isLast: true,
                    action: {
                        withAnimation(.easeInOut(duration: 0.15)) {
                            phahTaigiLayoutEnabled = false
                            settings.phahTaigiLayoutEnabled = false
                        }
                    }
                )
            }
            .themedCard()
        }
    }
}

// MARK: - 佈局選項列（水平排列，參考 azooKey）

private struct LayoutOptionRow: View {
    let title: LocalizedText
    let previewImageName: String
    let isSelected: Bool
    let namespace: Namespace.ID
    let checkmarkId: String
    let isLast: Bool
    let action: () -> Void

    /// 預覽圖尺寸（符合 Apple HIG，足夠辨識鍵盤佈局）
    private let previewWidth: CGFloat = 180
    private let previewHeight: CGFloat = 120

    var body: some View {
        Button(action: action) {
            VStack(spacing: 0) {
                HStack(spacing: 16) {
                    // 左側：預覽圖
                    ZStack {
                        previewImage
                            .frame(width: previewWidth, height: previewHeight)
                            .clipShape(RoundedRectangle(cornerRadius: 8))
                            .overlay(
                                RoundedRectangle(cornerRadius: 8)
                                    .stroke(Color.Theme.cardStroke, lineWidth: 1)
                            )

                        // 選中時的遮罩與勾選
                        if isSelected {
                            Color.black.opacity(0.3)
                                .frame(width: previewWidth, height: previewHeight)
                                .clipShape(RoundedRectangle(cornerRadius: 8))

                            Circle()
                                .fill(Color.Theme.accent)
                                .frame(width: 36, height: 36)
                                .overlay(
                                    Image(systemName: "checkmark")
                                        .font(.system(size: 18, weight: .bold))
                                        .foregroundColor(.white)
                                )
                                .matchedGeometryEffect(id: checkmarkId, in: namespace)
                        }
                    }

                    // 右側：標題（置中於剩餘空間）
                    LocalizedTextView(title)
                        .themeFontBody()
                        .foregroundColor(Color.Theme.textPrimary)
                        .frame(maxWidth: .infinity)
                }
                .padding(.horizontal, 16)
                .padding(.vertical, 12)

                // 分隔線（非最後一項時顯示）
                if !isLast {
                    Divider()
                        .background(Color.Theme.cardStroke)
                        .padding(.leading, 16)
                }
            }
        }
        .buttonStyle(.plain)
    }

    /// 預覽圖（嘗試載入圖片，否則顯示 Placeholder）
    @ViewBuilder
    private var previewImage: some View {
        if let uiImage = UIImage(named: previewImageName) {
            Image(uiImage: uiImage)
                .resizable()
                .aspectRatio(contentMode: .fill)
        } else {
            Rectangle()
                .fill(Color.Theme.surfaceSecondary)
                .overlay(
                    Image(systemName: "keyboard")
                        .font(.system(size: 20))
                        .foregroundColor(Color.Theme.textSecondary)
                )
        }
    }
}
