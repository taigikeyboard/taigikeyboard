// What the settings pages ask of the user's data, which the engine owns.

import Foundation

/// One page of a user-data list — custom words or learning records — and
/// the two counts the page shows.
struct UserDataListing<Row: Sendable>: Sendable {
    let rows: [Row]
    /// Every stored row.
    let total: Int
    /// The rows the filter matches, for paging.
    let matchingTotal: Int
    /// Where `rows` start: the requested offset, pulled back to the last
    /// page by the engine when the matches shrank under it.
    let offset: Int
}

struct CustomDictionaryImportResult: Equatable, Sendable {
    let imported: Int
    let skipped: Int
}

/// What a restored `.taigi` backup added to each store it counts.
struct BackupImportResult: Hashable, Sendable {
    let customDictionary: Int
    let frequency: Int
    let association: Int
}

/// Why a user-data request did nothing. The description is the alert's
/// diagnostic line, untranslated on purpose (`UserDataPageMessage.failure`).
enum UserDataClientError: Error, Equatable, CustomStringConvertible {
    /// The round-trip failed or the engine refused the request; the bridge
    /// logged why.
    case engineUnavailable(op: String)
    /// Something the user can be told — a full dictionary, an unusable file —
    /// in the engine's own words (`custom dictionary is full (max 30000
    /// entries)`), or desktop-core's where the engine answers only a code (a
    /// refused backup) or never saw the file (one that could not be read,
    /// whose tail is the system's reason, in the system's language).
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
    func list(filter: String, limit: Int, offset: Int) throws -> UserDataListing<CustomDictionaryRow>
    func save(_ row: CustomDictionaryRow) throws
    func delete(id: String) throws
    /// Empties the custom dictionary.
    func deleteAll() throws
    func exportCSV() throws -> Data
    func importCSV(at url: URL) throws -> CustomDictionaryImportResult
    /// Every store as a `.taigi` backup.
    func exportBackup() throws -> Data
    /// Restores the `.taigi` file at `url`, merged with what is stored. A
    /// file that cannot be read or is not a backup throws
    /// `UserDataClientError.refused`.
    func importBackup(at url: URL) throws -> BackupImportResult
    /// Empties what the input method learned — counts, bigrams, learned
    /// phrases — and leaves the custom dictionary alone.
    func clearLearningRecords() throws

    /// One page of one learning store, in `order`. The rows are the
    /// engine's own records, handed back whole by the two writes below: the
    /// engine applies a write only while the row still holds what was listed.
    func listLearningRecords(
        kind: Taigi_Engine_LearningRecordKind,
        order: Taigi_Engine_LearningRecordOrder,
        filter: String,
        limit: Int,
        offset: Int,
    ) throws -> UserDataListing<Taigi_Engine_LearningRecord>
    /// False when the row is gone — deleted, evicted, or its id taken by
    /// another word since it was listed.
    func setLearningRecordCount(_ record: Taigi_Engine_LearningRecord, count: Int) throws -> Bool
    /// False when the row is gone, as for `setLearningRecordCount`.
    func deleteLearningRecord(_ record: Taigi_Engine_LearningRecord) throws -> Bool
    /// Adds a row's word to the custom dictionary, filed as `roman` / `hanji`
    /// (the word as the user edited it in the Add Word sheet; the engine
    /// trims both) — unless the word is stored there already — then forgets
    /// the learned phrase (a frequency row is never added). A refusal (no romanization,
    /// a full dictionary, a reading that cannot be searched) keeps the row
    /// and is thrown as `UserDataClientError.refused`.
    func addLearningRecordToCustomDictionary(
        _ record: Taigi_Engine_LearningRecord,
        roman: String,
        hanji: String,
    ) throws
}

/// The shipped client: the engine's user-data ops.
struct EngineUserDataClient: UserDataClient {
    /// The largest file an import reads. CROSS-PLATFORM INVARIANT — the
    /// engine refuses the same size (`engine/userdata/src/csv.rs`); checked
    /// here too so a huge file is refused before it is read into memory.
    static let maxImportFileBytes = 5 * 1024 * 1024

    func list(filter: String, limit: Int, offset: Int) throws -> UserDataListing<CustomDictionaryRow> {
        guard let page = RustEngineBridge.customDictionaryList(
            filter: filter,
            limit: limit,
            offset: offset,
        ) else { throw UserDataClientError.engineUnavailable(op: "customDictionaryList") }
        return UserDataListing(
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
            hanji: row.hanji,
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
        let size: Int
        do {
            size = try url.resourceValues(forKeys: [.fileSizeKey]).fileSize ?? 0
        } catch {
            throw Self.unreadable(error)
        }
        guard size <= Self.maxImportFileBytes else {
            throw UserDataClientError.refused(detail: "file is larger than 5 MB")
        }
        guard let imported = try RustEngineBridge.customDictionaryImportCSV(Self.read(url)) else {
            throw UserDataClientError.engineUnavailable(op: "customDictionaryImportCSV")
        }
        guard imported.refusal == .none else { throw UserDataClientError.refused(detail: imported.detail) }
        return CustomDictionaryImportResult(
            imported: Int(imported.imported),
            skipped: Int(imported.skipped),
        )
    }

    func exportBackup() throws -> Data {
        guard let backup = RustEngineBridge.backupExport(appVersion: AppVersion.installed) else {
            throw UserDataClientError.engineUnavailable(op: "backupExport")
        }
        return backup
    }

    /// Read whole, with no size cap, as on the other platforms.
    func importBackup(at url: URL) throws -> BackupImportResult {
        guard let imported = try RustEngineBridge.backupImport(Self.read(url)) else {
            throw UserDataClientError.engineUnavailable(op: "backupImport")
        }
        guard imported.refusal == .none else {
            throw UserDataClientError.refused(detail: Self.refusalDetail(imported.refusal))
        }
        return BackupImportResult(
            customDictionary: Int(imported.customDictionary),
            frequency: Int(imported.frequency),
            association: Int(imported.association),
        )
    }

    /// The alert's line for a refused backup. The engine answers only the
    /// refusal, so the words are desktop-core's (`backup_refused`,
    /// `engine/user_data.rs`), which are `userdata::backup::BackupError`'s.
    /// CROSS-PLATFORM INVARIANT: the same words on every desktop.
    static func refusalDetail(_ refusal: Taigi_Engine_BackupRefusal) -> String {
        refusal == .unsupportedVersion ? "unsupported backup version" : "not a .taigi backup"
    }

    /// The file the user picked, or a refusal saying why it could not be read.
    private static func read(_ url: URL) throws -> Data {
        do {
            return try Data(contentsOf: url)
        } catch {
            throw unreadable(error)
        }
    }

    /// desktop-core's words for a file it could not read (`unreadable`,
    /// `engine/user_data.rs`), with the system's reason after them.
    private static func unreadable(_ error: Error) -> UserDataClientError {
        .refused(detail: "could not read the file: \(error.localizedDescription)")
    }

    func clearLearningRecords() throws {
        try reset {
            $0.frequency = true
            $0.association = true
            $0.learnedPhrases = true
        }
    }

    func listLearningRecords(
        kind: Taigi_Engine_LearningRecordKind,
        order: Taigi_Engine_LearningRecordOrder,
        filter: String,
        limit: Int,
        offset: Int,
    ) throws -> UserDataListing<Taigi_Engine_LearningRecord> {
        guard let page = RustEngineBridge.learningRecordsList(
            kind: kind,
            order: order,
            filter: filter,
            limit: limit,
            offset: offset,
        ) else { throw UserDataClientError.engineUnavailable(op: "learningRecordsList") }
        return UserDataListing(
            rows: page.records,
            total: Int(page.total),
            matchingTotal: Int(page.matchingTotal),
            offset: Int(page.offset),
        )
    }

    func setLearningRecordCount(_ record: Taigi_Engine_LearningRecord, count: Int) throws -> Bool {
        guard let saved = RustEngineBridge.learningRecordSetCount(record, count: count) else {
            throw UserDataClientError.engineUnavailable(op: "learningRecordSetCount")
        }
        return saved.hasRecord
    }

    func deleteLearningRecord(_ record: Taigi_Engine_LearningRecord) throws -> Bool {
        guard let removed = RustEngineBridge.learningRecordDelete(record) else {
            throw UserDataClientError.engineUnavailable(op: "learningRecordDelete")
        }
        return removed
    }

    func addLearningRecordToCustomDictionary(
        _ record: Taigi_Engine_LearningRecord,
        roman: String,
        hanji: String,
    ) throws {
        guard let added = RustEngineBridge.learningRecordAddToCustomDictionary(
            record,
            roman: roman,
            hanji: hanji,
        ) else {
            throw UserDataClientError.engineUnavailable(op: "learningRecordAddToCustomDictionary")
        }
        guard added.refusal == .none else { throw UserDataClientError.refused(detail: added.detail) }
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
        self.init(id: entry.id, roman: entry.roman, hanji: entry.hanji)
    }
}

/// A learned row is addressed by the store's row id on the page that listed
/// it — a table selection, never the word's identity (`(text, tl)`, which the
/// engine checks on every write).
extension Taigi_Engine_LearningRecord: Identifiable {}
