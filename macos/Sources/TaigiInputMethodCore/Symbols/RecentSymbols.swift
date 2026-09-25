// The symbols the user picked most recently, and how they lead the picker's list.

import Foundation

/// The last few picks, most recent first — the front of the picker's list,
/// so a symbol used a moment ago is on the first page under the first slot
/// keys (USER 2026-09-19: "the symbol menu should sort by most recently typed"). One page, not
/// the whole table: past `capacity` a symbol falls back to its file place,
/// so the brackets stay together and the rest of the list keeps its order.
///
/// Pure: what is stored and what is shown are two functions of the same
/// list. The store keeps the strings themselves (`SettingsStore.Keys
/// .recentSymbols`), so a table that drops a symbol simply stops showing it
/// (`ordered(_:)` never writes back).
struct RecentSymbols: Equatable, Sendable {
    /// One slot-key page.
    static let capacity = HorizontalPageLayout.pageSize

    /// Most recent first, no repeats, at most `capacity`.
    let symbols: [String]

    /// Normalizes what came from the store — a hand-edited list may repeat a
    /// symbol or run past a page — so every value holds the invariant.
    init(_ stored: [String]) {
        var seen: Set<String> = []
        symbols = Array(stored.filter { !$0.isEmpty && seen.insert($0).inserted }.prefix(Self.capacity))
    }

    /// The list with `symbol` moved to the front.
    func noting(_ symbol: String) -> RecentSymbols {
        RecentSymbols([symbol] + symbols)
    }

    /// `table` reordered: the recents it still has, most recent first, then
    /// the rest in table order.
    func ordered(_ table: [String]) -> [String] {
        symbols.filter { table.contains($0) } + table.filter { !symbols.contains($0) }
    }
}
