// What the settings pages ask of the user's data, which desktop-core and the engine own.

import Foundation
import os

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
    case refused(refusal: Taigi_DesktopShell_CustomDictionaryRefusal = .none, detail: String)
    /// A reset that emptied some stores and not others, one line per store
    /// that failed (`user_frequency: <error>`).
    case notEmptied([String])

    var description: String {
        switch self {
        case let .engineUnavailable(op): "the engine did not answer \(op)"
        case let .refused(_, detail): detail
        case let .notEmptied(failures): failures.joined(separator: "\n")
        }
    }
}

/// Desktop-core's shared UserData rules, pickers metadata, and export file
/// naming (`desktop/crates/taigi-desktop-core/src/settings/learning_records.rs`,
/// `custom_dictionary.rs`, `listing.rs`).
enum UserDataRules {
    private static let logger = DebugLogger(category: "UserDataRules")
    private static let cachedReply = OSAllocatedUnfairLock<Taigi_DesktopShell_UserDataRulesReply?>(initialState: nil)

    /// Cached read from `desktop-core`; never caches `nil` so a transient failure
    /// retries on the next caller.
    static func load() -> Taigi_DesktopShell_UserDataRulesReply? {
        if let cached = cachedReply.withLock({ $0 }) {
            return cached
        }
        guard let fetched = fetch(transport: DesktopCoreBridge.send) else { return nil }
        return cachedReply.withLock { state in
            if let existing = state {
                return existing
            }
            state = fetched
            return fetched
        }
    }

    static func fetch(
        transport: ([UInt8]) -> [UInt8] = DesktopCoreBridge.send,
    ) -> Taigi_DesktopShell_UserDataRulesReply? {
        let op = "userDataRules"
        guard
            case let .success(response) = DesktopCoreBridge.response(
                to: .userDataRules(.init()),
                settings: nil,
                op: op,
                transport: transport,
            ),
            response.error == .ok,
            case let .userDataRules(reply)? = response.reply
        else {
            logger.error("[\(op)] no rules reply")
            return nil
        }
        return reply
    }

    static var pageSize: Int {
        max(1, Int(load()?.pageSize ?? 1))
    }

    static var maxImportFileBytes: Int {
        Int(load()?.customCsvMaxFileBytes ?? 0)
    }

    static var learningRecordMaxCount: Int {
        max(1, Int(load()?.learningRecordMaxCount ?? 1))
    }

    static var learningRecordKinds: [Taigi_DesktopShell_LearningRecordKindRule] {
        load()?.learningRecordKinds ?? []
    }

    static var learningRecordOrders: [Taigi_DesktopShell_LearningRecordOrderRule] {
        load()?.learningRecordOrders ?? []
    }

    static var learningRecordKindSpellings: [(value: Taigi_DesktopShell_LearningRecordKind, spelling: String)] {
        learningRecordKinds.map { ($0.kind, $0.storedSpelling) }
    }

    static var learningRecordOrderSpellings: [(value: Taigi_DesktopShell_LearningRecordOrder, spelling: String)] {
        learningRecordOrders.map { ($0.order, $0.storedSpelling) }
    }

    static var backupOpenExtensions: [String] {
        load()?.backupOpenExtensions ?? []
    }

    static func offersAddToCustomDictionary(for kind: Taigi_DesktopShell_LearningRecordKind) -> Bool {
        learningRecordKinds.first { $0.kind == kind }?.offersAddToCustomDictionary ?? false
    }

    static func countNoteKey(for kind: Taigi_DesktopShell_LearningRecordKind) -> StringKey? {
        guard
            let rule = learningRecordKinds.first(where: { $0.kind == kind }),
            rule.hasCountNoteKey
        else { return nil }
        return StringKey(rawValue: rule.countNoteKey)
    }

    static func exportFileName(
        kind: Taigi_DesktopShell_UserDataExportKind,
        localDate: String,
        transport: ([UInt8]) -> [UInt8] = DesktopCoreBridge.send,
    ) -> String? {
        var req = Taigi_DesktopShell_UserDataExportFileNameRequest()
        req.kind = kind
        req.localDate = localDate
        let op = "userDataExportFileName"
        guard
            case let .success(response) = DesktopCoreBridge.response(
                to: .userDataExportFileName(req),
                settings: nil,
                op: op,
                transport: transport,
            ),
            response.error == .ok,
            case let .userDataExportFileName(reply)? = response.reply
        else {
            logger.error("[\(op)] failed to format export file name")
            return nil
        }
        return reply.fileName
    }
}

/// The user-data requests the settings pages make. Synchronous: every call
/// is a `desktop-core` / engine round-trip that may touch SQLite, so callers
/// run it off the main actor. A protocol so the page's paging and work-slot
/// logic can be driven from tests without opening the process-wide user data
/// — the test process shares one engine, and an open there would reach every
/// later fetch in the run.
protocol UserDataClient: Sendable {
    /// One page of the words `filter` matches, each row's `displayRoman`
    /// spelled in `inputMode`.
    func list(filter: String, limit: Int, offset: Int, inputMode: InputMode) throws -> UserDataListing<CustomDictionaryRow>
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
        kind: Taigi_DesktopShell_LearningRecordKind,
        order: Taigi_DesktopShell_LearningRecordOrder,
        filter: String,
        limit: Int,
        offset: Int,
    ) throws -> UserDataListing<Taigi_DesktopShell_LearningRecord>
    /// False when the row is gone — deleted, evicted, or its id taken by
    /// another word since it was listed.
    func setLearningRecordCount(_ record: Taigi_DesktopShell_LearningRecord, count: Int) throws -> Bool
    /// False when the row is gone, as for `setLearningRecordCount`.
    func deleteLearningRecord(_ record: Taigi_DesktopShell_LearningRecord) throws -> Bool
    /// Adds a row's word to the custom dictionary, filed as `roman` / `hanji`
    /// (the word as the user edited it in the Add Word sheet; the engine
    /// trims both) — unless the word is stored there already — then forgets
    /// the learned phrase (a frequency row is never added). A refusal (no romanization,
    /// a full dictionary, a reading that cannot be searched) keeps the row
    /// and is thrown as `UserDataClientError.refused`.
    func addLearningRecordToCustomDictionary(
        _ record: Taigi_DesktopShell_LearningRecord,
        roman: String,
        hanji: String,
    ) throws
}

/// The shipped client: `desktop-core`'s UserData operations via `DesktopCoreBridge`.
struct EngineUserDataClient: UserDataClient {
    private let transport: @Sendable ([UInt8]) -> [UInt8]

    init(transport: @escaping @Sendable ([UInt8]) -> [UInt8] = DesktopCoreBridge.send) {
        self.transport = transport
    }

    func list(filter: String, limit: Int, offset: Int, inputMode: InputMode) throws -> UserDataListing<CustomDictionaryRow> {
        let page = try perform(op: "customDictionaryList", {
            var req = Taigi_DesktopShell_ListCustomEntriesRequest()
            req.filter = filter
            req.limit = UInt32(clamping: max(0, limit))
            req.offset = UInt32(clamping: max(0, offset))
            req.inputMode = inputMode.rawValue
            $0.listCustomEntries = req
        }) {
            if case let .customEntries(entries) = $0 {
                entries
            } else {
                nil
            }
        }
        return UserDataListing(
            rows: page.entries,
            total: Int(page.total),
            matchingTotal: Int(page.matchingTotal),
            offset: Int(page.offset),
        )
    }

    func save(_ row: CustomDictionaryRow) throws {
        _ = try perform(op: "customDictionarySave", {
            var req = Taigi_DesktopShell_SaveCustomEntryRequest()
            req.id = row.id
            req.roman = row.roman
            req.hanji = row.hanji
            $0.saveCustomEntry = req
        }) {
            if case let .customEntrySaved(saved) = $0 {
                saved
            } else {
                nil
            }
        }
    }

    func delete(id: String) throws {
        _ = try perform(op: "customDictionaryDelete", {
            var req = Taigi_DesktopShell_DeleteCustomEntryRequest()
            req.id = id
            $0.deleteCustomEntry = req
        }) {
            if case let .ack(ack) = $0 {
                ack
            } else {
                nil
            }
        }
    }

    func deleteAll() throws {
        _ = try perform(op: "userDataReset", {
            $0.deleteAllCustomEntries = .init()
        }) {
            if case let .ack(ack) = $0 {
                ack
            } else {
                nil
            }
        }
    }

    func exportCSV() throws -> Data {
        let exported = try perform(op: "customDictionaryExportCSV", {
            $0.exportCustomCsv = .init()
        }) {
            if case let .customCsvExported(csv) = $0 {
                csv
            } else {
                nil
            }
        }
        return exported.csv
    }

    func importCSV(at url: URL) throws -> CustomDictionaryImportResult {
        let imported = try perform(op: "customDictionaryImportCSV", {
            var req = Taigi_DesktopShell_ImportCustomCsvFileRequest()
            req.path = url.path(percentEncoded: false)
            $0.importCustomCsvFile = req
        }) {
            if case let .customCsvImported(imported) = $0 {
                imported
            } else {
                nil
            }
        }
        return CustomDictionaryImportResult(
            imported: Int(imported.imported),
            skipped: Int(imported.skipped),
        )
    }

    func exportBackup() throws -> Data {
        let exported = try perform(op: "backupExport", {
            var req = Taigi_DesktopShell_ExportBackupRequest()
            req.appVersion = AppVersion.installed
            $0.exportBackup = req
        }) {
            if case let .backupExported(backup) = $0 {
                backup
            } else {
                nil
            }
        }
        return exported.backup
    }

    func importBackup(at url: URL) throws -> BackupImportResult {
        let imported = try perform(op: "backupImport", {
            var req = Taigi_DesktopShell_ImportBackupFileRequest()
            req.path = url.path(percentEncoded: false)
            $0.importBackupFile = req
        }) {
            if case let .backupImported(imported) = $0 {
                imported
            } else {
                nil
            }
        }
        return BackupImportResult(
            customDictionary: Int(imported.customDictionary),
            frequency: Int(imported.frequency),
            association: Int(imported.association),
        )
    }

    func clearLearningRecords() throws {
        _ = try perform(op: "userDataReset", {
            $0.clearLearningRecords_p = .init()
        }) {
            if case let .ack(ack) = $0 {
                ack
            } else {
                nil
            }
        }
    }

    func listLearningRecords(
        kind: Taigi_DesktopShell_LearningRecordKind,
        order: Taigi_DesktopShell_LearningRecordOrder,
        filter: String,
        limit: Int,
        offset: Int,
    ) throws -> UserDataListing<Taigi_DesktopShell_LearningRecord> {
        let page = try perform(op: "learningRecordsList", {
            var req = Taigi_DesktopShell_ListLearningRecordsRequest()
            req.kind = kind
            req.order = order
            req.filter = filter
            req.limit = UInt32(clamping: max(0, limit))
            req.offset = UInt32(clamping: max(0, offset))
            $0.listLearningRecords = req
        }) {
            if case let .learningRecords(records) = $0 {
                records
            } else {
                nil
            }
        }
        return UserDataListing(
            rows: page.records,
            total: Int(page.total),
            matchingTotal: Int(page.matchingTotal),
            offset: Int(page.offset),
        )
    }

    func setLearningRecordCount(_ record: Taigi_DesktopShell_LearningRecord, count: Int) throws -> Bool {
        let saved = try perform(op: "learningRecordSetCount", {
            var req = Taigi_DesktopShell_SetLearningRecordCountRequest()
            req.record = record
            req.count = Int64(count)
            $0.setLearningRecordCount = req
        }) {
            if case let .learningRecordSaved(saved) = $0 {
                saved
            } else {
                nil
            }
        }
        return saved.hasRecord
    }

    func deleteLearningRecord(_ record: Taigi_DesktopShell_LearningRecord) throws -> Bool {
        let deleted = try perform(op: "learningRecordDelete", {
            var req = Taigi_DesktopShell_DeleteLearningRecordRequest()
            req.record = record
            $0.deleteLearningRecord = req
        }) {
            if case let .learningRecordDeleted(deleted) = $0 {
                deleted
            } else {
                nil
            }
        }
        return deleted.removed
    }

    func addLearningRecordToCustomDictionary(
        _ record: Taigi_DesktopShell_LearningRecord,
        roman: String,
        hanji: String,
    ) throws {
        _ = try perform(op: "learningRecordAddToCustomDictionary", {
            var req = Taigi_DesktopShell_AddLearningRecordToCustomDictionaryRequest()
            req.record = record
            req.roman = roman
            req.hanji = hanji
            $0.addLearningRecordToCustomDictionary = req
        }) {
            if case let .ack(ack) = $0 {
                ack
            } else {
                nil
            }
        }
    }

    private func perform<Result>(
        op: String,
        _ build: (inout Taigi_DesktopShell_UserDataRequest) -> Void,
        extract: (Taigi_DesktopShell_UserDataReply.OneOf_Result) -> Result?,
    ) throws -> Result {
        var request = Taigi_DesktopShell_UserDataRequest()
        build(&request)
        guard
            case let .success(response) = DesktopCoreBridge.response(
                to: .userData(request),
                settings: nil,
                op: op,
                transport: transport,
            ),
            response.error == .ok,
            case let .userData(reply)? = response.reply
        else {
            throw UserDataClientError.engineUnavailable(op: op)
        }
        if reply.hasError {
            throw Self.clientError(from: reply.error, fallbackOp: op)
        }
        guard let result = reply.result.flatMap(extract) else {
            throw UserDataClientError.engineUnavailable(op: op)
        }
        return result
    }

    private static func clientError(
        from error: Taigi_DesktopShell_UserDataError,
        fallbackOp: String,
    ) -> UserDataClientError {
        switch error.kind {
        case let .engineUnavailableOp(op):
            .engineUnavailable(op: op)
        case let .refused(refused):
            .refused(refusal: refused.refusal, detail: refused.detail)
        case let .notEmptied(notEmptied):
            .notEmptied(notEmptied.failures)
        case nil:
            .engineUnavailable(op: fallbackOp)
        }
    }
}

typealias CustomDictionaryRow = Taigi_DesktopShell_CustomDictionaryEntry

extension CustomDictionaryRow: Identifiable {
    init(
        id: String = "",
        roman: String,
        hanji: String,
        displayRoman: String = "",
    ) {
        self.init()
        self.id = id
        self.roman = roman
        self.hanji = hanji
        self.displayRoman = displayRoman
    }
}

/// A learned row is addressed by the store's row id on the page that listed
/// it — a table selection, never the word's identity (`(text, tl)`, which the
/// engine checks on every write).
extension Taigi_DesktopShell_LearningRecord: Identifiable {}
