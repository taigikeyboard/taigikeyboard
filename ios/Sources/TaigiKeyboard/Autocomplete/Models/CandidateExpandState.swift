// Candidate-row expansion state — an ObservableObject shared by CandidateView and
// ExpandedCandidateOverlay.

import Combine
import SwiftUI

/// Tracks whether the candidate row is expanded or collapsed.
class CandidateExpandState: ObservableObject {
    @Published var isExpanded = false

    func toggle() {
        isExpanded.toggle()
    }

    func collapse() {
        isExpanded = false
    }
}
