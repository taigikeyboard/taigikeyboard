// In-memory stand-ins for the engine's user data.
//
// This test process never opens the engine's user data: the handle is
// process-wide, so an open here would reach every later fetch in the run.
// What the engine does with a pick, a bigram or a page request is asserted
// by the engine's own tests (`engine/userdata/src/requests.rs`,
// `engine/dispatch/src/user_data.rs`, `engine/dispatch/tests/user_data_*.rs`);
// these record what this side SENDS.

import Foundation
@testable import TaigiInputMethodCore

/// A custom dictionary held in memory, answering the pages' requests the way
/// the engine does: newest edit first, a case-insensitive substring filter,
/// trimmed.
final class FakeUserDataClient: UserDataClient, @unchecked Sendable {
    private let lock = NSLock()
    private var rows: [CustomDictionaryRow] = []

    /// The filter the last list request carried, as the page sent it.
    private(set) var lastCustomFilter: String?
    /// Set to make every list request fail, as an unreadable store does.
    var failsCustomReads = false
    /// Set to make every write fail AFTER it changed the store, as an import
    /// whose later chunk failed has already committed the earlier ones.
    var failsCustomWritesAfterApplying = false
    /// What `importCSV` adds to the store.
    var rowsToImport: [CustomDictionaryRow] = []

    func list(filter: String, limit: Int, offset: Int) throws -> UserDataListing<CustomDictionaryRow> {
        try lock.withLock {
            lastCustomFilter = filter
            if failsCustomReads {
                throw UserDataClientError.engineUnavailable(op: "customDictionaryList")
            }
            let trimmed = filter.trimmingCharacters(in: .whitespacesAndNewlines)
            let matching = trimmed.isEmpty
                ? rows
                : rows.filter {
                    $0.roman.localizedCaseInsensitiveContains(trimmed) || $0.hanji.contains(trimmed)
                }
            // Pulled back to the last page that exists, as the engine does.
            let lastPage = max(0, matching.count - 1) / max(1, limit) * limit
            let served = min(offset, lastPage)
            return UserDataListing(
                rows: Array(matching.dropFirst(served).prefix(limit)),
                total: rows.count,
                matchingTotal: matching.count,
                offset: served,
            )
        }
    }

    func save(_ row: CustomDictionaryRow) throws {
        try applyCustomWrite {
            rows.removeAll { $0.id == row.id }
            rows.insert(row, at: 0)
        }
    }

    func delete(id: String) throws {
        try applyCustomWrite { rows.removeAll { $0.id == id } }
    }

    func deleteAll() throws {
        try applyCustomWrite { rows.removeAll() }
    }

    func exportCSV() throws -> Data {
        Data()
    }

    func importCSV(at _: URL) throws -> CustomDictionaryImportResult {
        let imported = lock.withLock { rowsToImport }
        try applyCustomWrite { rows.insert(contentsOf: imported, at: 0) }
        return CustomDictionaryImportResult(imported: imported.count, skipped: 0)
    }

    private func applyCustomWrite(_ write: () -> Void) throws {
        try lock.withLock {
            write()
            if failsCustomWritesAfterApplying {
                throw UserDataClientError.engineUnavailable(op: "customDictionaryWrite")
            }
        }
    }

    private var learningRecordClears = 0

    /// How many times the learning records were emptied.
    var learningRecordClearCount: Int {
        lock.withLock { learningRecordClears }
    }

    /// Set to make every clear fail, as a store that cannot be emptied does.
    var failsLearningRecordClears = false

    /// Empties every learning store, as the engine's reset does.
    func clearLearningRecords() throws {
        if failsLearningRecordClears {
            throw UserDataClientError.engineUnavailable(op: "resetUserData")
        }
        lock.withLock {
            learningRecordClears += 1
            learningRecords = []
        }
    }

    // MARK: - Learning records

    private var learningRecords: [Taigi_Engine_LearningRecord] = []
    /// What the last list request asked for, as `(kind, order, filter)`.
    private(set) var lastLearningRecordsQuery: (
        kind: Taigi_Engine_LearningRecordKind,
        order: Taigi_Engine_LearningRecordOrder,
        filter: String,
    )?
    /// Set to make every list request fail, as an unreadable store does.
    var failsLearningRecordReads = false

    /// Set to hold every list request in the engine until the test signals
    /// it — a load still in flight while the page moves on.
    var learningRecordsListGate: DispatchSemaphore?
    /// Called as a list request reaches the store, before the gate.
    var onLearningRecordsListEntered: (@Sendable () -> Void)?

    func seedLearningRecords(_ records: [Taigi_Engine_LearningRecord]) {
        lock.withLock { learningRecords = records }
    }

    func listLearningRecords(
        kind: Taigi_Engine_LearningRecordKind,
        order: Taigi_Engine_LearningRecordOrder,
        filter: String,
        limit: Int,
        offset: Int,
    ) throws -> UserDataListing<Taigi_Engine_LearningRecord> {
        onLearningRecordsListEntered?()
        learningRecordsListGate?.wait()
        return try lock.withLock {
            lastLearningRecordsQuery = (kind, order, filter)
            if failsLearningRecordReads {
                throw UserDataClientError.engineUnavailable(op: "learningRecordsList")
            }
            let store = learningRecords.filter { $0.kind == kind }
            let matching = (filter.isEmpty
                ? store
                : store.filter { $0.text.contains(filter) || $0.tl.localizedCaseInsensitiveContains(filter) })
                .sorted {
                    order == .mostUsed
                        ? ($0.count, $0.lastUsedMs) > ($1.count, $1.lastUsedMs)
                        : $0.lastUsedMs > $1.lastUsedMs
                }
            let lastPage = max(0, matching.count - 1) / max(1, limit) * limit
            let served = min(offset, lastPage)
            return UserDataListing(
                rows: Array(matching.dropFirst(served).prefix(limit)),
                total: store.count,
                matchingTotal: matching.count,
                offset: served,
            )
        }
    }

    /// Applies only while the row still holds what was listed, as the engine
    /// does.
    func setLearningRecordCount(_ record: Taigi_Engine_LearningRecord, count: Int) throws -> Bool {
        lock.withLock {
            guard let index = learningRecords.firstIndex(where: { Self.isSameRow($0, record) }) else { return false }
            learningRecords[index].count = Int64(count)
            return true
        }
    }

    func deleteLearningRecord(_ record: Taigi_Engine_LearningRecord) throws -> Bool {
        lock.withLock {
            guard let index = learningRecords.firstIndex(where: { Self.isSameRow($0, record) }) else { return false }
            learningRecords.remove(at: index)
            return true
        }
    }

    /// Set to refuse every add, as the engine does for a full dictionary.
    var refusesLearningRecordAdds = false

    /// Files the row's word as a custom word unless that word is stored
    /// already, then forgets a learned phrase and keeps a frequency row; a
    /// refusal keeps it — as the engine does.
    func addLearningRecordToCustomDictionary(_ record: Taigi_Engine_LearningRecord) throws {
        try lock.withLock {
            if refusesLearningRecordAdds {
                throw UserDataClientError.refused(detail: "custom dictionary is full (max 30000 entries)")
            }
            if !rows.contains(where: { $0.roman == record.tl && $0.hanji == record.text }) {
                rows.insert(CustomDictionaryRow(roman: record.tl, hanji: record.text), at: 0)
            }
            if record.kind == .learnedPhrase {
                learningRecords.removeAll { Self.isSameRow($0, record) }
            }
        }
    }

    private static func isSameRow(_ stored: Taigi_Engine_LearningRecord, _ listed: Taigi_Engine_LearningRecord) -> Bool {
        stored.kind == listed.kind && stored.id == listed.id && stored.text == listed.text && stored.tl == listed.tl
    }
}
