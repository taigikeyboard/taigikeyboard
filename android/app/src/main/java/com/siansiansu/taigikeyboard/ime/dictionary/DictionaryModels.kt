package com.siansiansu.taigikeyboard.ime.dictionary

/**
 * Constants for dictionary operations
 */
object DictionaryConstants {
    const val DATABASE_NAME = "dictionary.db"
    const val DEFAULT_SEARCH_LIMIT = 100
    const val SUBSYSTEM = "com.siansiansu.taigikeyboard"
}

/**
 * Data class representing a Taigi word entry
 * @property id Database row ID
 * @property roman Romanized form with tone marks (POJ or TL)
 * @property hanzi Chinese characters representation (nullable)
 * @property lengthScore 詞庫頻率（frequency），用於排序。值越大代表越常用。
 * @property delimiter 分隔符（NextWord 用，羅馬字模式輸出時使用："-" 或 " "）
 */
data class TaigiWord(
    val id: Int,
    val roman: String,
    val hanzi: String?,
    val lengthScore: Int?,
    val delimiter: String? = null
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
 * Sealed class for dictionary-related errors
 */
sealed class DictionaryError : Exception() {
    object DatabaseNotFound : DictionaryError() {
        override val message: String = "Dictionary database file not found"
    }

    object DatabaseNotAvailable : DictionaryError() {
        override val message: String = "Dictionary database is not available"
    }

    data class DatabaseConnectionFailed(override val message: String) : DictionaryError()

    data class QueryExecutionFailed(override val message: String) : DictionaryError()

    data class QueryPreparationFailed(override val message: String) : DictionaryError()
}
