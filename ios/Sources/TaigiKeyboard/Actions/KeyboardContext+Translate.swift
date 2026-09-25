import Combine
import KeyboardKit
import SwiftUI

/// `KeyboardContext` extension adding the hanji / roman display-mode toggle, backed by
/// `SharedSettings`.
public extension KeyboardContext {
    /// Whether hanji-first mode is active (`true` = hanji, `false` = roman) —
    /// the DERIVED candidate projection (`true` under Hanji with Romanization, `false` under
    /// Romanization Only). Read-only: writers go through `toggleTranslateSwapped()`.
    var isTranslateSwapped: Bool {
        SharedSettings.shared.isTranslateSwapped
    }

    /// Whether the character / symbol layouts type full-width punctuation —
    /// the stored swap under Hanji–Romanization Pairing / Hanji with Romanization, never under Romanization Only, always under TPS.
    var isFullWidthPunctuation: Bool {
        SharedSettings.shared.isFullWidthPunctuation
    }

    /// Candidate cell rendering mode, mirrored on the context so a change from
    /// the in-keyboard settings overlay re-renders the strip + expanded overlay
    /// through the same `objectWillChange` path the 文/A toggle uses.
    var candidateDisplayMode: CandidateDisplayMode {
        get { SharedSettings.shared.candidateDisplayMode }
        set {
            SharedSettings.shared.candidateDisplayMode = newValue
            notifyDisplayChange()
        }
    }

    /// Flips the STORED swap. Under Hanji–Romanization Pairing that flips the lead script and the
    /// punctuation width; under Hanji with Romanization only the punctuation width (each cell
    /// already commits its own script). Inert under Romanization Only (always half-width)
    /// and TPS (always full-width, hanji-first) — where the key is hidden
    /// anyway (`LayoutConverter` / `ExpandedCandidateOverlay`); the guard keeps
    /// any other caller safe.
    func toggleTranslateSwapped() {
        guard candidateDisplayMode.allowsSwapToggle,
              SharedSettings.shared.keyboardLayoutType != .tps else { return }
        SharedSettings.shared.storedIsTranslateSwapped.toggle()
        notifyDisplayChange()
    }

    /// Re-renders every reader of the display pair (strip, expanded overlay,
    /// layout). Also the hook for `syncSettings()` when a host-app write lands.
    internal func notifyDisplayChange() {
        DispatchQueue.main.async {
            self.objectWillChange.send()
        }
    }
}
