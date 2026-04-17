package com.siansiansu.taigikeyboard.ime.dictionary

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
