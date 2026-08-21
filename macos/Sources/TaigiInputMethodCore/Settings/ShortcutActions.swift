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
    static let toggleRomanization = Self("toggleRomanization")
    static let toggleTranslateSwapped = Self("toggleTranslateSwapped")
}

/// One user-assignable action. The list is the single source for the recorder
/// rows, the handler registration, and the enable/disable gate — adding a case
/// adds the action everywhere at once.
enum ShortcutAction: CaseIterable, Sendable {
    case openSettings
    case toggleRomanization
    case toggleTranslateSwapped

    /// The roster split into the groups the input-source menu draws: opening
    /// the settings window, and the two switches a user flips mid-sentence.
    ///
    /// Written out rather than derived from `allCases` order so that adding a
    /// case has to say which group it belongs to — `ShortcutActionsTests` pins
    /// that every case appears exactly once.
    static let groups: [[ShortcutAction]] = [
        [.openSettings],
        [.toggleRomanization, .toggleTranslateSwapped],
    ]

    var name: KeyboardShortcuts.Name {
        switch self {
        case .openSettings: .openSettings
        case .toggleRomanization: .toggleRomanization
        case .toggleTranslateSwapped: .toggleTranslateSwapped
        }
    }

    /// The recorder row's label, under the active display language.
    ///
    /// Each row is authored whole rather than composed from the label of the setting it flips:
    /// those labels are verb phrases, and wrapping one in a "toggle X" frame reads wrong in
    /// every language — `漢羅対調を切り替える` doubles the verb.
    @MainActor
    func label(_ language: DisplayLanguageStore) -> String {
        switch self {
        case .openSettings: language.string(.macosShortcutOpenSettings)
        case .toggleRomanization: language.string(.macosShortcutToggleRomanization)
        case .toggleTranslateSwapped: language.string(.macosShortcutToggleTranslateSwapped)
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

    /// The coordinator calls this as sessions come and go; see
    /// `ComposingSessionCoordinator.registerShortcutTarget`.
    static func setEnabled(_ isEnabled: Bool) {
        let names = ShortcutAction.allCases.map(\.name)
        if isEnabled {
            KeyboardShortcuts.enable(names)
        } else {
            KeyboardShortcuts.disable(names)
        }
    }

    /// Opening settings is process-wide and needs no session; everything else
    /// changes what the active composition renders, so it routes through the
    /// controller that owns the session — which also dismisses the candidate
    /// bar those settings would invalidate.
    static func perform(_ action: ShortcutAction) {
        switch action {
        case .openSettings:
            SettingsWindowController.shared.show()
        case .toggleRomanization, .toggleTranslateSwapped:
            ComposingSessionCoordinator.shared.performShortcutAction(action)
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

    /// The impure half: called from each recorder's `onChange`.
    @MainActor
    static func resolve(after changed: ShortcutAction) {
        let losers = conflictingActions(with: changed) { action in
            KeyboardShortcuts.getShortcut(for: action.name)
        }
        for loser in losers {
            KeyboardShortcuts.setShortcut(nil, for: loser.name)
        }
    }
}
