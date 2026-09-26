// Where a pick is counted — the engine keeps the count
// (docs/architecture/user-data-engine-roadmap.md P8b).

package com.siansiansu.taigikeyboard.ime.text.composing

import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.engine.userDataRecordUsage

/**
 * One candidate the user took. This side decides WHAT a pick is — the
 * `(displayText, canonicalTl)` identity it counts under — and the engine
 * keeps the count. [hanji] is set for a continuous pick, so a learned phrase
 * taken whole is touched (§50).
 */
data class Usage(
    val displayText: String,
    val canonicalTl: String,
    val hanji: String? = null,
)

/**
 * CROSS-PLATFORM INVARIANT — mirrors macOS `UsageRecorder.swift` and the
 * desktop `UsageRecorder` (`desktop/crates/taigi-desktop-core/src/composing/usage.rs`).
 * An interface so the tap handler can be driven from JVM tests, which cannot
 * load the engine.
 */
fun interface UsageRecorder {
    fun record(usage: Usage)
}

/** The engine's own stores (`UserDataRequest.record_usage`). */
object EngineUsageRecorder : UsageRecorder {
    override fun record(usage: Usage) {
        RustEngineBridge.userDataRecordUsage(usage.displayText, usage.canonicalTl, usage.hanji)
    }
}
