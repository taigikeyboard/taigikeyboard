// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// Mirrors iOS Sources/TaigiKeyboard/Input/CharacterInputPipeline.swift.
// endregion

// 中文: TPS(注音)鍵級輸入前處理薄殼。實際邏輯已在 Rust phonetics::tps_input_adjust;
// 中文: 此檔僅將 (char, rawInput) 委派給 RustEngineBridge.tpsInputAdjust。
// 中文: TextInputManager 必須在 TPS layout 才呼叫,本物件不檢查 layout。

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
    data class Adjustment(val char: String, val replaceLast: String?)

    fun adjust(char: String, rawInput: String): Adjustment {
        val outcome = RustEngineBridge.tpsInputAdjust(char, rawInput)
        return Adjustment(outcome.adjusted, outcome.replaceLast)
    }
}
