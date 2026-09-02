// One cell of the candidate window, mapped back to the candidate and script it commits.

import Foundation

/// A cell as the window shows it, with the candidate it stands for and the
/// script its own commit writes.
///
/// Under 漢羅並排 and 羅馬字 a candidate is one cell, so the window's list IS the
/// fetched list and a cell's position is the candidate's index. 漢羅濫 breaks
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

    /// The window's list for `candidates`, under ONE settings snapshot — the
    /// same rules the commit reads. 並排/羅馬字: one `.primary` cell per
    /// candidate, exactly `CandidateCellContent.cell`. 漢羅濫: a Hanji candidate
    /// is `[漢字 (.primary), 羅馬字 (.alternate)]` under the mode's forced swap.
    ///
    /// Both scripts dedupe on the TEXT THE CELL SHOWS, first-seen wins: a
    /// one-script cell carries nothing that could tell it from an earlier cell
    /// reading the same, so a second one is a defect, not a second offer
    /// (USER 2026-09-03 「相同的漢字 or 羅馬字不能重複出現」). The two scripts keep
    /// separate keys — a 漢字 cell never collides with a 羅馬字 one. Hanji cells
    /// were exempt until 2026-09-03 on Core Principle #7 grounds: 重/tîng and
    /// 重/tāng ARE two words, but under 漢羅濫 they draw two identical 重 cells,
    /// and the losing reading stays reachable through its own romanization
    /// cell. 漢羅並排 is untouched — its subtitle tells the pair apart.
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
        var presentedHanjiCells = Set<String>()
        var presentedRomanCells = Set<String>()
        for (index, candidate) in candidates.enumerated() {
            let hanji = candidate.presentableHanji
            if let hanji, presentedHanjiCells.insert(hanji).inserted {
                presented.append(PresentedCandidate(
                    candidateIndex: index,
                    script: .primary,
                    cell: CandidateCellContent(text: hanji, annotation: nil),
                ))
            }
            guard presentedRomanCells.insert(candidate.roman).inserted else { continue }
            presented.append(PresentedCandidate(
                candidateIndex: index,
                script: hanji == nil ? .primary : .alternate,
                cell: CandidateCellContent(text: candidate.roman, annotation: nil),
            ))
        }
        return presented
    }
}
