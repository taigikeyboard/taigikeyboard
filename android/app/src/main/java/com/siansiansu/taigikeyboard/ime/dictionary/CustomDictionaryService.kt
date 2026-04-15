package com.siansiansu.taigikeyboard.ime.dictionary

import android.content.ContentValues
import android.content.Context
import android.database.sqlite.SQLiteDatabase
import android.database.sqlite.SQLiteOpenHelper
import android.net.Uri
import android.util.Log
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels.InputMode
import com.siansiansu.taigikeyboard.util.CsvUtils
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import java.io.BufferedReader
import java.io.InputStreamReader
import java.text.Normalizer
import java.util.UUID

/**
 * Custom dictionary service
 *
 * Handles CRUD, CSV export, and file import for user custom dictionary.
 * Follows the same singleton + SQLiteOpenHelper pattern as UserFrequencyService.
 */
object CustomDictionaryService {
    private const val TAG = "CustomDictionaryService"
    private const val DATABASE_NAME = "custom_dictionary.db"
    private const val DATABASE_VERSION = 5

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

    private var appContext: Context? = null
    private var dbHelper: DatabaseHelper? = null
    private val initMutex = Mutex()
    private var isInitialized = false

    fun init(context: Context) {
        appContext = context.applicationContext
    }

    private suspend fun initialize() {
        if (isInitialized) return
        initMutex.withLock {
            if (isInitialized) return
            val context =
                appContext ?: throw IllegalStateException(
                    "CustomDictionaryService not initialized",
                )
            dbHelper = DatabaseHelper(context)
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

    /**
     * Seed default example entries if the dictionary is empty.
     */
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
                if (BuildConfig.DEBUG) Log.e(TAG, "[FETCH] Failed", e)
                emptyList()
            }
        }

    @Suppress("SqlResolve")
    suspend fun save(entry: Entry) =
        withContext(Dispatchers.IO) {
            try {
                initialize()
                val db = dbHelper?.writableDatabase ?: return@withContext
                val notone = generateNotone(entry.roman)
                val abbrev = generateAbbrev(entry.roman)
                val romanNum = generateRomanNum(entry.roman)
                if (BuildConfig.DEBUG) Log.d(TAG, "[SAVE] roman='${entry.roman}' notone='$notone' abbrev='$abbrev' romanNum='$romanNum'")
                db.execSQL(
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
                    """.trimIndent(),
                    arrayOf(entry.id, entry.roman, entry.hanzi, notone, abbrev, romanNum),
                )
            } catch (e: Exception) {
                if (BuildConfig.DEBUG) Log.e(TAG, "[SAVE] Failed", e)
            }
        }

    suspend fun delete(id: String) =
        withContext(Dispatchers.IO) {
            try {
                initialize()
                val db = dbHelper?.writableDatabase ?: return@withContext
                db.delete(Table.NAME, "${Table.ID} = ?", arrayOf(id))
            } catch (e: Exception) {
                if (BuildConfig.DEBUG) Log.e(TAG, "[DELETE] Failed", e)
            }
        }

    suspend fun deleteAll() =
        withContext(Dispatchers.IO) {
            try {
                initialize()
                val db = dbHelper?.writableDatabase ?: return@withContext
                db.execSQL("DELETE FROM ${Table.NAME}")
            } catch (e: Exception) {
                if (BuildConfig.DEBUG) Log.e(TAG, "[DELETE_ALL] Failed", e)
            }
        }

    /**
     * Search by prefix (for autocomplete integration)
     * @param prefix Preprocessed search prefix (roman_num key for toned, notone key for toneless)
     * @param isToneAware If true, matches against roman_num column; if false, matches against notone column
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
                if (BuildConfig.DEBUG) Log.e(TAG, "[SEARCH] Failed", e)
                emptyList()
            }
        }

    // MARK: - Export

    suspend fun exportCSV(): String =
        withContext(Dispatchers.IO) {
            val entries = fetchAll()
            val sb = StringBuilder()
            for (entry in entries) {
                sb.append("${CsvUtils.escape(entry.roman)},${CsvUtils.escape(entry.hanzi)}\n")
            }
            sb.toString()
        }

    // MARK: - File Import

    private const val MAX_FILE_SIZE = 5L * 1024 * 1024 // 5 MB
    private const val MAX_ENTRY_COUNT = 30_000
    private const val IMPORT_BATCH_SIZE = 500

    @Suppress("SqlResolve")
    suspend fun importFromFile(
        context: Context,
        uri: Uri,
    ): ImportResult =
        withContext(Dispatchers.IO) {
            // Pre-validate file size
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

            // If the file has non-empty content lines but no valid entries, it's a format error
            val hasContentLines = csvString.split("\n").any { it.trim().isNotEmpty() }
            if (entries.isEmpty() && hasContentLines) {
                throw Exception("檔案格式無正確，請使用 CSV 格式（roman,hanzi）")
            }
            if (entries.isEmpty()) return@withContext ImportResult(0, 0)

            // Pre-validate entry count
            if (entries.size > MAX_ENTRY_COUNT) {
                throw Exception("tooManyEntries")
            }

            initialize()
            val db = dbHelper?.writableDatabase ?: return@withContext ImportResult(0, 0)

            // Build set of existing roman|hanzi keys for deduplication
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

            // Process in batches to avoid long-running single transaction
            for (batch in entries.chunked(IMPORT_BATCH_SIZE)) {
                db.beginTransaction()
                try {
                    for (entry in batch) {
                        val key = "${entry.roman}|${entry.hanzi}"
                        if (key in existingKeys) {
                            skippedCount++
                            continue
                        }
                        try {
                            val notone = generateNotone(entry.roman)
                            val abbrev = generateAbbrev(entry.roman)
                            val romanNum = generateRomanNum(entry.roman)
                            db.execSQL(
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
                                """.trimIndent(),
                                arrayOf(entry.id, entry.roman, entry.hanzi, notone, abbrev, romanNum),
                            )
                            existingKeys.add(key)
                            importedCount++
                        } catch (e: Exception) {
                            if (BuildConfig.DEBUG) Log.w(TAG, "[IMPORT] Skipped entry: ${entry.roman}", e)
                        }
                    }
                    db.setTransactionSuccessful()
                } finally {
                    db.endTransaction()
                }
            }

            val totalSkipped = entries.size - importedCount
            if (BuildConfig.DEBUG) Log.i(TAG, "[IMPORT] Imported $importedCount, skipped $totalSkipped (duplicates: $skippedCount)")
            ImportResult(importedCount, totalSkipped)
        }

    /**
     * Returns total entry count, or -1 if DB is not open.
     */
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
                val context = appContext ?: return@withContext
                val dbFile = context.getDatabasePath(DATABASE_NAME)
                if (dbFile.exists()) {
                    dbFile.delete()
                }
            } catch (e: Exception) {
                if (BuildConfig.DEBUG) Log.e(TAG, "[DELETE_DB] Failed", e)
            }
        }

    // MARK: - Notone / Abbrev Generation

    /**
     * Generate toneless form from romanization.
     * Strips tone diacritics (via NFD), trailing digits, hyphens, and spaces.
     */
    internal fun generateNotone(roman: String): String {
        // Convert POJ nasal markers ⁿ (U+207F) / ᴺ (U+1D3A) → nn
        val withNasalConverted =
            roman
                .lowercase()
                .replace("\u207F", "nn")
                .replace("\u1D3A", "nn")
        val decomposed = Normalizer.normalize(withNasalConverted, Normalizer.Form.NFD)
        return buildString {
            for (cp in decomposed.codePoints().toArray()) {
                // Skip combining marks (Unicode category Mn)
                if (Character.getType(cp) == Character.NON_SPACING_MARK.toInt()) continue
                // Skip digits
                if (cp in '0'.code..'9'.code) continue
                // Skip hyphens and spaces
                if (cp == '-'.code || cp == ' '.code) continue
                appendCodePoint(cp)
            }
        }.let { Normalizer.normalize(it, Normalizer.Form.NFC) }
    }

    /**
     * Generate abbreviation from romanization.
     * Takes first letter of each syllable (split by - or space), removes diacritics.
     * Returns empty string if fewer than 2 syllables.
     */
    internal fun generateAbbrev(roman: String): String {
        val syllables = roman.lowercase().split(Regex("[-\\s]+")).filter { it.isNotEmpty() }
        if (syllables.size < 2) return ""
        return syllables.joinToString("") { syllable ->
            val firstChar = syllable.first().toString()
            val decomposed = Normalizer.normalize(firstChar, Normalizer.Form.NFD)
            buildString {
                decomposed.codePoints().forEach { cp ->
                    if (Character.getType(cp) != Character.NON_SPACING_MARK.toInt()) {
                        appendCodePoint(cp)
                    }
                }
            }.let { Normalizer.normalize(it, Normalizer.Form.NFC) }
        }
    }

    /**
     * Generate numeric-toned form from romanization (for tone-aware search).
     * Converts diacritics to tone digits and strips hyphens.
     * Example: "gâu-tsá" → "gau5tsa2"
     */
    internal fun generateRomanNum(roman: String): String = InputNormalizer.normalize(roman, InputMode.TL)

    // MARK: - CSV Helpers

    internal fun parseCSV(csv: String): List<Entry> {
        val lines = csv.split("\n")
        val entries = mutableListOf<Entry>()

        for (i in lines.indices) {
            val line = lines[i].trim()
            if (line.isEmpty()) continue

            val columns = CsvUtils.parseLine(line)
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
            if (oldVersion < 2) {
                db.execSQL("ALTER TABLE ${Table.NAME} ADD COLUMN ${Table.NOTONE} TEXT DEFAULT '';")
                db.execSQL("ALTER TABLE ${Table.NAME} ADD COLUMN ${Table.ABBREV} TEXT DEFAULT '';")
                db.execSQL("CREATE INDEX IF NOT EXISTS idx_custom_notone ON ${Table.NAME}(${Table.NOTONE});")
                db.execSQL("CREATE INDEX IF NOT EXISTS idx_custom_abbrev ON ${Table.NAME}(${Table.ABBREV});")
                // Backfill existing entries
                val cursor = db.rawQuery("SELECT ${Table.ID}, ${Table.ROMAN} FROM ${Table.NAME}", null)
                cursor.use {
                    while (it.moveToNext()) {
                        val id = it.getString(0)
                        val roman = it.getString(1)
                        db.execSQL(
                            "UPDATE ${Table.NAME} SET ${Table.NOTONE} = ?, ${Table.ABBREV} = ? WHERE ${Table.ID} = ?",
                            arrayOf(generateNotone(roman), generateAbbrev(roman), id),
                        )
                    }
                }
            }
            // v3: Regenerate notone to strip spaces (generateNotone now removes spaces)
            if (oldVersion < 3) {
                val cursor = db.rawQuery("SELECT ${Table.ID}, ${Table.ROMAN} FROM ${Table.NAME}", null)
                cursor.use {
                    while (it.moveToNext()) {
                        val id = it.getString(0)
                        val roman = it.getString(1)
                        db.execSQL(
                            "UPDATE ${Table.NAME} SET ${Table.NOTONE} = ? WHERE ${Table.ID} = ?",
                            arrayOf(generateNotone(roman), id),
                        )
                    }
                }
            }
            // v4: Regenerate notone to handle POJ nasal ⁿ (U+207F)
            if (oldVersion < 4) {
                val cursor = db.rawQuery("SELECT ${Table.ID}, ${Table.ROMAN} FROM ${Table.NAME}", null)
                cursor.use {
                    while (it.moveToNext()) {
                        val id = it.getString(0)
                        val roman = it.getString(1)
                        db.execSQL(
                            "UPDATE ${Table.NAME} SET ${Table.NOTONE} = ? WHERE ${Table.ID} = ?",
                            arrayOf(generateNotone(roman), id),
                        )
                    }
                }
            }
            // v5: Add roman_num column for tone-aware search
            if (oldVersion < 5) {
                db.execSQL("ALTER TABLE ${Table.NAME} ADD COLUMN ${Table.ROMAN_NUM} TEXT DEFAULT '';")
                db.execSQL("CREATE INDEX IF NOT EXISTS idx_custom_roman_num ON ${Table.NAME}(${Table.ROMAN_NUM});")
                val cursor = db.rawQuery("SELECT ${Table.ID}, ${Table.ROMAN} FROM ${Table.NAME}", null)
                cursor.use {
                    while (it.moveToNext()) {
                        val id = it.getString(0)
                        val roman = it.getString(1)
                        db.execSQL(
                            "UPDATE ${Table.NAME} SET ${Table.ROMAN_NUM} = ? WHERE ${Table.ID} = ?",
                            arrayOf(generateRomanNum(roman), id),
                        )
                    }
                }
            }
            if (BuildConfig.DEBUG) {
                Log.i(TAG, "[UPGRADE] Database upgraded from $oldVersion to $newVersion")
            }
        }
    }
}
