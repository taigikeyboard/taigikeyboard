package com.siansiansu.taigikeyboard.ime.dictionary

import android.content.Context
import android.util.Log
import com.siansiansu.taigikeyboard.BuildConfig
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import java.io.File
import java.io.FileOutputStream

/**
 * MARISA-trie 查詢服務
 *
 * 提供前綴搜尋和完全匹配功能，用於快速查詢詞典索引。
 * 底層使用 C++ MARISA-trie library 透過 JNI 存取。
 */
object TrieService {
    private const val TAG = "TrieService"
    private const val TRIE_FILE_NAME = "dictionary.trie"

    @Volatile
    private var isInitialized = false
    private val initMutex = Mutex()

    init {
        try {
            System.loadLibrary("taigi_trie")
            if (BuildConfig.DEBUG) {
                Log.i(TAG, "[INIT] Native library loaded")
            }
        } catch (e: UnsatisfiedLinkError) {
            Log.e(TAG, "[INIT] Failed to load native library", e)
        }
    }

    /**
     * 初始化 trie（從 assets 複製並載入）
     */
    suspend fun init(context: Context): Boolean = withContext(Dispatchers.IO) {
        if (isInitialized) return@withContext true

        initMutex.withLock {
            if (isInitialized) return@withLock true

            try {
                val triePath = getTriePath(context)
                val success = nativeLoad(triePath)

                if (success) {
                    isInitialized = true
                    if (BuildConfig.DEBUG) {
                        Log.i(TAG, "[INIT] Trie loaded, keys=${nativeGetKeyCount()}")
                    }
                } else {
                    Log.e(TAG, "[INIT] Failed to load trie")
                }

                success
            } catch (e: Exception) {
                Log.e(TAG, "[INIT] Exception during init", e)
                false
            }
        }
    }

    /**
     * 前綴搜尋
     * @param prefix 搜尋前綴
     * @param limit 最大結果數
     * @return 匹配的 rowid 列表
     */
    fun prefixSearch(prefix: String, limit: Int = 1000): IntArray {
        if (!isInitialized) {
            Log.w(TAG, "[SEARCH] Trie not initialized")
            return IntArray(0)
        }
        if (prefix.isEmpty()) return IntArray(0)

        return try {
            nativePrefixSearch(prefix, limit)
        } catch (e: Exception) {
            Log.e(TAG, "[SEARCH] Prefix search failed", e)
            IntArray(0)
        }
    }

    /**
     * 完全匹配查詢
     * @param key 要查詢的 key
     * @param maxResults 最大結果數（notone key 可能對應數百個 rowid，需足夠大以避免截斷）
     * @return 匹配的 rowid 列表（一個 key 可能對應多個 rowid）
     */
    fun lookup(key: String, maxResults: Int = 1000): IntArray {
        if (!isInitialized) {
            Log.w(TAG, "[LOOKUP] Trie not initialized")
            return IntArray(0)
        }
        if (key.isEmpty()) return IntArray(0)

        return try {
            val results = nativeLookup(key)
            if (results.size > maxResults) results.copyOf(maxResults) else results
        } catch (e: Exception) {
            Log.e(TAG, "[LOOKUP] Lookup failed", e)
            IntArray(0)
        }
    }

    /**
     * 取得 trie 中的 key 數量
     */
    fun getKeyCount(): Int {
        return if (isInitialized) nativeGetKeyCount() else 0
    }

    /**
     * 檢查是否已初始化
     */
    val isReady: Boolean get() = isInitialized && nativeIsLoaded()

    /**
     * 釋放資源
     */
    @Synchronized
    fun close() {
        if (isInitialized) {
            nativeClose()
            isInitialized = false
            if (BuildConfig.DEBUG) {
                Log.i(TAG, "[CLOSE] Trie closed")
            }
        }
    }

    /**
     * 取得 trie 檔案路徑，必要時從 assets 複製
     */
    private fun getTriePath(context: Context): String {
        val trieFile = File(context.filesDir, TRIE_FILE_NAME)
        val versionFile = File(context.filesDir, "trie_app_version.txt")

        val currentAppVersion = BuildConfig.VERSION_CODE
        val lastCopiedVersion = if (versionFile.exists()) {
            versionFile.readText().trim().toIntOrNull() ?: 0
        } else {
            0
        }

        // App 版本更新時重新複製 trie
        if (currentAppVersion > lastCopiedVersion || !trieFile.exists()) {
            try {
                context.assets.open(TRIE_FILE_NAME).use { input ->
                    FileOutputStream(trieFile).use { output ->
                        input.copyTo(output)
                    }
                }
                versionFile.writeText(currentAppVersion.toString())

                if (BuildConfig.DEBUG) {
                    Log.i(TAG, "[UPDATE] Trie updated from v$lastCopiedVersion to v$currentAppVersion")
                }
            } catch (e: Exception) {
                Log.e(TAG, "[ERROR] Failed to copy trie from assets", e)
                throw e
            }
        }

        return trieFile.absolutePath
    }

    // Native methods
    private external fun nativeLoad(path: String): Boolean
    private external fun nativePrefixSearch(prefix: String, limit: Int): IntArray
    private external fun nativeLookup(key: String): IntArray
    private external fun nativeGetKeyCount(): Int
    private external fun nativeIsLoaded(): Boolean
    private external fun nativeClose()
}
