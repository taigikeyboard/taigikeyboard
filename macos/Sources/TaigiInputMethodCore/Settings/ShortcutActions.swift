// The user-configurable shortcuts: which actions exist, and how their global
// hotkeys are registered, gated, and kept from colliding with each other.

import KeyboardShortcuts

extension KeyboardShortcuts.Name {
    /// `initial:` carries the chord PR5 shipped hardcoded, so a user who never
    /// opens the recorder keeps `Ctrl+Shift+,` exactly as before.
    static let openSettings = Self(
        "openSettings",
        initial: .init(.comma, modifiers: [.control, .shift]),
    )
    /// The pane doorways, on ⌃⇧ plus the pane's position in the sidebar.
    ///
    /// One modifier family with 開啟設定 above, so every key that lands in this
    /// input method's settings reads as one namespace. Digits rather than
    /// initials because the settings window speaks five display languages and
    /// an initial is a mnemonic in exactly one of them; the sidebar's own
    /// order is the same in all five.
    ///
    /// ⌃⇧ rather than the neighbouring families: ⌃1–9 is the candidate-slot
    /// tier while Control holds it (`ComposingKeyIntent.directSelectionSlot`),
    /// ⌘1–5 is the host application's own tab switching, ⌥ plus a digit types
    /// a symbol on several layouts and is the other slot modifier a user can
    /// choose, and ⌃⌘ plus a key is where the mid-sentence switches live.
    /// Shift is also what keeps these off the slot tier for good: that tier
    /// refuses any chord carrying a modifier it was not bound to
    /// (`ComposingKeyIntent.swift:215-218`).
    static let openGeneralPane = Self(
        "openGeneralPane",
        initial: .init(.one, modifiers: [.control, .shift]),
    )
    static let openAppearancePane = Self(
        "openAppearancePane",
        initial: .init(.two, modifiers: [.control, .shift]),
    )
    static let openShortcutPane = Self(
        "openShortcutPane",
        initial: .init(.three, modifiers: [.control, .shift]),
    )
    static let openCustomDictionaryPane = Self(
        "openCustomDictionaryPane",
        initial: .init(.four, modifiers: [.control, .shift]),
    )
    static let openDictionarySourcesPane = Self(
        "openDictionarySourcesPane",
        initial: .init(.five, modifiers: [.control, .shift]),
    )

    /// ⌃⌘ plus a letter is what a Taiwanese input method puts its mid-sentence
    /// switches on: vChewing binds every one of its toggles that way
    /// (`references/vChewing-macOS/Packages/vChewing_MainAssembly4Darwin/Sources/MainAssembly4Darwin/SessionController/IMEMenuSputnik.swift:108-293`),
    /// and McBopomofo's 簡繁轉換 is ⌃⌘G with 半形標點 on ⌃⌘H
    /// (`references/McBopomofo/Source/InputMethodController.swift:74-81`). R for
    /// the romanization.
    static let toggleRomanization = Self(
        "toggleRomanization",
        initial: .init(.r, modifiers: [.control, .command]),
    )
    /// Bare backtick, the classic Taiwanese-IME function key (USER 2026-08-21):
    /// no TL or POJ syllable is spelled with it, and the hotkey is armed only
    /// while a Taigi session holds the engine, so it takes nothing from other
    /// input sources. `.backtick` is `kVK_ANSI_Grave` — a physical POSITION, so
    /// an ISO keyboard fires this from the key its layout puts there, whatever
    /// that key prints. The recorder cannot re-record a modifierless key
    /// (library validation), so a user who moves off this default has no UI
    /// path back to it — known gap, accepted 2026-08-21.
    ///
    /// `ShortcutDefaultMigration` moves installs still on the previous ⌃⌘H
    /// default onto this one; `initial:` alone only reaches installs that have
    /// never persisted the action.
    static let toggleTranslateSwapped = Self(
        "toggleTranslateSwapped",
        initial: .init(.backtick),
    )
}

/// One user-assignable action. The list is the single source for the recorder
/// rows, the handler registration, and the enable/disable gate — adding a case
/// adds the action everywhere at once.
enum ShortcutAction: CaseIterable, Sendable {
    case openSettings
    case openGeneralPane
    case openAppearancePane
    case openShortcutPane
    case openCustomDictionaryPane
    case openDictionarySourcesPane
    case toggleRomanization
    case toggleTranslateSwapped

    var name: KeyboardShortcuts.Name {
        switch self {
        case .openSettings: .openSettings
        case .openGeneralPane: .openGeneralPane
        case .openAppearancePane: .openAppearancePane
        case .openShortcutPane: .openShortcutPane
        case .openCustomDictionaryPane: .openCustomDictionaryPane
        case .openDictionarySourcesPane: .openDictionarySourcesPane
        case .toggleRomanization: .toggleRomanization
        case .toggleTranslateSwapped: .toggleTranslateSwapped
        }
    }

    /// What an action is for: a settings pane to land on, or a command with a
    /// name of its own. One table, because the pane an action opens and the
    /// words on its row are the same fact — two switches could disagree, and
    /// a row labelled 外觀 that opens 一般 would still compile.
    private enum Destination {
        case pane(SettingsPane)
        case named(StringKey)
    }

    private var destination: Destination {
        switch self {
        case .openSettings: .named(.macosShortcutOpenSettings)
        case .openGeneralPane: .pane(.general)
        case .openAppearancePane: .pane(.appearance)
        case .openShortcutPane: .pane(.shortcuts)
        case .openCustomDictionaryPane: .pane(.customDictionary)
        case .openDictionarySourcesPane: .pane(.dictionarySources)
        case .toggleRomanization: .named(.macosShortcutToggleRomanization)
        case .toggleTranslateSwapped: .named(.macosShortcutToggleTranslateSwapped)
        }
    }

    /// The pane this action lands the settings window on, or `nil` when it
    /// lands on none of them in particular.
    ///
    /// 開啟設定 is the `nil` case on purpose: it reopens wherever the user last
    /// was, which is Apple's guidance for a settings window and a different
    /// command from "go to 一般".
    var settingsPane: SettingsPane? {
        guard case let .pane(pane) = destination else { return nil }
        return pane
    }

    /// The chord a fresh install has on this action, read back from the
    /// registry the library seeds itself from.
    ///
    /// Named here as well as on the `Name` so `ShortcutAction` stays the one
    /// place that knows everything about an action — and so the composing half
    /// (`ComposingAction.defaultChord`) has a sibling with the same name.
    var defaultShortcut: KeyboardShortcuts.Shortcut? { name.initialShortcut }

    /// The recorder row's label, under the active display language.
    ///
    /// A pane doorway is named after the pane, in every surface that draws it: the sidebar row, the
    /// menu row and the recorder row all read 外觀, so a user who learns the name in one finds it in
    /// the others. A switch row is authored whole rather than composed from the label of the setting
    /// it flips: those labels are verb phrases, and wrapping one in a "toggle X" frame reads wrong
    /// in every language — `漢羅対調を切り替える` doubles the verb.
    @MainActor
    func label(_ language: DisplayLanguageStore) -> String {
        switch destination {
        case let .pane(pane): language.string(pane.labelKey)
        case let .named(key): language.string(key)
        }
    }
}

/// Registration and gating of the Carbon hotkeys behind the actions above.
///
/// The hotkeys are active only while a Taigi input session is — the same scope
/// the PR5 menu key equivalent had. A `RegisterEventHotKey` chord is otherwise
/// process-global: this input method's process stays alive after the user
/// switches to another input source, and an always-on chord would keep eating
/// a system-wide key combination on behalf of a keyboard that is not in use.
@MainActor
enum ShortcutHotkeys {
    /// Wires every action's handler once, and starts disabled: nothing is
    /// active until a session registers with the coordinator.
    ///
    /// Handlers go through `perform(_:)` so what a chord does is stated once,
    /// whether it ever grows a second caller or not.
    static func registerHandlers() {
        for action in ShortcutAction.allCases {
            KeyboardShortcuts.onKeyUp(for: action.name) { perform(action) }
        }
        setEnabled(false)
    }

    /// Built once: this runs on every focus change, and the roster never
    /// varies.
    private static let allNames = ShortcutAction.allCases.map(\.name)

    /// The coordinator calls this as sessions come and go; see
    /// `ComposingSessionCoordinator.registerShortcutTarget`.
    static func setEnabled(_ isEnabled: Bool) {
        if isEnabled {
            KeyboardShortcuts.enable(allNames)
        } else {
            KeyboardShortcuts.disable(allNames)
        }
    }

    /// Opening settings is process-wide and needs no session; everything else
    /// changes what the active composition renders, so it routes through the
    /// controller that owns the session — which also dismisses the candidate
    /// bar those settings would invalidate.
    static func perform(_ action: ShortcutAction) {
        switch action {
        case .openSettings, .openGeneralPane, .openAppearancePane, .openShortcutPane,
             .openCustomDictionaryPane, .openDictionarySourcesPane:
            openSettings(on: action.settingsPane, in: SettingsStore())
        case .toggleRomanization, .toggleTranslateSwapped:
            ComposingSessionCoordinator.shared.performShortcutAction(action)
        }
    }

    /// What every doorway into the settings window does, stated once for the
    /// hotkey above and for the menu rows that send the same commands.
    ///
    /// The pane is written BEFORE the window is asked to show, so an already
    /// open window moves to it too. `nil` leaves the stored pane alone, which
    /// is what reopens the window where the user left it.
    ///
    /// The store is a parameter rather than read here: the input controller
    /// owns one already, and a test drives it through its own suite instead of
    /// writing into the settings of whoever runs the tests.
    /// `show` is a seam for the tests, and the one place the shipped presenter
    /// is named: driving a menu command would otherwise order a real window in
    /// front of whoever is running them. `nil` is production.
    static func openSettings(
        on pane: SettingsPane?,
        in settings: SettingsStore,
        show: (@MainActor () -> Void)? = nil,
    ) {
        if let pane {
            settings.selectedSettingsPane = pane
        }
        if let show {
            show()
        } else {
            SettingsWindowController.shared.show()
        }
    }
}

/// Keeps one chord meaning one thing. `KeyboardShortcuts` happily lets two
/// names store the same shortcut, in which case BOTH handlers fire on one
/// keypress; recording a chord therefore clears it from any other action —
/// last writer wins, and the other recorder row visibly empties, which is how
/// the System Settings keyboard pane behaves.
enum ShortcutConflicts {
    /// The pure half: which other actions currently hold the same shortcut as
    /// `changed`. Separated so the policy is testable without touching the
    /// defaults domain the library stores shortcuts in.
    static func conflictingActions(
        with changed: ShortcutAction,
        shortcutFor: (ShortcutAction) -> KeyboardShortcuts.Shortcut?,
    ) -> [ShortcutAction] {
        guard let recorded = shortcutFor(changed) else { return [] }
        return ShortcutAction.allCases.filter { other in
            other != changed && shortcutFor(other) == recorded
        }
    }

    /// The pure half of the upgrade case: which actions hold a chord only
    /// because it is their default, while another action holds the same chord
    /// because the user recorded it there.
    ///
    /// An action is "on its default" when what it holds equals what it ships
    /// with. A user who recorded that same chord by hand is indistinguishable
    /// from one who never touched the row — and the outcome is the same either
    /// way, so nothing rests on telling them apart.
    static func defaultsShadowedByRecordings(
        shortcutFor: (ShortcutAction) -> KeyboardShortcuts.Shortcut?,
    ) -> [ShortcutAction] {
        // Read once per action rather than once per pair: the scan below asks
        // about every pair, and each read goes to `UserDefaults`.
        let held = ShortcutAction.allCases.map { (action: $0, shortcut: shortcutFor($0)) }
        let recorded = held.filter { $0.shortcut != nil && $0.shortcut != $0.action.defaultShortcut }
        return held
            .filter { $0.shortcut != nil && $0.shortcut == $0.action.defaultShortcut }
            .filter { row in recorded.contains { $0.shortcut == row.shortcut } }
            .map(\.action)
    }

    /// The impure half of it, run once at startup.
    @MainActor
    static func resolveDefaultsShadowedByRecordings() {
        clear(defaultsShadowedByRecordings { KeyboardShortcuts.getShortcut(for: $0.name) })
    }

    /// The impure half: called from each recorder's `onChange`.
    @MainActor
    static func resolve(after changed: ShortcutAction) {
        clear(conflictingActions(with: changed) { KeyboardShortcuts.getShortcut(for: $0.name) })
    }

    /// What losing a chord means, stated once for both resolutions.
    @MainActor
    private static func clear(_ losers: [ShortcutAction]) {
        for loser in losers {
            KeyboardShortcuts.setShortcut(nil, for: loser.name)
        }
    }
}
