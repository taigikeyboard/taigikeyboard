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
    /// ⌃⌘ plus a letter is what a Taiwanese input method puts its mid-sentence
    /// switches on: vChewing binds every one of its toggles that way
    /// (`references/vChewing-macOS/Packages/vChewing_MainAssembly4Darwin/Sources/MainAssembly4Darwin/SessionController/IMEMenuSputnik.swift:108-293`),
    /// and McBopomofo's 簡繁轉換 is ⌃⌘G with 半形標點 on ⌃⌘H
    /// (`references/McBopomofo/Source/InputMethodController.swift:74-81`). R for
    /// the romanization, H for the Hanji the 漢羅 switch swaps in.
    static let toggleRomanization = Self(
        "toggleRomanization",
        initial: .init(.r, modifiers: [.control, .command]),
    )
    static let toggleTranslateSwapped = Self(
        "toggleTranslateSwapped",
        initial: .init(.h, modifiers: [.control, .command]),
    )
}

/// One user-assignable action. The list is the single source for the recorder
/// rows, the handler registration, and the enable/disable gate — adding a case
/// adds the action everywhere at once.
enum ShortcutAction: CaseIterable, Sendable {
    case openSettings
    case toggleRomanization
    case toggleTranslateSwapped

    var name: KeyboardShortcuts.Name {
        switch self {
        case .openSettings: .openSettings
        case .toggleRomanization: .toggleRomanization
        case .toggleTranslateSwapped: .toggleTranslateSwapped
        }
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
