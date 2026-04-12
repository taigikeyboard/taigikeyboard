/**
 * MARISA-trie C 橋接層實作（handle-based multi-trie API）
 *
 * 提供 Swift 可呼叫的 C 函式介面，支援多個 trie 同時載入。
 * 參考 Android trie_jni.cpp 實作。
 *
 * RecordTrie 格式：utf8_key + \xff + packed_uint32_le (rowid)
 */

#include "marisa_bridge.h"
#include <marisa/trie.h>
#include <string>
#include <cstring>

// RecordTrie 分隔符（Python marisa_trie 使用 0xFF）
static const char VALUE_SEPARATOR = '\xff';

// Handle-based multi-trie 儲存
static const int MAX_TRIES = 8;
static marisa::Trie* g_tries[MAX_TRIES] = {};

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
 * 內部輔助：取得 handle 對應的 trie 指標
 */
static marisa::Trie* getTrie(trie_handle_t handle) {
    if (handle < 0 || handle >= MAX_TRIES) {
        return nullptr;
    }
    return g_tries[handle];
}

trie_handle_t trie_create(const char* path) {
    if (path == nullptr) {
        return -1;
    }

    // 找空位
    int slot = -1;
    for (int i = 0; i < MAX_TRIES; i++) {
        if (g_tries[i] == nullptr) {
            slot = i;
            break;
        }
    }
    if (slot < 0) {
        return -1; // 滿了
    }

    try {
        auto* trie = new marisa::Trie();
        trie->mmap(path);
        g_tries[slot] = trie;
        return static_cast<trie_handle_t>(slot);
    } catch (...) {
        return -1;
    }
}

int32_t trie_h_prefix_search(trie_handle_t handle, const char* prefix,
                              int32_t* results, int32_t max_results) {
    marisa::Trie* trie = getTrie(handle);
    if (trie == nullptr || prefix == nullptr || results == nullptr || max_results <= 0) {
        return 0;
    }

    int32_t count = 0;
    try {
        marisa::Agent agent;
        agent.set_query(prefix);

        while (trie->predictive_search(agent)) {
            int32_t rowId = extractRowId(agent.key().ptr(), agent.key().length());
            if (rowId >= 0) {
                results[count++] = rowId;
            }
            if (count >= max_results) {
                break;
            }
        }
    } catch (...) {}

    return count;
}

int32_t trie_h_lookup(trie_handle_t handle, const char* key,
                       int32_t* results, int32_t max_results) {
    marisa::Trie* trie = getTrie(handle);
    if (trie == nullptr || key == nullptr || results == nullptr || max_results <= 0) {
        return 0;
    }

    int32_t count = 0;
    try {
        std::string queryWithSep = std::string(key) + VALUE_SEPARATOR;
        marisa::Agent agent;
        agent.set_query(queryWithSep.c_str(), queryWithSep.length());

        size_t keyLen = strlen(key);

        while (trie->predictive_search(agent)) {
            const char* rawKey = agent.key().ptr();
            size_t rawLength = agent.key().length();

            if (rawLength > keyLen + 1 &&
                memcmp(rawKey, key, keyLen) == 0 &&
                rawKey[keyLen] == VALUE_SEPARATOR) {

                int32_t rowId = extractRowId(rawKey, rawLength);
                if (rowId >= 0) {
                    results[count++] = rowId;
                }
            }
            if (count >= max_results) {
                break;
            }
        }
    } catch (...) {}

    return count;
}

int32_t trie_h_get_key_count(trie_handle_t handle) {
    marisa::Trie* trie = getTrie(handle);
    if (trie == nullptr) {
        return 0;
    }
    return static_cast<int32_t>(trie->num_keys());
}

bool trie_h_is_loaded(trie_handle_t handle) {
    return getTrie(handle) != nullptr;
}

void trie_h_close(trie_handle_t handle) {
    if (handle < 0 || handle >= MAX_TRIES) {
        return;
    }
    if (g_tries[handle] != nullptr) {
        delete g_tries[handle];
        g_tries[handle] = nullptr;
    }
}

} // extern "C"
