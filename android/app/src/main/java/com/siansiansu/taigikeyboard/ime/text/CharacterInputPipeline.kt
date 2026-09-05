// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// Mirrors iOS Sources/TaigiKeyboard/Input/CharacterInputPipeline.swift.
// endregion

package com.siansiansu.taigikeyboard.ime.text

import com.siansiansu.taigikeyboard.engine.RustEngineBridge

/**
 * Pure-function pipeline for TPS key-level character preprocessing.
 *
 * D9.4: thin wrapper over `RustEngineBridge.tpsInputAdjust` (Rust owns
 * the orchestration). Caller (`TextInputManager`) MUST gate by TPS layout.
 */
object CharacterInputPipeline {
    /** Result of TPS key-level adjustment. */
    data class Adjustment(
        val char: String,
        val replaceLast: String?,
    )

    fun adjust(
        char: String,
        rawInput: String,
    ): Adjustment {
        val outcome = RustEngineBridge.tpsInputAdjust(char, rawInput)
        return Adjustment(outcome.adjusted, outcome.replaceLast)
    }
}
