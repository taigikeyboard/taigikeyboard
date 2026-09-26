// In-memory stand-ins for the engine's user data.
//
// This test process never opens the engine's user data: the handle is
// process-wide, so an open here would reach every later fetch in the run.
// What the engine does with a pick, a bigram or a page request is asserted
// by the engine's own tests (`engine/dispatch/src/user_data.rs`,
// `engine/dispatch/tests/user_data_*.rs`); these record what this side SENDS.

import Foundation
@testable import TaigiInputMethodCore

/// Every pick the manager reported, in order.
@MainActor
final class RecordingUsageRecorder: UsageRecorder {
    private(set) var recorded: [Usage] = []

    nonisolated init() {}

    func record(_ usage: Usage) {
        recorded.append(usage)
    }
}

/// One next-word handshake the manager reported. What the engine learns from
/// it — the window, noise, sentence ends, compounds — is the engine's
/// (`engine/nextword/src/decide.rs`); when and what the manager reports is
/// macOS's.
enum Handshake: Equatable {
    case selected(text: String, roman: String)
    case nailed(text: String, roman: String)
    case forgot
}

/// Every next-word handshake the manager reported, in order.
@MainActor
final class RecordingNextWordPort: NextWordPort {
    private(set) var handshakes: [Handshake] = []

    nonisolated init() {}

    /// The reported texts in order, `"∅"` for a forgotten context.
    var reported: [String] {
        handshakes.map { handshake in
            switch handshake {
            case let .selected(text, _), let .nailed(text, _): text
            case .forgot: "∅"
            }
        }
    }

    func wordSelected(text: String, roman: String, settings _: EngineSettings, generation _: UInt64) {
        handshakes.append(.selected(text: text, roman: roman))
    }

    func segmentNailed(text: String, roman: String, settings _: EngineSettings, generation _: UInt64) {
        handshakes.append(.nailed(text: text, roman: roman))
    }

    func forgetContext(settings _: EngineSettings, generation _: UInt64) {
        handshakes.append(.forgot)
    }
}

/// A custom dictionary held in memory, answering the pages' requests the way
/// the engine does: newest edit first, a case-insensitive substring filter.
/// The search matches by romanization prefix — a stand-in for the engine's
/// key-prefix lookup, which the engine's tests cover.
final class FakeUserDataClient: UserDataClient, @unchecked Sendable {
    private let lock = NSLock()
    private var rows: [CustomDictionaryRow] = []

    func list(filter: String, limit: Int, offset: Int) throws -> CustomDictionaryListing {
        lock.withLock {
            let matching = filter.isEmpty
                ? rows
                : rows.filter {
                    $0.roman.localizedCaseInsensitiveContains(filter) || $0.hanzi.contains(filter)
                }
            // Pulled back to the last page that exists, as the engine does.
            let lastPage = max(0, matching.count - 1) / max(1, limit) * limit
            let served = min(offset, lastPage)
            return CustomDictionaryListing(
                rows: Array(matching.dropFirst(served).prefix(limit)),
                total: rows.count,
                matchingTotal: matching.count,
                offset: served,
            )
        }
    }

    func save(_ row: CustomDictionaryRow) throws {
        lock.withLock {
            rows.removeAll { $0.id == row.id }
            rows.insert(row, at: 0)
        }
    }

    func delete(id: String) throws {
        lock.withLock { rows.removeAll { $0.id == id } }
    }

    func deleteAll() throws {
        lock.withLock { rows.removeAll() }
    }

    func exportCSV() throws -> Data {
        Data()
    }

    func importCSV(at _: URL) throws -> CustomDictionaryImportResult {
        CustomDictionaryImportResult(imported: 0, skipped: 0)
    }

    func clearLearningRecords() throws {}

    func search(query: String, mode _: InputMode, limit: Int) -> [CustomDictionaryRow] {
        lock.withLock {
            Array(rows.filter { $0.roman.lowercased().hasPrefix(query.lowercased()) }.prefix(limit))
        }
    }
}
