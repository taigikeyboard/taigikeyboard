// The candidates one fetch produced and the cells the window shows for them.

import Foundation

/// The candidates the engine offered for one context and the cells presented
/// for them, held together so neither can outlive the other: an index resolved
/// against a presentation of some other fetch would commit the wrong word, so
/// every window index comes back through `resolve`.
/// CROSS-PLATFORM INVARIANT — mirrors the Windows
/// `taigi-windows-core/src/composing/presentation.rs` `CandidateSource`.
struct CandidateSource: Equatable, Sendable {
    /// The fetched list, in the engine's order — what a commit hands back to
    /// the engine, and what a display change presents again
    /// (`init(candidates:manager:)` over the previous `candidates`).
    private(set) var candidates: [ContinuousCandidate]
    /// `candidates` as the window shows them (`PresentedCandidate`).
    private(set) var presented: [PresentedCandidate]

    /// Whether the first cell is the §34 literal, which takes no slot key
    /// (`ComposingManager.leadsWithLiteralRomanCandidate`). Held with the
    /// presentation it describes: it is a fact about THIS fetch under the
    /// settings snapshot that presented it, and a stale copy would shift the
    /// keys off by one.
    let leadsWithLiteralRoman: Bool

    static let empty = CandidateSource(candidates: [], presented: [], leadsWithLiteralRoman: false)

    private init(
        candidates: [ContinuousCandidate],
        presented: [PresentedCandidate],
        leadsWithLiteralRoman: Bool,
    ) {
        self.candidates = candidates
        self.presented = presented
        self.leadsWithLiteralRoman = leadsWithLiteralRoman
    }

    /// `candidates` presented under the settings in force right now.
    @MainActor
    init(candidates: [ContinuousCandidate], manager: ComposingManager) {
        let presentation = manager.presentation(for: candidates)
        self.init(
            candidates: candidates,
            presented: presentation.cells,
            leadsWithLiteralRoman: presentation.leadsWithLiteralRoman,
        )
    }

    /// Read off the presented list — the one truth every "is a bar showing"
    /// check reads.
    var isEmpty: Bool { presented.isEmpty }

    /// What the window draws, in display order.
    var cells: [CandidateCellContent] { presented.map(\.cell) }

    /// The candidate behind cell `cellIndex` and the script its commit writes:
    /// the cell's own, or the other one when `flip` (Space). nil past the
    /// list — nothing to commit.
    func resolve(cellIndex: Int, flip: Bool) -> (candidate: ContinuousCandidate, script: CandidateScript)? {
        guard presented.indices.contains(cellIndex) else { return nil }
        let cell = presented[cellIndex]
        return (candidates[cell.candidateIndex], flip ? cell.script.flipped : cell.script)
    }
}
