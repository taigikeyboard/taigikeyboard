// The user-configurable shortcuts: which actions exist, and how their global
// hotkeys are registered, gated, and kept from colliding with each other.

import AppKit
import KeyboardShortcuts

extension KeyboardShortcuts.Name {
    /// The way back to the settings window from the keyboard.
    ///
    /// ⌃⇧1–⌃⇧5, one per sidebar pane, until 2026-08-25. They went together
    /// (USER): five chords is a lot to hold for panes a user visits about once
    /// a day, and the menu bar lists every one of them by name already.
    ///
    /// Not because ⌃⇧ was hard to reach — that was the first reading and the
    /// USER withdrew it the same day, finding the chord comfortable. What did
    /// not survive the second look was the VALUE: five keys, each opening a
    /// window the user is not in, against one that reopens where they were.
    ///
    /// One doorway replaces them, opening wherever the user left off. It is a
    /// command reached for rarely, so a mnemonic earns its keep here in a way
    /// it does not on a key pressed all day: S for Settings, and for the
    /// siat-tīng and settei the other display languages say. ⌘S belongs to the
    /// host — this input method never takes a bare Command chord — so ⌃⌘ is
    /// the family, the same one the switch below lives in. Apple's own list
    /// leaves ⌃⌘S alone (support.apple.com/en-us/102650), and the Command
    /// keeps it clear of the bare ⌃S that is XOFF in a terminal.
    ///
    /// Not the retired `openSettings` raw value, whatever the resemblance:
    /// `RetiredSettingsCleanup` clears that name on every launch, and reusing
    /// it would wipe the user's recorded chord each time.
    static let openLastSettingsPane = Self(
        "openLastSettingsPane",
        initial: .init(.s, modifiers: [.control, .command]),
    )

    /// ⌃⌘ plus a letter is what a Taiwanese input method puts its mid-sentence
    /// switches on: vChewing binds every one of its toggles that way
    /// (`references/vChewing-macOS/Packages/vChewing_MainAssembly4Darwin/Sources/MainAssembly4Darwin/SessionController/IMEMenuSputnik.swift:108-293`),
    /// and McBopomofo's 簡繁轉換 is ⌃⌘G with 半形標點 on ⌃⌘H
    /// (`references/McBopomofo/Source/InputMethodController.swift:74-81`).
    ///
    /// C, and not the R for "romanization" this had until 2026-08-25 (USER).
    /// A mnemonic is worth having on a command the user has to RECALL; this
    /// one is reached for all day, so it is muscle memory by the second day
    /// and what remains is how far the hand has to travel. ⌃, ⌘ and C are all
    /// on the bottom row: pinky, thumb and middle finger, hand never leaving
    /// position. R is two rows up with the pinky still anchored on ⌃. Apple
    /// put ⌘Z/⌘X/⌘C/⌘V on that row for the same reason.
    /// `ShortcutDefaultMigration` moves installs still on ⌃⌘R.
    static let toggleRomanization = Self(
        "toggleRomanization",
        initial: .init(.c, modifiers: [.control, .command]),
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

    /// The way round the 候選詞顯示 picker from the keyboard (USER 2026-09-02):
    /// the backtick above is inert outside 並排, so a user in 合用 or 羅馬字
    /// had no key that led back. H for Hàn-Lô, the thing being switched — a
    /// command reached for now and then, where a mnemonic pays. ⌃⌘ is the
    /// family the two switches above live in. McBopomofo's ⌃⌘H is its 半形標點
    /// — another input method's roster, not a system chord — and Apple's own
    /// list keeps ⌃⌘D (查字典), not H.
    ///
    /// This chord WAS the swap's default until 2026-08-21.
    /// `ShortcutDefaultMigration` moved those installs onto the backtick, and
    /// it runs before `ShortcutConflicts.resolveDefaultsShadowedByRecordings`,
    /// so an install that had not yet migrated is moved before this default
    /// could collide with it; one that recorded ⌃⌘H by hand keeps it, and this
    /// row empties — recording beats default, the standing rule.
    static let cycleCandidateDisplayMode = Self(
        "cycleCandidateDisplayMode",
        initial: .init(.h, modifiers: [.control, .command]),
    )

    /// The Telex key table, on demand (USER 2026-09-09): the legend under the
    /// 聲調拍法 picker was a wall of text in a pane the user is not in while
    /// typing, so it became a floating card any key dismisses
    /// (`TelexGuidePanel`). `/` is the key help lives on — `?` is ⇧/, and
    /// every app that answers "which keys do what" answers it there — and
    /// ⌃⌘ is the family the rest of this roster is on. Not ⌃⌘T, the mnemonic
    /// first reached for: JetBrains binds it to Surround With, and a user in
    /// an IDE would lose one or the other.
    static let showTelexGuide = Self(
        "showTelexGuide",
        initial: .init(.slash, modifiers: [.control, .command]),
    )
}

/// One user-assignable action. The list is the single source for the recorder
/// rows, the handler registration, and the enable/disable gate — adding a case
/// adds the action everywhere at once.
enum ShortcutAction: CaseIterable, Sendable {
    /// Opens the settings window on whichever pane the user left it on.
    case openLastSettingsPane
    case toggleRomanization
    case toggleTranslateSwapped
    /// Steps the 候選詞顯示 picker one place: 並排 → 合用 → 羅馬字 → 並排.
    case cycleCandidateDisplayMode
    /// Toggles the floating Telex key table. Last, because this order is the
    /// order of the rows in the pane, and a guide sits after the switches.
    case showTelexGuide

    var name: KeyboardShortcuts.Name {
        switch self {
        case .openLastSettingsPane: .openLastSettingsPane
        case .toggleRomanization: .toggleRomanization
        case .toggleTranslateSwapped: .toggleTranslateSwapped
        case .cycleCandidateDisplayMode: .cycleCandidateDisplayMode
        case .showTelexGuide: .showTelexGuide
        }
    }

    /// Whether this action opens the settings window rather than doing
    /// something to the composition in flight. The two need different
    /// dispatch: a window is process-wide, while a switch has to reach the
    /// session that owns the engine (`ShortcutHotkeys.perform`).
    var opensSettings: Bool { self == .openLastSettingsPane }

    /// The chord a fresh install has on this action, read back from the
    /// registry the library seeds itself from.
    ///
    /// Named here as well as on the `Name` so `ShortcutAction` stays the one
    /// place that knows everything about an action — and so the composing half
    /// (`ComposingAction.defaultChord`) has a sibling with the same name.
    var defaultShortcut: KeyboardShortcuts.Shortcut? { name.initialShortcut }

    /// The recorder row's label, under the active display language.
    ///
    /// Authored whole rather than composed from the label of the setting it
    /// flips: those labels are verb phrases, and wrapping one in a "toggle X"
    /// frame reads wrong in every language — `漢羅対調を切り替える` doubles the
    /// verb.
    @MainActor
    func label(_ language: DisplayLanguageStore) -> String {
        switch self {
        case .openLastSettingsPane: language.string(.desktopShortcutOpenSettings)
        case .toggleRomanization: language.string(.desktopShortcutToggleRomanization)
        case .toggleTranslateSwapped: language.string(.desktopShortcutToggleTranslateSwapped)
        case .cycleCandidateDisplayMode: language.string(.desktopShortcutCycleCandidateDisplayMode)
        case .showTelexGuide: language.string(.desktopShortcutShowTelexGuide)
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
    ///
    /// Session scope is what makes a BARE key safe to register here — and a
    /// bare key is bindable (USER 2026-08-26, `ShortcutKeyRecorder`). A Carbon
    /// hotkey sits above this input method's own key path, so a bare `z` would
    /// otherwise eat a `z` the user meant to type. It cannot: the only state
    /// in which the user types plain letters is the system's own ABC input
    /// source, which this input method has no mode of its own for (USER
    /// 2026-08-26) — reaching it means SWITCHING input sources, which ends
    /// this session and so disables every name below before the first letter
    /// arrives.
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
    /// bar those settings would invalidate. The guide goes the same way: it
    /// is owned by the session that raised it, so its key path can take it
    /// down.
    static func perform(_ action: ShortcutAction) {
        switch action {
        case .openLastSettingsPane:
            // `nil`, so the window comes back where the user left it. Which
            // pane that is belongs to the settings window, not to a chord —
            // the named panes are reached from the menu bar now.
            openSettings(on: nil, in: SettingsStore())
        case .toggleRomanization, .toggleTranslateSwapped, .cycleCandidateDisplayMode, .showTelexGuide:
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
        // The guide comes down first, whoever raised it: this path never
        // reaches the session, and the settings window taking focus is not
        // guaranteed to end the session that owns the card.
        TelexGuidePanel.shared.hideNow()
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
        try? translation(of: shortcut).get()
    }

    /// The bridge with its refusal kept: the launch pass reads WHY a global
    /// row failed to translate, because a row on a typing key is one the
    /// recorder would refuse today and Carbon would still dispatch first.
    @MainActor
    static func translation(of shortcut: KeyboardShortcuts.Shortcut)
        -> Result<ComposingKeyChord, ComposingKeyChord.Rejection>
    {
        ComposingKeyChord.make(
            key: shortcut.key.flatMap { namedKeyCharacters[$0] } ?? shortcut.nsMenuItemKeyEquivalent,
            modifiers: shortcut.modifiers,
        )
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
    /// `⌃3` are one chord.
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
    /// Without this the bridge would miss the collisions it exists for: the
    /// composing roster keeps its commit keys in the Return family.
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
    /// side, for a composing recording.
    @MainActor
    static func globalActionsHolding(_ chord: ComposingKeyChord) -> [ShortcutAction] {
        ShortcutAction.allCases.filter { action in
            guard let shortcut = KeyboardShortcuts.getShortcut(for: action.name) else { return false }
            return composingChord(occupiedBy: shortcut) == chord
        }
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
        // goes to the keyboard layout.
        let recorded = ShortcutAction.allCases.compactMap { action in
            KeyboardShortcuts.getShortcut(for: action.name).map { (action: action, shortcut: $0) }
        }
        let held = recorded.compactMap { action, shortcut in
            composingChord(occupiedBy: shortcut).map { (action: action, shortcut: shortcut, chord: $0) }
        }

        // A global row on a key the gate refuses as a typing key — a bare
        // `z` or `q` recorded while the eight non-syllable letters were
        // bindable (before 2026-09-08), or a `⇧3` — is not a collision to
        // compare: it is a row that predates the refusal, and Carbon would
        // dispatch it before the classifier ever saw the Telex key or the
        // slot key it now types. Cleared, the way the recorder would have
        // refused it. `reservedKey` rows cannot exist (the arrows and the
        // deletes were never recordable); only the typing-key refusal names
        // an upgrade path.
        clear(
            recorded
                .filter { translation(of: $0.shortcut) == .failure(.typesRomanization) }
                .map(\.action),
        )

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
    }
}
