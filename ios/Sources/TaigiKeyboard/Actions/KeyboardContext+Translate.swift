import Combine
import KeyboardKit
import SwiftUI

/// `KeyboardContext` extension adding the hanji / roman display-mode toggle, backed by
/// `SharedSettings`.
public extension KeyboardContext {
    /// Whether hanji-first mode is active (`true` = hanji, `false` = roman).
    ///
    /// The getter is the DERIVED value (`false` while 候選詞顯示 = 羅馬字); the
    /// setter writes the stored flag so a stored `true` survives the mode.
    var isTranslateSwapped: Bool {
        get { SharedSettings.shared.isTranslateSwapped }
        set {
            SharedSettings.shared.storedIsTranslateSwapped = newValue
            notifyDisplayChange()
        }
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

    /// Inert unless 候選詞顯示 = 漢羅對應: under 羅馬字 there is no hanji to
    /// lead with, under 漢羅濫 every cell is already single-script (the split
    /// happens upstream, each cell commits its own script), and toggling
    /// the derived getter would overwrite the stored flag. The key stays
    /// visible; its active state reads the derived value.
    func toggleTranslateSwapped() {
        guard candidateDisplayMode.allowsSwapToggle else { return }
        isTranslateSwapped.toggle()
    }

    private func notifyDisplayChange() {
        DispatchQueue.main.async {
            self.objectWillChange.send()
        }
    }
}
