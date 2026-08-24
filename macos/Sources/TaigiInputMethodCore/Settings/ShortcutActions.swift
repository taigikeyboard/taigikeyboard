// The user-configurable shortcuts: which actions exist, and how their global
// hotkeys are registered, gated, and kept from colliding with each other.

import AppKit
import KeyboardShortcuts

extension KeyboardShortcuts.Name {
    /// The pane doorways, on ⌃⇧ plus the pane's position in the sidebar.
    ///
    /// One modifier family, so every key that lands in this input method's
    /// settings reads as one namespace. Digits rather than initials because the
    /// settings window speaks five display languages and an initial is a
    /// mnemonic in exactly one of them; the sidebar's own order is the same in
    /// all five.
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
    case openGeneralPane
    case openAppearancePane
    case openShortcutPane
    case openCustomDictionaryPane
    case openDictionarySourcesPane
    case toggleRomanization
    case toggleTranslateSwapped

    var name: KeyboardShortcuts.Name {
        switch self {
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
        case .openGeneralPane: .pane(.general)
        case .openAppearancePane: .pane(.appearance)
        case .openShortcutPane: .pane(.shortcuts)
        case .openCustomDictionaryPane: .pane(.customDictionary)
        case .openDictionarySourcesPane: .pane(.dictionarySources)
        case .toggleRomanization: .named(.macosShortcutToggleRomanization)
        case .toggleTranslateSwapped: .named(.macosShortcutToggleTranslateSwapped)
        }
    }

    /// The pane this action lands the settings window on, or `nil` for the two
    /// switches, which open no window at all.
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
        case .openGeneralPane, .openAppearancePane, .openShortcutPane,
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

    // MARK: - Across the two registries

    /// The composing chord a global shortcut occupies, or nil when the two
    /// cannot be compared.
    ///
    /// The bridge between the registries. A global shortcut is a Carbon key
    /// CODE; a composing chord is the CHARACTER that key types with no
    /// modifiers held. `nsMenuItemKeyEquivalent` is the library's own
    /// translation between them — through the current ASCII-capable layout for
    /// ordinary keys, and through AppKit's key equivalents for the named ones
    /// (`Shortcut.swift:689-752`), which is what makes Return, Tab and the
    /// backtick — the keys this seam is actually about — comparable.
    ///
    /// A layout comparison, not a physical-key one: two keys a custom layout
    /// maps to the same character read as the same chord here, which matches
    /// how the composing tier identifies its keys in the first place.
    ///
    /// Built through the same gate a binding passes, so a chord the gate
    /// REFUSES answers nil — and correctly so: every chord a composing action
    /// can hold went through `make` (the recorder, and `init?(rawValue:)` on
    /// the way back out of storage), so a chord `make` rejects is one no
    /// binding can hold, and therefore one nothing can collide with.
    ///
    /// Nil also when the library cannot name the key at all. Either way nil
    /// means "no conflict found", never "clear something": wrongly emptying a
    /// row the user can see is worse than leaving an undetectable collision on
    /// a key neither tier can hold a binding on.
    @MainActor
    static func composingChord(occupiedBy shortcut: KeyboardShortcuts.Shortcut) -> ComposingKeyChord? {
        try? ComposingKeyChord.make(
            key: shortcut.key.flatMap { namedKeyCharacters[$0] } ?? shortcut.nsMenuItemKeyEquivalent,
            modifiers: shortcut.modifiers,
        ).get()
    }

    /// Every key the library does not report as the character this side
    /// stores, and what it types here instead.
    ///
    /// Two families need translating. The keys AppKit names come back as
    /// DISPLAY GLYPHS — `↩` for Return, `⇥` for Tab, `↖` for Home
    /// (`Shortcut.swift:567-600`) — where a composing chord holds what the key
    /// types (`\r`, `\t`) or the private-use scalar AppKit names it with. The
    /// number pad comes back as nil, deliberately, because no SwiftUI key
    /// equivalent can address it (`Shortcut.swift:527-564`) — but a keypad key
    /// still TYPES the digit or operator on its face, and neither tier keeps
    /// the `.numericPad` flag that would tell it apart, so `⌃`-keypad-3 and
    /// `⌃3` are one chord as far as the candidate slots are concerned.
    ///
    /// The reserved keys — the arrows, the paging keys, Escape and the two
    /// deletes — are here too, though no binding can hold one. Translating
    /// them is what lets the gate RECOGNISE them: fed the glyph, `make` would
    /// happily build a `⎋`-the-character chord that matches nothing; fed the
    /// scalar, it refuses, and the bridge answers nil, which is the honest
    /// "nothing can collide here". Space and the function keys need no row —
    /// the library already reports those as the character and the
    /// `NSF1FunctionKey`-style scalars this side stores.
    /// `CrossTierShortcutConflictTests` pins both sides of that split, so a
    /// key that changes sides fails a test rather than going quiet.
    ///
    /// Without this the bridge would miss the collisions it exists for: four of
    /// the eight composing defaults are Return chords, and the slot tier is
    /// nine digits.
    private static let namedKeyCharacters: [KeyboardShortcuts.Key: String] = {
        var characters: [KeyboardShortcuts.Key: String] = [
            .return: "\r",
            .keypadEnter: "\r",
            .tab: "\t",
            .escape: "\u{1B}",
            .delete: "\u{8}",
            .deleteForward: "\u{7F}",
            .keypad0: "0", .keypad1: "1", .keypad2: "2", .keypad3: "3", .keypad4: "4",
            .keypad5: "5", .keypad6: "6", .keypad7: "7", .keypad8: "8", .keypad9: "9",
            .keypadDecimal: ".", .keypadDivide: "/", .keypadEquals: "=",
            .keypadMinus: "-", .keypadMultiply: "*", .keypadPlus: "+",
        ]
        let namedFunctionKeys: [(KeyboardShortcuts.Key, Int)] = [
            (.home, NSHomeFunctionKey), (.end, NSEndFunctionKey), (.help, NSHelpFunctionKey),
            (.keypadClear, NSClearLineFunctionKey),
            (.leftArrow, NSLeftArrowFunctionKey), (.rightArrow, NSRightArrowFunctionKey),
            (.upArrow, NSUpArrowFunctionKey), (.downArrow, NSDownArrowFunctionKey),
            (.pageUp, NSPageUpFunctionKey), (.pageDown, NSPageDownFunctionKey),
        ]
        for (key, functionKey) in namedFunctionKeys {
            guard let scalar = UnicodeScalar(functionKey) else { continue }
            characters[key] = String(scalar)
        }
        return characters
    }()

    /// Which composing actions hold the chord `shortcut` occupies.
    ///
    /// The pure half of "a global recording takes the key from the composing
    /// tier". Both halves of the seam are asked the same way the intra-tier
    /// rules are (`conflictingActions`, `ComposingKeyBindings.actionsHolding`),
    /// so the pane resolves every collision by one rule: last writer wins, and
    /// the loser's row empties in front of the user.
    @MainActor
    static func composingActionsHolding(
        _ shortcut: KeyboardShortcuts.Shortcut,
        in bindings: ComposingKeyBindings,
    ) -> [ComposingAction] {
        guard let chord = composingChord(occupiedBy: shortcut) else { return [] }
        return bindings.actionsHolding(chord)
    }

    /// Which global actions hold `chord` — the same question from the other
    /// side, for a composing recording or a slot-modifier change.
    @MainActor
    static func globalActionsHolding(_ chord: ComposingKeyChord) -> [ShortcutAction] {
        globalActionsHolding(where: { $0 == chord })
    }

    /// The scan both of those are: bridge what each global action holds, and
    /// keep the ones whose chord answers `predicate`. A family of chords a
    /// future setting claims is a new predicate here, not a third near-copy.
    @MainActor
    static func globalActionsHolding(
        where predicate: (ComposingKeyChord) -> Bool,
        // The one spelling of "what the registry holds now". A stored constant
        // cannot carry it: `getShortcut` is main-actor isolated, and only a
        // default argument may call it from this position.
        shortcutFor: (ShortcutAction) -> KeyboardShortcuts.Shortcut? = {
            KeyboardShortcuts.getShortcut(for: $0.name)
        },
    ) -> [ShortcutAction] {
        ShortcutAction.allCases.filter { action in
            guard let shortcut = shortcutFor(action),
                  let chord = composingChord(occupiedBy: shortcut)
            else { return false }
            return predicate(chord)
        }
    }

    /// Which global actions hold one of the nine candidate-slot chords under
    /// `slotModifier`.
    ///
    /// The slot tier is a picker rather than a row, so it cannot lose a chord —
    /// but it CAN take one, when the user switches the modifier onto chords a
    /// global shortcut already holds. That makes the picker the last writer,
    /// and these are the rows that empty. The recorder refuses the other order
    /// (`ShortcutSettingsView`), so between them no global shortcut can sit on
    /// a live slot chord.
    /// Whether `shortcut` sits on one of the nine slot chords — the question
    /// the recorder refuses on and the launch pass clears on, asked the same
    /// way in both so the two cannot drift.
    @MainActor
    static func isSlotChord(
        _ shortcut: KeyboardShortcuts.Shortcut,
        under slotModifier: CandidateSlotModifier,
    ) -> Bool {
        composingChord(occupiedBy: shortcut)?.isCandidateSlotChord(under: slotModifier) == true
    }

    @MainActor
    static func globalActionsHoldingSlotChords(
        under slotModifier: CandidateSlotModifier,
    ) -> [ShortcutAction] {
        globalActionsHolding(where: { $0.isCandidateSlotChord(under: slotModifier) })
    }

    /// A global recording just landed: take the chord off any composing row
    /// that held it.
    @MainActor
    static func resolveComposingRows(after changed: ShortcutAction, in store: SettingsStore) {
        guard let shortcut = KeyboardShortcuts.getShortcut(for: changed.name) else { return }
        for loser in composingActionsHolding(shortcut, in: store.composingKeyBindings) {
            store.setComposingChord(nil, for: loser)
        }
    }

    /// A composing recording just landed: take the chord off any global row
    /// that held it.
    @MainActor
    static func resolveGlobalRows(after chord: ComposingKeyChord) {
        clear(globalActionsHolding(chord))
    }

    /// The slot modifier just changed: take the nine slot chords off any
    /// global row that held one.
    @MainActor
    static func resolveGlobalRows(afterSlotModifierChangedTo slotModifier: CandidateSlotModifier) {
        clear(globalActionsHoldingSlotChords(under: slotModifier))
    }

    /// Reconciles the two registries at launch, where no recorder ran.
    ///
    /// Recording-beats-default, the rule the intra-tier pass above already
    /// applies: a chord a user chose outranks one an action merely shipped
    /// with. That is what carries an upgrade — a version that gives a global
    /// action a default chord a user had already put on a composing row must
    /// not silently kill that row, and the reverse holds too.
    ///
    /// When BOTH sides are user recordings there is nothing honest to compare:
    /// the stored values carry no timestamps, so the last writer is unknowable.
    /// The global tier wins that tie, because it is the tier that actually
    /// fires — Carbon dispatches before the classifier ever runs — so the
    /// alternative would be keeping a composing row that cannot work. A
    /// one-time deterministic tie-break, not a claim about who wrote last.
    @MainActor
    static func resolveAcrossRegistries(in store: SettingsStore) {
        let bindings = store.composingKeyBindings
        // Read once per action, not once per question — the same rule
        // `defaultsShadowedByRecordings` states above, and for the same
        // reason: every read goes to `UserDefaults`, and every bridged chord
        // goes to the keyboard layout. Both passes below ask about the same
        // seven actions.
        let held = ShortcutAction.allCases.compactMap { action in
            KeyboardShortcuts.getShortcut(for: action.name).flatMap { shortcut in
                composingChord(occupiedBy: shortcut).map { (action: action, shortcut: shortcut, chord: $0) }
            }
        }

        for (action, shortcut, chord) in held {
            let holders = bindings.actionsHolding(chord)
            guard !holders.isEmpty else { continue }

            let globalIsDefault = shortcut == action.defaultShortcut
            for holder in holders {
                // A chord the user chose outranks one an action merely shipped
                // with — and the composing side counts as chosen when it holds
                // anything other than its own default.
                let composingRecordingOutranks = globalIsDefault && chord != holder.defaultChord
                if composingRecordingOutranks {
                    KeyboardShortcuts.setShortcut(nil, for: action.name)
                } else {
                    store.setComposingChord(nil, for: holder)
                }
            }
        }

        // The slot tier last, and off the same snapshot: a live slot chord on
        // a global row is the one collision the recorder cannot refuse
        // retroactively, and clearing a row the loop already cleared is a
        // no-op.
        clear(
            held
                .filter { $0.chord.isCandidateSlotChord(under: bindings.slotModifier) }
                .map(\.action),
        )
    }
}
