// Which candidate is highlighted, and which page of them is on screen.

import Foundation

/// The candidate list's navigation state.
///
/// Deliberately knows nothing about AppKit, the engine, or the window that
/// renders it: navigation is a permanent platform-side concern
/// (`.claude/rules/cross-platform-alignment.md` §5.1), and keeping it in a plain
/// value type is what lets the whole key contract be tested without a screen.
///
/// The highlight is this type's alone. The engine has a
/// `selected_candidate_index` field, but nothing in the engine reads it
/// (`engine/composing/src/transition.rs:576-582`), so mirroring the highlight
/// into it would be a second copy of the truth that no one consults.
struct CandidateListModel: Equatable {
    /// How many candidates one page of the bar shows. Nine because the direct
    /// selection chords are `⌃1` to `⌃9`: a page the chords cannot address in
    /// full would leave candidates reachable only by arrow key.
    static let pageSize = 9

    private(set) var candidates: [ContinuousCandidate] = []
    /// Always a valid index into `candidates`, except when the list is empty,
    /// where it is 0 and `highlighted` is nil.
    private(set) var highlightedIndex = 0

    var isEmpty: Bool { candidates.isEmpty }

    var highlighted: ContinuousCandidate? {
        candidates.indices.contains(highlightedIndex) ? candidates[highlightedIndex] : nil
    }

    /// The candidates of the page the highlight is on, in display order.
    var visiblePage: ArraySlice<ContinuousCandidate> {
        candidates[pageRange]
    }

    /// Where the highlight sits within its page — the `⌃n` chord that selects
    /// it, counting from zero. Nil for an empty list.
    var highlightedSlotInPage: Int? {
        isEmpty ? nil : highlightedIndex - pageRange.lowerBound
    }

    /// Replaces the list, starting again from the first candidate.
    ///
    /// The highlight resets rather than being preserved by index: a fresh
    /// keystroke re-ranks the whole list, so holding position would leave the
    /// highlight on an unrelated word that happens to have landed there.
    mutating func replace(with candidates: [ContinuousCandidate]) {
        self.candidates = candidates
        highlightedIndex = 0
    }

    mutating func reset() {
        replace(with: [])
    }

    /// Moves the highlight one candidate, stopping at either end.
    ///
    /// Clamping rather than wrapping, matching McBopomofo's horizontal
    /// candidate list (`references/McBopomofo/Packages/CandidateUI/Sources/
    /// CandidateUI/HorizontalCandidateController.swift:509`). azooKey-Desktop
    /// wraps by modulo; not adopted, because arrowing past the last candidate
    /// would silently jump back to the top-ranked one the user has just
    /// rejected.
    mutating func moveHighlight(_ direction: NavigationDirection) {
        guard !isEmpty else { return }
        highlightedIndex = clampToList(highlightedIndex + direction.step)
    }

    /// Moves the highlight to the first candidate of the next or previous page,
    /// and does nothing when there is no such page.
    ///
    /// Landing on the first slot keeps the page start, the highlight and the
    /// `⌃1` chord label describing the same candidate — the alternative,
    /// preserving the offset within the page, splits them apart on the last
    /// page, which is rarely full. Matches McBopomofo's horizontal controller
    /// (`HorizontalCandidateController.swift:479`).
    mutating func page(_ direction: NavigationDirection) {
        guard !isEmpty else { return }
        let firstOfNextPage = pageRange.lowerBound + direction.step * Self.pageSize
        guard candidates.indices.contains(firstOfNextPage) else { return }
        highlightedIndex = firstOfNextPage
    }

    /// Highlights and returns the candidate in the given slot of the CURRENT
    /// page — the `⌃n` chords address what the user can see, not absolute ranks
    /// somewhere off screen. Nil when that slot is empty, which is what the last
    /// page's unused slots are.
    mutating func selectSlotInPage(_ slot: Int) -> ContinuousCandidate? {
        let index = pageRange.lowerBound + slot
        guard (0 ..< Self.pageSize).contains(slot), candidates.indices.contains(index) else {
            return nil
        }
        highlightedIndex = index
        return candidates[index]
    }

    /// The half-open range of the page the highlight is on. Empty for an empty
    /// list, so the slice-based accessors stay total.
    private var pageRange: Range<Int> {
        guard !isEmpty else { return 0 ..< 0 }
        let start = (highlightedIndex / Self.pageSize) * Self.pageSize
        return start ..< min(start + Self.pageSize, candidates.count)
    }

    private func clampToList(_ index: Int) -> Int {
        min(max(index, 0), candidates.count - 1)
    }
}

/// Which way a navigation key moves through the list. Named for the list rather
/// than for the keys, because the same two directions are what `←`/`→` and
/// `↑`/`↓`/`PgUp`/`PgDn` both express.
enum NavigationDirection: Equatable {
    case backward
    case forward

    fileprivate var step: Int {
        switch self {
        case .backward: -1
        case .forward: 1
        }
    }
}
