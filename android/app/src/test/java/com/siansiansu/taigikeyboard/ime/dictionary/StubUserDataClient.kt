package com.siansiansu.taigikeyboard.ime.dictionary

import com.siansiansu.taigikeyboard.engine.proto.LearningRecord
import com.siansiansu.taigikeyboard.engine.proto.LearningRecordKind
import com.siansiansu.taigikeyboard.engine.proto.LearningRecordOrder
import com.siansiansu.taigikeyboard.engine.proto.LearningRecords

/**
 * A [UserDataClient] whose every method fails as unused; a test's fake
 * extends it and overrides only what the code under test calls (the JVM
 * cannot load the engine).
 */
open class StubUserDataClient : UserDataClient {
    override suspend fun listCustomEntries(
        filter: String,
        limit: Int,
        offset: Int,
    ): CustomDictionaryPage = error("unused")

    override suspend fun save(word: CustomDictionaryWord): Unit = error("unused")

    override suspend fun delete(id: String): Unit = error("unused")

    override suspend fun deleteAll(): Unit = error("unused")

    override suspend fun exportCsv(): ByteArray = error("unused")

    override suspend fun importCsv(csv: ByteArray): CustomDictionaryImportResult = error("unused")

    override suspend fun clearLearningRecords(): Unit = error("unused")

    override suspend fun search(
        query: String,
        inputMode: String,
        limit: Int,
    ): List<CustomDictionaryWord> = error("unused")

    override suspend fun exportBackup(appVersion: String): ByteArray = error("unused")

    override suspend fun importBackup(backup: ByteArray): BackupImportResult = error("unused")

    override suspend fun listLearningRecords(
        kind: LearningRecordKind,
        order: LearningRecordOrder,
        filter: String,
        limit: Int,
        offset: Int,
    ): LearningRecords = error("unused")

    override suspend fun setLearningRecordCount(
        record: LearningRecord,
        count: Long,
    ): LearningRecord? = error("unused")

    override suspend fun deleteLearningRecord(record: LearningRecord): Boolean = error("unused")

    override suspend fun addLearningRecordToCustomDictionary(record: LearningRecord): Unit = error("unused")
}
