// The control that records which key runs a composing action.

import AppKit
import Carbon.HIToolbox
import KeyboardShortcuts
import SwiftUI

/// A recording field for one composing action's key.
///
/// An `NSSearchField` subclass because that is what the global chords in the
/// same section are (`KeyboardShortcuts.RecorderCocoa`), and a pane where half
/// the rows are recording fields and half are push buttons reads as two
/// features rather than one. Focus drives recording, the placeholder says what
/// the field wants, and the cancel button clears the row — the shape a user has
/// already learnt from every other shortcut field on the Mac.
///
/// `KeyboardShortcuts.Recorder` itself cannot be reused: it records a Carbon
/// hotkey, which needs a modifier, and the keys that drive a candidate window
/// are mostly bare — Return, Space, `[`.
///
/// Refusing is part of the job. A user who recorded `a` here would have no way
/// left to type the letter, so the keys a syllable is spelled with are turned
/// down — beep, and the reason in the placeholder — rather than accepted
/// (`ComposingKeyChord.make`).
struct ComposingKeyRecorder: NSViewRepresentable {
    /// The chord as stored, or nil for an empty row.
    let chord: ComposingKeyChord?
    /// Which modifier currently holds the candidate slots, so a chord that tier
    /// would swallow can be refused rather than recorded and left inert.
    let slotModifier: CandidateSlotModifier
    /// Passed rather than read from the environment: this is an AppKit view,
    /// and the strings are resolved inside it.
    let language: DisplayLanguageStore
    /// Called with the recorded chord, or nil when the user clears the row.
    let onChange: (ComposingKeyChord?) -> Void

    func makeNSView(context _: Context) -> ComposingKeyRecorderField {
        let field = ComposingKeyRecorderField()
        apply(to: field)
        field.chord = chord
        return field
    }

    func updateNSView(_ field: ComposingKeyRecorderField, context _: Context) {
        apply(to: field)
        // Guarded: assigning redraws the field, and doing that on every SwiftUI
        // update would fight the row the user is recording into.
        if field.chord != chord {
            field.chord = chord
        }
    }

    private func apply(to field: ComposingKeyRecorderField) {
        field.language = language
        field.slotModifier = slotModifier
        field.onChange = onChange
    }
}

/// The field itself.
final class ComposingKeyRecorderField: NSSearchField, NSSearchFieldDelegate {
    /// Upstream's width for the same control, so a section mixing the two does
    /// not step (`KeyboardShortcuts.RecorderCocoa`).
    private static let minimumWidth: Double = 130

    var language: DisplayLanguageStore?
    var slotModifier: CandidateSlotModifier = .control
    var onChange: ((ComposingKeyChord?) -> Void)?

    var chord: ComposingKeyChord? {
        didSet { renderChord() }
    }

    /// Why the last key press was turned down, shown in place of the prompt
    /// until the next one. Cleared on focus and on a successful recording.
    private var rejection: ComposingKeyChord.Rejection?

    private var eventMonitor: Any?
    private var cancelButtonCell: NSButtonCell?

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
        MainActor.assumeIsolated { stopMonitoring() }
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

    /// Recording starts and ends with the field's EDITING session, not with
    /// `becomeFirstResponder`/`resignFirstResponder`: a text field's first
    /// responder is its field editor, so the field itself is never asked to
    /// resign, and the editor is only attached once editing has begun. Same
    /// hook upstream uses (`KeyboardShortcuts.RecorderCocoa`).
    func controlTextDidBeginEditing(_: Notification) {
        beginRecording()
    }

    func controlTextDidEndEditing(_: Notification) {
        endRecording()
    }

    /// Split from the delegate callbacks so a test can drive them: an editing
    /// session only starts on a key window, which a unit test has no reliable
    /// way to arrange.
    func beginRecording() {
        rejection = nil
        // Blanked while recording, so the prompt is visible on a row that
        // already holds a chord — a placeholder only shows while the text is
        // empty, and a refusal has nowhere else to be read.
        super.stringValue = ""
        showsCancelButton = false
        placeholderString = language?.string(.macosShortcutRecording) ?? ""
        // No caret: the field takes keys, it does not take text.
        (currentEditor() as? NSTextView)?.insertionPointColor = .clear
        startMonitoring()
    }

    func endRecording() {
        stopMonitoring()
        rejection = nil
        placeholderString = prompt
        // `showChord`, not `renderChord`: the field editor may still be
        // attached at this point, and the guarded version would skip the very
        // row it is putting back.
        showChord()
    }

    /// Puts the bound chord on screen — or leaves the field empty, showing the
    /// prompt, when the row has none.
    private func showChord() {
        super.stringValue = chord.map(ComposingKeyDisplay.text(for:)) ?? ""
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
        onChange?(nil)
    }

    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        // Resolved here rather than in `init`, which runs before the language
        // store is handed over.
        if window != nil, currentEditor() == nil {
            placeholderString = prompt
        }
    }

    /// What the field says while nothing is being recorded — or why the last
    /// attempt was turned down.
    private var prompt: String {
        guard let language else { return "" }
        guard let rejection else { return language.string(.macosShortcutUnbound) }
        switch rejection {
        case .typesRomanization: return language.string(.macosShortcutRejectedTypingKey)
        case .reservedKey: return language.string(.macosShortcutRejectedReservedKey)
        case .noKey: return language.string(.macosShortcutRejectedNoKey)
        case .candidateSlotChord: return language.string(.macosShortcutRejectedSlotChord)
        }
    }

    private func startMonitoring() {
        guard eventMonitor == nil else { return }
        eventMonitor = NSEvent.addLocalMonitorForEvents(matching: [.keyDown]) { [weak self] event in
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
        case let .success(recorded) where recorded.isCandidateSlotChord(under: slotModifier):
            refuse(.candidateSlotChord)
        case let .success(recorded):
            rejection = nil
            chord = recorded
            onChange?(recorded)
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
enum ComposingKeyDisplay {
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
    static func text(for chord: ComposingKeyChord) -> String {
        chord.modifiers.ks_symbolicRepresentation
            + (keyNames[chord.key] ?? chord.key.uppercased())
    }
}
