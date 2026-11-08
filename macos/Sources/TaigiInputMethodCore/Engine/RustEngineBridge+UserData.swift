// User-data slice of the engine bridge: the engine owns the four stores
// (`docs/architecture/user-data-engine-roadmap.md` P6) and counts the picks
// itself (R5, `composingCommitContinuous`); the desktop core opens them at
// launch (`DesktopCoreRuntime.prepare`) and this side drives the Custom
// Dictionary and Learning Records pages.

import Foundation

extension RustEngineBridge {
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
        hanji: String,
    ) -> Taigi_Engine_CustomEntrySaved? {
        var save = Taigi_Engine_SaveCustomEntry()
        if let id {
            save.id = id
        }
        save.roman = roman
        save.hanji = hanji
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

    // MARK: - Learning records

    static func learningRecordsList(
        kind: Taigi_Engine_LearningRecordKind,
        order: Taigi_Engine_LearningRecordOrder,
        filter: String,
        limit: Int,
        offset: Int,
    ) -> Taigi_Engine_LearningRecords? {
        var list = Taigi_Engine_ListLearningRecords()
        list.kind = kind
        list.order = order
        list.filter = filter
        list.limit = UInt32(clamping: limit)
        list.offset = UInt32(clamping: offset)
        guard case let .learningRecords(records)? = userDataResult(
            .listLearningRecords(list),
            op: "learningRecordsList",
        ) else { return nil }
        return records
    }

    /// `record` is the row as listed; the answer carries no record when the
    /// row is gone.
    static func learningRecordSetCount(
        _ record: Taigi_Engine_LearningRecord,
        count: Int,
    ) -> Taigi_Engine_LearningRecordSaved? {
        var set = Taigi_Engine_SetLearningRecordCount()
        set.record = record
        set.count = Int64(count)
        guard case let .learningRecordSaved(saved)? = userDataResult(
            .setLearningRecordCount(set),
            op: "learningRecordSetCount",
        ) else { return nil }
        return saved
    }

    /// False when the row is gone.
    static func learningRecordDelete(_ record: Taigi_Engine_LearningRecord) -> Bool? {
        var delete = Taigi_Engine_DeleteLearningRecord()
        delete.record = record
        guard case let .learningRecordDeleted(deleted)? = userDataResult(
            .deleteLearningRecord(delete),
            op: "learningRecordDelete",
        ) else { return nil }
        return deleted.removed
    }

    /// Files a row's word in the custom dictionary as `roman` / `hanji` (the
    /// word as the user edited it), then forgets a learned phrase (a
    /// frequency row stays); the answer's `refusal` says why it stayed. `nil`
    /// for a row the engine does not add too — it refuses the request.
    static func learningRecordAddToCustomDictionary(
        _ record: Taigi_Engine_LearningRecord,
        roman: String,
        hanji: String,
    ) -> Taigi_Engine_LearningRecordAddedToCustomDictionary? {
        var add = Taigi_Engine_AddLearningRecordToCustomDictionary()
        add.record = record
        add.roman = roman
        add.hanji = hanji
        guard case let .learningRecordAddedToCustomDictionary(added)? = userDataResult(
            .addLearningRecordToCustomDictionary(add),
            op: "learningRecordAddToCustomDictionary",
        ) else { return nil }
        return added
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
