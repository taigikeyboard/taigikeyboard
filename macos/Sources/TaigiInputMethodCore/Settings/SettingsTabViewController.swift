// The toolbar-tab container of the settings window: [一般] [詞庫].

import AppKit
import SwiftUI

/// Preferences-style tabs over the two settings panes.
///
/// `tabStyle = .toolbar` is what turns the tabs into window-toolbar items, the
/// Safari/Mail preferences look (`NSTabViewController.h:25`); the window pairs
/// it with `toolbarStyle = .preference`, which the SDK marks "For Settings
/// windows only" (`NSWindow.h:239`).
final class SettingsTabViewController: NSTabViewController {
    /// One tab, and everything that differs between them. The tabs are added in
    /// `allCases` order, so the raw value stays the tab-view index the selection
    /// callback maps back through — a new case cannot land in the toolbar
    /// without bringing its pane, its icon and its size floor with it.
    enum ContentTab: Int, CaseIterable {
        case general = 0
        case dictionary = 1

        var label: String {
            switch self {
            case .general: String(localized: "一般")
            case .dictionary: String(localized: "詞庫")
            }
        }

        /// Toolbar tabs are icon-first: a label with no image renders as a bare
        /// word in the toolbar.
        var symbolName: String {
            switch self {
            case .general: "gearshape"
            case .dictionary: "character.book.closed"
            }
        }

        /// The floor this tab puts under the window's content size. 一般 is the
        /// width its form is pinned to; 詞庫 carries lists and needs the room.
        var minimumContentSize: NSSize {
            switch self {
            case .general: NSSize(width: GeneralSettingsView.formWidth, height: 360)
            case .dictionary: NSSize(width: 560, height: 420)
            }
        }

        @MainActor
        func makePane() -> NSViewController {
            switch self {
            case .general: NSHostingController(rootView: GeneralSettingsView())
            case .dictionary: NSHostingController(rootView: DictionarySettingsPane())
            }
        }
    }

    override func viewDidLoad() {
        super.viewDidLoad()
        tabStyle = .toolbar

        for tab in ContentTab.allCases {
            let item = NSTabViewItem(viewController: tab.makePane())
            item.label = tab.label
            item.image = NSImage(systemSymbolName: tab.symbolName, accessibilityDescription: tab.label)
            addTabViewItem(item)
        }
    }

    /// The first tab is selected before the controller has a window — the tabs
    /// are built in `viewDidLoad`, and `contentViewController` is assigned
    /// after that — so the floor for the tab the window OPENS on has to be
    /// applied here as well. Without it, a window restoring a small autosaved
    /// frame would come up under its own content's minimum.
    override func viewWillAppear() {
        super.viewWillAppear()
        applyMinimumSizeForSelectedTab()
    }

    /// Applies the current tab's floor. Called by the window controller once
    /// the window exists, which is what covers the tab the window opens on.
    func applyMinimumSizeForSelectedTab() {
        applyMinimumSize(for: tabView.selectedTabViewItem)
    }

    override func tabView(_ tabView: NSTabView, didSelect tabViewItem: NSTabViewItem?) {
        super.tabView(tabView, didSelect: tabViewItem)
        applyMinimumSize(for: tabViewItem)
    }

    /// Raises the window's content-size floor to the selected tab's, growing
    /// the window if it currently sits below it.
    ///
    /// AppKit does not promise to consult `preferredContentSize` on a tab
    /// switch, so the floor is applied by hand here: the window only ever GROWS
    /// to meet it — never shrinks a size the user chose, and never touches the
    /// style mask, so the resize affordance stays put across tabs.
    private func applyMinimumSize(for tabViewItem: NSTabViewItem?) {
        guard let window = view.window,
              let tabViewItem,
              let tab = ContentTab(rawValue: tabView.indexOfTabViewItem(tabViewItem))
        else { return }

        let minimum = tab.minimumContentSize
        window.contentMinSize = minimum

        let current = window.contentLayoutRect.size
        let grown = NSSize(
            width: max(current.width, minimum.width),
            height: max(current.height, minimum.height),
        )
        if grown != current {
            window.setContentSize(grown)
        }
    }
}
