// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion
package com.siansiansu.taigikeyboard.ime.dictionary

import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.ime.core.settings.InputMode

/**
 * Input normalizer — thin wrapper over [RustEngineBridge] after D9.4.
 *
 * Converts user input to numeric tone format (mode-native spelling). Full
 * pipeline (TPS preprocess + per-syllable diacritic→digit + checked-ending
 * heuristic) lives in Rust as `OP_NORMALIZE_INPUT`. The `mode` parameter
 * is no longer used by the engine (Codex v3 §2 — both platforms ignored
 * it pre-D9.4) but is kept on the public signature for call-site source
 * compatibility; remove in a follow-up sweep.
 */
object InputNormalizer {
    /** Normalize input to Trie query format (TL numeric tones). */
    fun normalize(
        input: String,
        @Suppress("UNUSED_PARAMETER") mode: InputMode,
    ): String {
        if (input.isEmpty()) return ""
        return RustEngineBridge.normalizeInput(input)
    }

    /** Build a search key from raw input (TPS preprocess only). */
    fun buildSearchKey(
        input: String,
        @Suppress("UNUSED_PARAMETER") mode: InputMode,
    ): String {
        if (input.isEmpty()) return ""
        return if (RustEngineBridge.containsTps(input)) RustEngineBridge.tpsToTl(input) else input
    }

    /** True if `input` (after NFD) contains any combining tone mark. */
    fun hasToneMarks(input: String): Boolean = RustEngineBridge.hasToneMarks(input)
}
