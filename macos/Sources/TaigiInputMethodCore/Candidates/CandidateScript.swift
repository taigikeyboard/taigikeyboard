// Which of a candidate's two scripts a commit writes.

/// Which of a candidate's two scripts a commit writes.
///
/// A named pair rather than a `Bool` because both values are meaningful at the
/// call site, and because "the other one" is resolved from the output settings
/// — this says WHICH RELATIVE script, never which absolute one. That is the
/// difference from the per-action forced rendering removed in #609, where each
/// shortcut pinned 漢字 or 羅馬字 outright and the user had to remember which
/// key was which.
enum CandidateScript: Equatable, Sendable {
    /// What the output settings lead with — what Return writes.
    case primary
    /// The other one — what Space writes (`CandidateDocumentText.alternateText`).
    case alternate
}
