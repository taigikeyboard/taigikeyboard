package com.siansiansu.taigikeyboard.ime.dictionary

import android.content.Context
import android.database.sqlite.SQLiteDatabase
import android.database.sqlite.SQLiteOpenHelper
import android.net.Uri
import androidx.core.database.sqlite.transaction
import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.debug
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import java.io.BufferedReader
import java.io.InputStreamReader
import java.util.UUID

/**
 * Custom-dictionary CRUD, CSV export, and file import. Persists to
 * `custom_dictionary.db` via `SQLiteOpenHelper`. Owned by `CompositionRoot`.
 */
class CustomDictionaryService(
    appContext: Context,
    private val logger: LoggerBackend,
) {
    private val appContext: Context = appContext.applicationContext

    companion object {
        private const val TAG = "CustomDictionaryService"
        private const val DATABASE_NAME = "custom_dictionary.db"
        private const val DATABASE_VERSION = 5
    }

    private object Table {
        const val NAME = "custom_dictionary"
        const val ID = "id"
        const val ROMAN = "roman"
        const val HANZI = "hanzi"
        const val NOTONE = "notone"
        const val ABBREV = "abbrev"
        const val ROMAN_NUM = "roman_num"
        const val CREATED_AT = "created_at"
        const val UPDATED_AT = "updated_at"
    }

    private var dbHelper: DatabaseHelper? = null
    private val initMutex = Mutex()
    private var isInitialized = false

    /**
     * UPSERT template shared between [save] and [importFromFile]. Both paths
     * previously duplicated this SQL block verbatim (runtime-identical after
     * `trimIndent()`). Single source of truth now.
     */
    private val UPSERT_SQL =
        """
        INSERT INTO ${Table.NAME} (${Table.ID}, ${Table.ROMAN}, ${Table.HANZI}, ${Table.NOTONE}, ${Table.ABBREV}, ${Table.ROMAN_NUM}, ${Table.CREATED_AT}, ${Table.UPDATED_AT})
        VALUES (?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
        ON CONFLICT(${Table.ID}) DO UPDATE SET
            ${Table.ROMAN} = excluded.${Table.ROMAN},
            ${Table.HANZI} = excluded.${Table.HANZI},
            ${Table.NOTONE} = excluded.${Table.NOTONE},
            ${Table.ABBREV} = excluded.${Table.ABBREV},
            ${Table.ROMAN_NUM} = excluded.${Table.ROMAN_NUM},
            ${Table.UPDATED_AT} = CURRENT_TIMESTAMP
        """.trimIndent()

    private fun executeUpsert(
        db: SQLiteDatabase,
        entry: Entry,
    ) {
        val notone = CustomDictionaryDerivation.generateNotone(entry.roman)
        val abbrev = CustomDictionaryDerivation.generateAbbrev(entry.roman)
        val romanNum = CustomDictionaryDerivation.generateRomanNum(entry.roman)
        logger.debug(TAG) { "[UPSERT] roman='${entry.roman}' notone='$notone' abbrev='$abbrev' romanNum='$romanNum'" }
        db.execSQL(UPSERT_SQL, arrayOf(entry.id, entry.roman, entry.hanzi, notone, abbrev, romanNum))
    }

    private suspend fun initialize() {
        if (isInitialized) return
        initMutex.withLock {
            if (isInitialized) return
            dbHelper = DatabaseHelper(appContext, logger)
            isInitialized = true
        }
    }

    // MARK: - Data Model

    data class Entry(
        val id: String = UUID.randomUUID().toString(),
        val roman: String,
        val hanzi: String,
        val createdAt: String = "",
        val updatedAt: String = "",
    )

    data class ImportResult(
        val imported: Int,
        val skipped: Int,
    )

    // MARK: - Default Entries

    private data class DefaultEntry(
        val id: String,
        val roman: String,
        val hanzi: String,
    )

    private val defaultEntries =
        listOf(
            DefaultEntry("default-gau-tsa", "gâu-tsá", "𠢕早"),
            DefaultEntry("default-tsiah-pa-bue", "tsia̍h-pá--buē", "食飽未"),
        )

    /** Seed default example entries when the dictionary is empty (called from `TaigiKeyboard.onCreate`). */
    suspend fun seedDefaultEntryIfEmpty() =
        withContext(Dispatchers.IO) {
            initialize()
            val db = dbHelper?.readableDatabase ?: return@withContext
            val count =
                db.rawQuery("SELECT COUNT(*) FROM ${Table.NAME}", null).use {
                    if (it.moveToFirst()) it.getInt(0) else 0
                }
            if (count > 0) return@withContext
            for (entry in defaultEntries) {
                save(Entry(id = entry.id, roman = entry.roman, hanzi = entry.hanzi))
            }
        }

    // MARK: - CRUD

    suspend fun fetchAll(): List<Entry> =
        withContext(Dispatchers.IO) {
            try {
                initialize()
                val db = dbHelper?.readableDatabase ?: return@withContext emptyList()
                val cursor =
                    db.rawQuery(
                        "SELECT ${Table.ID}, ${Table.ROMAN}, ${Table.HANZI}, ${Table.CREATED_AT}, ${Table.UPDATED_AT} FROM ${Table.NAME} ORDER BY ${Table.UPDATED_AT} DESC",
                        null,
                    )
                val results = mutableListOf<Entry>()
                cursor.use {
                    while (it.moveToNext()) {
                        results.add(
                            Entry(
                                id = it.getString(0),
                                roman = it.getString(1),
                                hanzi = it.getString(2),
                                createdAt = it.getString(3) ?: "",
                                updatedAt = it.getString(4) ?: "",
                            ),
                        )
                    }
                }
                results
            } catch (e: Exception) {
                logger.e(TAG, "[FETCH] Failed", e)
                emptyList()
            }
        }

    @Suppress("SqlResolve")
    suspend fun save(entry: Entry) =
        withContext(Dispatchers.IO) {
            try {
                initialize()
                val db = dbHelper?.writableDatabase ?: return@withContext
                executeUpsert(db, entry)
            } catch (e: Exception) {
                logger.e(TAG, "[SAVE] Failed", e)
            }
        }

    suspend fun delete(id: String) =
        withContext(Dispatchers.IO) {
            try {
                initialize()
                val db = dbHelper?.writableDatabase ?: return@withContext
                db.delete(Table.NAME, "${Table.ID} = ?", arrayOf(id))
            } catch (e: Exception) {
                logger.e(TAG, "[DELETE] Failed", e)
            }
        }

    suspend fun deleteAll() =
        withContext(Dispatchers.IO) {
            try {
                initialize()
                val db = dbHelper?.writableDatabase ?: return@withContext
                db.execSQL("DELETE FROM ${Table.NAME}")
            } catch (e: Exception) {
                logger.e(TAG, "[DELETE_ALL] Failed", e)
            }
        }

    /**
     * Prefix search for autocomplete integration.
     * @param prefix Preprocessed prefix — `roman_num` key when [isToneAware], else `notone` key.
     * @param isToneAware `true` matches against the toned column.
     */
    suspend fun search(
        prefix: String,
        isToneAware: Boolean,
        limit: Int = 50,
    ): List<Entry> =
        withContext(Dispatchers.IO) {
            try {
                initialize()
                val db = dbHelper?.readableDatabase ?: return@withContext emptyList()
                val lowered = prefix.lowercase()
                val matchColumn = if (isToneAware) Table.ROMAN_NUM else Table.NOTONE
                val cursor =
                    db.rawQuery(
                        """SELECT ${Table.ID}, ${Table.ROMAN}, ${Table.HANZI}, ${Table.CREATED_AT}, ${Table.UPDATED_AT}
                   FROM ${Table.NAME}
                   WHERE $matchColumn LIKE ? || '%'
                      OR ${Table.ABBREV} LIKE ? || '%'
                   ORDER BY ${Table.ROMAN} LIMIT ?""",
                        arrayOf(lowered, lowered, limit.toString()),
                    )
                val results = mutableListOf<Entry>()
                cursor.use {
                    while (it.moveToNext()) {
                        results.add(
                            Entry(
                                id = it.getString(0),
                                roman = it.getString(1),
                                hanzi = it.getString(2),
                                createdAt = it.getString(3) ?: "",
                                updatedAt = it.getString(4) ?: "",
                            ),
                        )
                    }
                }
                results
            } catch (e: Exception) {
                logger.e(TAG, "[SEARCH] Failed", e)
                emptyList()
            }
        }

    // MARK: - Export

    suspend fun exportCSV(): String =
        withContext(Dispatchers.IO) {
            val entries = fetchAll()
            val sb = StringBuilder()
            for (entry in entries) {
                sb.append("${DictionaryCsvCodec.escape(entry.roman)},${DictionaryCsvCodec.escape(entry.hanzi)}\n")
            }
            sb.toString()
        }

    // MARK: - File Import

    private val MAX_FILE_SIZE = 5L * 1024 * 1024 // 5 MB
    private val MAX_ENTRY_COUNT = 30_000
    private val IMPORT_BATCH_SIZE = 500

    @Suppress("SqlResolve")
    suspend fun importFromFile(
        context: Context,
        uri: Uri,
    ): ImportResult =
        withContext(Dispatchers.IO) {
            context.contentResolver.openFileDescriptor(uri, "r")?.use { fd ->
                if (fd.statSize > MAX_FILE_SIZE) {
                    throw Exception("fileTooLarge")
                }
            }

            val csvString =
                context.contentResolver.openInputStream(uri)?.use { inputStream ->
                    BufferedReader(InputStreamReader(inputStream, Charsets.UTF_8)).readText()
                } ?: throw Exception("Cannot read file")

            val entries = parseCSV(csvString)

            val hasContentLines = csvString.split("\n").any { it.trim().isNotEmpty() }
            if (entries.isEmpty() && hasContentLines) {
                throw Exception("檔案格式無正確，請使用 CSV 格式（roman,hanzi）")
            }
            if (entries.isEmpty()) return@withContext ImportResult(0, 0)

            if (entries.size > MAX_ENTRY_COUNT) {
                throw Exception("tooManyEntries")
            }

            initialize()
            val db = dbHelper?.writableDatabase ?: return@withContext ImportResult(0, 0)

            val existingKeys = mutableSetOf<String>()
            db
                .rawQuery(
                    "SELECT ${Table.ROMAN}, ${Table.HANZI} FROM ${Table.NAME}",
                    null,
                ).use { cursor ->
                    while (cursor.moveToNext()) {
                        val key = "${cursor.getString(0)}|${cursor.getString(1)}"
                        existingKeys.add(key)
                    }
                }

            var importedCount = 0
            var skippedCount = 0

            for (batch in entries.chunked(IMPORT_BATCH_SIZE)) {
                db.transaction {
                    for (entry in batch) {
                        val key = "${entry.roman}|${entry.hanzi}"
                        if (key in existingKeys) {
                            skippedCount++
                            continue
                        }
                        try {
                            executeUpsert(this, entry)
                            existingKeys.add(key)
                            importedCount++
                        } catch (e: Exception) {
                            logger.w(TAG, "[IMPORT] Skipped entry: ${entry.roman}", e)
                        }
                    }
                }
            }

            val totalSkipped = entries.size - importedCount
            logger.i(TAG, "[IMPORT] Imported $importedCount, skipped $totalSkipped (duplicates: $skippedCount)")
            ImportResult(importedCount, totalSkipped)
        }

    /** Returns total entry count, or -1 if DB is not open. */
    fun totalCount(): Int {
        return try {
            val db = dbHelper?.readableDatabase ?: return -1
            val cursor = db.rawQuery("SELECT COUNT(*) FROM ${Table.NAME}", null)
            cursor.use { if (it.moveToFirst()) it.getInt(0) else -1 }
        } catch (_: Exception) {
            -1
        }
    }

    // MARK: - Database Management

    suspend fun deleteDatabase() =
        withContext(Dispatchers.IO) {
            try {
                dbHelper?.close()
                dbHelper = null
                isInitialized = false
                val dbFile = appContext.getDatabasePath(DATABASE_NAME)
                if (dbFile.exists()) {
                    dbFile.delete()
                }
            } catch (e: Exception) {
                logger.e(TAG, "[DELETE_DB] Failed", e)
            }
        }

    // MARK: - CSV Helpers

    internal fun parseCSV(csv: String): List<Entry> {
        val lines = csv.split("\n")
        val entries = mutableListOf<Entry>()

        for (i in lines.indices) {
            val line = lines[i].trim()
            if (line.isEmpty()) continue

            val columns = DictionaryCsvCodec.parseLine(line)
            if (columns.size < 2) continue

            val roman = columns[0].trim()
            val hanzi = columns[1].trim()
            if (roman.isEmpty() || hanzi.isEmpty()) continue

            entries.add(Entry(roman = roman, hanzi = hanzi))
        }

        return entries
    }

    // MARK: - DatabaseHelper

    private class DatabaseHelper(
        context: Context,
        private val logger: LoggerBackend,
    ) : SQLiteOpenHelper(
            context,
            DATABASE_NAME,
            null,
            DATABASE_VERSION,
        ) {
        override fun onCreate(db: SQLiteDatabase) {
            db.execSQL(
                """
                CREATE TABLE ${Table.NAME} (
                    ${Table.ID} TEXT PRIMARY KEY,
                    ${Table.ROMAN} TEXT NOT NULL,
                    ${Table.HANZI} TEXT NOT NULL,
                    ${Table.NOTONE} TEXT DEFAULT '',
                    ${Table.ABBREV} TEXT DEFAULT '',
                    ${Table.ROMAN_NUM} TEXT DEFAULT '',
                    ${Table.CREATED_AT} TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    ${Table.UPDATED_AT} TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                );
                """.trimIndent(),
            )
            db.execSQL("CREATE INDEX idx_custom_roman ON ${Table.NAME}(${Table.ROMAN});")
            db.execSQL("CREATE INDEX idx_custom_notone ON ${Table.NAME}(${Table.NOTONE});")
            db.execSQL("CREATE INDEX idx_custom_abbrev ON ${Table.NAME}(${Table.ABBREV});")
            db.execSQL("CREATE INDEX idx_custom_roman_num ON ${Table.NAME}(${Table.ROMAN_NUM});")
        }

        override fun onUpgrade(
            db: SQLiteDatabase,
            oldVersion: Int,
            newVersion: Int,
        ) {
            if (oldVersion < 2) migrateV1ToV2(db)
            if (oldVersion < 3) migrateV2ToV3(db)
            if (oldVersion < 4) migrateV3ToV4(db)
            if (oldVersion < 5) migrateV4ToV5(db)
            logger.i(TAG, "[UPGRADE] Database upgraded from $oldVersion to $newVersion")
        }

        /** v1 → v2: add notone/abbrev columns and backfill existing rows. */
        private fun migrateV1ToV2(db: SQLiteDatabase) {
            db.execSQL("ALTER TABLE ${Table.NAME} ADD COLUMN ${Table.NOTONE} TEXT DEFAULT '';")
            db.execSQL("ALTER TABLE ${Table.NAME} ADD COLUMN ${Table.ABBREV} TEXT DEFAULT '';")
            db.execSQL("CREATE INDEX IF NOT EXISTS idx_custom_notone ON ${Table.NAME}(${Table.NOTONE});")
            db.execSQL("CREATE INDEX IF NOT EXISTS idx_custom_abbrev ON ${Table.NAME}(${Table.ABBREV});")
            forEachRomanRow(db) { id, roman ->
                db.execSQL(
                    "UPDATE ${Table.NAME} SET ${Table.NOTONE} = ?, ${Table.ABBREV} = ? WHERE ${Table.ID} = ?",
                    arrayOf(
                        CustomDictionaryDerivation.generateNotone(roman),
                        CustomDictionaryDerivation.generateAbbrev(roman),
                        id,
                    ),
                )
            }
        }

        /** v2 → v3: regenerate notone (generateNotone now strips spaces). */
        private fun migrateV2ToV3(db: SQLiteDatabase) = regenerateNotone(db)

        /** v3 → v4: regenerate notone to handle POJ nasal ⁿ (U+207F). */
        private fun migrateV3ToV4(db: SQLiteDatabase) = regenerateNotone(db)

        /** v4 → v5: add roman_num column for tone-aware search and backfill. */
        private fun migrateV4ToV5(db: SQLiteDatabase) {
            db.execSQL("ALTER TABLE ${Table.NAME} ADD COLUMN ${Table.ROMAN_NUM} TEXT DEFAULT '';")
            db.execSQL("CREATE INDEX IF NOT EXISTS idx_custom_roman_num ON ${Table.NAME}(${Table.ROMAN_NUM});")
            forEachRomanRow(db) { id, roman ->
                db.execSQL(
                    "UPDATE ${Table.NAME} SET ${Table.ROMAN_NUM} = ? WHERE ${Table.ID} = ?",
                    arrayOf(CustomDictionaryDerivation.generateRomanNum(roman), id),
                )
            }
        }

        private fun regenerateNotone(db: SQLiteDatabase) {
            forEachRomanRow(db) { id, roman ->
                db.execSQL(
                    "UPDATE ${Table.NAME} SET ${Table.NOTONE} = ? WHERE ${Table.ID} = ?",
                    arrayOf(CustomDictionaryDerivation.generateNotone(roman), id),
                )
            }
        }

        private inline fun forEachRomanRow(
            db: SQLiteDatabase,
            action: (id: String, roman: String) -> Unit,
        ) {
            db.rawQuery("SELECT ${Table.ID}, ${Table.ROMAN} FROM ${Table.NAME}", null).use {
                while (it.moveToNext()) {
                    action(it.getString(0), it.getString(1))
                }
            }
        }
    }
}
