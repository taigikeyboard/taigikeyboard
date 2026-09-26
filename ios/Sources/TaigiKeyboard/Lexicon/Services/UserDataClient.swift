// What the app's Dictionary pages ask of the user's data, which the engine
// owns (docs/architecture/user-data-engine-roadmap.md P7b). Mirrors macOS
// `UserDataClient.swift` and Android `UserDataClient.kt`.

import Foundation

/// Rows a `.taigi` restore merged, per store.
struct BackupImportResult: Equatable {
    let customDict: Int
    let frequency: Int
    let association: Int
}

/// A custom-dictionary import the user can be told about. Engine-layer error —
/// a plain typed enum, unaware of the App/Strings presentation layer; the App
/// layer maps each case to a localized message at the display boundary
/// (`CustomDictionaryView.localizedImportMessage`), mirroring Android where
/// `CustomDictionaryScreen` resolves the refusal. `errorDescription` carries an
/// English developer fallback so an unmapped case stays diagnosable.
enum CustomDictionaryError: LocalizedError {
    case invalidCSVData
    case invalidCSVFormat
    case fileTooLarge
    case tooManyEntries

    var errorDescription: String? {
        switch self {
        case .invalidCSVData: "Invalid CSV data"
        case .invalidCSVFormat: "Invalid CSV format"
        case .fileTooLarge: "File too large"
        case .tooManyEntries: "Too many entries"
        }
    }
}

/// A `.taigi` file the engine would not restore.
enum BackupError: LocalizedError {
    case unreadable
    case unsupportedVersion

    var errorDescription: String? {
        switch self {
        case .unreadable: "Not a readable backup file"
        case .unsupportedVersion: "Unsupported backup version"
        }
    }
}

/// The round-trip failed or the engine refused the request (before the open);
/// the bridge recorded why in its diagnostics.
struct UserDataUnavailable: LocalizedError {
    let op: String

    var errorDescription: String? {
        "The engine did not answer \(op)"
    }
}

/// The engine would not do what was asked — an unsearchable romanization, a
/// store a reset could not empty — in its own words.
struct UserDataRefused: LocalizedError {
    let detail: String

    var errorDescription: String? {
        detail
    }
}

/// The user-data requests the app's pages make. Each is an engine round-trip
/// that may wait on SQLite — and right after launch on the engine finishing
/// its takeover of the old files — so the shipped client runs every one off
/// the main actor. A protocol so view models can be driven from tests, which
/// never open the user data (the test process shares the app's engine).
protocol UserDataClient: Sendable {
    /// Every word, newest edit first.
    func listAll() async throws -> [CustomDictionaryEntry]
    /// A refusal (a full dictionary, an unsearchable romanization) throws.
    func save(_ entry: CustomDictionaryEntry) async throws
    func delete(id: String) async throws
    /// Empties the custom dictionary.
    func deleteAll() async throws
    func exportCSV() async throws -> Data
    /// A `roman,hanzi` CSV file the user picked; a refusal throws `CustomDictionaryError`.
    func importCSV(url: URL) async throws -> (imported: Int, skipped: Int)
    /// Empties what the keyboard learned — counts, bigrams, learned phrases —
    /// and leaves the custom dictionary alone.
    func clearLearningRecords() async throws
    /// The dictionary search's lookup, by the key `query` derives under the
    /// settings `inputMode`; empty when nothing matches or the engine did not answer.
    func search(query: String, mode: InputMode, limit: Int) async -> [CustomDictionaryEntry]
    func exportBackup(appVersion: String) async throws -> Data
    /// A `.taigi` file the user picked; a refusal throws `BackupError`.
    func importBackup(url: URL) async throws -> BackupImportResult
}

/// The shipped client: the engine's user-data ops, one at a time on a queue
/// of their own.
struct EngineUserDataClient: UserDataClient {
    /// The page requests, off the main actor. A GCD queue rather than a task
    /// per request: a request can wait on SQLite — right after launch on the
    /// engine finishing its takeover — and that wait must not hold a thread of
    /// the shared pool.
    private static let requests = DispatchQueue(label: "EngineUserDataClient.requests", qos: .userInitiated)

    /// The largest file an import reads. CROSS-PLATFORM INVARIANT — the engine
    /// refuses the same size (`engine/userdata/src/csv.rs`); checked on this
    /// side too so a huge file is refused before it is read into memory.
    static let maxImportFileBytes = 5 * 1024 * 1024

    func listAll() async throws -> [CustomDictionaryEntry] {
        try await engine("customDictionaryList") { RustEngineBridge.customDictionaryList() }
            .entries
            .map(CustomDictionaryEntry.init)
    }

    func save(_ entry: CustomDictionaryEntry) async throws {
        let saved = try await engine("customDictionarySave") {
            RustEngineBridge.customDictionarySave(id: entry.id, roman: entry.roman, hanzi: entry.hanzi)
        }
        if saved.refusal == .full {
            throw CustomDictionaryError.tooManyEntries
        }
        if saved.refusal != .none {
            throw UserDataRefused(detail: saved.detail)
        }
    }

    func delete(id: String) async throws {
        _ = try await engine("customDictionaryDelete") { RustEngineBridge.customDictionaryDelete(id: id) }
    }

    func deleteAll() async throws {
        var reset = Taigi_Engine_ResetUserData()
        reset.customDictionary = true
        try await self.reset(reset)
    }

    func exportCSV() async throws -> Data {
        try await engine("customDictionaryExportCSV") { RustEngineBridge.customDictionaryExportCSV() }
    }

    func importCSV(url: URL) async throws -> (imported: Int, skipped: Int) {
        let csv = try await Self.read(url, sizeLimit: Self.maxImportFileBytes)
        let imported = try await engine("customDictionaryImportCSV") { RustEngineBridge.customDictionaryImportCSV(csv) }
        switch imported.refusal {
        case .none: return (imported: Int(imported.imported), skipped: Int(imported.skipped))
        case .fileTooLarge: throw CustomDictionaryError.fileTooLarge
        case .full: throw CustomDictionaryError.tooManyEntries
        case .notUtf8: throw CustomDictionaryError.invalidCSVData
        default: throw CustomDictionaryError.invalidCSVFormat
        }
    }

    func clearLearningRecords() async throws {
        var reset = Taigi_Engine_ResetUserData()
        reset.frequency = true
        reset.association = true
        reset.learnedPhrases = true
        try await self.reset(reset)
    }

    func search(query: String, mode: InputMode, limit: Int) async -> [CustomDictionaryEntry] {
        let found = try? await engine("customDictionarySearch") {
            RustEngineBridge.customDictionarySearch(query: query, inputMode: mode.rawValue, limit: limit)
        }
        return found?.map(CustomDictionaryEntry.init) ?? []
    }

    func exportBackup(appVersion: String) async throws -> Data {
        try await engine("backupExport") { RustEngineBridge.backupExport(appVersion: appVersion) }
    }

    func importBackup(url: URL) async throws -> BackupImportResult {
        let backup = try await Self.read(url, sizeLimit: nil)
        let imported = try await engine("backupImport") { RustEngineBridge.backupImport(backup) }
        switch imported.refusal {
        case .none:
            return BackupImportResult(
                customDict: Int(imported.customDictionary),
                frequency: Int(imported.frequency),
                association: Int(imported.association),
            )
        case .unsupportedVersion: throw BackupError.unsupportedVersion
        default: throw BackupError.unreadable
        }
    }

    /// Empties the stores `request` selects; every one is attempted, and the
    /// ones that could not be emptied are reported together.
    private func reset(_ request: Taigi_Engine_ResetUserData) async throws {
        let removed = try await engine("userDataReset") { RustEngineBridge.userDataReset(request) }
        if !removed.failures.isEmpty {
            throw UserDataRefused(detail: removed.failures.joined(separator: "\n"))
        }
    }

    /// A file the user picked, read off the main actor under its
    /// security scope — refused before the read when over `sizeLimit`.
    private static func read(_ url: URL, sizeLimit: Int?) async throws -> Data {
        try await withCheckedThrowingContinuation { continuation in
            requests.async {
                continuation.resume(with: Result {
                    let accessing = url.startAccessingSecurityScopedResource()
                    defer {
                        if accessing {
                            url.stopAccessingSecurityScopedResource()
                        }
                    }
                    if let sizeLimit {
                        let size = try url.resourceValues(forKeys: [.fileSizeKey]).fileSize ?? 0
                        if size > sizeLimit {
                            throw CustomDictionaryError.fileTooLarge
                        }
                    }
                    return try Data(contentsOf: url)
                })
            }
        }
    }

    /// One request on `requests`; `nil` (a failed or refused round-trip,
    /// recorded by the bridge) throws.
    private func engine<T: Sendable>(_ op: String, _ request: @escaping @Sendable () -> T?) async throws -> T {
        let answer = await withCheckedContinuation { continuation in
            Self.requests.async { continuation.resume(returning: request()) }
        }
        guard let answer else {
            throw UserDataUnavailable(op: op)
        }
        return answer
    }
}

extension CustomDictionaryEntry {
    init(_ entry: Taigi_Engine_CustomDictionaryEntry) {
        self.init(id: entry.id, roman: entry.roman, hanzi: entry.hanzi)
    }
}
