// What the settings pages ask of the user's data, which the engine owns.

import Foundation

/// One page of the custom dictionary, and the two counts the page shows.
struct CustomDictionaryListing: Equatable, Sendable {
    let rows: [CustomDictionaryRow]
    /// Every stored word.
    let total: Int
    /// The words the filter matches, for paging.
    let matchingTotal: Int
    /// Where `rows` start: the requested offset, pulled back to the last
    /// page by the engine when the matches shrank under it.
    let offset: Int
}

struct CustomDictionaryImportResult: Equatable, Sendable {
    let imported: Int
    let skipped: Int
}

/// Why a user-data request did nothing. The description is the alert's
/// diagnostic line, English on purpose (`UserDataPageMessage.failure`).
enum UserDataClientError: Error, Equatable, CustomStringConvertible {
    /// The round-trip failed or the engine refused the request; the bridge
    /// logged why.
    case engineUnavailable(op: String)
    /// Something the user can be told — a full dictionary, an unusable file —
    /// in the engine's own words (`custom dictionary is full (max 30000
    /// entries)`).
    case refused(detail: String)
    /// A reset that emptied some stores and not others, one line per store
    /// that failed (`user_frequency: <error>`).
    case notEmptied([String])

    var description: String {
        switch self {
        case let .engineUnavailable(op): "the engine did not answer \(op)"
        case let .refused(detail): detail
        case let .notEmptied(failures): failures.joined(separator: "\n")
        }
    }
}

/// The user-data requests the settings pages make. Synchronous: every call
/// is an engine round-trip that may touch SQLite, so callers run it off the
/// main actor. A protocol so the page's paging and work-slot logic can be
/// driven from tests without opening the process-wide user data — the test
/// process shares one engine, and an open there would reach every later
/// fetch in the run.
protocol UserDataClient: Sendable {
    func list(filter: String, limit: Int, offset: Int) throws -> CustomDictionaryListing
    func save(_ row: CustomDictionaryRow) throws
    func delete(id: String) throws
    /// Empties the custom dictionary.
    func deleteAll() throws
    func exportCSV() throws -> Data
    func importCSV(at url: URL) throws -> CustomDictionaryImportResult
    /// Empties what the input method learned — counts, bigrams, learned
    /// phrases — and leaves the custom dictionary alone.
    func clearLearningRecords() throws
    /// The dictionary search's lookup, by the key `query` derives under
    /// `mode`; empty when there is nothing to find or the engine did not
    /// answer.
    func search(query: String, mode: InputMode, limit: Int) -> [CustomDictionaryRow]
}

/// The shipped client: the engine's user-data ops.
struct EngineUserDataClient: UserDataClient {
    /// The largest file an import reads. CROSS-PLATFORM INVARIANT — the
    /// engine refuses the same size (`engine/userdata/src/csv.rs`); checked
    /// here too so a huge file is refused before it is read into memory.
    static let maxImportFileBytes = 5 * 1024 * 1024

    func list(filter: String, limit: Int, offset: Int) throws -> CustomDictionaryListing {
        guard let page = RustEngineBridge.customDictionaryList(
            filter: filter,
            limit: limit,
            offset: offset,
        ) else { throw UserDataClientError.engineUnavailable(op: "customDictionaryList") }
        return CustomDictionaryListing(
            rows: page.entries.map(CustomDictionaryRow.init),
            total: Int(page.total),
            matchingTotal: Int(page.matchingTotal),
            offset: Int(page.offset),
        )
    }

    func save(_ row: CustomDictionaryRow) throws {
        guard let saved = RustEngineBridge.customDictionarySave(
            id: row.id,
            roman: row.roman,
            hanzi: row.hanzi,
        ) else { throw UserDataClientError.engineUnavailable(op: "customDictionarySave") }
        guard saved.refusal == .none else { throw UserDataClientError.refused(detail: saved.detail) }
    }

    func delete(id: String) throws {
        guard RustEngineBridge.customDictionaryDelete(id: id) != nil else {
            throw UserDataClientError.engineUnavailable(op: "customDictionaryDelete")
        }
    }

    func deleteAll() throws {
        try reset { $0.customDictionary = true }
    }

    func exportCSV() throws -> Data {
        guard let csv = RustEngineBridge.customDictionaryExportCSV() else {
            throw UserDataClientError.engineUnavailable(op: "customDictionaryExportCSV")
        }
        return csv
    }

    func importCSV(at url: URL) throws -> CustomDictionaryImportResult {
        let size = try url.resourceValues(forKeys: [.fileSizeKey]).fileSize ?? 0
        guard size <= Self.maxImportFileBytes else {
            throw UserDataClientError.refused(detail: "file is larger than 5 MB")
        }
        guard let imported = try RustEngineBridge.customDictionaryImportCSV(Data(contentsOf: url)) else {
            throw UserDataClientError.engineUnavailable(op: "customDictionaryImportCSV")
        }
        guard imported.refusal == .none else { throw UserDataClientError.refused(detail: imported.detail) }
        return CustomDictionaryImportResult(
            imported: Int(imported.imported),
            skipped: Int(imported.skipped),
        )
    }

    func clearLearningRecords() throws {
        try reset {
            $0.frequency = true
            $0.association = true
            $0.learnedPhrases = true
        }
    }

    func search(query: String, mode: InputMode, limit: Int) -> [CustomDictionaryRow] {
        RustEngineBridge.customDictionarySearch(query: query, mode: mode, limit: limit)?
            .map(CustomDictionaryRow.init) ?? []
    }

    /// Empties the stores `select` names; every one is attempted, and the
    /// ones that could not be emptied are reported together.
    private func reset(_ select: (inout Taigi_Engine_ResetUserData) -> Void) throws {
        var request = Taigi_Engine_ResetUserData()
        select(&request)
        guard let removed = RustEngineBridge.userDataReset(request) else {
            throw UserDataClientError.engineUnavailable(op: "userDataReset")
        }
        guard removed.failures.isEmpty else { throw UserDataClientError.notEmptied(removed.failures) }
    }
}

extension CustomDictionaryRow {
    init(_ entry: Taigi_Engine_CustomDictionaryEntry) {
        self.init(id: entry.id, roman: entry.roman, hanzi: entry.hanzi)
    }
}
