// 中文: 主題選擇器(Shelf)— 主題 tab 的 root。橫向捲動陳列「預設 + 內建主題」迷你預覽,
// 中文: 點選即套用(寫 selectedThemeId → keyboard extension 下次 render 重新解析配色)。
// 中文: 底部一條 NavigationLink 進「自訂外觀設定」= 既有 6 色 + 尺寸編輯器(PR-1 移入,本 PR 不動)。

import SwiftUI

/// The theme tab root: a KeyboardKit-`Shelf`-style picker of built-in themes
/// (plus `Default`), with a link down to the full appearance editor.
///
/// Selecting a cell writes `selectedThemeId`; the keyboard extension re-resolves
/// its colors on the next render (cross-process via `UserDefaults` change). User
/// themes ("+" create) and shadow controls land in PR-3.
// 中文: 主題選擇頁。每格是 light/dark 依當前 colorScheme 取的迷你配色預覽,非完整鍵盤(避免 7 份 live keyboard)。
struct ThemePickerView: View {
    @Environment(\.colorScheme) private var colorScheme
    @State private var selectedThemeId: String = SharedSettings.shared.selectedThemeId

    var body: some View {
        Form {
            Section(header: Text(ThemeTexts.builtInThemesSection)) {
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 12) {
                        ThemeShelfCell(
                            title: ThemeTexts.defaultThemeName,
                            colors: .default,
                            isSelected: selectedThemeId == ThemeId.default,
                            onTap: { apply(ThemeId.default) },
                        )
                        ForEach(BuiltInThemes.all, id: \.id) { theme in
                            ThemeShelfCell(
                                title: theme.displayName,
                                colors: theme.colors(for: colorScheme),
                                isSelected: selectedThemeId == theme.id,
                                onTap: { apply(theme.id) },
                            )
                        }
                    }
                    .padding(.vertical, 4)
                }
                .listRowInsets(EdgeInsets(top: 8, leading: 12, bottom: 8, trailing: 12))
            }

            Section {
                NavigationLink {
                    AppearanceSettingsView()
                } label: {
                    Text(ThemeTexts.customAppearance)
                }
            }
        }
        .navigationTitle(ThemeTexts.tabTitle)
    }

    // 中文: 套用主題。更新本地選取狀態(打勾即時反映)+ 寫入 SharedSettings(觸發 extension 重繪)。
    private func apply(_ id: String) {
        guard selectedThemeId != id else { return }
        selectedThemeId = id
        SharedSettings.shared.selectedThemeId = id
    }
}

// MARK: - Shelf cell

/// One theme on the shelf: a mini palette preview, a title, and a selection
/// checkmark. Tappable to apply.
// 中文: 單一主題格 — 迷你配色預覽 + 標題 + 選取打勾。
private struct ThemeShelfCell: View {
    let title: String
    let colors: KeyboardColorSettings
    let isSelected: Bool
    let onTap: () -> Void

    private let cellWidth: CGFloat = 96
    private let previewHeight: CGFloat = 64

    var body: some View {
        Button(action: onTap) {
            VStack(spacing: 6) {
                ThemeSwatch(colors: colors)
                    .frame(width: cellWidth, height: previewHeight)
                    .clipShape(RoundedRectangle(cornerRadius: 10))
                    .overlay(
                        RoundedRectangle(cornerRadius: 10)
                            .strokeBorder(
                                isSelected ? AppStyle.accentBlue : Color(.separator),
                                lineWidth: isSelected ? 2.5 : 1,
                            ),
                    )

                HStack(spacing: 4) {
                    if isSelected {
                        Image(latinSystemName: "checkmark.circle.fill")
                            .foregroundColor(AppStyle.accentBlue)
                            .font(.caption)
                    }
                    Text(title)
                        .font(.caption)
                        .foregroundColor(isSelected ? AppStyle.accentBlue : .primary)
                        .lineLimit(1)
                }
                .frame(width: cellWidth)
            }
        }
        .buttonStyle(.plain)
    }
}

// MARK: - Swatch

/// A static, lightweight mini-keyboard rendering of a 6-role palette: a
/// candidate bar plus two key rows (normal + one special key). Falls back to
/// neutral system colors for nil roles (the `Default` adaptive theme).
// 中文: 迷你鍵盤配色預覽。nil 角色退到中性系統色(對應 Default adaptive 主題)。
private struct ThemeSwatch: View {
    let colors: KeyboardColorSettings

    var body: some View {
        let background = colors.backgroundColor?.color ?? Color(.secondarySystemBackground)
        let normalFill = colors.normalKeyFillColor?.color ?? Color(.systemBackground)
        let specialFill = colors.specialKeyFillColor?.color ?? Color(.systemGray3)
        let keyText = colors.keyTextColor?.color ?? Color(.label)
        let candidateBackground = colors.candidateBackgroundColor?.color ?? background
        let candidateText = colors.candidateTextColor?.color ?? Color(.label)

        VStack(spacing: 4) {
            // Candidate bar with two sample候選詞 marks.
            HStack(spacing: 4) {
                ForEach(0 ..< 2, id: \.self) { _ in
                    Capsule().fill(candidateText.opacity(0.8)).frame(width: 18, height: 4)
                }
                Spacer(minLength: 0)
            }
            .padding(.horizontal, 6)
            .frame(height: 14)
            .frame(maxWidth: .infinity)
            .background(candidateBackground)

            // Two key rows: 3 normal keys + 1 special key, each with a key-text dot.
            ForEach(0 ..< 2, id: \.self) { row in
                HStack(spacing: 4) {
                    ForEach(0 ..< 4, id: \.self) { col in
                        // 中文: 第二排最後一鍵當特殊鍵(Shift/Enter 類)以呈現 specialKeyFill。
                        let isSpecialKey = row == 1 && col == 3
                        RoundedRectangle(cornerRadius: 3)
                            .fill(isSpecialKey ? specialFill : normalFill)
                            .overlay(
                                Circle().fill(keyText.opacity(0.85)).frame(width: 3, height: 3),
                            )
                            .frame(height: 14)
                    }
                }
                .padding(.horizontal, 6)
            }
            Spacer(minLength: 0)
        }
        .padding(.vertical, 4)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(background)
    }
}
