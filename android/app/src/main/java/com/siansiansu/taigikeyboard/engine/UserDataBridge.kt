// User-data ops — extensions on RustEngineBridge. The engine owns the four
// stores (docs/architecture/user-data-engine-roadmap.md P8b); this side names
// where the files live, reports the picks, and drives the dictionary pages.
// Mirrors macOS RustEngineBridge+UserData.swift.

package com.siansiansu.taigikeyboard.engine

import com.google.protobuf.UnsafeByteOperations
import com.siansiansu.taigikeyboard.engine.proto.BackupImported
import com.siansiansu.taigikeyboard.engine.proto.CustomCsvImported
import com.siansiansu.taigikeyboard.engine.proto.CustomDictionaryEntry
import com.siansiansu.taigikeyboard.engine.proto.CustomEntries
import com.siansiansu.taigikeyboard.engine.proto.CustomEntryDeleted
import com.siansiansu.taigikeyboard.engine.proto.CustomEntrySaved
import com.siansiansu.taigikeyboard.engine.proto.DeleteCustomEntry
import com.siansiansu.taigikeyboard.engine.proto.ExportBackup
import com.siansiansu.taigikeyboard.engine.proto.ExportCustomCsv
import com.siansiansu.taigikeyboard.engine.proto.ImportBackup
import com.siansiansu.taigikeyboard.engine.proto.ImportCustomCsv
import com.siansiansu.taigikeyboard.engine.proto.ListCustomEntries
import com.siansiansu.taigikeyboard.engine.proto.OpenUserData
import com.siansiansu.taigikeyboard.engine.proto.RecordUsage
import com.siansiansu.taigikeyboard.engine.proto.ResetUserData
import com.siansiansu.taigikeyboard.engine.proto.SaveCustomEntry
import com.siansiansu.taigikeyboard.engine.proto.SearchCustomEntries
import com.siansiansu.taigikeyboard.engine.proto.UserDataJournal
import com.siansiansu.taigikeyboard.engine.proto.UserDataRequest
import com.siansiansu.taigikeyboard.engine.proto.UserDataReset
import com.siansiansu.taigikeyboard.engine.proto.UserDataResponse
import java.io.File

/**
 * Opens the engine's stores, once per process: three files in [directory]
 * under the names every platform shares, and `user_association.db` at
 * [associationFile] — where Android has always kept it. Rollback journal, as
 * the phones use (U3). Answers at once: the engine puts the stores in use
 * before it returns (a pick reported meanwhile queues behind the open) and
 * finishes the takeover of the files the old services wrote — re-derivation
 * and seeding included — on a thread of its own. A store that cannot open
 * ranks neutrally; the engine logs why.
 */
fun RustEngineBridge.userDataOpen(
    directory: File,
    associationFile: File,
) {
    val open = OpenUserData
        .newBuilder()
        .setDirectory(directory.absolutePath)
        .setAssociationPath(associationFile.absolutePath)
        .setJournal(UserDataJournal.USER_DATA_JOURNAL_DELETE)
        .setInBackground(true)
        .build()
    userData("userDataOpen") { setOpen(open) }
}

/**
 * One pick, as the engine counts it — its frequency, and the touch of a
 * learned phrase taken whole ([hanji] set). Best-effort: the engine queues the
 * write, and a failed round-trip is logged, never surfaced.
 */
fun RustEngineBridge.userDataRecordUsage(
    displayText: String,
    canonicalTl: String,
    hanji: String?,
) {
    val usage = RecordUsage
        .newBuilder()
        .setDisplayText(displayText)
        .setCanonicalTl(canonicalTl)
        .apply { if (!hanji.isNullOrEmpty()) setHanji(hanji) }
        .build()
    userData("userDataRecordUsage") { setRecordUsage(usage) }
}

/** Empties the selected stores in place; `null` when the engine could not. */
fun RustEngineBridge.userDataReset(reset: ResetUserData): UserDataReset? = userData("userDataReset") { setReset(reset) }?.takeIf { it.hasReset() }?.reset

fun RustEngineBridge.customDictionaryList(
    filter: String,
    limit: Int,
    offset: Int,
): CustomEntries? {
    val list = ListCustomEntries
        .newBuilder()
        .setFilter(filter)
        .setLimit(limit)
        .setOffset(offset)
        .build()
    return userData("customDictionaryList") { setListCustomEntries(list) }
        ?.takeIf { it.hasCustomEntries() }
        ?.customEntries
}

/** A new word or an edit — the engine keys it by [id] either way. */
fun RustEngineBridge.customDictionarySave(
    id: String,
    roman: String,
    hanzi: String,
): CustomEntrySaved? {
    val save = SaveCustomEntry
        .newBuilder()
        .setId(id)
        .setRoman(roman)
        .setHanzi(hanzi)
        .build()
    return userData("customDictionarySave") { setSaveCustomEntry(save) }
        ?.takeIf { it.hasCustomEntrySaved() }
        ?.customEntrySaved
}

fun RustEngineBridge.customDictionaryDelete(id: String): CustomEntryDeleted? {
    val delete = DeleteCustomEntry.newBuilder().setId(id).build()
    return userData("customDictionaryDelete") { setDeleteCustomEntry(delete) }
        ?.takeIf { it.hasCustomEntryDeleted() }
        ?.customEntryDeleted
}

fun RustEngineBridge.customDictionaryImportCsv(csv: ByteArray): CustomCsvImported? {
    val import = ImportCustomCsv.newBuilder().setCsv(UnsafeByteOperations.unsafeWrap(csv)).build()
    return userData("customDictionaryImportCsv") { setImportCustomCsv(import) }
        ?.takeIf { it.hasCustomCsvImported() }
        ?.customCsvImported
}

fun RustEngineBridge.customDictionaryExportCsv(): ByteArray? =
    userData("customDictionaryExportCsv") { setExportCustomCsv(ExportCustomCsv.getDefaultInstance()) }
        ?.takeIf { it.hasCustomCsvExported() }
        ?.customCsvExported
        ?.csv
        ?.toByteArray()

/**
 * The entries a dictionary search finds for [query] — by the key the query
 * derives under the settings [inputMode] (`poj` / `tl` / `tps`; the engine
 * reads TPS as the TL family and upgrades to TPS when the query carries it),
 * the way the keyboard finds them.
 */
fun RustEngineBridge.customDictionarySearch(
    query: String,
    inputMode: String,
    limit: Int,
): List<CustomDictionaryEntry>? {
    val search = SearchCustomEntries
        .newBuilder()
        .setQuery(query)
        .setInputMode(inputMode)
        .setLimit(limit)
        .build()
    return userData("customDictionarySearch") { setSearchCustomEntries(search) }
        ?.takeIf { it.hasCustomEntryMatches() }
        ?.customEntryMatches
        ?.entriesList
}

/** The `.taigi` backup of what the engine holds, written by `android` [appVersion]. */
fun RustEngineBridge.backupExport(appVersion: String): ByteArray? {
    val export = ExportBackup
        .newBuilder()
        .setPlatform("android")
        .setAppVersion(appVersion)
        .build()
    return userData("backupExport") { setExportBackup(export) }
        ?.takeIf { it.hasBackupExported() }
        ?.backupExported
        ?.backup
        ?.toByteArray()
}

fun RustEngineBridge.backupImport(backup: ByteArray): BackupImported? {
    val import = ImportBackup.newBuilder().setBackup(UnsafeByteOperations.unsafeWrap(backup)).build()
    return userData("backupImport") { setImportBackup(import) }
        ?.takeIf { it.hasBackupImported() }
        ?.backupImported
}

/**
 * `null` for a failed round-trip or a refused request (one before the open);
 * the caller's `has…` check rejects an answer of the wrong kind. Every one
 * is logged through [RustEngineBridge.dispatch].
 */
private fun RustEngineBridge.userData(
    op: String,
    method: UserDataRequest.Builder.() -> Unit,
): UserDataResponse? {
    val request = UserDataRequest.newBuilder().apply(method).build()
    val response = dispatch(op) { setUserData(request) } ?: return null
    if (!response.hasUserData()) {
        recordFailure(op, "response carried no user-data payload")
        return null
    }
    return response.userData
}
