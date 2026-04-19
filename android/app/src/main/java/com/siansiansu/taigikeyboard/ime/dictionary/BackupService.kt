package com.siansiansu.taigikeyboard.ime.dictionary

import android.content.Context
import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import com.siansiansu.taigikeyboard.ime.text.composing.UserFrequencyService
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject

/**
 * Cross-artifact backup + restore.
 *
 * Exports and imports custom dictionary, user frequency, and next-word
 * associations as a single JSON document for `.taigi` file round-trips.
 * Owned by `CompositionRoot`; delegates to the three concrete services.
 */
class BackupService(
    private val logger: LoggerBackend,
    private val customDict: CustomDictionaryService,
    private val userFreq: UserFrequencyService,
    private val nextWord: NextWordService,
) {
    data class ImportResult(
        val customDict: Int,
        val frequency: Int,
        val association: Int,
    )

    /** Export every user-data artifact as a formatted JSON string. */
    suspend fun exportAll(context: Context): String =
        withContext(Dispatchers.IO) {
            val customEntries = customDict.fetchAll()
            val frequencyData = userFreq.getAllFrequencies()
            val associationData = nextWord.allAssociations()

            val appVersion =
                try {
                    context.packageManager.getPackageInfo(context.packageName, 0).versionName ?: "1.0"
                } catch (_: Exception) {
                    "1.0"
                }

            val json =
                JSONObject().apply {
                    put("version", 1)
                    put(
                        "exportedAt",
                        java.text
                            .SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss'Z'", java.util.Locale.US)
                            .apply {
                                timeZone = java.util.TimeZone.getTimeZone("UTC")
                            }.format(java.util.Date()),
                    )
                    put("platform", "android")
                    put("appVersion", appVersion)

                    put(
                        "customDictionary",
                        JSONArray().apply {
                            for (entry in customEntries) {
                                put(
                                    JSONObject().apply {
                                        put("roman", entry.roman)
                                        put("hanzi", entry.hanzi)
                                    },
                                )
                            }
                        },
                    )

                    put(
                        "userFrequency",
                        JSONArray().apply {
                            for ((word, count) in frequencyData) {
                                put(
                                    JSONObject().apply {
                                        put("word", word)
                                        put("count", count)
                                        put("lastUsed", "")
                                    },
                                )
                            }
                        },
                    )

                    put(
                        "userAssociation",
                        JSONArray().apply {
                            for (entry in associationData) {
                                put(
                                    JSONObject().apply {
                                        put("prevWord", entry.prevWord)
                                        put("prevTl", entry.prevTl)
                                        put("nextWord", entry.nextWord)
                                        put("nextTl", entry.nextTl)
                                        put("count", entry.count)
                                        put("lastUsed", "")
                                    },
                                )
                            }
                        },
                    )
                }

            json.toString(2)
        }

    /** Import all three artifact kinds from a JSON string using merge semantics. */
    suspend fun importAll(jsonString: String): ImportResult =
        withContext(Dispatchers.IO) {
            val json = JSONObject(jsonString)
            val version = json.optInt("version", 0)
            require(version >= 1) { "Unsupported backup version: $version" }

            val customCount = importCustomDictionary(json.optJSONArray("customDictionary"))
            val freqCount = importFrequency(json.optJSONArray("userFrequency"))
            val assocCount = importAssociations(json.optJSONArray("userAssociation"))

            ImportResult(customDict = customCount, frequency = freqCount, association = assocCount)
        }

    private suspend fun importCustomDictionary(array: JSONArray?): Int {
        array ?: return 0
        val existing = customDict.fetchAll()
        val existingPairs = existing.map { "${it.roman}\t${it.hanzi}" }.toSet()

        var imported = 0
        for (i in 0 until array.length()) {
            val obj = array.getJSONObject(i)
            val roman = obj.optString("roman", "")
            val hanzi = obj.optString("hanzi", "")
            if (roman.isEmpty() || hanzi.isEmpty()) continue

            val key = "$roman\t$hanzi"
            if (key in existingPairs) continue

            customDict.save(CustomDictionaryService.Entry(roman = roman, hanzi = hanzi))
            imported++
        }
        return imported
    }

    private suspend fun importFrequency(array: JSONArray?): Int {
        array ?: return 0
        val entries =
            (0 until array.length())
                .map { i ->
                    val obj = array.getJSONObject(i)
                    Pair(obj.optString("word", ""), obj.optInt("count", 1))
                }.filter { it.first.isNotEmpty() }

        return userFreq.batchImportMerge(entries)
    }

    private suspend fun importAssociations(array: JSONArray?): Int {
        array ?: return 0
        val entries =
            (0 until array.length())
                .map { i ->
                    val obj = array.getJSONObject(i)
                    // Normalize prevTl/nextTl to TL format (old backups or cross-platform may contain POJ).
                    NextWordService.AssociationEntry(
                        prevWord = obj.optString("prevWord", ""),
                        prevTl = TaigiPhonetics.pojDisplayToTLDisplay(obj.optString("prevTl", "")),
                        nextWord = obj.optString("nextWord", ""),
                        nextTl = TaigiPhonetics.pojDisplayToTLDisplay(obj.optString("nextTl", "")),
                        count = obj.optInt("count", 1),
                    )
                }.filter { it.prevWord.isNotEmpty() && it.nextWord.isNotEmpty() }

        return nextWord.batchImportAssociations(entries)
    }
}
