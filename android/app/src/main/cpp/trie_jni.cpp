/**
 * MARISA-trie JNI wrapper
 *
 * 提供 Kotlin 層存取 MARISA-trie 的功能：
 * - 載入 trie 檔案（mmap）
 * - 前綴搜尋（predictive_search）
 * - 完全匹配（lookup）
 *
 * 注意：此實作支援 Python marisa_trie.RecordTrie 格式
 * RecordTrie 編碼格式：utf8_key + \xff + packed_value
 */

#include <jni.h>
#include <android/log.h>
#include <string>
#include <vector>
#include <cstring>
#include <marisa/trie.h>

#define LOG_TAG "TrieJNI"
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO, LOG_TAG, __VA_ARGS__)
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, LOG_TAG, __VA_ARGS__)

// RecordTrie 分隔符（Python marisa_trie 使用 0xFF）
static const char VALUE_SEPARATOR = '\xff';

// 全域 Trie 實例
static marisa::Trie* g_trie = nullptr;

/**
 * 從 RecordTrie raw key 中解析出 rowid
 * 格式：utf8_key + \xff + packed_uint32_le
 */
static int32_t extractRowId(const char* rawKey, size_t length) {
    // 找到分隔符位置
    const char* sep = static_cast<const char*>(memchr(rawKey, VALUE_SEPARATOR, length));
    if (sep == nullptr) {
        return -1;
    }

    size_t valueOffset = (sep - rawKey) + 1;
    size_t valueLength = length - valueOffset;

    // value 應該是 4 bytes (unsigned int, little-endian)
    if (valueLength < 4) {
        return -1;
    }

    // 解析 little-endian uint32
    const unsigned char* valuePtr = reinterpret_cast<const unsigned char*>(rawKey + valueOffset);
    uint32_t rowId = valuePtr[0] |
                     (static_cast<uint32_t>(valuePtr[1]) << 8) |
                     (static_cast<uint32_t>(valuePtr[2]) << 16) |
                     (static_cast<uint32_t>(valuePtr[3]) << 24);

    return static_cast<int32_t>(rowId);
}

extern "C" {

/**
 * 載入 trie 檔案
 * @param path trie 檔案路徑
 * @return 是否成功
 */
JNIEXPORT jboolean JNICALL
Java_com_siansiansu_taigikeyboard_ime_dictionary_TrieService_nativeLoad(
    JNIEnv* env,
    jobject /* this */,
    jstring path
) {
    const char* pathStr = env->GetStringUTFChars(path, nullptr);
    if (pathStr == nullptr) {
        LOGE("Failed to get path string");
        return JNI_FALSE;
    }

    try {
        // 釋放舊的 trie
        if (g_trie != nullptr) {
            delete g_trie;
            g_trie = nullptr;
        }

        // 載入新的 trie（使用 mmap 以節省記憶體）
        g_trie = new marisa::Trie();
        g_trie->mmap(pathStr);

        LOGI("Trie loaded: %s, keys=%zu", pathStr, g_trie->num_keys());
        env->ReleaseStringUTFChars(path, pathStr);
        return JNI_TRUE;

    } catch (const std::exception& e) {
        LOGE("Failed to load trie: %s", e.what());
        env->ReleaseStringUTFChars(path, pathStr);
        return JNI_FALSE;
    }
}

/**
 * 前綴搜尋（支援 RecordTrie 格式）
 * @param prefix 前綴字串
 * @param limit 最大結果數
 * @return 匹配的 rowid 陣列（從 RecordTrie value 解析）
 */
JNIEXPORT jintArray JNICALL
Java_com_siansiansu_taigikeyboard_ime_dictionary_TrieService_nativePrefixSearch(
    JNIEnv* env,
    jobject /* this */,
    jstring prefix,
    jint limit
) {
    if (g_trie == nullptr) {
        LOGE("Trie not loaded");
        return env->NewIntArray(0);
    }

    const char* prefixStr = env->GetStringUTFChars(prefix, nullptr);
    if (prefixStr == nullptr) {
        return env->NewIntArray(0);
    }

    std::vector<jint> results;
    results.reserve(limit);

    try {
        marisa::Agent agent;
        agent.set_query(prefixStr);

        // predictive_search: 找出所有以 prefix 開頭的 key
        // RecordTrie 的 raw key 格式：utf8_key + \xff + packed_value
        while (g_trie->predictive_search(agent)) {
            const char* rawKey = agent.key().ptr();
            size_t rawLength = agent.key().length();

            // 從 raw key 解析 rowid
            int32_t rowId = extractRowId(rawKey, rawLength);
            if (rowId >= 0) {
                results.push_back(rowId);
            }

            if (results.size() >= static_cast<size_t>(limit)) {
                break;
            }
        }

    } catch (const std::exception& e) {
        LOGE("Prefix search failed: %s", e.what());
    }

    env->ReleaseStringUTFChars(prefix, prefixStr);

    // 轉換為 Java int array
    jintArray result = env->NewIntArray(static_cast<jsize>(results.size()));
    if (result != nullptr && !results.empty()) {
        env->SetIntArrayRegion(result, 0, static_cast<jsize>(results.size()), results.data());
    }

    return result;
}

/**
 * 完全匹配查詢（支援 RecordTrie 格式）
 * @param key 要查詢的 key
 * @return 匹配的 rowid 陣列（一個 key 可能對應多個 rowid）
 */
JNIEXPORT jintArray JNICALL
Java_com_siansiansu_taigikeyboard_ime_dictionary_TrieService_nativeLookup(
    JNIEnv* env,
    jobject /* this */,
    jstring key
) {
    if (g_trie == nullptr) {
        LOGE("Trie not loaded");
        return env->NewIntArray(0);
    }

    const char* keyStr = env->GetStringUTFChars(key, nullptr);
    if (keyStr == nullptr) {
        return env->NewIntArray(0);
    }

    std::vector<jint> results;

    try {
        // RecordTrie 的完全匹配：需要用 predictive_search 找出所有
        // 以 "key + \xff" 開頭的 raw key
        std::string queryWithSep = std::string(keyStr) + VALUE_SEPARATOR;

        marisa::Agent agent;
        agent.set_query(queryWithSep.c_str(), queryWithSep.length());

        while (g_trie->predictive_search(agent)) {
            const char* rawKey = agent.key().ptr();
            size_t rawLength = agent.key().length();

            // 確認 raw key 的 user key 部分完全匹配
            // raw key 格式：utf8_key + \xff + value
            size_t keyLen = strlen(keyStr);
            if (rawLength > keyLen + 1 &&
                memcmp(rawKey, keyStr, keyLen) == 0 &&
                rawKey[keyLen] == VALUE_SEPARATOR) {

                int32_t rowId = extractRowId(rawKey, rawLength);
                if (rowId >= 0) {
                    results.push_back(rowId);
                }
            }
        }

    } catch (const std::exception& e) {
        LOGE("Lookup failed: %s", e.what());
    }

    env->ReleaseStringUTFChars(key, keyStr);

    // 轉換為 Java int array
    jintArray result = env->NewIntArray(static_cast<jsize>(results.size()));
    if (result != nullptr && !results.empty()) {
        env->SetIntArrayRegion(result, 0, static_cast<jsize>(results.size()), results.data());
    }

    return result;
}

/**
 * 取得 trie 中的 key 數量
 */
JNIEXPORT jint JNICALL
Java_com_siansiansu_taigikeyboard_ime_dictionary_TrieService_nativeGetKeyCount(
    JNIEnv* /* env */,
    jobject /* this */
) {
    if (g_trie == nullptr) {
        return 0;
    }
    return static_cast<jint>(g_trie->num_keys());
}

/**
 * 檢查 trie 是否已載入
 */
JNIEXPORT jboolean JNICALL
Java_com_siansiansu_taigikeyboard_ime_dictionary_TrieService_nativeIsLoaded(
    JNIEnv* /* env */,
    jobject /* this */
) {
    return (g_trie != nullptr) ? JNI_TRUE : JNI_FALSE;
}

/**
 * 釋放 trie 資源
 */
JNIEXPORT void JNICALL
Java_com_siansiansu_taigikeyboard_ime_dictionary_TrieService_nativeClose(
    JNIEnv* /* env */,
    jobject /* this */
) {
    if (g_trie != nullptr) {
        delete g_trie;
        g_trie = nullptr;
        LOGI("Trie closed");
    }
}

} // extern "C"
