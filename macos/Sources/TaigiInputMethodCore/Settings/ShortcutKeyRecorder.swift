// The control that records which key runs a shortcut, in either registry.

import AppKit
import Carbon.HIToolbox
import KeyboardShortcuts
import SwiftUI

/// One key press, in both the forms the 快捷鍵 pane's two registries store.
///
/// The composing registry stores the CHARACTER a key types unmodified; the
/// global one stores a Carbon key CODE. A press yields both at once, and only
/// the recording moment has the `NSEvent` the second is read from — asking for
/// it later would mean translating a character back into a key code through
/// whatever layout happened to be active.
struct RecordedShortcutKey {
    let chord: ComposingKeyChord
    /// Nil when the press carries no Carbon key code to store. A composing row
    /// does not care; a global row must refuse (`Rejection.notAGlobalKey`).
    let globalShortcut: KeyboardShortcuts.Shortcut?
}

/// What a GLOBAL row refuses on top of the shared gate.
///
/// Named rather than written inline at the row, so the policy is one testable
/// statement: the composing tier's extra refusal is `isCandidateSlotChord`,
/// and this is its opposite number.
enum GlobalShortcutPolicy {
    static func rejection(for key: RecordedShortcutKey) -> ComposingKeyChord.Rejection? {
        // A global row stores a Carbon key CODE. A press that yields none has
        // nothing to store, so it cannot be recorded here even though the
        // shared gate passed it.
        guard let shortcut = key.globalShortcut else { return .notAGlobalKey }
        // ⌘ with nothing but Shift beside it belongs to the application being
        // typed into: that is where a Mac puts its menu commands, and this
        // input method never takes one (`ShortcutActions`, on ⌃⌘S). The
        // library used to be the thing enforcing it — indirectly, by refusing
        // the modifier-less keys we now accept — so stating it is part of
        // owning the recorder. ⌃⌘ and ⌥⌘ are ours to offer; bare ⌘ is not.
        if key.chord.modifiers.contains(.command),
           key.chord.modifiers.isDisjoint(with: [.control, .option])
        {
            return .belongsToHost
        }
        // A hotkey never receives a chord the window server answers first, so
        // recording one would leave a row that reads as bound and does
        // nothing. This is the "unless it collides with a system shortcut"
        // half of USER 2026-08-26 — but narrowed twice, because the API behind
        // it is blunter than its name.
        //
        // `isTakenBySystem` asks `CopySymbolicHotKeys`, whose "enabled" table
        // carried 170 entries on the development Mac (probe, 2026-08-26) —
        // including BARE `a`, `s`, `f`, `q` and the bare backtick this very
        // action ships on. Those are slots, not shortcuts the user could name,
        // and a blanket refusal would have made the shipped default
        // unrecordable: exactly the one-way door this whole change exists to
        // remove. Upstream never blocks on it either; its own policy for a
        // system collision is `.warn`, a "Use Anyway" dialog this recorder has
        // no room for.
        //
        // So: only a chord that CARRIES a chording modifier is judged, which
        // is the shape a real system shortcut has — and never a chord one of
        // this app's own actions ships on, because a default the app hands out
        // has to be recordable or 恢復預設設定 would produce a row the recorder
        // itself rejects.
        guard !key.chord.modifiers.isDisjoint(with: [.command, .control, .option]) else { return nil }
        guard !isAShippedDefault(shortcut) else { return nil }
        return shortcut.isTakenBySystem ? .takenBySystem : nil
    }

    private static func isAShippedDefault(_ shortcut: KeyboardShortcuts.Shortcut) -> Bool {
        ShortcutAction.allCases.contains { $0.defaultShortcut == shortcut }
    }
}

/// A recording field for one shortcut row, composing tier or global.
///
/// An `NSSearchField` subclass because that is the shape a user has already
/// learnt from every other shortcut field on the Mac: focus drives recording,
/// the placeholder says what the field wants, and the cancel button clears the
/// row.
///
/// Both tiers, since 2026-08-26. The global rows used
/// `KeyboardShortcuts.Recorder` until then, and it refuses a modifier-less key
/// outright — `RecorderCocoa.swift:404-410` beeps and swallows the event
/// before any validation of ours runs — so a user could not put 漢羅代先 on a
/// bare `z` even though the action SHIPS on a bare backtick (USER 2026-08-26,
/// real device). One field for both tiers is also what the pane already
/// claims to be: one list, whose seam is not supposed to show.
///
/// Refusing is part of the job. A user who recorded `a` here would have no way
/// left to type the letter, so the keys a syllable is spelled with are turned
/// down — beep, and the reason in the placeholder — rather than accepted
/// (`ComposingKeyChord.make`). What each tier refuses ON TOP of that is
/// `additionalRejection`'s to say.
struct ShortcutKeyRecorder: NSViewRepresentable {
    /// The chord as stored, or nil for an empty row.
    let chord: ComposingKeyChord?
    /// Which keys currently hold the candidate slots, so a chord that tier
    /// would swallow can be refused rather than recorded and left inert.
    let slotKeySet: CandidateSlotKeySet
    /// Passed rather than read from the environment: this is an AppKit view,
    /// and the strings are resolved inside it.
    let language: DisplayLanguageStore
    /// The tier's own refusals, run after the shared gate passes. Nil accepts
    /// everything the gate does.
    var additionalRejection: ((RecordedShortcutKey) -> ComposingKeyChord.Rejection?)?
    /// Called with the recorded key, or nil when the user clears the row.
    let onRecord: (RecordedShortcutKey?) -> Void

    func makeNSView(context _: Context) -> ShortcutKeyRecorderField {
        let field = ShortcutKeyRecorderField()
        apply(to: field)
        field.chord = chord
        return field
    }

    func updateNSView(_ field: ShortcutKeyRecorderField, context _: Context) {
        apply(to: field)
        // Guarded: assigning redraws the field, and doing that on every SwiftUI
        // update would fight the row the user is recording into.
        if field.chord != chord {
            field.chord = chord
        }
    }

    private func apply(to field: ShortcutKeyRecorderField) {
        field.language = language
        field.slotKeySet = slotKeySet
        field.additionalRejection = additionalRejection
        field.onRecord = onRecord
    }
}

/// The field itself.
final class ShortcutKeyRecorderField: NSSearchField, NSSearchFieldDelegate {
    /// Upstream's width for the same control, so a section mixing the two does
    /// not step (`KeyboardShortcuts.RecorderCocoa`).
    private static let minimumWidth: Double = 130

    var language: DisplayLanguageStore?
    var slotKeySet: CandidateSlotKeySet = .bareKeys
    var additionalRejection: ((RecordedShortcutKey) -> ComposingKeyChord.Rejection?)?
    var onRecord: ((RecordedShortcutKey?) -> Void)?

    var chord: ComposingKeyChord? {
        didSet { renderChord() }
    }

    /// Why the last key press was turned down, shown in place of the prompt
    /// until the next one. Cleared on focus and on a successful recording.
    private var rejection: ComposingKeyChord.Rejection?

    private var eventMonitor: Any?
    private var cancelButtonCell: NSButtonCell?

    /// Refuses the initial key-view focus a window hands out when it opens or
    /// becomes key, without refusing the click or Tab that comes after — the
    /// same gate the global recorders run (`RecorderCocoa.canBecomeKeyView`).
    /// Without it, opening the pane would focus the first composing field and
    /// start recording into it unasked.
    private var canBecomeKey = false

    /// Whether a recording session is live. Teardown has several entry
    /// points — editing end, window resign, view detach — that can fire for
    /// one session or for none; this is what keeps the global side effects
    /// (the hotkey pause) paired one begin to one end.
    private var isRecordingSession = false
    private var windowDidResignKeyObserver: NSObjectProtocol?
    private var windowDidBecomeKeyObserver: NSObjectProtocol?

    init() {
        super.init(frame: NSRect(x: 0, y: 0, width: Self.minimumWidth, height: 24))
        alignment = .center
        delegate = self
        // The magnifying glass belongs to a search field, not to a recorder.
        (cell as? NSSearchFieldCell)?.searchButtonCell = nil
        setContentHuggingPriority(.defaultHigh, for: .vertical)
        setContentHuggingPriority(.defaultHigh, for: .horizontal)
        // Read last: hiding the cancel button is what lets the placeholder
        // centre while the row is empty.
        cancelButtonCell = (cell as? NSSearchFieldCell)?.cancelButtonCell
        cancelButtonCell?.target = self
        cancelButtonCell?.action = #selector(clearChord)
        showsCancelButton = false
    }

    @available(*, unavailable)
    required init?(coder _: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    deinit {
        MainActor.assumeIsolated {
            stopMonitoring()
            removeWindowObservers()
        }
    }

    override var intrinsicContentSize: NSSize {
        NSSize(width: Self.minimumWidth, height: super.intrinsicContentSize.height)
    }

    /// Whether the clear button is drawn. Not an `NSSearchField` property —
    /// swapping the cell in and out is how upstream does it, and it is what
    /// lets the placeholder centre while the row is empty.
    private var showsCancelButton: Bool {
        get { (cell as? NSSearchFieldCell)?.cancelButtonCell != nil }
        set { (cell as? NSSearchFieldCell)?.cancelButtonCell = newValue ? cancelButtonCell : nil }
    }

    /// Recording starts the moment FOCUS arrives, not when the editing
    /// session does: `controlTextDidBeginEditing` is only sent "upon the
    /// first user input since the text view became the first responder"
    /// (Cocoa Text Architecture Guide), so a monitor armed there misses the
    /// very key press it exists to record — a bare letter landed as field
    /// text, and a modifier chord, which inserts nothing, could never begin
    /// the session at all. Same hook upstream uses
    /// (`RecorderCocoa.becomeFirstResponder`).
    override func becomeFirstResponder() -> Bool {
        // No window yet means a SwiftUI hierarchy still assembling itself —
        // upstream's guard, kept for the same reason.
        guard window != nil else { return false }
        guard super.becomeFirstResponder() else { return false }
        beginRecording()
        return true
    }

    override var canBecomeKeyView: Bool { canBecomeKey }

    /// The normal way a session ends: the field editor detaching posts this
    /// whenever focus leaves, text change or none — only the BEGIN
    /// notification waits for input. Window resign and view detach
    /// (`viewDidMoveToWindow`) are the backstop teardowns for the exits that
    /// never detach an editor; `endRecording` is idempotent so the paths may
    /// overlap.
    func controlTextDidEndEditing(_: Notification) {
        endRecording()
    }

    /// Split from the responder hooks so a test can drive them: becoming
    /// first responder needs a key window, which a unit test has no reliable
    /// way to arrange.
    func beginRecording() {
        rejection = nil
        // Blanked while recording, so the prompt is visible on a row that
        // already holds a chord — a placeholder only shows while the text is
        // empty, and a refusal has nowhere else to be read.
        super.stringValue = ""
        showsCancelButton = false
        placeholderString = language?.string(.desktopShortcutRecording) ?? ""
        // No caret: the field takes keys, it does not take text.
        (currentEditor() as? NSTextView)?.insertionPointColor = .clear
        // The global hotkeys are off while a chord is recorded: this recorder
        // accepts modifier chords, and one that collides with a live hotkey
        // must be recorded, not acted on (upstream pauses the same way).
        // Guarded by the session flag so the pause pairs one begin to one
        // end whatever order the teardown entry points fire in.
        if !isRecordingSession {
            isRecordingSession = true
            KeyboardShortcuts.isEnabled = false
        }
        startMonitoring()
    }

    func endRecording() {
        stopMonitoring()
        if isRecordingSession {
            isRecordingSession = false
            KeyboardShortcuts.isEnabled = true
        }
        rejection = nil
        placeholderString = prompt
        // The field editor is the window's, shared with every text field in
        // it — a caret hidden for recording must not stay hidden for the next
        // field that borrows the editor.
        (currentEditor() as? NSTextView)?.insertionPointColor = .labelColor
        // `showChord`, not `renderChord`: the field editor may still be
        // attached at this point, and the guarded version would skip the very
        // row it is putting back.
        showChord()
    }

    /// Puts the bound chord on screen — or leaves the field empty, showing the
    /// prompt, when the row has none.
    private func showChord() {
        super.stringValue = chord.map(ShortcutKeyDisplay.text(for:)) ?? ""
        showsCancelButton = !stringValue.isEmpty
    }

    /// The same, unless the field is recording: a row being recorded into is
    /// deliberately blank, and a SwiftUI update arriving mid-recording must not
    /// write the old chord back over the prompt.
    private func renderChord() {
        guard currentEditor() == nil else { return }
        showChord()
    }

    @objc
    private func clearChord() {
        chord = nil
        onRecord?(nil)
    }

    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        removeWindowObservers()
        guard let window else {
            // Detached mid-recording — a pane switch tears the view down
            // without ever ending the editing session, and a monitor that
            // outlived its window would swallow every key in the app.
            endRecording()
            return
        }

        // Resolved here rather than in `init`, which runs before the language
        // store is handed over.
        if currentEditor() == nil {
            placeholderString = prompt
        }

        // A hidden settings window only hides — recording must not survive
        // the window losing key, and must not start by itself when it gets
        // key back (same pair upstream installs).
        windowDidResignKeyObserver = NotificationCenter.default.addObserver(
            forName: NSWindow.didResignKeyNotification, object: window, queue: nil,
        ) { [weak self] _ in
            Task { @MainActor [weak self] in
                guard let self, let window = self.window else { return }
                endRecording()
                window.makeFirstResponder(nil)
            }
        }
        windowDidBecomeKeyObserver = NotificationCenter.default.addObserver(
            forName: NSWindow.didBecomeKeyNotification, object: window, queue: nil,
        ) { [weak self] _ in
            Task { @MainActor [weak self] in
                self?.preventBecomingKey()
            }
        }
        preventBecomingKey()
    }

    /// Closes the door for exactly one runloop turn — long enough for the
    /// window's initial key-view pass to walk past this field, short enough
    /// that the user's own click or Tab still lands.
    private func preventBecomingKey() {
        canBecomeKey = false
        Task { @MainActor [weak self] in
            self?.canBecomeKey = true
        }
    }

    private func removeWindowObservers() {
        if let windowDidResignKeyObserver {
            NotificationCenter.default.removeObserver(windowDidResignKeyObserver)
            self.windowDidResignKeyObserver = nil
        }
        if let windowDidBecomeKeyObserver {
            NotificationCenter.default.removeObserver(windowDidBecomeKeyObserver)
            self.windowDidBecomeKeyObserver = nil
        }
    }

    /// What the field says while nothing is being recorded — or why the last
    /// attempt was turned down.
    private var prompt: String {
        guard let language else { return "" }
        guard let rejection else { return language.string(.desktopShortcutUnbound) }
        switch rejection {
        case .typesRomanization: return language.string(.desktopShortcutRejectedTypingKey)
        case .reservedKey: return language.string(.desktopShortcutRejectedReservedKey)
        case .noKey: return language.string(.desktopShortcutRejectedNoKey)
        case .candidateSlotChord: return language.string(.desktopShortcutRejectedSlotChord)
        case .takenBySystem: return language.string(.desktopShortcutRejectedSystemShortcut)
        // The same words a reserved key is refused with: to the reader both
        // mean "not this key", and a press the Carbon registry cannot name is
        // not a distinction worth a sentence of its own.
        case .notAGlobalKey: return language.string(.desktopShortcutRejectedReservedKey)
        case .belongsToHost: return language.string(.desktopShortcutRejectedHostShortcut)
        }
    }

    private func startMonitoring() {
        guard eventMonitor == nil else { return }
        eventMonitor = NSEvent.addLocalMonitorForEvents(
            matching: [.keyDown, .leftMouseUp, .rightMouseUp],
        ) { [weak self] event in
            // An orphaned monitor hands the key back rather than swallowing it:
            // one that outlived its field and ate every keystroke in the window
            // would be far worse than one that records nothing.
            guard let self else { return event }
            return handle(event)
        }
    }

    private func stopMonitoring() {
        guard let eventMonitor else { return }
        NSEvent.removeMonitor(eventMonitor)
        self.eventMonitor = nil
    }

    /// Nil swallows the key; returning the event lets it through.
    private func handle(_ event: NSEvent) -> NSEvent? {
        // A click outside the field is the way most people leave one — the
        // same escape upstream's monitor grants. Handed through so it also
        // does whatever it was aimed at.
        if event.type == .leftMouseUp || event.type == .rightMouseUp {
            let clickPoint = convert(event.locationInWindow, from: nil)
            let clickMargin = 3.0
            if !bounds.insetBy(dx: -clickMargin, dy: -clickMargin).contains(clickPoint) {
                blur()
                return event
            }
            return nil
        }

        // Key repeat is dropped: holding a key would otherwise record it over
        // and over, each time re-running conflict resolution.
        guard !event.isARepeat else { return nil }

        let modifiers = event.modifierFlags.intersection([.command, .control, .option, .shift])
        if modifiers.isEmpty {
            switch event.specialKey {
            case .tab, .backTab:
                // Bubbled up on purpose, so Tab still walks the form. The cost
                // is that Tab cannot be recorded here — the trade every
                // shortcut field on the Mac makes, and the reason Tab is not
                // among the defaults.
                blur()
                return event
            case .delete, .deleteForward, .backspace:
                stringValue = ""
                return nil
            default:
                break
            }
            if event.keyCode == kVK_Escape {
                // The way out. While a field has focus it swallows every key,
                // so a user who opened one by accident needs the key that
                // cancels everywhere else in the system.
                blur()
                return nil
            }
        }

        switch ComposingKeyChord.make(KeyEventSnapshot(event)) {
        case let .success(recorded) where recorded.isCandidateSlotChord(under: slotKeySet):
            refuse(.candidateSlotChord)
        case let .success(recorded):
            // Both storage forms are read here, off the one event that carries
            // them: the Carbon key code is gone the moment this returns.
            let key = RecordedShortcutKey(
                chord: recorded,
                globalShortcut: KeyboardShortcuts.Shortcut(event: event),
            )
            if let reason = additionalRejection?(key) {
                refuse(reason)
                return nil
            }
            rejection = nil
            chord = recorded
            onRecord?(key)
            blur()
        case let .failure(reason):
            refuse(reason)
        }
        return nil
    }

    private func refuse(_ reason: ComposingKeyChord.Rejection) {
        rejection = reason
        placeholderString = prompt
        NSSound.beep()
    }

    /// Gives up focus, which is what ends recording — `resignFirstResponder`
    /// takes the monitor down.
    private func blur() {
        window?.makeFirstResponder(nil)
    }
}

/// How a recorded chord reads on screen.
enum ShortcutKeyDisplay {
    /// The names AppKit has no glyph for, and the glyphs it does. Written out
    /// rather than resolved from the system because these are the keycap
    /// legends: they are the same on a keyboard sold anywhere, and translating
    /// "Space" would name a key the user cannot find.
    ///
    /// No ⌤ and no ⇤ among them: `ComposingKeyChord.normalized` folds the
    /// keypad's Enter onto Return and the back tab onto Tab, so a chord never
    /// arrives here carrying either.
    private static let keyNames: [String: String] = [
        " ": "Space",
        "\r": "↩",
        "\t": "⇥",
    ]

    /// The modifiers come from the shortcut library's own renderer rather than
    /// a second copy of the same four branches: the input-source menu prints
    /// global chords through `KeyboardShortcuts.Shortcut.description` and
    /// composing chords through this, side by side in one column, so the two
    /// have to speak the same glyph vocabulary.
    ///
    /// A chord WITH modifiers keeps the uppercase keycap legend the system
    /// and the global rows print (`⌃⌥J`). A bare key shows the character it
    /// types: an uppercase `Z` on a modifier-less row reads as ⇧Z, a key the
    /// row does not hold (USER 2026-08-22).
    static func text(for chord: ComposingKeyChord) -> String {
        let keycap = keyNames[chord.key]
            ?? (chord.modifiers.isEmpty ? chord.key : chord.key.uppercased())
        return chord.modifiers.ks_symbolicRepresentation + keycap
    }
}
