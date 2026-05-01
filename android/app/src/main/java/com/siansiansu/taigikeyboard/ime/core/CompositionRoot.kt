package com.siansiansu.taigikeyboard.ime.core

import android.content.Context
import com.siansiansu.taigikeyboard.ime.core.logging.AndroidLoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import com.siansiansu.taigikeyboard.ime.dictionary.BackupService
import com.siansiansu.taigikeyboard.ime.dictionary.CustomDictionaryService
import com.siansiansu.taigikeyboard.ime.dictionary.LexiconService
import com.siansiansu.taigikeyboard.ime.dictionary.NextWordService
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
 * itself on `applicationContext`. Owned by `TaigiKeyboardApplication`
 * after A7 (`android-state-audit.md` §A7 + §8 #1 resolution). The IME
 * manager graph (TextInputManager / SmartbarManager / MediaInputManager)
 * stays IME-service-scoped and is constructed inside `TaigiKeyboard.onCreate`;
 * it is intentionally NOT held here because manager lifecycles follow the
 * IME service, not the Application process.
 */
class CompositionRoot private constructor(
    appContext: Context,
) {
    val logger: LoggerBackend = AndroidLoggerBackend()
    val customDict: CustomDictionaryService = CustomDictionaryService(appContext, logger)
    val userFreq: UserFrequencyService = UserFrequencyService(appContext, logger)
    val nextWord: NextWordService = NextWordService(appContext, logger)
    val lexicon: LexiconService = LexiconService(appContext, logger, customDict, userFreq)
    val backup: BackupService = BackupService(logger, customDict, userFreq, nextWord)

    /**
     * Lexicon engine readiness gate. Completed by
     * `TaigiKeyboardApplication.installLexiconEngine` on success;
     * `completeExceptionally` on copy/install failure. Callers must use
     * [awaitLexiconReady] (never `lexiconReady.await()` directly) — fail-open
     * per Codex r3173440132: install errors should let queries degrade to
     * empty results, not poison every search invocation.
     */
    val lexiconReady: kotlinx.coroutines.CompletableDeferred<Unit> =
        kotlinx.coroutines.CompletableDeferred()

    /**
     * Suspend until lexicon install completes. Returns `true` on success,
     * `false` if install failed. Failures are logged at install time; this
     * helper is intentionally quiet so query paths don't spam logs.
     */
    suspend fun awaitLexiconReady(): Boolean = try {
        lexiconReady.await()
        true
    } catch (_: Throwable) {
        false
    }

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
