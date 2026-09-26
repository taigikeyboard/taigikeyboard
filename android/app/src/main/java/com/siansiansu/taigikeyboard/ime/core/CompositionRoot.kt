// Manual DI composition root (no Hilt) — holds stateful engine services (Lexicon / user data /
// Logger) shared by the IME service, Settings
// Activity, and Compose screens. Owned by TaigiKeyboardApplication; IME-internal managers
// (TextInputManager / SmartbarManager) live and die with the IME service, so they are
// deliberately built in TaigiKeyboard.onCreate instead.

package com.siansiansu.taigikeyboard.ime.core

import android.content.Context
import com.siansiansu.taigikeyboard.ime.core.logging.AndroidLoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import com.siansiansu.taigikeyboard.ime.dictionary.EngineUserDataClient
import com.siansiansu.taigikeyboard.ime.dictionary.LexiconService
import com.siansiansu.taigikeyboard.ime.dictionary.UserDataClient

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
 * after A7 (the 2026-04 Android state audit §A7 + §8 #1 resolution). The IME
 * manager graph (TextInputManager / SmartbarManager / MediaInputManager)
 * stays IME-service-scoped and is constructed inside `TaigiKeyboard.onCreate`;
 * it is intentionally NOT held here because manager lifecycles follow the
 * IME service, not the Application process.
 */
class CompositionRoot private constructor(
    appContext: Context,
) {
    val logger: LoggerBackend = AndroidLoggerBackend()
    val lexicon: LexiconService = LexiconService(appContext, logger)

    /** The user's data — counts, bigrams, custom dictionary, learned phrases — which the engine owns (roadmap P8b). */
    val userData: UserDataClient = EngineUserDataClient

    /** Decoded theme photos (custom-theme photo background), over the app-private photo store. */
    val themeImages: ThemeImageCache = ThemeImageCache(ThemeImageStore.forApp(appContext))

    /** The user-theme store over [prefs], with the photo sweep wired as its mutation hook. */
    fun userThemeStore(prefs: PrefHelper): UserThemeStore = UserThemeStore(read = { prefs.userThemes }, write = { prefs.userThemes = it }, onMutated = themeImages::sweep)

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
     *
     * `CancellationException` is re-thrown unchanged — swallowing it
     * would break structured concurrency, leaving canceled lifecycle
     * scopes (IME service, ViewModel) running lexicon / NextWord query
     * paths to completion with empty-result fallback instead of
     * honoring the cancel (Codex r3173789380).
     */
    suspend fun awaitLexiconReady(): Boolean =
        try {
            lexiconReady.await()
            true
        } catch (ce: kotlinx.coroutines.CancellationException) {
            throw ce
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
