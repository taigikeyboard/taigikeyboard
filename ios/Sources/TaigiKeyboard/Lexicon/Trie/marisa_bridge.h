/**
 * MARISA-trie C 橋接層（handle-based multi-trie API）
 *
 * 提供 Swift 可呼叫的 C 函式介面，支援多個 trie 同時載入。
 * 底層使用 MARISA-trie C++ library。
 *
 * 支援 RecordTrie 格式：key\xff + packed_uint32_le (rowid)
 */

#ifndef marisa_bridge_h
#define marisa_bridge_h

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// --- Handle-based multi-trie API ---

typedef int32_t trie_handle_t;

/**
 * 建立並載入 trie（支援多個同時載入）
 * @param path trie 檔案路徑
 * @return handle (>= 0) 或 -1 表示失敗
 */
trie_handle_t trie_create(const char* path);

/**
 * 前綴搜尋（handle-based）
 */
int32_t trie_h_prefix_search(trie_handle_t handle, const char* prefix,
                              int32_t* results, int32_t max_results);

/**
 * 完全匹配查詢（handle-based）
 */
int32_t trie_h_lookup(trie_handle_t handle, const char* key,
                       int32_t* results, int32_t max_results);

/**
 * 取得 key 數量（handle-based）
 */
int32_t trie_h_get_key_count(trie_handle_t handle);

/**
 * 檢查是否已載入（handle-based）
 */
bool trie_h_is_loaded(trie_handle_t handle);

/**
 * 釋放 trie 資源（handle-based）
 */
void trie_h_close(trie_handle_t handle);

#ifdef __cplusplus
}
#endif

#endif /* marisa_bridge_h */
