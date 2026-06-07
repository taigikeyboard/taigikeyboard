// 中文: 主題選擇器 — 主題 tab 的 root。Custom Themes shelf(自訂主題 CRUD + Create New)+
// 中文: 預設 shelf + 內建主題依 family(Standard/Swifty/Minimal)各一條橫向 shelf。
// 中文: 排版對齊齒盤佈局頁(240pt 卡 + 截圖預覽);scaffold 階段內建卡顯示截圖槽而非色塊。

import SwiftUI

/// The theme tab root: a gallery of horizontal shelves (mirrors the齒盤佈局 page
/// layout — 240pt cards with screenshot previews, horizontal scroll).
///
/// - **Custom Themes** — the user's saved themes (apply / edit / delete via a
///   per-card menu) plus a `Create New…` card (hidden at the cap). These keep
///   the live color swatch (user-picked palettes).
/// - **預設** — the `Default` (adaptive) theme.
/// - **Standard / Swifty / Minimal …** — one shelf per built-in family
///   (`BuiltInThemes.families`); each card shows its screenshot slot and applies
///   on tap (`selectedThemeId`). Scaffold stage — palettes authored later.
///
/// `selectedThemeId` / `themeRevision` are read via `@AppStorage` on the App
/// Group store so selection + the user-theme list refresh reactively when the
/// editor (or the keyboard) writes them — no `SharedSettings` publishing needed.
// 中文: 選定主題 / themeRevision 走 @AppStorage(App Group),選取與清單變更即時反映;免讓 SharedSettings 變 @Published。
struct ThemePickerView: View {
    @Environment(\.colorScheme) private var colorScheme

    @AppStorage("selectedThemeId", store: UserDefaults(suiteName: SharedSettings.appGroupId))
    private var selectedThemeId = ThemeId.default

    // 中文: 每次主題檔變更(新增/編輯/刪除)bump,驅動 userThemes 重新載入。
    @AppStorage("themeRevision", store: UserDefaults(suiteName: SharedSettings.appGroupId))
    private var themeRevision = 0

    @State private var userThemes: [UserTheme] = []
    @State private var editorRoute: ThemeEditorRoute?

    private let shelfSpacing: CGFloat = 28

    var body: some View {
        ScrollView {
            LazyVStack(alignment: .leading, spacing: shelfSpacing) {
                // Custom Themes: user's saved themes + Create New.
                ThemeShelf(title: ThemeTexts.customThemesSection) {
                    if userThemes.count < UserThemeStore.maxUserThemes {
                        CreateNewThemeCard { editorRoute = .create }
                    }
                    ForEach(userThemes) { theme in
                        ThemeGalleryCard(
                            title: theme.name,
                            colors: theme.appearance.colors,
                            previewImageName: nil,
                            isSelected: selectedThemeId == theme.id.uuidString,
                            onTap: { apply(theme.id.uuidString) },
                            actions: [
                                ThemeCardAction(title: ThemeTexts.themeMenuApply) { apply(theme.id.uuidString) },
                                ThemeCardAction(title: ThemeTexts.themeMenuEdit) { editorRoute = .edit(theme) },
                                ThemeCardAction(title: ThemeTexts.themeMenuDelete, role: .destructive) { delete(theme) },
                            ],
                        )
                    }
                }

                // Built-in families: one horizontal shelf each (Standard / Swifty / Minimal …).
                // The Standard family's first card is the app default (id == ThemeId.default).
                ForEach(BuiltInThemes.families, id: \.title) { family in
                    ThemeShelf(title: family.title) {
                        ForEach(family.themes, id: \.id) { theme in
                            ThemeGalleryCard(
                                title: theme.displayName,
                                colors: theme.colors(for: colorScheme),
                                previewImageName: theme.previewImageName,
                                isSelected: selectedThemeId == theme.id,
                                onTap: { apply(theme.id) },
                                actions: [],
                            )
                        }
                    }
                }
            }
            .padding(.vertical, AppStyle.horizontalPadding)
        }
        .navigationTitle(ThemeTexts.tabTitle)
        // 中文: themeRevision(任何 CRUD bump)變更即重載清單;新增/編輯/刪除皆涵蓋,pop 回此頁亦 onAppear 重載。
        .onAppear(perform: reloadUserThemes)
        .onChange(of: themeRevision) { _, _ in reloadUserThemes() }
        // 中文: 編輯器改為子頁面 push(非彈出 sheet),用 ThemeTab 的 NavigationStack。route id 穩定(create="create"、edit=theme.id)。
        .navigationDestination(item: $editorRoute) { route in
            switch route {
            case .create:
                ThemeEditorView()
            case let .edit(theme):
                ThemeEditorView(editing: theme)
            }
        }
    }

    // 中文: 套用主題:寫入 selectedThemeId(@AppStorage → App Group → keyboard 下次 render 重新解析)。
    private func apply(_ id: String) {
        selectedThemeId = id
    }

    // 中文: 刪除自訂主題;若刪掉的是當前選取,退回預設主題,避免渲染孤兒 id。
    private func delete(_ theme: UserTheme) {
        SharedSettings.shared.deleteUserTheme(id: theme.id)
        if selectedThemeId == theme.id.uuidString {
            selectedThemeId = ThemeId.default
        }
        reloadUserThemes()
    }

    private func reloadUserThemes() {
        userThemes = SharedSettings.shared.loadUserThemes()
    }
}

// MARK: - Editor route

/// Navigation route for the theme editor: create a new theme, or edit an existing one.
///
/// `Hashable` is required by `navigationDestination(item:)`; both `==` and
/// `hash(into:)` key on route `id` only, so `UserTheme` need not be `Hashable`.
// 中文: 編輯器 navigation 路由(push 子頁面)— 新增或編輯既有主題。
// 中文: navigationDestination(item:) 要求 Hashable;==/hash 只看 route id,故 UserTheme 不必 Hashable。
enum ThemeEditorRoute: Hashable {
    case create
    case edit(UserTheme)

    var id: String {
        switch self {
        case .create: "create"
        case let .edit(theme): theme.id.uuidString
        }
    }

    static func == (lhs: ThemeEditorRoute, rhs: ThemeEditorRoute) -> Bool {
        lhs.id == rhs.id
    }

    func hash(into hasher: inout Hasher) {
        hasher.combine(id)
    }
}

// MARK: - Card metrics

/// Shared dimensions for every card so the theme shelves line up with the 齒盤佈局
/// page. `width` matches `LayoutOptionCard.cardWidth`; `previewAspectRatio`
/// matches the `layout_*_preview` assets (585×369) so theme screenshots render
/// at the identical size.
// 中文: 卡片共用尺寸 — 對齊齒盤佈局頁。width 同 LayoutOptionCard(200);aspect 同 layout 預覽圖(585×369)。
private enum ThemeCardMetrics {
    static let width: CGFloat = 240
    static let previewAspectRatio: CGFloat = 585.0 / 369.0
    static let cardSpacing: CGFloat = 12
}

// MARK: - Shelf

/// One shelf: a gray section header above a horizontally scrolling row of cards
/// (mirrors the 齒盤佈局 page's `layoutSection`).
// 中文: 單一 shelf — 灰色標題 + 橫向捲動卡列(對齊齒盤佈局頁排版)。
private struct ThemeShelf<Content: View>: View {
    let title: String
    @ViewBuilder let content: Content

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(title)
                .font(AppStyle.sectionHeaderFont)
                .foregroundStyle(.secondary)
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

/// The leading card on the Custom Themes shelf: a gray panel with a centered
/// white "+" tile. Tapping opens the theme editor for a new theme.
// 中文: Custom Themes 第一格 — 灰底卡 + 中央白色「+」磚;點選開新主題編輯器。
private struct CreateNewThemeCard: View {
    let onTap: () -> Void

    private let plusTileSize: CGFloat = 84
    private let plusGlyphSize: CGFloat = 28

    var body: some View {
        Button(action: onTap) {
            VStack(alignment: .leading, spacing: 6) {
                // Color.clear sets the aspect-ratio box; the panel is overlaid to fill it.
                // 中文: Color.clear 定 aspect-ratio 外框,面板 overlay 填滿。
                Color.clear
                    .aspectRatio(ThemeCardMetrics.previewAspectRatio, contentMode: .fit)
                    .overlay(
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
                        },
                    )
                    .frame(width: ThemeCardMetrics.width)

                Text(ThemeTexts.createNewTheme)
                    .font(AppStyle.captionFont)
                    .fontWeight(.semibold)
                    .foregroundColor(.primary)
                    .lineLimit(1)
            }
            .frame(width: ThemeCardMetrics.width, alignment: .leading)
        }
        .buttonStyle(.plain)
    }
}

// MARK: - Theme gallery card

/// A trailing-menu action for a theme card (apply / edit / delete).
// 中文: 主題卡「…」選單的單一動作。
private struct ThemeCardAction: Identifiable {
    let id = UUID()
    let title: String
    var role: ButtonRole? = nil
    let action: () -> Void
}

/// A theme cell: a preview (screenshot when `previewImageName` is set, else the
/// live color swatch) + a title with a selection checkmark + an optional `…`
/// action menu (user themes only). Tapping the preview applies the theme.
// 中文: 主題卡 — 預覽(有 previewImageName 用截圖,否則用即時色塊)+ 標題/打勾 + 「…」選單。點預覽即套用。
private struct ThemeGalleryCard: View {
    let title: String
    let colors: KeyboardColorSettings
    /// Screenshot asset name; `nil` → render the live swatch.
    let previewImageName: String?
    let isSelected: Bool
    let onTap: () -> Void
    let actions: [ThemeCardAction]

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Button(action: onTap) {
                // Color.clear sets the aspect-ratio box (same ratio as the 齒盤佈局
                // assets); the preview is overlaid to fill it. Selection marker mirrors
                // LayoutOptionCard: a dimming mask + a blue circle checkmark over the
                // preview, plus a blue stroke when selected (no border otherwise).
                // 中文: Color.clear 定 aspect-ratio 外框(同齒盤佈局比例),預覽 overlay 填滿。
                // 中文: 選取標記對齊 LayoutOptionCard — 遮罩 + 藍圈白勾 + 選中時藍框(未選無框)。
                Color.clear
                    .aspectRatio(ThemeCardMetrics.previewAspectRatio, contentMode: .fit)
                    .overlay(preview)
                    .overlay {
                        if isSelected {
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
                    .clipShape(RoundedRectangle(cornerRadius: AppStyle.previewCornerRadius))
                    .frame(width: ThemeCardMetrics.width)
                    .overlay(
                        RoundedRectangle(cornerRadius: AppStyle.previewCornerRadius)
                            .stroke(isSelected ? AppStyle.accentBlue : Color.clear, lineWidth: 2.5),
                    )
            }
            .buttonStyle(.plain)

            HStack(spacing: 4) {
                Text(title)
                    .font(AppStyle.captionFont)
                    .fontWeight(.semibold)
                    .foregroundColor(.primary)
                    .lineLimit(1)

                Spacer(minLength: 0)

                if !actions.isEmpty {
                    Menu {
                        ForEach(actions) { action in
                            Button(action.title, role: action.role, action: action.action)
                        }
                    } label: {
                        Image(latinSystemName: "ellipsis")
                            .foregroundColor(.secondary)
                            .frame(width: 28, height: 28)
                    }
                }
            }
            .frame(width: ThemeCardMetrics.width)
        }
        .frame(width: ThemeCardMetrics.width, alignment: .leading)
    }

    /// Preview content: the screenshot asset (filling the aspect box) when
    /// `previewImageName` is set and the asset exists; a neutral placeholder when
    /// the asset is missing (scaffold stage); otherwise the live color swatch.
    // 中文: 預覽內容 — 有截圖 asset 用截圖;asset 缺(scaffold)用佔位圖;無 previewImageName 用色塊。
    @ViewBuilder
    private var preview: some View {
        if let previewImageName {
            if let uiImage = UIImage(named: previewImageName) {
                Image(uiImage: uiImage)
                    .resizable()
                    .aspectRatio(contentMode: .fill)
            } else {
                ThemeScreenshotPlaceholder(title: title)
            }
        } else {
            ThemeSwatch(colors: colors)
        }
    }
}

// MARK: - Screenshot placeholder

/// Neutral fallback for a built-in card whose screenshot asset is not yet added
/// (scaffold stage) — mirrors the 齒盤佈局 page's missing-image fallback.
// 中文: 內建卡截圖尚未加入時的佔位圖(scaffold 階段),對齊齒盤佈局頁缺圖樣式。
private struct ThemeScreenshotPlaceholder: View {
    let title: String

    var body: some View {
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
                        .lineLimit(1)
                },
            )
    }
}

// MARK: - Swatch

/// A static, lightweight mini-keyboard rendering of a 6-role palette: a
/// candidate bar plus two key rows. Falls back to neutral system colors for nil
/// roles (the `Default` adaptive theme).
// 中文: 迷你鍵盤配色預覽。nil 角色退到中性系統色(對應 Default adaptive 主題)。
private struct ThemeSwatch: View {
    let colors: KeyboardColorSettings

    var body: some View {
        // nil roles fall back to the SAME adaptive defaults the real keyboard uses,
        // so the "Default" swatch matches what the keyboard renders.
        let defaults = ThemeDefaults.self
        let background = colors.backgroundColor?.color ?? defaults.keyboardBackground
        let normalFill = colors.normalKeyFillColor?.color ?? defaults.normalKeyFill
        let specialFill = colors.specialKeyFillColor?.color ?? defaults.specialKeyFill
        let keyText = colors.keyTextColor?.color ?? defaults.keyText
        let candidateBackground = colors.candidateBackgroundColor?.color ?? background
        let candidateText = colors.candidateTextColor?.color ?? defaults.candidateText

        VStack(spacing: 5) {
            // Candidate bar with two sample 候選詞 marks.
            HStack(spacing: 5) {
                ForEach(0 ..< 2, id: \.self) { _ in
                    Capsule().fill(candidateText.opacity(0.8)).frame(width: 26, height: 5)
                }
                Spacer(minLength: 0)
            }
            .padding(.horizontal, 10)
            .frame(height: 20)
            .frame(maxWidth: .infinity)
            .background(candidateBackground)

            // Two key rows: 3 normal keys + 1 special key, each with a key-text dot.
            ForEach(0 ..< 2, id: \.self) { row in
                HStack(spacing: 6) {
                    ForEach(0 ..< 4, id: \.self) { col in
                        // 中文: 第二排最後一鍵當特殊鍵(Shift/Enter 類)以呈現 specialKeyFill。
                        let isSpecialKey = row == 1 && col == 3
                        RoundedRectangle(cornerRadius: 5)
                            .fill(isSpecialKey ? specialFill : normalFill)
                            .overlay(
                                Circle().fill(keyText.opacity(0.85)).frame(width: 5, height: 5),
                            )
                            .frame(height: 26)
                    }
                }
                .padding(.horizontal, 10)
            }
            Spacer(minLength: 0)
        }
        .padding(.vertical, 8)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(background)
    }
}
