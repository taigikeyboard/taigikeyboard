// What the settings window shows: the pane roster, its two columns, and the widths they agree on.

import SwiftUI

/// One sidebar destination, and everything that differs between them. A new
/// pane cannot land in the sidebar without bringing its label, its icon and
/// its detail view with it.
///
/// `String` raw values so the selected pane can persist through
/// `@AppStorage` — Apple's Settings guidance is to reopen on the pane the
/// user last used. A value persisted by a build whose case is since removed is
/// cleared at launch (`RetiredSettingsCleanup`), landing on the default, General.
enum SettingsPane: String, CaseIterable, Identifiable {
    case general
    case appearance
    case shortcuts
    /// The sources before the user's own words on top of them (USER
    /// 2026-09-21).
    case dictionarySources
    case customDictionary
    /// Last (USER 2026-09-08): what the input method draws IN, after what
    /// it draws FROM.
    case fontManagement
    /// Unlisted: the input-source menu's About row opens it, and the sidebar
    /// shows no row for it (USER 2026-09-20: "it does not need to appear in the settings menu").
    case about

    var id: String {
        rawValue
    }

    /// The panes the sidebar lists, top to bottom: every case but About.
    static let sidebar: [SettingsPane] = allCases.filter { $0 != .about }

    /// A key rather than a resolved string, so the sidebar re-renders under
    /// the current display language instead of the one it was built in.
    var labelKey: StringKey {
        switch self {
        case .general: .desktopGeneralTab
        case .appearance: .desktopAppearanceTab
        case .shortcuts: .desktopShortcutsTab
        case .customDictionary: .dictionaryCustomDictionary
        case .dictionarySources: .desktopDictionarySourcesLink
        case .fontManagement: .desktopFontManagementTab
        case .about: .homeAboutKeyboard
        }
    }

    /// The sidebar row's icon; About has no row, and names the symbol its
    /// title would carry anywhere else.
    var symbolName: String {
        switch self {
        case .general: "gearshape"
        case .appearance: "paintpalette"
        case .shortcuts: "keyboard"
        case .customDictionary: "character.book.closed"
        case .dictionarySources: "books.vertical"
        case .fontManagement: "textformat"
        case .about: "info.circle"
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
    /// persisted raw value falls back to this default on its own — an edge
    /// the framework owns, not this view.
    @AppStorage(SettingsStore.Keys.selectedSettingsPane.name)
    private var selectedPane = SettingsStore.Keys.selectedSettingsPane.defaultValue

    /// The list's own selection: the stored pane when the list has a row for
    /// it, nothing when it is About. An optional binding says "no row" in the
    /// list's own vocabulary rather than handing it a tag it cannot find,
    /// and a `nil` written back (the list clearing itself) leaves the stored
    /// pane alone — only a row the user picked moves it.
    private var listSelection: Binding<SettingsPane?> {
        Binding(
            get: { SettingsPane.sidebar.contains(selectedPane) ? selectedPane : nil },
            set: { picked in
                if let picked {
                    selectedPane = picked
                }
            },
        )
    }

    var body: some View {
        // One flat list, no section headers — the sidebar is short enough
        // to read at a glance, and a group label above the dictionary rows
        // was a heading with nothing to disambiguate (USER 2026-08-18).
        // `sidebar` IS the sidebar order, which mirrors the iOS Tab3
        // listing for the dictionary rows.
        List(selection: listSelection) {
            ForEach(SettingsPane.sidebar) { pane in
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
/// The Dictionary Search pane's `EngineSettingsProvider` is not threaded through here.
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
        case .fontManagement:
            FontManagementPage()
        case .about:
            AboutPage()
        }
    }
}
