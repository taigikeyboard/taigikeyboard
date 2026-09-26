// User-data slice of the engine bridge: the engine owns the four stores
// (`docs/architecture/user-data-engine-roadmap.md` P7b); this side names the
// App Group directory, reports the picks, and drives the Dictionary pages.
// Mirrors macOS `RustEngineBridge+UserData.swift`.

import Foundation

extension RustEngineBridge {
    /// The four files the engine keeps in the directory `userDataOpen` names
    /// — for the backup exclusion, which is this side's (§29).
    static let userDataFileNames = [
        "user_frequency.db",
        "user_association.db",
        "custom_dictionary.db",
        "learned_phrases.db",
    ]

    /// Opens the engine's stores over `directory`, once per process — the app
    /// and its keyboard each open the same App Group files.
    ///
    /// Rollback journal, as the phones always used (U3: the App Group is shared
    /// by two processes, and a suspended one must not hold a `-wal` lock).
    /// With `inBackground` the call answers at once: the engine puts the
    /// stores in use before it returns and finishes the takeover of the files
    /// the old repositories wrote on a thread of its own. Without it the call
    /// waits for that — never on the main thread. A repeat with the same
    /// directory answers the stores' readiness. `false` when the engine did
    /// not open them (a failed round-trip, or other paths than this process
    /// opened).
    @discardableResult
    static func userDataOpen(directory: URL, inBackground: Bool) -> Bool {
        var open = Taigi_Engine_OpenUserData()
        open.directory = directory.path
        open.journal = .delete
        open.inBackground = inBackground
        guard case .opened? = userDataResult(.open(open), op: "userDataOpen") else { return false }
        return true
    }

    /// One pick, as the engine counts it — its frequency, and the touch of a
    /// learned phrase taken whole (`hanji` set). Best-effort: the engine
    /// queues the write, and a failed round-trip is logged, never surfaced.
    static func userDataRecordUsage(displayText: String, canonicalTl: String, hanji: String?) {
        var usage = Taigi_Engine_RecordUsage()
        usage.displayText = displayText
        usage.canonicalTl = canonicalTl
        if let hanji, !hanji.isEmpty {
            usage.hanji = hanji
        }
        _ = userDataResult(.recordUsage(usage), op: "userDataRecordUsage")
    }

    /// Empties the selected stores in place; `nil` when the engine could not.
    static func userDataReset(_ reset: Taigi_Engine_ResetUserData) -> Taigi_Engine_UserDataReset? {
        guard case let .reset(removed)? = userDataResult(.reset(reset), op: "userDataReset") else {
            return nil
        }
        return removed
    }

    // MARK: - Custom dictionary

    /// `limit` 0 lists every entry.
    static func customDictionaryList(limit: UInt32 = 0) -> Taigi_Engine_CustomEntries? {
        var list = Taigi_Engine_ListCustomEntries()
        list.limit = limit
        guard case let .customEntries(entries)? = userDataResult(
            .listCustomEntries(list),
            op: "customDictionaryList",
        ) else { return nil }
        return entries
    }

    /// A new word or an edit — the engine keys it by `id` either way.
    static func customDictionarySave(id: String, roman: String, hanzi: String) -> Taigi_Engine_CustomEntrySaved? {
        var save = Taigi_Engine_SaveCustomEntry()
        save.id = id
        save.roman = roman
        save.hanzi = hanzi
        guard case let .customEntrySaved(saved)? = userDataResult(
            .saveCustomEntry(save),
            op: "customDictionarySave",
        ) else { return nil }
        return saved
    }

    static func customDictionaryDelete(id: String) -> Taigi_Engine_CustomEntryDeleted? {
        var delete = Taigi_Engine_DeleteCustomEntry()
        delete.id = id
        guard case let .customEntryDeleted(deleted)? = userDataResult(
            .deleteCustomEntry(delete),
            op: "customDictionaryDelete",
        ) else { return nil }
        return deleted
    }

    static func customDictionaryImportCSV(_ csv: Data) -> Taigi_Engine_CustomCsvImported? {
        var importing = Taigi_Engine_ImportCustomCsv()
        importing.csv = csv
        guard case let .customCsvImported(imported)? = userDataResult(
            .importCustomCsv(importing),
            op: "customDictionaryImportCSV",
        ) else { return nil }
        return imported
    }

    static func customDictionaryExportCSV() -> Data? {
        guard case let .customCsvExported(exported)? = userDataResult(
            .exportCustomCsv(Taigi_Engine_ExportCustomCsv()),
            op: "customDictionaryExportCSV",
        ) else { return nil }
        return exported.csv
    }

    /// The entries a dictionary search finds for `query` — by the key the
    /// query derives under the settings `inputMode` (`tl` / `poj` / `tps`),
    /// the way the keyboard finds them.
    static func customDictionarySearch(
        query: String,
        inputMode: String,
        limit: Int,
    ) -> [Taigi_Engine_CustomDictionaryEntry]? {
        var search = Taigi_Engine_SearchCustomEntries()
        search.query = query
        search.inputMode = inputMode
        search.limit = UInt32(clamping: limit)
        guard case let .customEntryMatches(matches)? = userDataResult(
            .searchCustomEntries(search),
            op: "customDictionarySearch",
        ) else { return nil }
        return matches.entries
    }

    // MARK: - Backup

    /// The `.taigi` backup of what the engine holds, written by `ios` `appVersion`.
    static func backupExport(appVersion: String) -> Data? {
        var export = Taigi_Engine_ExportBackup()
        export.platform = "ios"
        export.appVersion = appVersion
        guard case let .backupExported(exported)? = userDataResult(
            .exportBackup(export),
            op: "backupExport",
        ) else { return nil }
        return exported.backup
    }

    static func backupImport(_ backup: Data) -> Taigi_Engine_BackupImported? {
        var importing = Taigi_Engine_ImportBackup()
        importing.backup = backup
        guard case let .backupImported(imported)? = userDataResult(
            .importBackup(importing),
            op: "backupImport",
        ) else { return nil }
        return imported
    }

    /// `nil` for a failed round-trip, a refusal (a request before the open),
    /// or an answer of the wrong kind — the caller's `case let` rejects the
    /// last; every one is recorded in the diagnostics.
    private static func userDataResult(
        _ method: Taigi_Engine_UserDataRequest.OneOf_Method,
        op: String,
    ) -> Taigi_Engine_UserDataResponse.OneOf_Result? {
        var userData = Taigi_Engine_UserDataRequest()
        userData.method = method
        var request = Taigi_Engine_Request()
        request.id = nextRequestID()
        request.payload = .userData(userData)
        guard let response = send(request, op: op) else { return nil }
        guard response.error == .ok else {
            recordFailure(op: op, message: "engine returned \(response.error)", code: Int32(response.error.rawValue))
            return nil
        }
        guard case let .userData(answer) = response.payload, let result = answer.result else {
            recordFailure(op: op, message: "missing user-data result")
            return nil
        }
        return result
    }
}
