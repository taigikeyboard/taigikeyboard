// What the settings screens ask of the user's data, which the engine owns
// (docs/architecture/user-data-engine-roadmap.md P8b). Mirrors macOS
// UserDataClient.swift.

package com.siansiansu.taigikeyboard.ime.dictionary

import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.engine.backupExport
import com.siansiansu.taigikeyboard.engine.backupImport
import com.siansiansu.taigikeyboard.engine.customDictionaryDelete
import com.siansiansu.taigikeyboard.engine.customDictionaryExportCsv
import com.siansiansu.taigikeyboard.engine.customDictionaryImportCsv
import com.siansiansu.taigikeyboard.engine.customDictionaryList
import com.siansiansu.taigikeyboard.engine.customDictionarySave
import com.siansiansu.taigikeyboard.engine.customDictionarySearch
import com.siansiansu.taigikeyboard.engine.learningRecordAddToCustomDictionary
import com.siansiansu.taigikeyboard.engine.learningRecordDelete
import com.siansiansu.taigikeyboard.engine.learningRecordSetCount
import com.siansiansu.taigikeyboard.engine.learningRecordsList
import com.siansiansu.taigikeyboard.engine.proto.BackupRefusal
import com.siansiansu.taigikeyboard.engine.proto.CustomDictionaryEntry
import com.siansiansu.taigikeyboard.engine.proto.CustomDictionaryRefusal
import com.siansiansu.taigikeyboard.engine.proto.LearningRecord
import com.siansiansu.taigikeyboard.engine.proto.LearningRecordKind
import com.siansiansu.taigikeyboard.engine.proto.LearningRecordOrder
import com.siansiansu.taigikeyboard.engine.proto.LearningRecords
import com.siansiansu.taigikeyboard.engine.proto.ResetUserData
import com.siansiansu.taigikeyboard.engine.userDataReset
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.util.UUID

/** One word in the user's own dictionary, as the screens show and edit it. */
data class CustomDictionaryWord(
    /** Stable across edits: the engine stores an edit under the same id. */
    val id: String = UUID.randomUUID().toString(),
    val roman: String,
    val hanji: String,
)

/** One page of the custom dictionary list, as the engine cut it. */
data class CustomDictionaryPage(
    val words: List<CustomDictionaryWord>,
    /** Every stored word. */
    val total: Int,
    /** The words the filter matches: what paging runs up to. */
    val matchingTotal: Int,
    /** The offset the page really starts at: the engine pulls the request back to a multiple of the limit when it falls in the last page, or the matches shrank under it (`engine/userdata/src/paging.rs`). */
    val offset: Int,
)

data class CustomDictionaryImportResult(
    val imported: Int,
    val skipped: Int,
)

/** Rows a `.taigi` restore merged, per store. */
data class BackupImportResult(
    val customDict: Int,
    val frequency: Int,
    val association: Int,
)

/** Why a user-data request did nothing. */
sealed class UserDataException(
    message: String,
) : Exception(message) {
    /** The round-trip failed or the engine refused the request; the bridge logged why. */
    class EngineUnavailable(
        op: String,
    ) : UserDataException("the engine did not answer $op")

    /** Something the user can be told — a full dictionary, an unusable file — in the engine's words. */
    class Refused(
        val refusal: CustomDictionaryRefusal,
        detail: String,
    ) : UserDataException(detail)

    /** Not a `.taigi` file, or one from before version 1. */
    class BackupRefused(
        val refusal: BackupRefusal,
    ) : UserDataException("backup refused: $refusal")

    /** A reset that emptied some stores and not others, one line per store that failed. */
    class NotEmptied(
        val failures: List<String>,
    ) : UserDataException(failures.joinToString("\n"))
}

/**
 * The user-data requests the settings screens make. Every call is an engine
 * round-trip that may wait on SQLite — and right after launch on the engine
 * finishing its takeover of the old files — so the shipped client runs each
 * on `Dispatchers.IO`. An interface so view models can be driven from JVM
 * tests, which cannot load the engine.
 */
interface UserDataClient {
    /** One page of the words [filter] matches, newest edit first; the engine trims [filter] and cuts the page. [limit] is at least 1. */
    suspend fun listCustomEntries(
        filter: String,
        limit: Int,
        offset: Int,
    ): CustomDictionaryPage

    suspend fun save(word: CustomDictionaryWord)

    suspend fun delete(id: String)

    /** Empties the custom dictionary. */
    suspend fun deleteAll()

    suspend fun exportCsv(): ByteArray

    /** A `roman,hanji` CSV file's bytes, at most [MAX_IMPORT_FILE_BYTES]. */
    suspend fun importCsv(csv: ByteArray): CustomDictionaryImportResult

    /** Empties what the keyboard learned — counts, bigrams, learned phrases — and leaves the custom dictionary alone. */
    suspend fun clearLearningRecords()

    /**
     * The dictionary search's lookup, by the key [query] derives under the
     * settings [inputMode] (`poj` / `tl` / `tps`); empty when nothing matches
     * or the engine did not answer.
     */
    suspend fun search(
        query: String,
        inputMode: String,
        limit: Int,
    ): List<CustomDictionaryWord>

    suspend fun exportBackup(appVersion: String): ByteArray

    suspend fun importBackup(backup: ByteArray): BackupImportResult

    /**
     * One page of what the keyboard learned: the rows of [kind] in [order]
     * whose text or TL holds [filter], [limit] (at least 1) from [offset].
     * The answer's `offset` is the one served — pulled back to the last page
     * that exists when the matches shrank under it.
     */
    suspend fun listLearningRecords(
        kind: LearningRecordKind,
        order: LearningRecordOrder,
        filter: String,
        limit: Int,
        offset: Int,
    ): LearningRecords

    /**
     * Sets the listed [record]'s count (the engine clamps it to 1..1 000 000
     * and keeps the last-used time). The row as stored now, or `null` when it
     * is gone — deleted, evicted, or its id taken by another word.
     */
    suspend fun setLearningRecordCount(
        record: LearningRecord,
        count: Long,
    ): LearningRecord?

    /** Forgets the listed [record]; `false` when it was already gone. */
    suspend fun deleteLearningRecord(record: LearningRecord): Boolean

    /**
     * Adds the listed row [record]'s word to the custom dictionary (TL as
     * roman, text as Hanji; nothing added when the word is already there);
     * the engine then forgets a learned phrase and keeps a frequency row. A refusal — a full dictionary, a reading it cannot
     * store — throws [UserDataException.Refused] and keeps the row.
     */
    suspend fun addLearningRecordToCustomDictionary(record: LearningRecord)

    companion object {
        /**
         * The largest file an import reads. CROSS-PLATFORM INVARIANT — the
         * engine refuses the same size (`engine/userdata/src/csv.rs`); checked
         * on this side too so a huge file is refused before it is read.
         */
        const val MAX_IMPORT_FILE_BYTES: Long = 5L * 1024 * 1024
    }
}

/** The shipped client: the engine's user-data ops, each on `Dispatchers.IO`. */
object EngineUserDataClient : UserDataClient {
    override suspend fun listCustomEntries(
        filter: String,
        limit: Int,
        offset: Int,
    ): CustomDictionaryPage {
        val page = engine("customDictionaryList") { RustEngineBridge.customDictionaryList(filter, limit, offset) }
        return CustomDictionaryPage(
            words = page.entriesList.map(::word),
            total = page.total,
            matchingTotal = page.matchingTotal,
            offset = page.offset,
        )
    }

    override suspend fun save(word: CustomDictionaryWord) {
        val saved = engine("customDictionarySave") { RustEngineBridge.customDictionarySave(word.id, word.roman, word.hanji) }
        if (saved.refusal != CustomDictionaryRefusal.CUSTOM_DICTIONARY_REFUSAL_NONE) {
            throw UserDataException.Refused(saved.refusal, saved.detail)
        }
    }

    override suspend fun delete(id: String) {
        engine("customDictionaryDelete") { RustEngineBridge.customDictionaryDelete(id) }
    }

    override suspend fun deleteAll() = reset(ResetUserData.newBuilder().setCustomDictionary(true).build())

    override suspend fun exportCsv(): ByteArray = engine("customDictionaryExportCsv") { RustEngineBridge.customDictionaryExportCsv() }

    override suspend fun importCsv(csv: ByteArray): CustomDictionaryImportResult {
        val imported = engine("customDictionaryImportCsv") { RustEngineBridge.customDictionaryImportCsv(csv) }
        if (imported.refusal != CustomDictionaryRefusal.CUSTOM_DICTIONARY_REFUSAL_NONE) {
            throw UserDataException.Refused(imported.refusal, imported.detail)
        }
        return CustomDictionaryImportResult(imported = imported.imported, skipped = imported.skipped)
    }

    override suspend fun clearLearningRecords() =
        reset(
            ResetUserData
                .newBuilder()
                .setFrequency(true)
                .setAssociation(true)
                .setLearnedPhrases(true)
                .build(),
        )

    override suspend fun search(
        query: String,
        inputMode: String,
        limit: Int,
    ): List<CustomDictionaryWord> =
        withContext(Dispatchers.IO) {
            RustEngineBridge.customDictionarySearch(query, inputMode, limit)?.map(::word) ?: emptyList()
        }

    override suspend fun exportBackup(appVersion: String): ByteArray = engine("backupExport") { RustEngineBridge.backupExport(appVersion) }

    override suspend fun importBackup(backup: ByteArray): BackupImportResult {
        val imported = engine("backupImport") { RustEngineBridge.backupImport(backup) }
        if (imported.refusal != BackupRefusal.BACKUP_REFUSAL_NONE) {
            throw UserDataException.BackupRefused(imported.refusal)
        }
        return BackupImportResult(
            customDict = imported.customDictionary,
            frequency = imported.frequency,
            association = imported.association,
        )
    }

    override suspend fun listLearningRecords(
        kind: LearningRecordKind,
        order: LearningRecordOrder,
        filter: String,
        limit: Int,
        offset: Int,
    ): LearningRecords = engine("learningRecordsList") { RustEngineBridge.learningRecordsList(kind, order, filter, limit, offset) }

    override suspend fun setLearningRecordCount(
        record: LearningRecord,
        count: Long,
    ): LearningRecord? =
        engine("learningRecordSetCount") { RustEngineBridge.learningRecordSetCount(record, count) }
            .takeIf { it.hasRecord() }
            ?.record

    override suspend fun deleteLearningRecord(record: LearningRecord): Boolean = engine("learningRecordDelete") { RustEngineBridge.learningRecordDelete(record) }.removed

    override suspend fun addLearningRecordToCustomDictionary(record: LearningRecord) {
        val added = engine("learningRecordAddToCustomDictionary") { RustEngineBridge.learningRecordAddToCustomDictionary(record) }
        if (added.refusal != CustomDictionaryRefusal.CUSTOM_DICTIONARY_REFUSAL_NONE) {
            throw UserDataException.Refused(added.refusal, added.detail)
        }
    }

    /** Empties the stores [request] selects; every one is attempted, and the ones that could not be emptied are reported together. */
    private suspend fun reset(request: ResetUserData) {
        val removed = engine("userDataReset") { RustEngineBridge.userDataReset(request) }
        if (removed.failuresCount > 0) throw UserDataException.NotEmptied(removed.failuresList)
    }

    /** One request on IO; `null` (a failed or refused round-trip, logged by the bridge) throws. */
    private suspend fun <T : Any> engine(
        op: String,
        request: () -> T?,
    ): T = withContext(Dispatchers.IO) { request() } ?: throw UserDataException.EngineUnavailable(op)

    private fun word(entry: CustomDictionaryEntry) = CustomDictionaryWord(id = entry.id, roman = entry.roman, hanji = entry.hanji)
}
