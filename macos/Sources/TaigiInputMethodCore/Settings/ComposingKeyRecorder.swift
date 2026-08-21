// The control that records which key runs a composing action.

import AppKit
import SwiftUI

/// A click-to-record field for one composing action's key.
///
/// `KeyboardShortcuts.Recorder` records the three global chords next to this
/// one, and cannot record these: it registers a Carbon hotkey, which needs a
/// modifier, while the keys that drive a candidate window are mostly bare —
/// Return, Space, `[`. So this records the key itself and stores it, and the
/// key classifier does the matching.
///
/// Refusing is part of the job. A user who recorded `a` here would have no way
/// left to type the letter, so the keys a syllable is spelled with are turned
/// down with a reason on screen rather than accepted (`ComposingKeyChord.make`).
struct ComposingKeyRecorder: View {
    let action: ComposingAction
    /// The chord as stored, or nil for an empty row.
    let chord: ComposingKeyChord?
    /// Which modifier the candidate-slot chords are held with. Carried because
    /// it is the one refusal `ComposingKeyChord.make` cannot make on its own:
    /// whether ⌥3 is bindable depends on a setting, and that tier is
    /// classified before any binding, so recording one would store a row that
    /// never fires.
    let slotModifier: CandidateSlotModifier
    /// Which row is listening, if any. Owned by the pane so that clicking a
    /// second row disarms the first: two armed rows would install two event
    /// monitors, and whichever ran first would swallow the key.
    @Binding var recordingAction: ComposingAction?
    /// Called with the recorded chord, or nil when the user clears the row.
    let onChange: (ComposingKeyChord?) -> Void

    @Environment(DisplayLanguageStore.self) private var language
    @State private var rejection: ComposingKeyChord.Rejection?

    private var isRecording: Bool { recordingAction == action }

    var body: some View {
        LabeledContent(action.label(language)) {
            HStack(spacing: 8) {
                if isRecording, let rejection {
                    Text(message(for: rejection))
                        .font(.caption)
                        .foregroundStyle(.red)
                }
                Button(buttonTitle) {
                    rejection = nil
                    recordingAction = isRecording ? nil : action
                }
                .background(RecordingKeyCatcher(isRecording: isRecording, onKey: record))
                Button {
                    stopRecording()
                    onChange(nil)
                } label: {
                    Image(systemName: "xmark.circle.fill")
                        .accessibilityLabel(language.string(.macosShortcutClear))
                }
                .buttonStyle(.borderless)
                .foregroundStyle(.secondary)
                .opacity(chord == nil ? 0 : 1)
                .disabled(chord == nil)
            }
        }
    }

    private var buttonTitle: String {
        if isRecording { return language.string(.macosShortcutRecording) }
        guard let chord else { return language.string(.macosShortcutUnbound) }
        return ComposingKeyDisplay.text(for: chord)
    }

    private func record(_ key: KeyEventSnapshot) {
        // Escape gets out of recording rather than being turned down for it.
        // While a row listens it swallows every key, so a user who opened one
        // by accident needs a key that closes it — and Escape is the key that
        // cancels everywhere else in the system.
        if key.characters == "\u{1B}", key.modifiers.isDisjoint(with: [.command, .control, .option]) {
            stopRecording()
            return
        }
        switch ComposingKeyChord.make(key) {
        case let .success(chord):
            if chord.isCandidateSlotChord(under: slotModifier) {
                rejection = .candidateSlotChord
                return
            }
            stopRecording()
            onChange(chord)
        case let .failure(reason):
            rejection = reason
        }
    }

    private func stopRecording() {
        rejection = nil
        recordingAction = nil
    }

    private func message(for rejection: ComposingKeyChord.Rejection) -> String {
        switch rejection {
        case .typesRomanization: language.string(.macosShortcutRejectedTypingKey)
        case .reservedKey: language.string(.macosShortcutRejectedReservedKey)
        case .noKey: language.string(.macosShortcutRejectedNoKey)
        case .candidateSlotChord: language.string(.macosShortcutRejectedSlotChord)
        }
    }
}

/// Catches key presses for the recorder above, while it is recording.
///
/// A local event monitor rather than a first-responder view: the settings
/// window is a SwiftUI form whose focus moves with Tab, and a view that had to
/// hold focus would fight the form for it — and would miss Tab itself, which is
/// a key a user may well want to bind.
private struct RecordingKeyCatcher: NSViewRepresentable {
    let isRecording: Bool
    let onKey: (KeyEventSnapshot) -> Void

    func makeNSView(context: Context) -> NSView {
        context.coordinator.onKey = onKey
        return NSView()
    }

    func updateNSView(_: NSView, context: Context) {
        context.coordinator.onKey = onKey
        context.coordinator.setMonitoring(isRecording)
    }

    func makeCoordinator() -> Coordinator { Coordinator() }

    static func dismantleNSView(_: NSView, coordinator: Coordinator) {
        coordinator.setMonitoring(false)
    }

    @MainActor
    final class Coordinator {
        var onKey: ((KeyEventSnapshot) -> Void)?
        private var monitor: Any?

        func setMonitoring(_ isMonitoring: Bool) {
            guard isMonitoring != (monitor != nil) else { return }
            if isMonitoring {
                monitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] event in
                    // An orphaned monitor hands the key back rather than
                    // swallowing it: a monitor that outlived its recorder and
                    // ate every keystroke in the window would be far worse than
                    // one that records nothing.
                    guard let self else { return event }
                    // Key repeat is dropped: holding a key would otherwise
                    // record it over and over, each time re-running conflict
                    // resolution.
                    guard !event.isARepeat else { return nil }
                    onKey?(KeyEventSnapshot(event))
                    // Swallowed so the keystroke being recorded does not also
                    // move the form's focus or type into a field.
                    return nil
                }
            } else if let monitor {
                NSEvent.removeMonitor(monitor)
                self.monitor = nil
            }
        }
    }
}

/// How a recorded chord reads on screen.
enum ComposingKeyDisplay {
    /// The names AppKit has no glyph for, and the glyphs it does. Written out
    /// rather than resolved from the system because these are the keycap
    /// legends: they are the same on a keyboard sold anywhere, and translating
    /// "Return" would name a key the user cannot find.
    ///
    /// No ⌤ among them: the keypad's Enter is stored as Return
    /// (`ComposingKeyChord`), so a chord never arrives here carrying it.
    private static let keyNames: [String: String] = [
        " ": "Space",
        "\r": "↩",
        "\t": "⇥",
        "\u{19}": "⇤",
    ]

    static func text(for chord: ComposingKeyChord) -> String {
        var text = ""
        if chord.modifiers.contains(.control) { text += "⌃" }
        if chord.modifiers.contains(.option) { text += "⌥" }
        if chord.modifiers.contains(.shift) { text += "⇧" }
        if chord.modifiers.contains(.command) { text += "⌘" }
        return text + (keyNames[chord.key] ?? chord.key.uppercased())
    }
}
