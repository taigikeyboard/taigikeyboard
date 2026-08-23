// The System Settings-style shell: a sidebar of panes, one detail at a time.

import SwiftUI

/// One sidebar destination, and everything that differs between them. A new
/// pane cannot land in the sidebar without bringing its label, its icon and
/// its detail view with it.
///
/// `String` raw values so the selected pane can persist through
/// `@AppStorage` — Apple's Settings guidance is to reopen on the pane the
/// user last used. A value persisted by a build whose case is since removed is
/// cleared at launch (`RetiredSettingsCleanup`), landing on the default, 一般.
enum SettingsPane: String, CaseIterable, Identifiable {
    case general
    case appearance
    case shortcuts
    case customDictionary
    case dictionarySources

    var id: String {
        rawValue
    }

    /// A key rather than a resolved string, so the sidebar re-renders under
    /// the current display language instead of the one it was built in.
    var labelKey: StringKey {
        switch self {
        case .general: .macosGeneralTab
        case .appearance: .macosAppearanceTab
        case .shortcuts: .macosShortcutsTab
        case .customDictionary: .dictionaryCustomDictionary
        case .dictionarySources: .macosDictionarySourcesLink
        }
    }

    var symbolName: String {
        switch self {
        case .general: "gearshape"
        case .appearance: "paintpalette"
        case .shortcuts: "keyboard"
        case .customDictionary: "character.book.closed"
        case .dictionarySources: "books.vertical"
        }
    }
}

/// Layout the pane forms agree on.
enum SettingsPaneLayout {
    /// The 一般 / 外觀 / 快捷鍵 panes are columns of labelled controls, which
    /// read best bounded — stretched across a wide detail pane, every row
    /// becomes a label staring at a far-away control. The 詞庫 pages beside
    /// them are deliberately unbounded: their rows are content, and content
    /// takes whatever width the window has.
    static let maximumFormWidth: CGFloat = 640
}

/// The settings window's content: a `NavigationSplitView` with every pane in
/// the sidebar, the System Settings shape.
///
/// The detail router owns each pane's `.navigationTitle` — the pane views
/// carry none of their own — so exactly one place feeds the window title,
/// which the hosting controller bridges to the titlebar.
struct SettingsSplitView: View {
    /// The stores the dictionary pages read and write. Named here rather than
    /// reached for inside each page so the whole window is driven by one
    /// composition root, and a test could hand it its own.
    let stores: UserDataStores

    /// Was read by the 揣辭典 pane's search service; the pane is unlisted for
    /// now (not released yet, USER 2026-08-21) and the injection point stays so
    /// relisting it is one `detailView` case again.
    let settingsProvider: any EngineSettingsProvider

    @Environment(DisplayLanguageStore.self) private var language

    /// `@AppStorage` reads a `String`-backed enum directly: an unknown
    /// persisted raw value falls back to this default on its own, and the
    /// non-optional `List(selection:)` below rules out deselection — both
    /// edges the framework owns, not this view.
    @AppStorage(SettingsStore.Keys.selectedSettingsPane.name)
    private var selectedPane = SettingsStore.Keys.selectedSettingsPane.defaultValue

    var body: some View {
        // Visibility pinned to `.all`, matching System Settings: the sidebar
        // IS the navigation, so collapsing it strands the user — and with no
        // way to collapse, the toggle below is removed rather than orphaned.
        NavigationSplitView(columnVisibility: .constant(.all)) {
            // One flat list, no section headers — the sidebar is short enough
            // to read at a glance, and a group label above the dictionary rows
            // was a heading with nothing to disambiguate (USER 2026-08-18).
            // `allCases` IS the sidebar order, which mirrors the iOS Tab3
            // listing for the dictionary rows.
            List(selection: $selectedPane) {
                ForEach(SettingsPane.allCases) { pane in
                    sidebarRow(pane)
                }
            }
            // System Settings shows no sidebar toggle; without this, the
            // split view puts one above the sidebar column.
            .toolbar(removing: .sidebarToggle)
            .navigationSplitViewColumnWidth(min: 180, ideal: 200, max: 240)
        } detail: {
            detailView
                .navigationTitle(language.string(selectedPane.labelKey))
        }
    }

    private func sidebarRow(_ pane: SettingsPane) -> some View {
        Label(language.string(pane.labelKey), systemImage: pane.symbolName)
            .tag(pane)
    }

    /// Uniform pane→page mapping: every page brings its own `Form`, so the
    /// router adds no layout of its own.
    @ViewBuilder
    private var detailView: some View {
        switch selectedPane {
        case .general:
            GeneralSettingsView(stores: stores)
        case .appearance:
            AppearanceSettingsView()
        case .shortcuts:
            ShortcutSettingsView()
        case .customDictionary:
            CustomDictionaryPage(store: stores.customDictionary)
        case .dictionarySources:
            DictionaryTogglesView()
        }
    }
}

/// The hosting root: the split view with its language store injected. A named
/// type rather than an inline `.environment(...)` expression so the window's
/// content controller has a concrete `NSHostingController<SettingsRootView>`
/// type a test can cast to and inspect.
struct SettingsRootView: View {
    let stores: UserDataStores
    let settingsProvider: any EngineSettingsProvider
    let language: DisplayLanguageStore

    var body: some View {
        SettingsSplitView(stores: stores, settingsProvider: settingsProvider)
            .environment(language)
    }
}
