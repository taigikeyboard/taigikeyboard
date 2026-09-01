// What one candidate cell shows: the primary script, and the other one beside it.

import Foundation

/// One candidate as the window renders it — both scripts, in the order the
/// user's swap setting puts them; both in one Hanji-led label under the
/// combined display; or the romanization alone under the romanization-only
/// display.
///
/// A Taigi candidate is a `(漢字, 羅馬字)` pair (Core Principle #7), and showing
/// only one of them makes several candidates read identically: two Hanji with
/// the same reading, or one Hanji under two readings. iOS and Android have
/// always shown both — primary text with the other script under it
/// (`ios/Sources/TaigiKeyboard/Autocomplete/Views/CandidateButtonView.swift:65-77`)
/// — and this is the macOS counterpart, rendered as MacishType's annotation
/// column rather than a second line because the window is one row tall.
///
/// Display only. What committing writes into the document stays
/// `CandidateDocumentText`'s decision: the two agree on which script leads, but
/// the document string can carry both in brackets while the cell keeps them in
/// separate columns.
struct CandidateCellContent: Equatable, Sendable {
    /// The script this cell leads with — romanization, or Hanji when swapped.
    let text: String

    /// The other script, shown smaller beside `text`, or nil for a candidate
    /// that has only one (a romanization-only candidate has no Hanji).
    let annotation: String?

    /// An empty annotation is normalized to nil so the cell reserves no width
    /// for it — a producer that emitted `""` for "no Hanji" means the same
    /// thing as omitting it, and the layout must not read the two differently.
    init(text: String, annotation: String?) {
        self.text = text
        self.annotation = (annotation?.isEmpty == false) ? annotation : nil
    }

    /// The cell for `candidate` under `settings`.
    ///
    /// CROSS-PLATFORM INVARIANT — mirrors
    /// ios/Sources/TaigiKeyboard/Autocomplete/Services/TaigiAutocompleteService.swift:150-162
    /// (primary = romanization, secondary = Hanji) and the swap flip in
    /// `CandidateCellHelper`. Drift changes which script a candidate leads with.
    static func cell(for candidate: ContinuousCandidate, settings: EngineSettings) -> Self {
        guard let hanji = candidate.hanji, !hanji.isEmpty else {
            // Romanization-only candidate: there is no second script to show,
            // in either direction — the same case `CandidateDocumentText`
            // answers with the bare romanization.
            return Self(text: candidate.roman, annotation: nil)
        }
        // Before the swap arm, so the swap cannot put Hanji into a cell this
        // mode says shows none. No annotation on purpose: Space commits the
        // annotation (`CandidateDocumentText.alternateText`), and with no
        // other script on offer it must fall to `.ignored` rather than write
        // Hanji the user never saw.
        if settings.candidateDisplayMode == .romanOnly {
            return Self(text: candidate.roman, annotation: nil)
        }
        // One label, Hanji first, one ASCII space between: the Hanji is the
        // value and the romanization its hint, so the commit writes the Hanji
        // (the effective swap is `true` under this mode) and Space, with no
        // annotation to write, falls to `.ignored` as it does above. Before
        // the swap arm for the same reason the arm above is.
        // CROSS-PLATFORM INVARIANT — mirrors iOS `CandidateCellHelper.displayTitle`
        // and Windows `document_text.rs` (`CandidateCellContent::cell`), which
        // join with the same single space. Drift changes what 合用 shows.
        if settings.candidateDisplayMode == .combined {
            return Self(text: "\(hanji) \(candidate.roman)", annotation: nil)
        }
        return settings.isTranslateSwapped
            ? Self(text: hanji, annotation: candidate.roman)
            : Self(text: candidate.roman, annotation: hanji)
    }
}
