package com.siansiansu.taigikeyboard.ime.dictionary

import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels.InputMode

/**
 * Constants for dictionary operations
 */
object DictionaryConstants {
    const val DEFAULT_SEARCH_LIMIT = 200
    const val SUBSYSTEM = "com.siansiansu.taigikeyboard"

    const val TRIE_PREFIX_TL = "tl:"
    const val TRIE_PREFIX_POJ = "poj:"
    const val TRIE_PREFIX_HANZI = "hanzi:"

    const val DICT_BIN_NAME = "dictionary.bin"
    const val ASSOC_BIN_NAME = "association.bin"

    fun triePrefix(mode: InputMode): String =
        when (mode) {
            InputMode.POJ -> TRIE_PREFIX_POJ
            else -> TRIE_PREFIX_TL
        }
}

/**
 * Data class representing a Taigi word entry
 * @property id Database row ID
 * @property roman Romanized form with tone marks (POJ or TL)
 * @property hanzi Chinese characters representation (nullable)
 * @property lengthScore 詞庫頻率（frequency），用於排序。值越大代表越常用。
 */
data class TaigiWord(
    val id: Int,
    val roman: String,
    val hanzi: String?,
    val lengthScore: Int?,
) {
    /**
     * Display text prioritizes hanzi over roman
     */
    val displayText: String
        get() = if (!hanzi.isNullOrEmpty()) hanzi else roman
}

/**
 * Enum representing the type of input for dictionary search
 */
sealed class InputType {
    /** Search by romanization without tone marks (e.g., "goa") */
    object RomanWithoutTone : InputType()

    /** Search by romanization with tone marks (e.g., "góa") */
    object RomanWithTone : InputType()

    /** Search by Chinese characters (e.g., "我") */
    object Hanzi : InputType()
}

/**
 * Breakdown of candidate score components (single source of truth).
 * Used by both sorting and debug logging — no recalculation needed.
 * Aligned with iOS CandidateProcessor.ScoreBreakdown.
 */
data class ScoreBreakdown(
    val userFreqScore: Int,
    val recencyBonus: Int,
    val exactBonus: Int,
    val completionPenalty: Int,
    val closenessBonus: Int,
    val baseFreqScore: Int,
) {
    val total: Int
        get() = userFreqScore + recencyBonus + exactBonus + completionPenalty + closenessBonus + baseFreqScore
}

/**
 * Sealed class for dictionary-related errors
 */
sealed class DictionaryError : Exception() {
    object DatabaseNotFound : DictionaryError() {
        override val message: String = "Dictionary database file not found"
    }

    object DatabaseNotAvailable : DictionaryError() {
        override val message: String = "Dictionary database is not available"
    }

    data class DatabaseConnectionFailed(
        override val message: String,
    ) : DictionaryError()

    data class QueryExecutionFailed(
        override val message: String,
    ) : DictionaryError()

    data class QueryPreparationFailed(
        override val message: String,
    ) : DictionaryError()

    object TrieNotLoaded : DictionaryError() {
        override val message: String = "Trie index not loaded"
    }
}
