// 中文: 自訂字典 CRUD + CSV 匯出 + 檔案匯入服務。
// 中文: 持久化於 custom_dictionary.db(SQLiteOpenHelper);user_data SQLite 平台原生(設計如此)。
// 中文: 由 CompositionRoot 持有。對應 iOS CustomDictionaryService.swift。

package com.siansiansu.taigikeyboard.ime.dictionary

import android.content.Context
import android.database.sqlite.SQLiteDatabase
import android.database.sqlite.SQLiteOpenHelper
import android.net.Uri
import androidx.core.database.sqlite.transaction
import com.siansiansu.taigikeyboard.ime.core.db.vacuumBestEffort
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
        private const val DATABASE_VERSION = 7

        /**
         * v3.6.1 R3 cross-mode side-table DDL + indexes + query. `internal` so
         * the JVM SQL-structure test (`CustomDictionaryServiceCrossModeTest`)
         * runs the EXACT production strings against a JDBC in-memory DB — no
         * SQL duplication that could drift from the live query.
         */
        internal const val CREATE_SEARCH_KEY_TABLE_SQL =
            "CREATE TABLE IF NOT EXISTS custom_search_key (" +
                "entry_id TEXT NOT NULL, family TEXT NOT NULL, form TEXT NOT NULL, key TEXT NOT NULL);"
        internal const val CREATE_SEARCH_KEY_LOOKUP_INDEX_SQL =
            "CREATE INDEX IF NOT EXISTS idx_csk_lookup ON custom_search_key(family, form, key);"
        internal const val CREATE_SEARCH_KEY_ENTRY_INDEX_SQL =
            "CREATE INDEX IF NOT EXISTS idx_csk_entry ON custom_search_key(entry_id);"

        // CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Lexicon/Database/CustomDictionaryRepository.swift query.
        // Drift causes silent divergence.
        internal const val SEARCH_SQL =
            "SELECT DISTINCT c.id, c.roman, c.hanzi, c.created_at, c.updated_at " +
                "FROM custom_dictionary c " +
                "JOIN custom_search_key k ON k.entry_id = c.id " +
                "WHERE k.family = ? " +
                "AND k.form IN (?, 'abbrev') " +
                "AND k.key LIKE ? || '%' " +
                "ORDER BY c.roman " +
                "LIMIT ?"

        /**
         * Delete-then-insert the `custom_search_key` rows for one entry. Shared
         * by [executeUpsert] (every save / import) and the v5→v6 backfill.
         * Delete-first keeps the side table consistent when an UPSERT edits the
         * entry's `roman`.
         */
        // 中文: 先刪後插重建 entry 的側表鍵;upsert 路徑與 v5→v6 回填共用;roman 改動時保持一致。
        private fun rewriteSearchKeys(
            db: SQLiteDatabase,
            entryId: String,
            roman: String,
        ) {
            db.execSQL(
                "DELETE FROM ${SearchKeyTable.NAME} WHERE ${SearchKeyTable.ENTRY_ID} = ?",
                arrayOf(entryId),
            )
            for (key in CustomDictionaryDerivation.deriveCustomSearchKeys(roman)) {
                db.execSQL(
                    "INSERT INTO ${SearchKeyTable.NAME} (${SearchKeyTable.ENTRY_ID}, ${SearchKeyTable.FAMILY}, ${SearchKeyTable.FORM}, ${SearchKeyTable.KEY}) VALUES (?, ?, ?, ?)",
                    arrayOf(entryId, key.family, key.form, key.key),
                )
            }
        }
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

    /**
     * Cross-mode search side table (v3.6.1 R3). One row per (entry, family,
     * form) search key produced by `CustomDictionaryDerivation
     * .deriveCustomSearchKeys`. The NEW query path joins here by the current
     * input's family; the legacy `notone` / `abbrev` / `roman_num` columns on
     * `custom_dictionary` stay write-only for backcompat / rollback.
     */
    // 中文: 跨模式搜尋側表 — 一筆 (entry, family, form) 搜尋鍵。新查詢路徑走這張表,
    // 中文: 舊衍生欄位保留為 write-only(回滾用)。
    private object SearchKeyTable {
        const val NAME = "custom_search_key"
        const val ENTRY_ID = "entry_id"
        const val FAMILY = "family"
        const val FORM = "form"
        const val KEY = "key"
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
        // v3.6.1 R3 — refresh the cross-mode side-table keys for this entry.
        // The legacy notone/abbrev/roman_num columns above stay written for
        // backcompat / rollback; the side table is the NEW query path.
        // 中文: R3 — 重建此 entry 的跨模式側表鍵;舊欄位仍寫(回滾用),側表為新查詢路徑。
        rewriteSearchKeys(db, entry.id, entry.roman)
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
                // Capacity guard + write in one transaction (TOCTOU-safe). A NEW
                // id over the cap throws CustomDictionaryFullException → caught
                // below → entry not added (matches iOS silent single-save).
                // 中文: 容量 guard 與寫入同一 transaction;新 id 超過上限會拋出 → 下方吞掉 → 不新增(對齊 iOS)。
                db.transaction {
                    CustomDictionaryCapacityPolicy.guardInsertCapacity(this, entry.id)
                    executeUpsert(this, entry)
                }
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
                // v3.6.1 R3 — drop the entry's side-table search keys too.
                db.delete(SearchKeyTable.NAME, "${SearchKeyTable.ENTRY_ID} = ?", arrayOf(id))
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
                // v3.6.1 R3 — clear the side table alongside the main table.
                db.execSQL("DELETE FROM ${SearchKeyTable.NAME}")
                vacuumBestEffort(db, logger, TAG)
            } catch (e: Exception) {
                logger.e(TAG, "[DELETE_ALL] Failed", e)
            }
        }

    /**
     * Cross-mode prefix search (v3.6.1 R3). Joins `custom_search_key` by the
     * current input's [family]; matches the primary [form] OR `abbrev` rows by
     * [key] prefix. [key] comes from `CustomDictionaryDerivation
     * .deriveCustomQueryKey` (already lowercased family-native form) and is
     * bound verbatim. Returns each entry once (`DISTINCT`).
     *
     * @param family `tl` / `poj` / `tps` — the query key's family.
     * @param form `num` / `notone` — the query key's primary form (`abbrev` is always also matched).
     * @param key Family-native prefix (bound verbatim).
     */
    // 中文: R3 跨模式 prefix 查詢 — 依 family JOIN 側表,比對 primary form 或 abbrev 列的 key 前綴。
    // SQL is the `SEARCH_SQL` companion constant (shared with the JVM test).
    suspend fun search(
        family: String,
        form: String,
        key: String,
        limit: Int = 50,
    ): List<Entry> =
        withContext(Dispatchers.IO) {
            try {
                initialize()
                val db = dbHelper?.readableDatabase ?: return@withContext emptyList()
                val cursor =
                    db.rawQuery(
                        SEARCH_SQL,
                        arrayOf(family, form, key, limit.toString()),
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

            if (entries.size > CustomDictionaryCapacityPolicy.MAX_ENTRIES) {
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

            // Cumulative row cap: grandfather existing rows, stop at MAX_ENTRIES,
            // overflow entries fall into totalSkipped below (no eviction, no throw).
            // 中文: 累計 row 上限 — 既有列 grandfather,到上限停;超量列計入 totalSkipped(不驅逐、不拋錯)。
            val remaining =
                CustomDictionaryCapacityPolicy.remainingCapacity(
                    CustomDictionaryCapacityPolicy.currentEntryCount(db),
                )
            var importedCount = 0
            var skippedCount = 0

            for (batch in entries.chunked(IMPORT_BATCH_SIZE)) {
                if (importedCount >= remaining) break
                db.transaction {
                    for (entry in batch) {
                        if (importedCount >= remaining) return@transaction
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
            createSearchKeyTable(db)
        }

        // v3.6.1 R3 cross-mode search side table + indexes. Shared by [onCreate]
        // and [migrateV5ToV6] (the latter via `IF NOT EXISTS`). DDL strings are
        // companion constants reused by the JVM SQL-structure test.
        // CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Lexicon/Database/CustomDictionarySchema.swift (custom_search_key).
        // Drift causes silent divergence.
        private fun createSearchKeyTable(db: SQLiteDatabase) {
            db.execSQL(CREATE_SEARCH_KEY_TABLE_SQL)
            db.execSQL(CREATE_SEARCH_KEY_LOOKUP_INDEX_SQL)
            db.execSQL(CREATE_SEARCH_KEY_ENTRY_INDEX_SQL)
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
            if (oldVersion < 6) migrateV5ToV6(db)
            if (oldVersion < 7) migrateV6ToV7(db)
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

        /**
         * v5 → v6: add the `custom_search_key` cross-mode side table + indexes
         * and backfill one bundle per existing entry. Non-destructive — the
         * legacy `notone` / `abbrev` / `roman_num` columns are untouched.
         */
        // 中文: v5→v6 — 建 custom_search_key 側表 + 索引,對既有每筆 entry 回填 bundle;非破壞,舊欄位不動。
        private fun migrateV5ToV6(db: SQLiteDatabase) {
            createSearchKeyTable(db)
            regenerateSearchKeys(db)
        }

        /**
         * v6 → v7: re-derive after the POJ spelling-glyph fix. `o͘` (U+0358)
         * used to be dropped from a derived key as if it were a tone
         * diacritic and the nasal ⁿ survived into the tone-aware key as a
         * display glyph, while a query key built from the raw keyboard buffer
         * carries the ASCII `oo` / `nn` the user types — so POJ entries
         * containing either were unreachable from the keyboard. Same shape as
         * v3 → v4, which re-derived for the nasal marker.
         */
        // 中文: v6→v7 — POJ 拼寫字符修正後重新衍生。o͘ (U+0358) 舊版被當聲調砍掉、
        // 中文:   鼻化 ⁿ 則以顯示字符留在 num 鍵裡,查詢端 raw buffer 卻是 ASCII oo/nn,
        // 中文:   含這兩者的 POJ 自訂詞從鍵盤查無。與 v3→v4 (鼻化) 同一種重新衍生。
        private fun migrateV6ToV7(db: SQLiteDatabase) {
            regenerateNotone(db)
            regenerateSearchKeys(db)
        }

        private fun regenerateSearchKeys(db: SQLiteDatabase) {
            forEachRomanRow(db) { id, roman -> rewriteSearchKeys(db, id, roman) }
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
