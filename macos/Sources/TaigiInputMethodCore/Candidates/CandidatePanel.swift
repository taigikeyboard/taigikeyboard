// The process-wide candidate window: ownership guard over the layout panels.

import AppKit

/// The candidate window as the controllers share it.
///
/// One per process, like the composing engine behind it: only one input session
/// is focused at a time, so a window per session would be a pile of hidden
/// windows and a race over which of them is on top. Ownership is tracked here —
/// see `CandidatePresenter.hide(ownedBy:)` for what that guard buys — and the
/// layout panels underneath never learn which session they are showing for.
///
/// Not `IMKCandidates`: it renders a fixed table of Apple's own design, with no
/// control over cell content — this window shows both scripts of every
/// candidate (§42), which that table cannot express. It is instead a port of
/// MacishType's, which replicates the native look (accent colour,
/// vibrancy/glass chrome) while leaving what a cell says to us.
@MainActor
final class CandidatePanel: CandidatePresenter {
    static let shared = CandidatePanel()

    private static let logger = DebugLogger(category: "CandidatePanel")

    /// The session the window is currently showing for, or nil when nothing is
    /// showing.
    private var owner: ComposingSessionToken?

    /// One panel per layout the user has actually used, built the first time a
    /// candidate is shown under that layout.
    ///
    /// A dictionary rather than a `lazy` panel because every path that only
    /// HIDES has to leave it alone: a session that never offers a candidate —
    /// an English document, a password field — must not cause an input method
    /// to open a window, and `activateServer` hides on every focus change.
    private var panels: [CandidateLayout: CandidateBasePanel] = [:]
    private var panel: CandidateBasePanel?

    /// The chrome generation every panel this process builds is drawn in. Read
    /// once rather than per show: it comes from the running OS, not a setting.
    private let style = CandidateWindowStyle.systemStyle

    /// Live-read on every show, so a layout switched in the settings window
    /// applies to the very next keystroke — the store caches nothing.
    var settings = SettingsStore()

    private init() {}

    func show(
        _ content: CandidateWindowContent,
        anchoredTo caretRect: CGRect,
        hostWindowLevel: CGWindowLevel,
        hostBundleIdentifier: String?,
        ownedBy owner: ComposingSessionToken,
    ) {
        let panel = panel(for: settings.candidateLayout)
        // Before the cells: the key each of them is drawn with is resolved as
        // they are built.
        panel.slotKeySet = content.slotKeySet
        panel.leadCellIsUnkeyed = content.leadCellIsUnkeyed
        let panelSize = panel.layout(content.cells, forCaret: caretRect)
        let presented = panel.present(
            panelSize: panelSize,
            anchoredTo: caretRect,
            hostWindowLevel: hostWindowLevel,
            hostBundleIdentifier: hostBundleIdentifier,
            forcedAppearance: settings.appearanceMode.forcedAppearance,
        )
        guard presented else {
            // No display to place it on. The panel still holds the fresh list,
            // so leaving it up would show it somewhere unrelated — it comes
            // down, and ownership goes with it.
            Self.logger.debug("no screen for caret \(String(describing: caretRect))")
            hideNow()
            return
        }
        // Taken only once the window is really on screen: an owner recorded for
        // a window that was never shown would turn the NEXT session's
        // `hide(ownedBy:)` into a no-op against a window it does not own.
        self.owner = owner
    }

    func updateCells(_ content: CandidateWindowContent, ownedBy owner: ComposingSessionToken) {
        guard let panel = livePanel(ownedBy: owner) else { return }
        // Before the cells, as `show` sets them: the key each cell is drawn
        // with is resolved as it is built.
        panel.slotKeySet = content.slotKeySet
        panel.leadCellIsUnkeyed = content.leadCellIsUnkeyed
        panel.rerender(content.cells)
    }

    /// The panel `owner` may repaint in place, or nil.
    ///
    /// `isVisible` on top of the owner guard: ownership is cleared on every hide
    /// path, so the two should agree — but a window taken down behind the
    /// panel's back must not be re-laid-out as if it were on screen. Stated once
    /// because both in-place update paths need exactly this, and two copies
    /// could drift into disagreeing about what "showing" means.
    private func livePanel(ownedBy owner: ComposingSessionToken) -> CandidateBasePanel? {
        guard self.owner == owner, let panel, !panel.isEmpty, panel.isVisible else { return nil }
        return panel
    }

    func navigate(_ direction: CandidateNavigation, ownedBy owner: ComposingSessionToken) {
        guard self.owner == owner else { return }
        panel?.navigate(direction)
    }

    func selectedCandidateIndex(ownedBy owner: ComposingSessionToken) -> Int? {
        guard self.owner == owner, let panel, !panel.isEmpty else { return nil }
        return panel.selectedIndex
    }

    func candidateIndex(forKeySlot slot: Int, ownedBy owner: ComposingSessionToken) -> Int? {
        guard self.owner == owner else { return nil }
        return panel?.candidateIndex(forKeySlot: slot)
    }

    func hide(ownedBy owner: ComposingSessionToken) {
        guard self.owner == owner else { return }
        hideNow()
    }

    func hideForHandover() {
        hideNow()
    }

    private func hideNow() {
        owner = nil
        // Cleared, not just ordered out: state kept alive behind a hidden
        // window would let the selection queries answer for candidates nobody
        // can see.
        panel?.clear()
    }

    /// Drops the cached panels built in `font`, hidden ones included.
    ///
    /// Called before a typeface the user installed is unregistered
    /// (`AppearanceSettingsView.remove`): Core Text refuses to unregister a font
    /// that is still in use, and a panel built in that face — including one for
    /// a layout that is not on screen — is exactly such a use. Panels in any
    /// other face are left alone: rebuilding them would cost a window's cells
    /// and constraints to delete a typeface they were never set in.
    func releaseCachedPanels(drawing font: CustomFont) {
        let selection = CandidateFontSelection.custom(font)
        let affected = panels.filter { $0.value.configuredMetrics.fontSelection == selection }
        guard !affected.isEmpty else { return }
        for (layout, cached) in affected {
            cached.clear()
            panels[layout] = nil
            if cached === panel {
                owner = nil
                panel = nil
            }
        }
    }

    /// The panel for `layout`, taking down whichever other layout's panel was
    /// up: the settings are live-read per show, so a switch mid-composition
    /// swaps windows on the next keystroke rather than leaving two on screen.
    /// A cached panel built under a size the settings no longer name is
    /// rebuilt — the metrics are baked into every cell's constraints at
    /// construction. A stale panel of some OTHER layout can only ever reach
    /// the screen through this method, so leaving it cached until it is asked
    /// for costs nothing.
    private func panel(for layout: CandidateLayout) -> CandidateBasePanel {
        // The settings resolve the sizes; the layout resolves how its cells
        // hold their two scripts. Both are baked into the panel's cells, so
        // both are settled before the cache below is asked for one.
        let metrics = settings.candidateMetrics.arranged(layout.cellArrangement)
        var target = panels[layout]
        if let cached = target, cached.configuredMetrics != metrics {
            cached.clear()
            target = nil
        }
        let resolved = target ?? makePanel(for: layout, style: style, metrics: metrics)
        panels[layout] = resolved
        if let previous = panel, previous !== resolved {
            previous.clear()
        }
        panel = resolved
        return resolved
    }

    private func makePanel(
        for layout: CandidateLayout,
        style: CandidateWindowStyle,
        metrics: CandidateMetrics,
    ) -> CandidateBasePanel {
        switch layout {
        case .horizontal: HorizontalCandidatePanel(style: style, metrics: metrics)
        case .vertical: VerticalCandidatePanel(style: style, metrics: metrics)
        case .expandable: ExpandableCandidatePanel(style: style, metrics: metrics)
        }
    }
}
