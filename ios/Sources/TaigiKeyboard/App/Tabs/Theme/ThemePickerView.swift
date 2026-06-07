// 中文: 主題選擇器 — 主題 tab 的 root。版面草稿階段(只排版,不接主題邏輯)。
// 中文: 仿 KeyboardKit「Themes」頁:頂部 Custom Themes shelf(Create New… 卡 → 外觀設定子頁),
// 中文: 下方多個分類 shelf(暫用 KK 原文分類 + placeholder 預覽卡;真主題之後逐一加回)。

import SwiftUI

/// The theme tab root: a KeyboardKit-`Themes`-style gallery of horizontal
/// "shelves". The first shelf (`Custom Themes`) holds a `Create New…` card that
/// links into the existing appearance editor. The remaining shelves are layout
/// placeholders — category headers + empty preview cards that the user fills with
/// real theme screenshots once the page layout is settled.
///
/// Layout-only on purpose: no theme is applied here yet. The six built-in themes
/// (`BuiltInThemes`) and selection/apply wiring are temporarily detached and
/// return one-by-one in a later pass. The resolver/render path is untouched.
// 中文: 主題選擇頁。每格 = placeholder 預覽框 + 名稱;真主題截圖之後補上。
struct ThemePickerView: View {
    var body: some View {
        ScrollView {
            LazyVStack(alignment: .leading, spacing: shelfSpacing) {
                // Custom Themes: entry to the appearance editor (user themes land here too, later).
                ThemeShelf(title: ThemeTexts.customThemesSection) {
                    CreateNewThemeCard()
                }

                // Placeholder built-in categories (KK names; user replaces later).
                ForEach(PlaceholderThemes.categories, id: \.title) { category in
                    ThemeShelf(title: category.title) {
                        ForEach(category.themeNames, id: \.self) { name in
                            ThemePlaceholderCard(name: name)
                        }
                    }
                }
            }
            .padding(.vertical, AppStyle.horizontalPadding)
        }
        .navigationTitle(ThemeTexts.tabTitle)
    }

    private let shelfSpacing: CGFloat = 28
}

// MARK: - Placeholder data

/// Placeholder theme categories + names. Temporarily borrowed from KeyboardKit's
/// predefined collections so the gallery reads correctly while empty. The user
/// swaps these for the real Taigi themes (and screenshots) once the layout is
/// approved — delete this enum at that point.
// 中文: 版面草稿用的分類 + 主題名(暫借 KK 預設集合)。真主題加回時整個刪掉。
private enum PlaceholderThemes {
    struct Category {
        let title: String
        let themeNames: [String]
    }

    static let categories: [Category] = [
        Category(title: "Standard", themeNames: ["Standard", "Blue", "Green", "Yellow", "Red", "Purple"]),
        Category(title: "Swifty", themeNames: ["Swifty", "Swifty Blue", "Swifty Green", "Swifty Yellow", "Swifty Red"]),
        Category(title: "Minimal", themeNames: ["Minimal", "Minimal Blue", "Minimal Green", "Minimal Yellow"]),
        Category(title: "Colorful", themeNames: ["Colorful Blue", "Colorful Green", "Colorful Orange", "Colorful Purple"]),
    ]
}

// MARK: - Card metrics

/// Shared dimensions for every card on a shelf so previews line up across shelves.
// 中文: 所有 shelf 卡共用尺寸,跨 shelf 對齊。
private enum ThemeCardMetrics {
    static let width: CGFloat = 220
    static let previewHeight: CGFloat = 140
    static let cardSpacing: CGFloat = 14
}

// MARK: - Shelf

/// One horizontal shelf: a gray section header above a horizontally scrolling row
/// of cards.
// 中文: 單一 shelf — 灰色標題 + 橫向捲動的卡列。
private struct ThemeShelf<Content: View>: View {
    let title: String
    @ViewBuilder let content: Content

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(title)
                .font(AppStyle.headlineFont)
                .foregroundColor(.secondary)
                .padding(.horizontal, AppStyle.horizontalPadding)

            ScrollView(.horizontal, showsIndicators: false) {
                HStack(alignment: .top, spacing: ThemeCardMetrics.cardSpacing) {
                    content
                }
                .padding(.horizontal, AppStyle.horizontalPadding)
            }
        }
    }
}

// MARK: - Create New card

/// The first card on the Custom Themes shelf: a gray panel with a centered white
/// "+" tile. Tapping navigates to the existing appearance editor.
// 中文: Custom Themes 第一格 — 灰底卡 + 中央白色「+」磚;點選進外觀設定。
private struct CreateNewThemeCard: View {
    private let plusTileSize: CGFloat = 84
    private let plusGlyphSize: CGFloat = 28

    var body: some View {
        NavigationLink {
            AppearanceSettingsView()
        } label: {
            VStack(alignment: .leading, spacing: 8) {
                ZStack {
                    RoundedRectangle(cornerRadius: AppStyle.previewCornerRadius)
                        .fill(Color(.systemGray4))

                    RoundedRectangle(cornerRadius: AppStyle.smallCornerRadius)
                        .fill(Color(.systemBackground))
                        .frame(width: plusTileSize, height: plusTileSize)
                        .overlay(
                            Image(latinSystemName: "plus")
                                .resizable()
                                .scaledToFit()
                                .frame(width: plusGlyphSize, height: plusGlyphSize)
                                .foregroundColor(.primary),
                        )
                }
                .frame(width: ThemeCardMetrics.width, height: ThemeCardMetrics.previewHeight)

                Text(ThemeTexts.createNewTheme)
                    .font(AppStyle.bodyFont)
                    .foregroundColor(.primary)
                    .lineLimit(1)
            }
            .frame(width: ThemeCardMetrics.width, alignment: .leading)
        }
        .buttonStyle(.plain)
    }
}

// MARK: - Theme placeholder card

/// A placeholder theme cell: an empty preview slot (a theme screenshot drops in
/// here later) plus the theme name and a trailing action affordance. Non-
/// interactive while the page is layout-only.
// 中文: 主題 placeholder 卡 — 空預覽框(之後放主題截圖)+ 名稱 + 動作圖示(版面草稿暫不可點)。
private struct ThemePlaceholderCard: View {
    let name: String

    private let previewGlyphSize: CGFloat = 30

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            RoundedRectangle(cornerRadius: AppStyle.previewCornerRadius)
                .fill(Color(.secondarySystemBackground))
                .frame(width: ThemeCardMetrics.width, height: ThemeCardMetrics.previewHeight)
                .overlay(
                    RoundedRectangle(cornerRadius: AppStyle.previewCornerRadius)
                        .strokeBorder(Color(.separator), lineWidth: 1),
                )
                .overlay(
                    // 中文: 「截圖待補」提示圖示。
                    Image(latinSystemName: "photo")
                        .resizable()
                        .scaledToFit()
                        .frame(width: previewGlyphSize, height: previewGlyphSize)
                        .foregroundColor(Color(.tertiaryLabel)),
                )

            HStack(spacing: 4) {
                Text(name)
                    .font(AppStyle.bodyFont)
                    .foregroundColor(.primary)
                    .lineLimit(1)

                Spacer(minLength: 0)

                // 中文: 每主題的動作選單(套用 / 編輯 / 刪除)入口。版面草稿先放圖示,邏輯之後補。
                Image(latinSystemName: "ellipsis")
                    .foregroundColor(.secondary)
            }
            .frame(width: ThemeCardMetrics.width)
        }
    }
}
