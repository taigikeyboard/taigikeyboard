package com.siansiansu.taigikeyboard.ime.core

import android.content.Context
import com.siansiansu.taigikeyboard.ime.core.logging.AndroidLoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import com.siansiansu.taigikeyboard.ime.dictionary.BackupService
import com.siansiansu.taigikeyboard.ime.dictionary.CustomDictionaryService
import com.siansiansu.taigikeyboard.ime.dictionary.LexiconService
import com.siansiansu.taigikeyboard.ime.dictionary.NextWordService
import com.siansiansu.taigikeyboard.ime.dictionary.TrieService
import com.siansiansu.taigikeyboard.ime.text.composing.UserFrequencyService

/**
 * Service-graph composition root.
 *
 * Holds the single instance of every stateful engine service so that the
 * IME service, Settings Activities, and Compose screens all see the same
 * database handles and caches — matching the observable behavior of the
 * prior `object`-singleton pattern.
 *
 * Access through [shared]; call-sites pass any `Context`, the root keys
 * itself on `applicationContext`. This interim shape replaces `object`
 * singletons without introducing an `Application` subclass (that decision
 * is deferred to A3 per `android-state-audit.md` §8 #1). A7 extends this
 * graph to include IME managers (TextInputManager / SmartbarManager /
 * MediaInputManager).
 */
class CompositionRoot private constructor(
    appContext: Context,
) {
    val logger: LoggerBackend = AndroidLoggerBackend()
    val trie: TrieService = TrieService(appContext, logger)
    val customDict: CustomDictionaryService = CustomDictionaryService(appContext, logger)
    val userFreq: UserFrequencyService = UserFrequencyService(appContext, logger)
    val nextWord: NextWordService = NextWordService(appContext, logger)
    val lexicon: LexiconService = LexiconService(appContext, logger, trie, customDict, userFreq)
    val backup: BackupService = BackupService(logger, customDict, userFreq, nextWord)

    companion object {
        @Volatile private var instance: CompositionRoot? = null

        /**
         * Returns the process-wide [CompositionRoot], constructing it on
         * first call with [context]`.applicationContext`. Safe to invoke
         * from any scope (IME service, Activities, ViewModels).
         */
        fun shared(context: Context): CompositionRoot {
            val existing = instance
            if (existing != null) return existing
            return synchronized(this) {
                instance ?: CompositionRoot(context.applicationContext).also { instance = it }
            }
        }
    }
}
