import SwiftUI

/// The theme tab root: a gallery of horizontal shelves (`GalleryShelf`, shared with the Layout
/// tab — 240pt cards with screenshot previews, horizontal scroll).
///
/// - **Custom Themes** — the user's saved themes (apply / edit / delete via a
///   per-card menu) plus a `Create New…` card (hidden at the cap). These show a
///   live background preview (the theme surface, no sample key).
/// - **Filled / Outlined / Borderless** — one shelf per built-in key-style family
///   (`BuiltInThemes.families`). All three carry the SAME 7 colors (Default + 5 light
///   gradients + dark Catppuccin); they differ only in key style (Filled = filled keys, Outlined =
///   transparent keys + outline, Borderless = transparent keys). Each card shows its
///   screenshot (or a neutral placeholder until one ships) and applies on tap.
///
/// `selectedThemeId` / `themeRevision` are read via `@AppStorage` on the App
/// Group store so selection + the user-theme list refresh reactively when the
/// editor (or the keyboard) writes them — no `SharedSettings` publishing needed.
struct ThemePickerView: View {
    @AppStorage("selectedThemeId", store: UserDefaults(suiteName: SharedSettings.appGroupId))
    private var selectedThemeId = ThemeId.default

    // Bumped on every theme-file change (create / edit / delete) to drive a userThemes reload.
    @AppStorage("themeRevision", store: UserDefaults(suiteName: SharedSettings.appGroupId))
    private var themeRevision = 0

    @State private var userThemes: [UserTheme] = []
    @State private var editorRoute: ThemeEditorRoute?

    @Environment(DisplayLanguageStore.self) private var lang

    var body: some View {
        ScrollView {
            LazyVStack(alignment: .leading, spacing: GalleryCardMetrics.shelfSpacing) {
                // Custom Themes: user's saved themes + Create New.
                GalleryShelf(title: lang.string(.themeCustomThemesSection)) {
                    if userThemes.count < UserThemeStore.maxUserThemes {
                        GalleryCreateCard(title: lang.string(.themeCreateNewTheme)) { editorRoute = .create }
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
                                ThemeCardAction(title: lang.string(.commonEdit)) { editorRoute = .edit(theme) },
                                ThemeCardAction(title: lang.string(.commonDelete), role: .destructive) { delete(theme) },
                            ],
                        )
                    }
                }

                // Built-in families: one horizontal shelf each (Standard / Swifty / Minimal …).
                // The Standard family's first card is the app default (id == ThemeId.default).
                ForEach(BuiltInThemes.families, id: \.titleKey) { family in
                    GalleryShelf(title: lang.string(family.titleKey)) {
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
        // Matches the Form-backed tabs (systemGroupedBackground); a bare ScrollView defaults to systemBackground.
        .background(Color(.systemGroupedBackground))
        .navigationTitle(lang.string(TabType.theme.titleKey))
        // Any CRUD bump of themeRevision reloads the list; onAppear covers popping back to this page.
        .onAppear(perform: reloadUserThemes)
        .onChange(of: themeRevision) { _, _ in reloadUserThemes() }
        // The editor is pushed onto ThemeTab's NavigationStack instead of shown as a sheet; route ids
        // are stable ("create", or the theme id when editing).
        .navigationDestination(item: $editorRoute) { route in
            switch route {
            case .create:
                ThemeEditorView()
            case let .edit(theme):
                ThemeEditorView(editing: theme)
            }
        }
    }

    // The write flows @AppStorage → App Group, so the keyboard re-resolves on its next render.
    private func apply(_ id: String) {
        selectedThemeId = id
    }

    // Falls back to the default theme when the deleted one was selected, so no orphan id renders.
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

// MARK: - Theme gallery card

/// A trailing-menu action for a theme card (apply / edit / delete).
private struct ThemeCardAction: Identifiable {
    let id = UUID()
    let title: String
    var role: ButtonRole?
    let action: () -> Void
}

/// A theme cell (`GalleryCard`): a preview (screenshot when `previewImageName` is set, else a
/// live custom-theme background preview) + a title + an optional `…` action menu (user themes
/// only). Tapping the preview applies the theme.
private struct ThemeGalleryCard: View {
    let title: String
    /// Full appearance for the live custom-theme preview; `nil` for built-in cards
    /// (they render via `previewImageName` and never reach the live preview).
    var appearance: ThemeAppearance?
    /// Screenshot asset name; `nil` → render the live custom-theme background preview.
    let previewImageName: String?
    let isSelected: Bool
    let onTap: () -> Void
    let actions: [ThemeCardAction]

    var body: some View {
        GalleryCard(title: title, isSelected: isSelected, onTap: onTap) {
            if let previewImageName {
                GalleryScreenshot(imageName: previewImageName, title: title)
            } else {
                CustomThemeBackgroundPreview(appearance: appearance ?? .default)
            }
        } accessory: {
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
    }
}

// MARK: - Custom-theme background preview

/// A custom-theme card preview: the theme background alone (same surface view as the
/// keyboard root, `TaigiKeyboardView`) — no sample key, so the selection checkmark sits
/// on the bare surface like a built-in card's (USER 2026-09-26). User themes are seeded
/// on load, so a nil background only occurs for a malformed entry and falls back to the seed.
private struct CustomThemeBackgroundPreview: View {
    let appearance: ThemeAppearance

    var body: some View {
        ThemeBackgroundSurface(
            surface: appearance.colors.surface ?? ThemeSurface(background: UserThemeSeed.background, dimsTowardWhite: true),
            photoVariant: .thumbnail,
        ).equatable()
    }
}
