// 拼音輸入模式列舉 — POJ / TL / English 三種(對應 iOS 還多一個 TPS)。
// fromPrefString 把 DataStore 字串("poj"/"tl"/"tps"/"english")轉成 enum;
// "tps" 對應 TL(共用 TL 表)、未知值退回 POJ(對齊舊 KeyView / TextInputManager 預設)。

package com.siansiansu.taigikeyboard.ime.core.settings

/**
 * User-selected romanization input mode.
 *
 * Mirrors iOS `Settings/InputMode.swift`. Pure Kotlin enum — no platform
 * dependencies — suitable for shared-core extraction.
 *
 * Restored 2026-04-27 after `D9.4 commit 9` over-deleted alongside
 * `ToneConverterModels.kt` (which housed phonetic mappings now owned by
 * Rust). The enum itself is a Composing/UI domain concern, not phonetics.
 *
 * Note: iOS adds a fourth case (`tps`) for Taiwanese Phonetic Symbols.
 * Android keeps three cases here to match the original `when` exhaustion
 * across call sites; aligning with iOS is a separate scope.
 */
enum class InputMode {
    /** Pe̍h-ōe-jī (白話字) */
    POJ,

    /** Tâi-lô (台羅) */
    TL,

    /** English passthrough */
    ENGLISH,
    ;

    companion object {
        /** Coerce a stored preference string (`"poj"` / `"tl"` / `"tps"` /
         *  `"english"`) into an [InputMode]. `"tps"` maps to [TL] because
         *  TPS shares the TL phonetic-table path; unknown values fall back
         *  to [POJ] (matches the legacy `KeyView.getComputedLetter` and
         *  `TextInputManager.handleTaigiInput` defaults). */
        fun fromPrefString(value: String): InputMode =
            when (value) {
                "poj" -> POJ
                "tl", "tps" -> TL
                "english" -> ENGLISH
                else -> POJ
            }
    }
}
