// Where a pick is counted — the engine keeps the count
// (docs/architecture/user-data-engine-roadmap.md P8b).

package com.siansiansu.taigikeyboard.ime.text.composing

import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.engine.userDataRecordUsage

/**
 * One English suggestion the user took. This side decides WHAT the pick is —
 * the `(displayText, canonicalTl)` identity it counts under — and the engine
 * keeps the count. A Continuous pick and a NextWord prediction are counted by
 * the engine itself (`CommitContinuous` / `CommitPrediction`).
 */
data class Usage(
    val displayText: String,
    val canonicalTl: String,
)

/**
 * Android-only: iOS records no English pick, and every Taigi pick the engine
 * records itself (`engine/composing/src/transition.rs` `commit_continuous` /
 * `commit_prediction`, `Applied.usage`).
 * An interface so the tap handler can be driven from JVM tests, which cannot
 * load the engine.
 */
fun interface UsageRecorder {
    fun record(usage: Usage)
}

/** The engine's own stores (`UserDataRequest.record_usage`). */
object EngineUsageRecorder : UsageRecorder {
    override fun record(usage: Usage) {
        RustEngineBridge.userDataRecordUsage(usage.displayText, usage.canonicalTl)
    }
}
