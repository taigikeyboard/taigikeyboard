package com.siansiansu.taigikeyboard.ime.dictionary

import android.content.Context
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import java.io.File
import java.io.FileOutputStream

/**
 * MARISA-trie lookup service backed by a C++ library via JNI.
 * Provides exact-match and prefix search over dictionary indices. Owned
 * by `CompositionRoot`; all JNI state lives in native `g_trie` so only
 * one instance is expected per process.
 */
class TrieService(
    appContext: Context,
    private val logger: LoggerBackend,
) {
    private val appContext: Context = appContext.applicationContext

    @Volatile
    private var isInitialized = false
    private val initMutex = Mutex()

    companion object {
        private const val TAG = "TrieService"
        private const val TRIE_FILE_NAME = "dictionary.trie"

        init {
            try {
                System.loadLibrary("taigi_trie")
                if (BuildConfig.DEBUG) {
                    android.util.Log.i(TAG, "[INIT] Native library loaded")
                }
            } catch (e: UnsatisfiedLinkError) {
                if (BuildConfig.DEBUG) {
                    android.util.Log.e(TAG, "[INIT] Failed to load native library", e)
                }
            }
        }
    }

    /** Copy trie asset on first run and hand it to the native loader. */
    suspend fun init(): Boolean =
        withContext(Dispatchers.IO) {
            if (isInitialized) return@withContext true

            initMutex.withLock {
                if (isInitialized) return@withLock true

                try {
                    val triePath = getTriePath(appContext)
                    val success = nativeLoad(triePath)

                    if (success) {
                        isInitialized = true
                        logger.i(TAG, "[INIT] Trie loaded, keys=${nativeGetKeyCount()}")
                    } else {
                        logger.e(TAG, "[INIT] Failed to load trie")
                    }

                    success
                } catch (e: Exception) {
                    logger.e(TAG, "[INIT] Exception during init", e)
                    false
                }
            }
        }

    /** Prefix search — returns every rowid whose key starts with [prefix]. */
    fun prefixSearch(prefix: String): IntArray {
        if (!isInitialized) {
            logger.w(TAG, "[SEARCH] Trie not initialized")
            return IntArray(0)
        }
        if (prefix.isEmpty()) return IntArray(0)

        return try {
            val bufferSize = maxOf(nativeGetKeyCount(), 1000)
            nativePrefixSearch(prefix, bufferSize)
        } catch (e: Exception) {
            logger.e(TAG, "[SEARCH] Prefix search failed", e)
            IntArray(0)
        }
    }

    /** Exact-match lookup — returns every rowid mapped to [key] (one-to-many). */
    fun lookup(key: String): IntArray {
        if (!isInitialized) {
            logger.w(TAG, "[LOOKUP] Trie not initialized")
            return IntArray(0)
        }
        if (key.isEmpty()) return IntArray(0)

        return try {
            nativeLookup(key)
        } catch (e: Exception) {
            logger.e(TAG, "[LOOKUP] Lookup failed", e)
            IntArray(0)
        }
    }

    /** Current native key count, or 0 when trie is not yet loaded. */
    fun getKeyCount(): Int = if (isInitialized) nativeGetKeyCount() else 0

    /** True once [init] has returned success. */
    val isReady: Boolean get() = isInitialized && nativeIsLoaded()

    /** Release the native trie handle. Safe to call from any thread. */
    @Synchronized
    fun close() {
        if (isInitialized) {
            nativeClose()
            isInitialized = false
            logger.i(TAG, "[CLOSE] Trie closed")
        }
    }

    /** Resolve trie file path, copying from assets when version changes. */
    private fun getTriePath(context: Context): String {
        val trieFile = File(context.filesDir, TRIE_FILE_NAME)
        val versionFile = File(context.filesDir, "trie_app_version.txt")

        val currentAppVersion = BuildConfig.VERSION_CODE
        val lastCopiedVersion =
            if (versionFile.exists()) {
                versionFile.readText().trim().toIntOrNull() ?: 0
            } else {
                0
            }

        if (currentAppVersion > lastCopiedVersion || !trieFile.exists()) {
            try {
                context.assets.open(TRIE_FILE_NAME).use { input ->
                    FileOutputStream(trieFile).use { output ->
                        input.copyTo(output)
                    }
                }
                versionFile.writeText(currentAppVersion.toString())

                logger.i(TAG, "[UPDATE] Trie updated from v$lastCopiedVersion to v$currentAppVersion")
            } catch (e: Exception) {
                logger.e(TAG, "[ERROR] Failed to copy trie from assets", e)
                throw e
            }
        }

        return trieFile.absolutePath
    }

    private external fun nativeLoad(path: String): Boolean

    private external fun nativePrefixSearch(
        prefix: String,
        limit: Int,
    ): IntArray

    private external fun nativeLookup(key: String): IntArray

    private external fun nativeGetKeyCount(): Int

    private external fun nativeIsLoaded(): Boolean

    private external fun nativeClose()
}
