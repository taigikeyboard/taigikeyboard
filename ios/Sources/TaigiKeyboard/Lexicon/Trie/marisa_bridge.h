/**
 * MARISA-trie C 橋接層
 *
 * 提供 Swift 可呼叫的 C 函式介面。
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

/**
 * 載入 trie 檔案
 * @param path trie 檔案路徑
 * @return 是否成功
 */
bool trie_load(const char* path);

/**
 * 前綴搜尋
 * @param prefix 搜尋前綴
 * @param results 輸出緩衝區（rowid 陣列）
 * @param max_results 緩衝區大小
 * @return 實際結果數量
 */
int32_t trie_prefix_search(const char* prefix, int32_t* results, int32_t max_results);

/**
 * 完全匹配查詢
 * @param key 要查詢的 key
 * @param results 輸出緩衝區（rowid 陣列）
 * @param max_results 緩衝區大小
 * @return 實際結果數量
 */
int32_t trie_lookup(const char* key, int32_t* results, int32_t max_results);

/**
 * 取得 trie 中的 key 數量
 * @return key 數量，未載入時回傳 0
 */
int32_t trie_get_key_count(void);

/**
 * 檢查 trie 是否已載入
 * @return 是否已載入
 */
bool trie_is_loaded(void);

/**
 * 釋放 trie 資源
 */
void trie_close(void);

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
