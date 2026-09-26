// User-data slice of the engine bridge: the engine owns the four stores
// (`docs/architecture/user-data-engine-roadmap.md` P6); this side names the
// directory, reports the picks, and drives the Custom Dictionary page.

import Foundation

extension RustEngineBridge {
    /// Opens the engine's stores over `directory` — the four files under the
    /// names every platform shares, which the engine applies — once per
    /// process.
    ///
    /// Rollback journal, as this input method's own stores always used (U3).
    /// Answers at once: the engine puts the stores in use before it returns
    /// (a pick reported meanwhile queues behind the open) and finishes the
    /// takeover — re-derivation and seeding included — on a thread of its
    /// own. A store that cannot open ranks neutrally; the engine logs why.
    static func userDataOpen(directory: URL) {
        var open = Taigi_Engine_OpenUserData()
        open.directory = directory.path
        open.journal = .delete
        open.inBackground = true
        _ = userDataResult(.open(open), op: "userDataOpen")
    }

    /// One pick, as the engine counts it. Best-effort: the engine queues the
    /// write, and a failed round-trip is logged, never surfaced.
    ///
    /// CROSS-PLATFORM INVARIANT — mirrors
    /// `desktop/crates/taigi-desktop-core/src/engine/user_data.rs` `record_usage`.
    static func userDataRecordUsage(_ usage: Usage) {
        var request = Taigi_Engine_RecordUsage()
        request.displayText = usage.displayText
        request.canonicalTl = usage.canonicalTl
        if let hanji = usage.hanji, !hanji.isEmpty {
            request.hanji = hanji
        }
        request.frequencyRecordingDisabled = !usage.isFrequencyRecordingEnabled
        _ = userDataResult(.recordUsage(request), op: "userDataRecordUsage")
    }

    /// Empties the selected stores in place; `nil` when the engine could not.
    static func userDataReset(_ reset: Taigi_Engine_ResetUserData) -> Taigi_Engine_UserDataReset? {
        guard case let .reset(removed)? = userDataResult(.reset(reset), op: "userDataReset") else {
            return nil
        }
        return removed
    }

    // MARK: - Custom dictionary

    static func customDictionaryList(
        filter: String,
        limit: Int,
        offset: Int,
    ) -> Taigi_Engine_CustomEntries? {
        var list = Taigi_Engine_ListCustomEntries()
        list.filter = filter
        list.limit = UInt32(clamping: limit)
        list.offset = UInt32(clamping: offset)
        guard case let .customEntries(entries)? = userDataResult(
            .listCustomEntries(list),
            op: "customDictionaryList",
        ) else { return nil }
        return entries
    }

    /// A new word (`id` nil) or an edit.
    static func customDictionarySave(
        id: String?,
        roman: String,
        hanzi: String,
    ) -> Taigi_Engine_CustomEntrySaved? {
        var save = Taigi_Engine_SaveCustomEntry()
        if let id {
            save.id = id
        }
        save.roman = roman
        save.hanzi = hanzi
        guard case let .customEntrySaved(saved)? = userDataResult(
            .saveCustomEntry(save),
            op: "customDictionarySave",
        ) else { return nil }
        return saved
    }

    static func customDictionaryDelete(id: String) -> Bool? {
        var delete = Taigi_Engine_DeleteCustomEntry()
        delete.id = id
        guard case let .customEntryDeleted(deleted)? = userDataResult(
            .deleteCustomEntry(delete),
            op: "customDictionaryDelete",
        ) else { return nil }
        return deleted.removed
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
    /// query derives under `mode`, the way the keyboard finds them.
    static func customDictionarySearch(
        query: String,
        mode: InputMode,
        limit: Int,
    ) -> [Taigi_Engine_CustomDictionaryEntry]? {
        var search = Taigi_Engine_SearchCustomEntries()
        search.query = query
        search.inputMode = mode.rawValue
        search.limit = UInt32(clamping: limit)
        guard case let .customEntryMatches(matches)? = userDataResult(
            .searchCustomEntries(search),
            op: "customDictionarySearch",
        ) else { return nil }
        return matches.entries
    }

    /// `nil` for a failed round-trip, a refusal (a request before the open),
    /// or an answer of the wrong kind — the caller's `case let` rejects the
    /// last, and every one is logged.
    private static func userDataResult(
        _ method: Taigi_Engine_UserDataRequest.OneOf_Method,
        op: String,
    ) -> Taigi_Engine_UserDataResponse.OneOf_Result? {
        var request = Taigi_Engine_UserDataRequest()
        request.method = method
        guard let payload = roundtrip(payload: .userData(request), op: op) else { return nil }
        guard case let .userData(response) = payload else {
            recordFailure(op: op, message: "expected a user-data payload, got \(payload)")
            return nil
        }
        guard let result = response.result else {
            recordFailure(op: op, message: "response carried no user-data result")
            return nil
        }
        return result
    }
}
