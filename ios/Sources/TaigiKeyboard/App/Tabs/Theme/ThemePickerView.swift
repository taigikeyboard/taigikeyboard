// 中文: 主題選擇器 — 主題 tab 的 root。Custom Themes shelf(自訂主題 CRUD + Create New)+
// 中文: 內建主題依 family(經典/框線/簡潔)各一條橫向 shelf。三 family 同 6 色,差在按鍵風格。
// 中文: 排版對齊齒盤佈局頁(240pt 卡 + 截圖預覽);尚未補截圖的內建卡顯示佔位圖而非色塊。

import SwiftUI

/// The theme tab root: a gallery of horizontal shelves (mirrors the齒盤佈局 page
/// layout — 240pt cards with screenshot previews, horizontal scroll).
///
/// - **Custom Themes** — the user's saved themes (apply / edit / delete via a
///   per-card menu) plus a `Create New…` card (hidden at the cap). These show a
///   live button preview (background + a styled centered key).
/// - **經典 / 框線 / 簡潔** — one shelf per built-in key-style family
///   (`BuiltInThemes.families`). All three carry the SAME 7 colors (預設 + 5 light
///   gradients + dark 暗眠山貓); they differ only in key style (經典 = filled keys, 框線 =
///   transparent keys + outline, 簡潔 = transparent keys). Each card shows its
///   screenshot (or a neutral placeholder until one ships) and applies on tap.
///
/// `selectedThemeId` / `themeRevision` are read via `@AppStorage` on the App
/// Group store so selection + the user-theme list refresh reactively when the
/// editor (or the keyboard) writes them — no `SharedSettings` publishing needed.
// 中文: 選定主題 / themeRevision 走 @AppStorage(App Group),選取與清單變更即時反映;免讓 SharedSettings 變 @Published。
struct ThemePickerView: View {
    @AppStorage("selectedThemeId", store: UserDefaults(suiteName: SharedSettings.appGroupId))
    private var selectedThemeId = ThemeId.default

    // 中文: 每次主題檔變更(新增/編輯/刪除)bump,驅動 userThemes 重新載入。
    @AppStorage("themeRevision", store: UserDefaults(suiteName: SharedSettings.appGroupId))
    private var themeRevision = 0

    @State private var userThemes: [UserTheme] = []
    @State private var editorRoute: ThemeEditorRoute?

    @Environment(DisplayLanguageStore.self) private var lang

    private let shelfSpacing: CGFloat = 28

    var body: some View {
        ScrollView {
            LazyVStack(alignment: .leading, spacing: shelfSpacing) {
                // Custom Themes: user's saved themes + Create New.
                ThemeShelf(title: lang.string(.themeCustomThemesSection)) {
                    if userThemes.count < UserThemeStore.maxUserThemes {
                        CreateNewThemeCard { editorRoute = .create }
                    }
                    ForEach(userThemes) { theme in
                        ThemeGalleryCard(
                            title: theme.name,
                            appearance: theme.appearance,
                            previewImageName: nil,
                            isSelected: selectedThemeId == theme.id.uuidString,
                            onTap: { apply(theme.id.uuidString) },
                            actions: [
                                ThemeCardAction(title: lang.string(.themeCardMenuApply)) { apply(theme.id.uuidString) },
                                ThemeCardAction(title: lang.string(.themeCardMenuEdit)) { editorRoute = .edit(theme) },
                                ThemeCardAction(title: lang.string(.commonDelete), role: .destructive) { delete(theme) },
                            ],
                        )
                    }
                }

                // Built-in families: one horizontal shelf each (Standard / Swifty / Minimal …).
                // The Standard family's first card is the app default (id == ThemeId.default).
                ForEach(BuiltInThemes.families, id: \.titleKey) { family in
                    ThemeShelf(title: lang.string(family.titleKey)) {
                        ForEach(family.themes, id: \.id) { theme in
                            ThemeGalleryCard(
                                title: lang.string(theme.displayNameKey),
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
        .navigationTitle(lang.string(TabType.theme.titleKey))
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

    @Environment(DisplayLanguageStore.self) private var lang

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

                Text(lang.string(.themeCreateNewTheme))
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

/// A theme cell: a preview (screenshot when `previewImageName` is set, else a
/// live custom-theme button preview) + a title with a selection checkmark + an
/// optional `…` action menu (user themes only). Tapping the preview applies the theme.
// 中文: 主題卡 — 預覽(有 previewImageName 用截圖,否則用自訂主題大按鈕預覽)+ 標題/打勾 + 「…」選單。點預覽即套用。
private struct ThemeGalleryCard: View {
    let title: String
    /// Full appearance for the live custom-theme preview; `nil` for built-in cards
    /// (they render via `previewImageName` and never reach the live preview).
    var appearance: ThemeAppearance? = nil
    /// Screenshot asset name; `nil` → render the live custom-theme button preview.
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
    /// the asset is missing (scaffold stage); otherwise the live custom-theme
    /// button preview (background + one styled centered key).
    // 中文: 預覽內容 — 有截圖 asset 用截圖;asset 缺(scaffold)用佔位圖;無 previewImageName 用自訂主題大按鈕預覽。
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
            CustomThemeButtonPreview(appearance: appearance ?? .default)
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

// MARK: - Custom-theme button preview

/// A custom-theme card preview: the theme background with one large centered key
/// that applies the theme's full button style — fill, text glyph, corner radius,
/// border, and shadow — so the saved theme's distinctive key look reads at a
/// glance (the old colors-only swatch hid radius / border / shadow). Border and
/// shadow mirror the real keyboard (`TaigiKeyboardView`): a black stroke and a
/// soft drop shadow sized by `keyShadowIntensity`. nil color roles fall back to
/// the same adaptive defaults the keyboard uses.
// 中文: 自訂主題卡預覽 — 背景 + 正中一顆大鍵,套完整 button style(填色/字/圓角/邊框/陰影),一眼看出該主題的鍵長相。
private struct CustomThemeButtonPreview: View {
    let appearance: ThemeAppearance

    /// Sample glyph on the key face — a Taigi romanization letter with a tone mark.
    private static let sampleGlyph = "â"
    private static let keyWidth: CGFloat = 104
    private static let keyHeight: CGFloat = 64
    private static let glyphBaseSize: CGFloat = 32

    var body: some View {
        let defaults = ThemeDefaults.self
        let colors = appearance.colors
        let background = colors.backgroundColor?.color ?? defaults.keyboardBackground
        let keyFill = colors.normalKeyFillColor?.color ?? defaults.normalKeyFill
        let keyText = colors.keyTextColor?.color ?? defaults.keyText
        let cornerRadius = CGFloat(appearance.keyCornerRadius)
        let borderWidth = CGFloat(appearance.keyBorderWidth)
        let shadow = CGFloat(appearance.keyShadowIntensity)

        RoundedRectangle(cornerRadius: cornerRadius)
            .fill(keyFill)
            .overlay {
                if borderWidth > 0 {
                    // Border follows the key text color (mirrors the real keyboard's
                    // role-first border), so the preview matches the live 框線 look.
                    RoundedRectangle(cornerRadius: cornerRadius)
                        .strokeBorder(keyText, lineWidth: borderWidth)
                }
            }
            .overlay {
                Text(Self.sampleGlyph)
                    .font(AppStyle.appFont(size: Self.glyphBaseSize * CGFloat(appearance.keyFontSizeScale)))
                    .foregroundColor(keyText)
            }
            .frame(width: Self.keyWidth, height: Self.keyHeight)
            // shadow == 0 → radius 0 + opacity 0 = no shadow (flat themes).
            .shadow(color: .black.opacity(shadow > 0 ? 0.3 : 0), radius: shadow, y: shadow / 2)
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .background(background)
    }
}
