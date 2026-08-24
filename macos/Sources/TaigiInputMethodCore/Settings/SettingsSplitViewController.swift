// The settings window's two columns, and the window chrome that follows them.

import AppKit
import SwiftUI

/// The AppKit container the settings window's content lives in: a sidebar of
/// panes beside the selected pane's form.
///
/// AppKit rather than SwiftUI's `NavigationSplitView`, which cannot be told
/// not to collapse. Its `columnVisibility` binding is documented to be ignored
/// once the split view adopts its collapsed presentation, and a single-value
/// `navigationSplitViewColumnWidth` is a *preferred* width — USER confirmed on
/// device that the sidebar still collapsed under both. `NSSplitViewItem` is
/// the only public API that states "this column does not collapse and does not
/// resize": `canCollapse = false` plus a minimum thickness equal to the
/// maximum.
///
/// The two columns are separate `NSHostingController`s, so neither can write
/// the window's size limits — that write-back only happens for a hosting view
/// used AS the window's content view — which leaves
/// `SettingsWindowController` the single authority on the window's width.
@MainActor
final class SettingsSplitViewController: NSSplitViewController {
    private let language: DisplayLanguageStore

    /// Held for as long as the pane selection should keep re-titling this
    /// controller; the observation ends when it is released.
    private var paneObservation: AnyObject?

    init(stores: UserDataStores, language: DisplayLanguageStore) {
        self.language = language
        super.init(nibName: nil, bundle: nil)

        let sidebar = NSHostingController(
            rootView: SettingsSidebarView().environment(language),
        )
        let sidebarItem = NSSplitViewItem(sidebarWithViewController: sidebar)
        // Stated even though `false` is already the default: this is the whole
        // reason the pane is AppKit, and a later edit must not read it as
        // incidental.
        sidebarItem.canCollapse = false
        // Equal bounds are what make the divider immovable; `canCollapse`
        // alone would still let the column be dragged wider and narrower.
        sidebarItem.minimumThickness = SettingsPaneLayout.sidebarWidth
        sidebarItem.maximumThickness = SettingsPaneLayout.sidebarWidth
        addSplitViewItem(sidebarItem)

        let detail = NSHostingController(
            rootView: SettingsDetailView(stores: stores).environment(language),
        )
        // No thickness of its own: it takes what the window's fixed width
        // leaves after the sidebar and the divider, and pinning a second rigid
        // number here would fight that arithmetic over the divider's width.
        addSplitViewItem(NSSplitViewItem(viewController: detail))
    }

    @available(*, unavailable)
    required init?(coder _: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    /// This controller's `title`, which the window binds to
    /// (`SettingsWindowController`): `NSViewController.title` exists so a
    /// container can name whichever of several views it is showing, and
    /// `NSWindow` documents the binding as the way to carry it into the
    /// titlebar. Writing `window.title` from here instead would be the content
    /// reaching out to chrome it does not own.
    ///
    /// Two inputs, two mechanisms, because they are two different kinds of
    /// fact. The selected pane is a defaults key, so it is observed as one —
    /// key-scoped KVO, which also catches a `defaults write` from outside this
    /// process (`SettingsStore.observeChanges`). The display language is not a
    /// key at all under Automatic, where an OS language change writes nothing:
    /// `DisplayLanguageStore` is `@Observable`, so the language side tracks the
    /// store itself.
    override func viewDidLoad() {
        super.viewDidLoad()
        paneObservation = SettingsStore().observeChanges(of: SettingsStore.Keys.selectedSettingsPane) {
            // Fires on whichever thread wrote the value, and carries none —
            // hop, then re-read, so out-of-order writes still converge.
            Task { @MainActor in self.updateTitle() }
        }
        trackLanguage()
        updateTitle()
    }

    /// Re-titles on the next language change, then re-arms: an observation
    /// tracked this way fires once.
    private func trackLanguage() {
        withObservationTracking {
            _ = language.resolver
        } onChange: {
            Task { @MainActor in
                self.updateTitle()
                self.trackLanguage()
            }
        }
    }

    private func updateTitle() {
        let paneTitle = language.string(SettingsStore().selectedSettingsPane.labelKey)
        if title != paneTitle {
            title = paneTitle
        }
    }
}
