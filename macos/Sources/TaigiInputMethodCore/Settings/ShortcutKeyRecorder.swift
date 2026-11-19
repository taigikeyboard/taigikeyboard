// The control that records which key runs a shortcut, in either registry.

import AppKit
import KeyboardShortcuts
import SwiftUI

/// One key press, in both the forms the Shortcuts pane's two registries store.
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
/// statement. The composing tier has no extra refusal of its own: everything
/// it must defend — the typing keys of both tone schemes — is already in the
/// shared gate (desktop-core's, `KeyRules.press`).
enum GlobalShortcutPolicy {
    /// `@MainActor` because `isTakenBySystem` is (KeyboardShortcuts reads the
    /// system hotkey table through `HotKeyCenter`, a main-actor type; Swift
    /// 6.4 infers the isolation, Swift 6.2 did not). Every caller already runs
    /// there: the recorder field is an `NSView`.
    @MainActor
    static func rejection(for key: RecordedShortcutKey) -> ComposingKeyChord.Rejection? {
        // A global row stores a Carbon key CODE. A press that yields none has
        // nothing to store, so it cannot be recorded here even though the
        // shared gate passed it.
        guard let shortcut = key.globalShortcut else { return .notAGlobalKey }
        let modifiers = shortcut.modifiers
        // ⌘ with nothing but Shift beside it belongs to the application being
        // typed into: that is where a Mac puts its menu commands, and this
        // input method never takes one (`ShortcutActions`, on ⌃⌘S). The
        // library used to be the thing enforcing it — indirectly, by refusing
        // the modifier-less keys we now accept — so stating it is part of
        // owning the recorder. ⌃⌘ and ⌥⌘ are ours to offer; bare ⌘ is not.
        if modifiers.contains(.command),
           modifiers.isDisjoint(with: [.control, .option])
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
        // has to be recordable or Reset to Defaults would produce a row the recorder
        // itself rejects.
        guard !modifiers.isDisjoint(with: KeyEventSnapshot.hostChords) else { return nil }
        guard !isAShippedDefault(shortcut) else { return nil }
        return shortcut.isTakenBySystem ? .takenBySystem : nil
    }

    private static func isAShippedDefault(_ shortcut: KeyboardShortcuts.Shortcut) -> Bool {
        ShortcutAction.allCases.contains { $0.defaultShortcut == shortcut }
    }
}

/// A recording field for one shortcut row, composing tier or global.
///
/// Both tiers, since 2026-08-26. The global rows used
/// `KeyboardShortcuts.Recorder` until then, and it refuses a modifier-less key
/// outright — `RecorderCocoa.swift:404-410` beeps and swallows the event
/// before any validation of ours runs — so a user could not put the Hanji/romanization swap on a
/// bare `z` even though the action SHIPS on a bare backtick (USER 2026-08-26,
/// real device). One field for both tiers also keeps every row on the pane
/// looking alike, which is what lets the pane group its rows by what a key
/// does rather than by which registry holds it (`ShortcutSettingsView`).
///
/// Refusing is part of the job. A user who recorded `a` here would have no way
/// left to type the letter, so the keys a syllable is spelled with are turned
/// down with a beep rather than accepted (`KeyRules.press`). What each tier
/// refuses ON TOP of that is `additionalRejection`'s to say.
struct ShortcutKeyRecorder: NSViewRepresentable {
    /// The chord as stored, or nil for an empty row.
    let chord: ComposingKeyChord?
    /// Passed rather than read from the environment: this is an AppKit view,
    /// and the strings are resolved inside it.
    let language: DisplayLanguageStore
    /// The tier's own refusals, run after the shared gate passes. Nil accepts
    /// everything the gate does.
    var additionalRejection: (@MainActor (RecordedShortcutKey) -> ComposingKeyChord.Rejection?)?
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
        field.additionalRejection = additionalRejection
        field.onRecord = onRecord
    }
}

/// The field itself: a search field — the shape a shortcut field has on the
/// Mac, capsule, centred text and clear button — that never edits text.
///
/// Neither editable nor selectable (until 2026-11-19 it was both), so focus
/// installs no field editor. Recording through the editor failed on device:
/// AppKit ends and restarts it on its own — setting the placeholder while it
/// is installed does it (`NSCell _restartEditingWithTextView:`) — which tore
/// sessions down mid-begin, and the clear button took the first click on a
/// bound row. Rectangle's recorder (`MASShortcutView`) records without a
/// text editor for the same reason. Being the first responder IS
/// recording: a click or Tab begins it, resigning ends it, nothing in
/// between; the clicks are this class's own (`hitTest`, `mouseDown`).
final class ShortcutKeyRecorderField: NSSearchField {
    /// Upstream's width for the same control, so a section mixing the two does
    /// not step (`KeyboardShortcuts.RecorderCocoa`).
    private static let minimumWidth: Double = 130

    var language: DisplayLanguageStore? {
        didSet { refreshCell() }
    }

    var additionalRejection: (@MainActor (RecordedShortcutKey) -> ComposingKeyChord.Rejection?)?
    var onRecord: ((RecordedShortcutKey?) -> Void)?

    var chord: ComposingKeyChord? {
        didSet { refreshCell() }
    }

    /// What the row shows: blank while recording, so the caret alone says it
    /// is waiting for a key (USER 2026-11-19 — no prompt); otherwise the chord,
    /// or the unbound prompt.
    var displayedText: String {
        if isRecording {
            return ""
        }
        return chord?.display ?? language?.string(.desktopShortcutUnbound) ?? ""
    }

    private(set) var isRecording = false

    /// Swapped in and out of the cell: a search field draws its clear button
    /// whenever it holds one.
    private var clearButtonCell: NSButtonCell?

    private var searchCell: NSSearchFieldCell? {
        cell as? NSSearchFieldCell
    }

    /// The system caret (macOS 14), shown while recording.
    private let caret = NSTextInsertionIndicator()

    private var eventMonitor: Any?

    /// Refuses the initial key-view focus a window hands out when it opens or
    /// becomes key, without refusing the click or Tab that comes after — the
    /// same gate the global recorders run (`RecorderCocoa.canBecomeKeyView`).
    /// Without it, opening the pane would focus the first row and start
    /// recording into it unasked.
    private var canBecomeKey = false

    /// The field whose recording session holds the global hotkey pause; only
    /// the holder lifts it (upstream's `RecorderCocoa.activeRecorder`).
    private weak static var activeRecorder: ShortcutKeyRecorderField?
    private var windowDidResignKeyObserver: NSObjectProtocol?
    private var windowDidBecomeKeyObserver: NSObjectProtocol?

    init() {
        super.init(frame: NSRect(x: 0, y: 0, width: Self.minimumWidth, height: 24))
        isEditable = false
        isSelectable = false
        alignment = .center
        // The magnifying glass belongs to a search field, not to a recorder.
        searchCell?.searchButtonCell = nil
        clearButtonCell = searchCell?.cancelButtonCell
        refreshCell()
        caret.displayMode = .hidden
        addSubview(caret)
        setContentHuggingPriority(.defaultHigh, for: .vertical)
        setContentHuggingPriority(.defaultHigh, for: .horizontal)
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

    /// Puts the row's state on screen. No field editor is ever installed, so
    /// these writes restart nothing.
    private func refreshCell() {
        placeholderString = isRecording ? "" : language?.string(.desktopShortcutUnbound) ?? ""
        stringValue = isRecording ? "" : chord?.display ?? ""
        searchCell?.cancelButtonCell = showsClearButton ? clearButtonCell : nil
        needsDisplay = true
    }

    override func layout() {
        super.layout()
        placeCaret()
    }

    /// One text line tall, centred: the row is blank while the caret shows.
    /// Also placed on begin, so a row that has not been laid out yet still
    /// shows it.
    private func placeCaret() {
        let font = font ?? .systemFont(ofSize: NSFont.systemFontSize)
        let height = font.ascender - font.descender
        caret.frame = NSRect(x: bounds.midX - 1, y: bounds.midY - height / 2, width: 2, height: height)
    }

    private var showsClearButton: Bool {
        chord != nil && !isRecording
    }

    /// Every click inside the field is the field's own. On current macOS the
    /// search field draws through subviews it hosts (a SwiftUI host and an
    /// `_NSSearchFieldClearButton`), and those won hit-testing: a click on
    /// the clear button never reached `mouseDown`, so the binding was never
    /// cleared and the chord came back.
    override func hitTest(_ point: NSPoint) -> NSView? {
        !isHidden && frame.contains(point) ? self : nil
    }

    /// A click on the search field's own clear-button frame empties the row
    /// back to the unbound prompt and leaves it there; a click anywhere else
    /// records (USER 2026-11-19 — Rectangle's `MASShortcutView` clears the
    /// same way).
    override func mouseDown(with event: NSEvent) {
        if showsClearButton, cancelButtonBounds.contains(convert(event.locationInWindow, from: nil)) {
            clearChord()
            return
        }
        window?.makeFirstResponder(self)
    }

    private func clearChord() {
        chord = nil
        onRecord?(nil)
    }

    override var acceptsFirstResponder: Bool {
        true
    }

    override var canBecomeKeyView: Bool {
        canBecomeKey
    }

    /// Recording starts the moment FOCUS arrives — a click or Tab — so the
    /// very first key press is the one recorded. Same hook upstream uses
    /// (`RecorderCocoa.becomeFirstResponder`).
    override func becomeFirstResponder() -> Bool {
        // No window yet means a SwiftUI hierarchy still assembling itself —
        // upstream's guard, kept for the same reason.
        guard window != nil, super.becomeFirstResponder() else { return false }
        beginRecording()
        return true
    }

    /// The normal way a session ends: focus leaving — a recorded key, Escape,
    /// a click elsewhere, or another row taking it. Window resign and view
    /// detach (`viewDidMoveToWindow`) are the backstops; `endRecording` is
    /// idempotent so the paths may overlap.
    override func resignFirstResponder() -> Bool {
        endRecording()
        return super.resignFirstResponder()
    }

    /// Split from the responder hooks so a test can drive the session
    /// without focus.
    func beginRecording() {
        isRecording = true
        placeCaret()
        caret.displayMode = .automatic
        refreshCell()
        // The global hotkeys are off while a chord is recorded: this recorder
        // accepts modifier chords, and one that collides with a live hotkey
        // must be recorded, not acted on (upstream pauses the same way).
        Self.activeRecorder = self
        KeyboardShortcuts.isEnabled = false
        startMonitoring()
    }

    func endRecording() {
        stopMonitoring()
        if Self.activeRecorder === self {
            Self.activeRecorder = nil
            KeyboardShortcuts.isEnabled = true
        }
        isRecording = false
        caret.displayMode = .hidden
        refreshCell()
    }

    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        removeWindowObservers()
        guard let window else {
            // Detached mid-recording — a pane switch tears the view down
            // without the window ever moving focus, and a monitor that
            // outlived its window would swallow every key in the app.
            endRecording()
            return
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

        // The rest is desktop-core's (`KeyRules.press`): Tab is recorded, not
        // walked — it is the shipped key of Next Candidate, and a field that
        // let it move the focus instead would leave Reset to Defaults, which
        // resets every row, as the only way to put it back (USER 2026-09-19);
        // the Escape key leaves — while a field has focus it swallows every
        // key, so a user who opened one by accident needs the key that cancels
        // everywhere else in the system; everything else goes through the gate.
        switch KeyRules.press(KeyEventSnapshot(event)) {
        case let .recorded(recorded)?:
            // Both storage forms are read here, off the one event that carries
            // them: the Carbon key code is gone the moment this returns.
            let key = RecordedShortcutKey(
                chord: recorded,
                globalShortcut: KeyboardShortcuts.Shortcut(event: event),
            )
            if additionalRejection?(key) != nil {
                NSSound.beep()
                return nil
            }
            chord = recorded
            onRecord?(key)
            blur()
        case .refused?:
            // A beep and nothing else: the session stays, ready for the next
            // key (USER 2026-11-19 — no refusal text).
            NSSound.beep()
        case .blur?:
            blur()
        case .blank?, nil:
            // Backspace / Delete: the row is already blank while recording and
            // nothing is recorded — the clear button clears (desktop-core
            // `recorder.rs`). Nil: the seam failed (logged), key swallowed.
            break
        }
        return nil
    }

    /// Gives up focus, which is what ends recording.
    private func blur() {
        window?.makeFirstResponder(nil)
    }

    /// To assistive technology the row is a button that starts recording.
    override func isAccessibilityElement() -> Bool {
        true
    }

    override func accessibilityRole() -> NSAccessibility.Role? {
        .button
    }

    /// The value, not the label: the row names the field (`ShortcutSettingsView`
    /// sets the label), and the chord is read after it.
    override func accessibilityValue() -> String? {
        displayedText
    }

    override func accessibilityPerformPress() -> Bool {
        window?.makeFirstResponder(self) ?? false
    }

    /// The clear button's job for VoiceOver, which cannot aim at its square.
    override func accessibilityPerformDelete() -> Bool {
        guard chord != nil else { return false }
        clearChord()
        return true
    }
}
