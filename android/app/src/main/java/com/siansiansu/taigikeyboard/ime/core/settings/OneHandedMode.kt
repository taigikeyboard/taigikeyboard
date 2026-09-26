package com.siansiansu.taigikeyboard.ime.core.settings

/**
 * One-handed keyboard mode: the key area narrows to [KEY_AREA_FRACTION] and
 * docks to one edge.
 *
 * Mirrors iOS `Settings/OneHandedMode.swift`; [storageValue] is the persisted
 * string under `PreferenceKeys.ONE_HANDED_MODE`, identical on both platforms.
 */
enum class OneHandedMode(
    val storageValue: String,
) {
    OFF("off"),
    LEFT("left"),
    RIGHT("right"),
    ;

    /** The opposite side; [OFF] stays [OFF]. */
    val flipped: OneHandedMode
        get() =
            when (this) {
                OFF -> OFF
                LEFT -> RIGHT
                RIGHT -> LEFT
            }

    companion object {
        // CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Settings/OneHandedMode.swift
        // keyAreaFraction (KeyboardKit's dock width). Drift causes silent divergence.
        const val KEY_AREA_FRACTION = 0.8f

        /** Coerce a stored raw string into a mode; unknown / absent values fall back to [OFF]. */
        fun fromStorage(raw: String?): OneHandedMode = entries.firstOrNull { it.storageValue == raw } ?: OFF
    }
}

/**
 * The action behind the toolbar keyboard button — the most recent pick from
 * its long-press callout (choosing Normal there leaves it unchanged).
 *
 * Mirrors iOS `KeyboardToolbarAction`; [storageValue] is the persisted string
 * under `PreferenceKeys.KEYBOARD_TOOLBAR_ACTION`.
 */
enum class KeyboardToolbarAction(
    val storageValue: String,
    /** The one-handed mode this action docks to; `null` for [DISMISS]. */
    val mode: OneHandedMode?,
) {
    DISMISS("dismiss", null),
    LEFT("left", OneHandedMode.LEFT),
    RIGHT("right", OneHandedMode.RIGHT),
    ;

    /** Mode after a tap on the toolbar button while in [current]; `null` = dismiss the keyboard.
     *  A side action toggles between that side and [OneHandedMode.OFF]. */
    fun tapResult(current: OneHandedMode): OneHandedMode? {
        val target = mode ?: return null
        return if (current == target) OneHandedMode.OFF else target
    }

    companion object {
        /** Coerce a stored raw string into an action; unknown / absent values fall back to [DISMISS]. */
        fun fromStorage(raw: String?): KeyboardToolbarAction = entries.firstOrNull { it.storageValue == raw } ?: DISMISS

        /** The action that re-applies [mode]; `null` for [OneHandedMode.OFF] (Normal leaves the action unchanged). */
        fun forMode(mode: OneHandedMode): KeyboardToolbarAction? = entries.firstOrNull { it.mode == mode }
    }
}
