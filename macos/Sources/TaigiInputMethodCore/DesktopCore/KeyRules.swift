// The Shortcuts pane's and the symbol picker's key rules, asked of desktop-core.

import AppKit

/// desktop-core's chord gate, recorder decision, resolved composing rows and
/// symbol-picker reading, under the Mac's grammar — the key-rule requests of
/// the shell seam (`macos/crates/taigi-macos-ffi/src/key_rules.rs`; roadmap
/// P14). Each answer is nil when the seam fails or answers what this side
/// cannot read (logged). `press` and `chord` read no settings; the pane's
/// rows and the picker's keys are read under the given defaults, through the
/// runtime's settings whitelist — nil before `Configure`.
enum KeyRules {
    /// What a key press does to a shortcut-recording field.
    enum Press: Equatable {
        /// Bound; recording ends.
        case recorded(ComposingKeyChord)
        /// Turned down; recording goes on, the reason in place of the prompt.
        case refused(ComposingKeyChord.Rejection)
        /// The Escape key: the field gives up focus.
        case blur
        /// A bare Backspace, Delete or forward Delete: the field blanks.
        case blank
    }

    private static let logger = DebugLogger(category: "KeyRules")

    /// What `key` records in a recording field, or why not.
    static func press(_ key: KeyEventSnapshot) -> Press? {
        var request = Taigi_DesktopShell_PressRequest()
        request.event = Taigi_DesktopShell_KeyEvent(key)
        let reply = DesktopCoreBridge.roundtrip(.press(request), op: "press") {
            if case let .press(reply) = $0 {
                reply
            } else {
                nil
            }
        }
        return decoded(reply, op: "press") { reply -> Press? in
            switch reply.outcome {
            case let .recorded(chord)?: .recorded(ComposingKeyChord(chord))
            case let .refused(reason)?: ComposingKeyChord.Rejection(reason).map(Press.refused)
            case .action(.blur)?: .blur
            case .action(.blank)?: .blank
            default: nil
            }
        }
    }

    /// The chord `key` and `modifiers` make, or why they make none — the gate
    /// a recorded chord passes. Only ⌘ ⌃ ⌥ ⇧ are read.
    static func chord(
        key: String?,
        modifiers: NSEvent.ModifierFlags,
    ) -> Result<ComposingKeyChord, ComposingKeyChord.Rejection>? {
        var request = Taigi_DesktopShell_ChordRequest()
        if let key {
            request.key = key
        }
        request.modifierFlags = UInt64(modifiers.rawValue)
        let reply = DesktopCoreBridge.roundtrip(.chord(request), op: "chord") {
            if case let .chord(reply) = $0 {
                reply
            } else {
                nil
            }
        }
        return decoded(reply, op: "chord") { reply -> Result<ComposingKeyChord, ComposingKeyChord.Rejection>? in
            switch reply.answer {
            case let .chord(chord)?: .success(ComposingKeyChord(chord))
            case let .rejection(reason)?: ComposingKeyChord.Rejection(reason).map { .failure($0) }
            case nil: nil
            }
        }
    }

    /// The Shortcuts pane's composing rows as `userDefaults` holds them, and
    /// its fixed rows.
    static func composingShortcuts(in userDefaults: UserDefaults) -> ComposingShortcuts? {
        guard let settings = snapshot(in: userDefaults, op: "composingShortcuts") else { return nil }
        let reply = DesktopCoreBridge.roundtrip(
            .composingShortcuts(Taigi_DesktopShell_ComposingShortcutsRequest()),
            settings: settings,
            op: "composingShortcuts",
        ) {
            if case let .composingShortcuts(reply) = $0 {
                reply
            } else {
                nil
            }
        }
        return decoded(reply, op: "composingShortcuts", ComposingShortcuts.init)
    }

    /// What `key` means while the symbol picker is up, under the bindings
    /// `userDefaults` holds.
    static func symbolPickerIntent(for key: KeyEventSnapshot, in userDefaults: UserDefaults) -> SymbolPickerIntent? {
        guard let settings = snapshot(in: userDefaults, op: "symbolPickerKey") else { return nil }
        var request = Taigi_DesktopShell_SymbolPickerKeyRequest()
        request.event = Taigi_DesktopShell_KeyEvent(key)
        let reply = DesktopCoreBridge.roundtrip(.symbolPickerKey(request), settings: settings, op: "symbolPickerKey") {
            if case let .symbolPicker(reply) = $0 {
                reply
            } else {
                nil
            }
        }
        return decoded(reply, op: "symbolPickerKey") { reply -> SymbolPickerIntent? in
            switch reply.intent {
            case .action(.close)?: .close
            case .action(.confirm)?: .confirm
            case .action(.closeAndPassThrough)?: .closeAndPassThrough
            case let .navigate(direction)?: CandidateNavigation(direction).map(SymbolPickerIntent.navigate)
            case let .pickSlot(slot)?: .pickSlot(Int(slot))
            default: nil
            }
        }
    }

    /// Where `request` moves the stored mode, by desktop-core's one writer,
    /// given the two stored values it reads (`""` when absent): the mode to
    /// store, and the romanization to come back to when the switch stored a
    /// new one. Reads no snapshot, so it answers before `Configure` too.
    static func switchInputMode(
        _ request: InputModeSwitch,
        inputMode: String,
        lastRomanizationMode: String,
    ) -> (inputMode: InputMode, lastRomanizationMode: InputMode?)? {
        var message = Taigi_DesktopShell_SwitchInputModeRequest()
        message.inputMode = inputMode
        message.lastRomanizationMode = lastRomanizationMode
        switch request {
        case let .pick(mode): message.pick = mode.rawValue
        case .toggleRomanization: message.toggleRomanization = true
        case .toggleTps: message.toggleTps = true
        }
        let reply = DesktopCoreBridge.roundtrip(.switchInputMode(message), op: "switchInputMode") {
            if case let .switchInputMode(reply) = $0 {
                reply
            } else {
                nil
            }
        }
        return decoded(reply, op: "switchInputMode") { reply -> (InputMode, InputMode?)? in
            guard let mode = InputMode(rawValue: reply.inputMode) else { return nil }
            guard reply.hasLastRomanizationMode else { return (mode, nil) }
            guard let last = InputMode(rawValue: reply.lastRomanizationMode), last != .tps else { return nil }
            return (mode, last)
        }
    }

    /// The on-screen TPS key panel's rows, built by desktop-core from its TPS
    /// layout table (desktop TPS roadmap D6) — no Swift copy of the glyphs.
    /// Reads no snapshot, so it answers before `Configure` too; nil (logged)
    /// when the bridge gave no reply.
    static func tpsKeyboardRows() -> [Taigi_DesktopShell_TpsKeyboardRow]? {
        let request = Taigi_DesktopShell_TpsKeyboardRowsRequest()
        let reply = DesktopCoreBridge.roundtrip(.tpsKeyboardRows(request), op: "tpsKeyboardRows") {
            if case let .tpsKeyboardRows(reply) = $0 {
                reply
            } else {
                nil
            }
        }
        return reply?.rows
    }

    /// `reply` read by `decode`: nil when there is no reply (the bridge
    /// logged why) or it holds what this side cannot read (logged here).
    private static func decoded<Reply, Value>(
        _ reply: Reply?,
        op: String,
        _ decode: (Reply) -> Value?,
    ) -> Value? {
        guard let reply else { return nil }
        guard let value = decode(reply) else {
            logger.error("[\(op)] an answer this side does not know")
            return nil
        }
        return value
    }

    /// The whitelisted settings as `userDefaults` holds them; nil — logged —
    /// before the runtime exists.
    private static func snapshot(in userDefaults: UserDefaults, op: String) -> Taigi_DesktopShell_SettingsSnapshot? {
        guard let runtime = DesktopCoreRuntime.configured else {
            logger.error("[\(op)] no desktop-core runtime — no settings to read under")
            return nil
        }
        return runtime.settingsSnapshot(in: userDefaults)
    }
}

extension ComposingShortcuts {
    /// The core's reply, or nil for a row this side cannot draw.
    init?(_ reply: Taigi_DesktopShell_ComposingShortcutsReply) {
        var rows: [Row] = []
        for action in reply.actions {
            guard let label = StringKey(rawValue: action.labelKey), action.fields.count == 2 else { return nil }
            let fields = action.fields.map { field in
                Row.Field(
                    settingsKey: field.settingsKey,
                    chord: field.hasChord ? ComposingKeyChord(field.chord) : nil,
                    shippedChord: field.hasShippedChord ? ComposingKeyChord(field.shippedChord) : nil,
                    storedChord: field.hasStoredChord ? ComposingKeyChord(field.storedChord) : nil,
                )
            }
            rows.append(Row(
                name: action.name,
                label: label,
                group: Int(action.group),
                fields: fields,
            ))
        }
        self.init(
            rows: rows,
            slotKeys: reply.slotKeys,
            shiftedSlotKeys: reply.shiftedSlotKeys,
            navigationKeys: reply.navigationKeys,
            caretChords: reply.caretChords,
            cancelKey: reply.cancelKey,
        )
    }
}
