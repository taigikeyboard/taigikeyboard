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

    init(userData: any UserDataClient, language: DisplayLanguageStore) {
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
            rootView: SettingsDetailView(userData: userData).environment(language),
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
        paneObservation = SettingsStore().observeChanges(of: SettingsStore.Keys.selectedSettingsPane, onMainActor: {
            self.updateTitle()
            // A new pane opens at its top, and its fresh scroll view need not
            // post a bounds change to say so — a title hidden by the previous
            // pane's scroll position would otherwise stay hidden. Here and not
            // in `updateTitle()`: a language change re-titles a pane that is
            // still scrolled, where showing the title would bring the overlap
            // back.
            self.view.window?.titleVisibility = .visible
        })
        trackLanguage()
        updateTitle()
    }

    /// The window is `.fullSizeContentView` with a transparent titlebar, so
    /// the pane's rows scroll up under the title text with nothing painted
    /// between them. Rather than an opaque bar, the title steps aside: shown
    /// while the pane sits at its top, hidden once the rows have scrolled
    /// under it (USER 2026-09-14).
    ///
    /// Observed process-wide (`object: nil`) and only while the window is on
    /// screen: the pane's `Form` owns its scroll view and rebuilds it on every
    /// pane switch, so there is no clip view to hold, and the window-identity
    /// check below is what keeps the candidate panel's scrolling out.
    override func viewWillAppear() {
        super.viewWillAppear()
        NotificationCenter.default.addObserver(
            self, selector: #selector(clipViewBoundsDidChange(_:)),
            name: NSView.boundsDidChangeNotification, object: nil,
        )
    }

    override func viewDidDisappear() {
        super.viewDidDisappear()
        NotificationCenter.default.removeObserver(self, name: NSView.boundsDidChangeNotification, object: nil)
    }

    @objc private func clipViewBoundsDidChange(_ notification: Notification) {
        guard let clipView = notification.object as? NSClipView, let window = view.window,
              clipView.window === window, let detailView = splitViewItems.last?.viewController.view,
              Self.isPaneScrollView(clipView, in: detailView) else { return }
        let visibility = Self.titleVisibility(for: clipView)
        if window.titleVisibility != visibility {
            window.titleVisibility = visibility
        }
    }

    /// Whether `clipView` is the pane's own scroll view — not the sidebar's,
    /// and not a table nested inside the pane (`CustomDictionaryPage` scrolls
    /// its entries in one), which moves without the pane moving.
    static func isPaneScrollView(_ clipView: NSClipView, in detailView: NSView) -> Bool {
        clipView.isDescendant(of: detailView) && clipView.superview?.superview?.enclosingScrollView == nil
    }

    /// Visible at the top, hidden once scrolled. The clip view's bounds start
    /// at minus its top inset, which is where AppKit puts the titlebar's
    /// safe area under `.fullSizeContentView`; the one-point slack absorbs
    /// the fractional offset an elastic bounce settles at.
    static func titleVisibility(for clipView: NSClipView) -> NSWindow.TitleVisibility {
        clipView.bounds.minY <= -clipView.contentInsets.top + 1 ? .visible : .hidden
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
