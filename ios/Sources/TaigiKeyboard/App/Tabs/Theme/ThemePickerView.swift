// 中文: 主題選擇器 — 主題 tab 的 root。Custom Themes shelf(自訂主題 CRUD + Create New)+
// 中文: 內建主題 shelf(預設 + 6 套內建,點選即套用)+ 「自訂外觀設定」入口(編輯預設 buffer)。

import SwiftUI

/// The theme tab root: a gallery of horizontal "shelves".
///
/// - **Custom Themes** — the user's saved themes (apply / edit / delete via a
///   per-card menu) plus a `Create New…` card (hidden at the cap).
/// - **內建主題** — the `Default` (adaptive) theme plus the six built-in
///   palettes; tapping a card applies it (`selectedThemeId`).
/// - **自訂外觀設定** — edits the `Default` theme buffer (`AppearanceSettingsView`).
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

                // Built-in themes: Default (adaptive) + the six palettes.
                ThemeShelf(title: ThemeTexts.builtInThemesSection) {
                    ThemeGalleryCard(
                        title: ThemeTexts.defaultThemeName,
                        colors: .default,
                        isSelected: selectedThemeId == ThemeId.default,
                        onTap: { apply(ThemeId.default) },
                        actions: [],
                    )
                    ForEach(BuiltInThemes.all, id: \.id) { theme in
                        ThemeGalleryCard(
                            title: theme.displayName,
                            colors: theme.colors(for: colorScheme),
                            isSelected: selectedThemeId == theme.id,
                            onTap: { apply(theme.id) },
                            actions: [],
                        )
                    }
                }

                // Default-buffer editor entry.
                NavigationLink {
                    AppearanceSettingsView()
                } label: {
                    HStack {
                        Text(ThemeTexts.customAppearance)
                            .foregroundColor(.primary)
                        Spacer()
                        Image(latinSystemName: "chevron.right")
                            .font(.footnote.weight(.semibold))
                            .foregroundColor(Color(.tertiaryLabel))
                    }
                    .padding(.horizontal, AppStyle.horizontalPadding)
                }
            }
            .padding(.vertical, AppStyle.horizontalPadding)
        }
        .navigationTitle(ThemeTexts.tabTitle)
        // 中文: themeRevision(任何 CRUD bump)變更即重載清單;新增/編輯/刪除皆涵蓋,免 sheet onDismiss 重複讀。
        .onAppear(perform: reloadUserThemes)
        .onChange(of: themeRevision) { _, _ in reloadUserThemes() }
        .sheet(item: $editorRoute) { route in
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

/// Sheet route for the theme editor: create a new theme, or edit an existing one.
// 中文: 編輯器 sheet 路由 — 新增或編輯既有主題。
enum ThemeEditorRoute: Identifiable {
    case create
    case edit(UserTheme)

    var id: String {
        switch self {
        case .create: "create"
        case let .edit(theme): theme.id.uuidString
        }
    }
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

/// The leading card on the Custom Themes shelf: a gray panel with a centered
/// white "+" tile. Tapping opens the theme editor for a new theme.
// 中文: Custom Themes 第一格 — 灰底卡 + 中央白色「+」磚;點選開新主題編輯器。
private struct CreateNewThemeCard: View {
    let onTap: () -> Void

    private let plusTileSize: CGFloat = 84
    private let plusGlyphSize: CGFloat = 28

    var body: some View {
        Button(action: onTap) {
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

// MARK: - Theme gallery card

/// A trailing-menu action for a theme card (apply / edit / delete).
// 中文: 主題卡「…」選單的單一動作。
private struct ThemeCardAction: Identifiable {
    let id = UUID()
    let title: String
    var role: ButtonRole? = nil
    let action: () -> Void
}

/// A theme cell: a mini `ThemeSwatch` preview + a title with a selection
/// checkmark + an optional `…` action menu (user themes only). Tapping the
/// preview applies the theme.
// 中文: 主題卡 — 迷你 swatch 預覽 + 標題/打勾 + 自訂主題的「…」動作選單(actions 為空則不顯示)。點預覽即套用。
private struct ThemeGalleryCard: View {
    let title: String
    let colors: KeyboardColorSettings
    let isSelected: Bool
    let onTap: () -> Void
    let actions: [ThemeCardAction]

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Button(action: onTap) {
                ThemeSwatch(colors: colors)
                    .frame(width: ThemeCardMetrics.width, height: ThemeCardMetrics.previewHeight)
                    .clipShape(RoundedRectangle(cornerRadius: AppStyle.previewCornerRadius))
                    .overlay(
                        RoundedRectangle(cornerRadius: AppStyle.previewCornerRadius)
                            .strokeBorder(
                                isSelected ? AppStyle.accentBlue : Color(.separator),
                                lineWidth: isSelected ? 2.5 : 1,
                            ),
                    )
            }
            .buttonStyle(.plain)

            HStack(spacing: 4) {
                if isSelected {
                    Image(latinSystemName: "checkmark.circle.fill")
                        .foregroundColor(AppStyle.accentBlue)
                }
                Text(title)
                    .font(AppStyle.bodyFont)
                    .foregroundColor(isSelected ? AppStyle.accentBlue : .primary)
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
        let defaults = AppearanceSettingsViewModel.Defaults.self
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
