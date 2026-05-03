package com.siansiansu.taigikeyboard.ime.text.key

import com.siansiansu.taigikeyboard.engine.CaseTransformBridge
import com.siansiansu.taigikeyboard.ime.core.settings.InputMode

/**
 * R3 mitigation for the case-transform Rust slice — KeyView render-path
 * FFI cost.
 *
 * `KeyView.getComputedLetter` runs once per visible key per redraw frame.
 * Naive per-call FFI = O(visible_keys × frames) JNI hops; with ~30 visible
 * keys per layout pass and frequent invalidates from caps-toggle / layout
 * switches, that's O(thousands) of hops per minute of typing.
 *
 * This object is a process-wide LRU keyed on
 * `(baseLabel, mode, caps, capsLock)`. The cache:
 *   - holds at most ~256 entries (≈ 4 visible keysets × ~16 keys × 4
 *     case-state combos — generous slack)
 *   - evicts least-recently-used on insert past cap (LinkedHashMap
 *     access-order eviction is O(1))
 *   - is invalidated automatically by `KeyView.invalidate` (each redraw
 *     reads through this cache, so stale results never reach the screen
 *     because the lookup key carries the current state)
 *
 * Thread-safety: KeyView.getComputedLetter runs on the Android UI thread
 * exclusively, so the `LinkedHashMap` is fine without synchronization.
 * (If a future change moves rendering off the UI thread, wrap with
 * `synchronized(this)`.)
 */
internal object KeyLabelCaseCache {
    private const val MAX_ENTRIES = 256

    private data class Key(
        val baseLabel: String,
        val mode: InputMode,
        val caps: Boolean,
        val capsLock: Boolean,
    )

    private val cache: LinkedHashMap<Key, String> =
        object : LinkedHashMap<Key, String>(MAX_ENTRIES, 0.75f, /* accessOrder = */ true) {
            override fun removeEldestEntry(eldest: MutableMap.MutableEntry<Key, String>?): Boolean =
                size > MAX_ENTRIES
        }

    /**
     * Look up (or compute via CaseTransformBridge) the case-transformed
     * label for `baseLabel` under the given state. Idempotent and free
     * of side effects beyond the cache update.
     */
    fun getOrCompute(
        baseLabel: String,
        mode: InputMode,
        caps: Boolean,
        capsLock: Boolean,
    ): String {
        val key = Key(baseLabel, mode, caps, capsLock)
        cache[key]?.let { return it }

        val computed = when {
            capsLock -> CaseTransformBridge.fullUppercaseToneString(baseLabel, mode)
            caps -> CaseTransformBridge.uppercaseToneChar(baseLabel, mode)
            else -> CaseTransformBridge.lowercaseToneChar(baseLabel, mode)
        }
        cache[key] = computed
        return computed
    }
}
