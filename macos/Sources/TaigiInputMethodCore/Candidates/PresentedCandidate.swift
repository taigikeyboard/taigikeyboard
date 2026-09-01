// One cell of the candidate window, mapped back to the candidate and script it commits.

import Foundation

/// A cell as the window shows it, with the candidate it stands for and the
/// script its own commit writes.
///
/// Under 漢羅並排 and 羅馬字 a candidate is one cell, so the window's list IS the
/// fetched list and a cell's position is the candidate's index. 漢羅合用 breaks
/// that: a candidate carrying both scripts is TWO adjacent cells — the Hanji,
/// then the romanization — each committing its own script (USER 2026-09-02:
/// 「漢字單獨做一個候選詞、羅馬字也當作候選詞」, not one formatted label). So the
/// absolute index the `CandidatePresenter` seam answers with names a PRESENTED
/// cell, and this record is what turns it back into `(candidate, script)`.
/// Every consumer of a presenter index goes through the presented list; none
/// may index the fetched list directly.
struct PresentedCandidate: Equatable, Sendable {
    /// The position in the fetched list this cell commits.
    let candidateIndex: Int
    /// Which of the candidate's two scripts this cell's own commit (Return, a
    /// slot key) writes. Space writes `script.flipped` — the other script of
    /// the same candidate.
    let script: CandidateScript
    let cell: CandidateCellContent

    /// The window's list for `candidates` under `settings`.
    ///
    /// One settings snapshot for the whole list, because the cells and the
    /// scripts have to be resolved under the same rules the commit will read.
    ///
    /// - 漢羅並排 / 羅馬字: one `.primary` cell per candidate, exactly the cell
    ///   `CandidateCellContent.cell(for:settings:)` has always built.
    /// - 漢羅合用: a candidate with Hanji becomes `[漢字 (.primary), 羅馬字
    ///   (.alternate)]`, neither annotated; a candidate without becomes its
    ///   romanization alone (`.primary`, which writes the romanization for a
    ///   Hanji-less candidate). The effective swap is `true` under this mode
    ///   (`CandidateDisplayMode.effectiveTranslateSwapped`), which is what
    ///   makes `.primary` the Hanji and `.alternate` the romanization.
    ///
    ///   Romanization cells are deduplicated by `(text, consumed span)`, in
    ///   fetched order: 食 and 𤆬 are both `tsia̍h` for the same span, and a
    ///   second `tsia̍h` cell would commit the same document text as the first
    ///   — likewise the §34 literal `tâi` at slot 0 absorbs 台's romanization
    ///   cell. Hanji cells are never deduplicated: 重 tîng and 重 tāng are two
    ///   morphemes (Core Principle #7), and their adjacent romanizations are
    ///   what tells them apart. A single walk over the fetched order, so the
    ///   slot mapping keeps that order.
    static func presentation(
        of candidates: [ContinuousCandidate],
        settings: EngineSettings,
    ) -> [PresentedCandidate] {
        guard settings.candidateDisplayMode == .combined else {
            return candidates.enumerated().map { index, candidate in
                PresentedCandidate(
                    candidateIndex: index,
                    script: .primary,
                    cell: CandidateCellContent.cell(for: candidate, settings: settings),
                )
            }
        }

        var presented: [PresentedCandidate] = []
        var presentedRomanCells = Set<RomanCellKey>()
        for (index, candidate) in candidates.enumerated() {
            let hanji = candidate.hanji.flatMap { $0.isEmpty ? nil : $0 }
            if let hanji {
                presented.append(PresentedCandidate(
                    candidateIndex: index,
                    script: .primary,
                    cell: CandidateCellContent(text: hanji, annotation: nil),
                ))
            }
            guard presentedRomanCells.insert(RomanCellKey(candidate)).inserted else { continue }
            presented.append(PresentedCandidate(
                candidateIndex: index,
                script: hanji == nil ? .primary : .alternate,
                cell: CandidateCellContent(text: candidate.roman, annotation: nil),
            ))
        }
        return presented
    }

    /// What makes two romanization cells the same offer: the same text for the
    /// same stretch of the buffer. The span matters because `tâi` consuming
    /// three bytes and `tâi` consuming five are different commits.
    private struct RomanCellKey: Hashable {
        let text: String
        let consumedSpanStart: UInt32
        let consumedSpanEnd: UInt32

        init(_ candidate: ContinuousCandidate) {
            text = candidate.roman
            consumedSpanStart = candidate.consumedSpanStart
            consumedSpanEnd = candidate.consumedSpanEnd
        }
    }
}
