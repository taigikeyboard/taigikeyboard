// Input-source menu of TaigiInputController: the rows, the settings doorway, the update check.

import InputMethodKit
import KeyboardShortcuts

extension TaigiInputController {
    /// The menu under the input-source icon in the menu bar.
    ///
    /// Built fresh on every call, which is what the contract asks for: the
    /// system calls this "whenever the menu needs to be drawn so that input
    /// methods can update the menu to reflect their current state"
    /// (`IMKInputController.h:307-310`). That is what keeps each row printing
    /// the key it currently answers to, with no refresh wiring of its own.
    ///
    /// Nothing here reads session state, so unlike the other entry points this
    /// one asserts no isolation: the mode comes from `UserDefaults`, which is
    /// thread-safe, and the rest is titles and selectors. It does build AppKit
    /// objects, and so still rests on the process-wide assumption that IMK
    /// calls its controllers on the main run loop — the assumption
    /// `onMainActor` exists to turn into a crash rather than a data race
    /// everywhere it can be checked.
    override public func menu() -> NSMenu! {
        // Each row's chord is read and assigned by hand rather than set through
        // the library's `NSMenuItem.setShortcut(for:)`: that helper registers a
        // `NotificationCenter` observer per item so a long-lived item can update
        // itself, and this menu is rebuilt from scratch every time the system
        // draws it (`IMKInputController.h:307-310`) — one observer would be
        // added per draw and never removed, since removal only happens by
        // re-binding the same item. Rebuilding IS the update mechanism the
        // observer exists to provide. Why only the global rows claim a key
        // equivalent is `InputSourceMenuRow`'s to say.
        //
        // The global chords fire through the Carbon hotkeys `ShortcutHotkeys`
        // registers, active only while a session holds the engine.
        //
        // `assumeIsolated` for the same reason the rest of this class hops
        // through `onMainActor`: IMK calls its controllers on the main run
        // loop, and the shortcut store is main-actor-isolated. Asserting turns
        // a broken assumption into a crash rather than a data race.
        //
        // The titles are resolved in the same hop, and the store is synced first: under Automatic
        // the OS language can change while the persisted tag stays `"system"`, so nothing writes the
        // key and no observation fires — a menu rebuilt per draw is exactly the right place to
        // notice.
        //
        // Read BEFORE the hop, so the closure captures a value rather than this
        // controller: `menu()` is nonisolated and the controller is not
        // Sendable, so sending `self` into a main-actor closure does not
        // compile. The store itself is `@unchecked Sendable`.
        let injectedLanguage = displayLanguageOverride
        let groups = MainActor.assumeIsolated { () -> [[InputSourceMenuRow]] in
            let language = injectedLanguage ?? DisplayLanguageStore.shared
            // Can rebuild the menu bar and relabel the settings window as a side effect: the sync
            // commits a language change, and committing one runs the chrome renderer.
            language.syncFromSettings()

            // A row for a global shortcut may claim a key equivalent because it
            // IS a shortcut: an entry in the registry the Shortcuts pane records,
            // whose chord carries modifiers no composition types. Read live, so
            // the row prints whatever the user last recorded on it. (No
            // composing key can ever claim one — the agent proved unable to
            // DISPLAY one without also DISPATCHING it, `InputSourceMenuRow`.)
            //
            // Only a chord WITH modifiers: the recorder lets a user move any
            // row onto a bare key, and a bare key equivalent here would be
            // eaten by the agent everywhere for as long as this input source is
            // selected — the Carbon hotkey is session-scoped, the menu is not.
            // Such a row prints nothing, like a cleared one; the pane still
            // shows the key.
            @MainActor
            func shortcutRow(_ action: ShortcutAction, label: String, selector: Selector) -> InputSourceMenuRow {
                guard let shortcut = KeyboardShortcuts.getShortcut(for: action.name),
                      !shortcut.modifiers.isEmpty
                else {
                    return InputSourceMenuRow(label: label, action: selector)
                }
                return InputSourceMenuRow(
                    label: label,
                    keyEquivalent: shortcut.nsMenuItemKeyEquivalent ?? "",
                    modifiers: shortcut.modifiers,
                    action: selector,
                )
            }

            // The global shortcuts a click can stand in for (USER 2026-09-19):
            // the switches, each under the name the Shortcuts pane gives it,
            // so the menu is where a user looks up what they last recorded,
            // then the TPS key panel (desktop TPS P5; the rows are desktop-core's
            // `keys::MENU`, the same on the three desktops).
            // Not the Hanji/romanization swap — its default is the bare backtick, which
            // the rule above would never print; not the symbol picker, which
            // needs the caret a click has no hold of; and not the Telex guide
            // (USER 2026-09-20: "very few people use it").
            let shortcuts = [
                shortcutRow(
                    .toggleRomanization,
                    label: ShortcutAction.toggleRomanization.label(language),
                    selector: #selector(toggleRomanization(_:)),
                ),
                shortcutRow(
                    .toggleTps,
                    label: ShortcutAction.toggleTps.label(language),
                    selector: #selector(toggleTps(_:)),
                ),
                shortcutRow(
                    .cycleCandidateDisplayMode,
                    label: ShortcutAction.cycleCandidateDisplayMode.label(language),
                    selector: #selector(cycleCandidateDisplayMode(_:)),
                ),
                shortcutRow(
                    .showTpsKeyboard,
                    label: ShortcutAction.showTpsKeyboard.label(language),
                    selector: #selector(showTpsKeyboard(_:)),
                ),
            ]
            // One doorway (USER 2026-08-26). There was a row per settings pane
            // from 2026-08-21 until then: each carried its own ⌃⇧ chord, and
            // once those were retired the five rows were five names for one
            // window. What is left is the window itself, opening wherever the
            // user left it — naming the panes is the sidebar's job. Named
            // TaigiKeyboard Settings, as on Windows and Linux (USER 2026-09-24: the three
            // desktops' menus identical, i18n included; the shared row list is
            // `taigi_desktop_core::keys::MENU`).
            let settings = shortcutRow(
                .openLastSettingsPane,
                label: language.string(.desktopMenuSettings),
                selector: #selector(showPreferences(_:)),
            )
            // No chord, by design: an on-demand check is a command a user
            // reaches for once in a while, and a key equivalent claimed here
            // is taken from the host application for as long as this input
            // source is selected.
            let checkForUpdates = InputSourceMenuRow(
                label: language.string(.desktopUpdateCheckNow),
                action: #selector(checkForUpdates(_:)),
            )
            // The one doorway to the About page: it has no sidebar row (USER
            // 2026-09-20), so the menu is where it is found. No chord, as
            // for Check for Updates.
            let about = InputSourceMenuRow(
                label: language.string(.homeAboutKeyboard),
                action: #selector(showAbout(_:)),
            )
            return [shortcuts, [settings], [checkForUpdates, about]]
        }
        return InputSourceMenuRenderer.menu(groups)
    }

    /// Deliberately does not call `super`. The inherited implementation looks
    /// for a `preferences.nib` (`IMKInputController.h:165-170`); this package is
    /// built by SwiftPM and has no nib to find.
    ///
    /// What the menu's Settings row sends, since 2026-08-26: the header promises
    /// that a row whose action IS `showPreferences:` routes here
    /// (`IMKInputController.h:165-170`), which is the selector the system
    /// reserves for this exact command — so a second one of our own would be
    /// two names for one body. Opening no particular pane is what both callers
    /// want: the user returns where they left off, which is also where the
    /// ⌃⌘S chord the row prints lands.
    override public func showPreferences(_: Any!) {
        Self.logger.debug("showPreferences")
        openSettings(on: nil)
    }

    /// Brings the window up on this controller's own settings store, so a test
    /// drives the menu through its own defaults suite. `nil` leaves the stored
    /// pane alone, reopening the window where the user left it.
    private func openSettings(on pane: SettingsPane?) {
        Self.logger.debug("open settings on \(pane?.rawValue ?? "the last-used pane")")
        onMainActor(nil) { controller, _ in
            // The settings window taking focus does not guarantee this
            // session ends, and a picker left up would swallow keys behind it.
            controller.dismissSymbolPicker()
            ShortcutHotkeys.openSettings(
                on: pane,
                in: controller.settings,
                show: controller.settingsPresenterOverride,
            )
        }
    }

    /// Checks for a new version, with the settings window already up.
    ///
    /// The window first, then the check, and never the other way round: a
    /// fetch can take seconds, and the answer is a sheet on that window
    /// (`UpdateAlertPresenter`). Opening it at the moment the user picks the
    /// command is what puts the answer somewhere they are already looking —
    /// and this process is an `LSUIElement` whose activation makes the focused
    /// client resign, committing whatever was composing into the user's
    /// document (`UpdateNotificationOffer`), so that cost is paid on their
    /// click rather than seconds later when the network happens to answer.
    ///
    /// General because that is the pane the update state lives on
    /// (`GeneralSettingsView`), so the outcome has somewhere to land.
    @objc
    private func checkForUpdates(_: Any!) {
        Self.logger.debug("check for updates")
        openSettings(on: .general)
        onMainActor(nil) { controller, _ in
            let checker = controller.updateCheckerOverride ?? UpdateChecker.shared
            checker.checkManually()
        }
    }

    /// The settings window on the About page, the only pane the sidebar does
    /// not list.
    @objc
    private func showAbout(_: Any!) {
        Self.logger.debug("show about")
        openSettings(on: .about)
    }

    /// The menu's stand-ins for the global chords: when a session holds
    /// `shortcutTarget`, each row dispatches through the coordinator so the
    /// armed session updates its candidate bar, layout, and floating panels.
    /// When no session is armed (`shortcutTarget == nil`), the click still
    /// writes the setting (and flashes the HUD for mode switches), matching
    /// Windows (`lang_bar.rs`) and Linux (`chrome.rs`), while Carbon hotkeys
    /// remain strictly session-scoped.
    @objc
    private func toggleRomanization(_: Any!) {
        performGlobalShortcut(.toggleRomanization)
    }

    @objc
    private func toggleTps(_: Any!) {
        performGlobalShortcut(.toggleTps)
    }

    @objc
    private func cycleCandidateDisplayMode(_: Any!) {
        performGlobalShortcut(.cycleCandidateDisplayMode)
    }

    @objc
    private func showTpsKeyboard(_: Any!) {
        performGlobalShortcut(.showTpsKeyboard)
    }

    private func performGlobalShortcut(_ action: ShortcutAction) {
        Self.logger.debug("menu shortcut \(action)")
        onMainActor(nil) { controller, _ in
            if ComposingSessionCoordinator.shared.performShortcutAction(action) {
                return
            }
            controller.performUnarmedMenuAction(action)
        }
    }

    @MainActor
    private func performUnarmedMenuAction(_ action: ShortcutAction) {
        guard !action.isInert(under: settings.inputMode) else { return }
        switch action {
        case .toggleRomanization:
            guard let mode = settings.switchInputMode(.toggleRomanization) else { return }
            flash(mode.displayNameKey)
        case .toggleTps:
            guard let mode = settings.switchInputMode(.toggleTps) else { return }
            flash(mode.displayNameKey)
        case .cycleCandidateDisplayMode:
            let next = settings.candidateDisplayMode.next
            settings.candidateDisplayMode = next
            flash(next.displayNameKey)
        case .showTpsKeyboard:
            settings.isTpsKeyboardShown.toggle()
        // None of these route through `performGlobalShortcut` (`openLastSettingsPane`
        // uses `showPreferences:`, and the other three have no menu row). If a row
        // is added for one later, add its unarmed handling here too.
        case .toggleTranslateSwapped, .showTelexGuide, .showSymbolPicker, .openLastSettingsPane:
            break
        }
    }
}
