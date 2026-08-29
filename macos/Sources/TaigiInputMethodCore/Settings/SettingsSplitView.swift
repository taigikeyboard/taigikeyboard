// What the settings window shows: the pane roster, its two columns, and the widths they agree on.

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
        case .general: .desktopGeneralTab
        case .appearance: .desktopAppearanceTab
        case .shortcuts: .desktopShortcutsTab
        case .customDictionary: .dictionaryCustomDictionary
        case .dictionarySources: .desktopDictionarySourcesLink
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

/// Every number the settings window is sized from.
///
/// One place, and stated to the WINDOW (`SettingsWindowController`) rather
/// than to SwiftUI: the two columns are child hosting controllers, and a
/// hosting view only writes `contentMinSize`/`contentMaxSize` when it is the
/// window's own content view. An earlier layout had one hosting controller as
/// that content view, where the write-back silently replaced the pinned width
/// with whatever SwiftUI measured.
enum SettingsPaneLayout {
    /// Wide enough for the longest pane label in the five display languages to
    /// sit on one line beside its icon.
    static let sidebarWidth: CGFloat = 215

    /// The room the panes were sized for. Not measured from their content:
    /// it is what the window's previous floor of 760 leaves once the sidebar
    /// takes 215, and the panes have been read at that width in five
    /// languages rather than argued to it.
    static let detailWidth: CGFloat = 545

    /// The window's one width, in both directions: System Settings cannot be
    /// resized horizontally, and neither can this. Summed from the two columns
    /// rather than written out, so widening the sidebar cannot silently take
    /// room from the panes it was sized for.
    static var contentWidth: CGFloat {
        sidebarWidth + detailWidth
    }

    /// The shortest the window goes. One floor for the whole window — the
    /// sidebar shows every pane, so there is no per-tab size to switch
    /// between.
    static let minimumContentHeight: CGFloat = 470

    /// What a first launch opens at.
    static let initialContentHeight: CGFloat = 560
}

/// The sidebar: every pane, one flat list.
///
/// Selection lives in `UserDefaults` rather than in a shared object passed to
/// both columns: `@AppStorage` observes the key, so the detail view beside
/// this one re-renders off the same write with nothing wired between them —
/// and so does a pane written from outside the window entirely.
struct SettingsSidebarView: View {
    @Environment(DisplayLanguageStore.self) private var language

    /// `@AppStorage` reads a `String`-backed enum directly: an unknown
    /// persisted raw value falls back to this default on its own, and the
    /// non-optional `List(selection:)` below rules out deselection — both
    /// edges the framework owns, not this view.
    @AppStorage(SettingsStore.Keys.selectedSettingsPane.name)
    private var selectedPane = SettingsStore.Keys.selectedSettingsPane.defaultValue

    var body: some View {
        // One flat list, no section headers — the sidebar is short enough
        // to read at a glance, and a group label above the dictionary rows
        // was a heading with nothing to disambiguate (USER 2026-08-18).
        // `allCases` IS the sidebar order, which mirrors the iOS Tab3
        // listing for the dictionary rows.
        List(selection: $selectedPane) {
            ForEach(SettingsPane.allCases) { pane in
                Label(language.string(pane.labelKey), systemImage: pane.symbolName)
                    .tag(pane)
            }
        }
        // The sidebar list style the split view item's material expects; a
        // `List` outside a `NavigationSplitView` does not infer it.
        .listStyle(.sidebar)
    }
}

/// The selected pane's form. Carries no `navigationTitle`: the titlebar takes
/// its name from `SettingsSplitViewController`, the one place that knows both
/// the selection and the display language.
///
/// The 揣辭典 pane's `EngineSettingsProvider` is not threaded through here.
/// That pane is unlisted (not released yet, USER 2026-08-21) and nothing else
/// on this side reads a provider, so carrying one would be three signatures
/// held open for a caller that does not exist; relisting the pane adds it back
/// where it is needed.
struct SettingsDetailView: View {
    /// The stores the dictionary pages read and write.
    let stores: UserDataStores

    @AppStorage(SettingsStore.Keys.selectedSettingsPane.name)
    private var selectedPane = SettingsStore.Keys.selectedSettingsPane.defaultValue

    /// Uniform pane→page mapping: every page brings its own `Form`, so this
    /// adds no layout of its own.
    var body: some View {
        switch selectedPane {
        case .general:
            GeneralSettingsView()
        case .appearance:
            AppearanceSettingsView()
        case .shortcuts:
            ShortcutSettingsView()
        case .customDictionary:
            CustomDictionaryPage(stores: stores)
        case .dictionarySources:
            DictionaryTogglesView()
        }
    }
}
