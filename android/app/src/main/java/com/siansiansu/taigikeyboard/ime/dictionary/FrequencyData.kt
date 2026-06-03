// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion

// 中文: 單一詞的使用者頻次值型別 —(count, lastUsedMillis)。
// 中文: 從 UserFrequencyService 抽出以對齊 iOS Lexicon/Models/FrequencyData.swift。

package com.siansiansu.taigikeyboard.ime.dictionary

/**
 * User-frequency lookup result for a single word. Hoisted from
 * `UserFrequencyService` to mirror iOS `Lexicon/Models/FrequencyData.swift`
 * so that shared-core scoring code can depend on the type without reaching
 * into the platform service.
 */
data class FrequencyData(
    val count: Int,
    val lastUsedMillis: Long,
) {
    companion object {
        val EMPTY = FrequencyData(count = 0, lastUsedMillis = 0L)
    }
}

/**
 * One `user_frequency.db` row in R5 `(word, tl)` pair-key form: the
 * display-text key, its canonical-TL reading, and the snapshot. Returned by
 * `UserFrequencyService.frequencyDataBatch` so the engine can build a
 * `(display_text, canonical_tl)`-keyed `FrequencyMap` (Core Principle #7).
 * `tl == ""` is the legacy fallback bucket. Mirrors iOS `FrequencyRow`.
 */
// 中文: R5 (word, tl) pair-key 的一列 — 顯示鍵 + canonical TL 讀音 + 快照;tl='' 為 legacy fallback 桶。
data class FrequencyRow(
    val word: String,
    val tl: String,
    val data: FrequencyData,
)
