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
}
