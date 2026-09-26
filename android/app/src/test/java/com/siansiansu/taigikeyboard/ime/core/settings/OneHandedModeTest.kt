package com.siansiansu.taigikeyboard.ime.core.settings

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/**
 * Pins the toolbar keyboard button's tap rule and the one-handed storage values
 * (shared with iOS `OneHandedModeTests`).
 */
class OneHandedModeTest {
    @Test
    fun storageValues_matchIos() {
        assertEquals(listOf("off", "left", "right"), OneHandedMode.entries.map { it.storageValue })
        assertEquals(listOf("dismiss", "left", "right"), KeyboardToolbarAction.entries.map { it.storageValue })
    }

    @Test
    fun fromStorage_roundTripsAndFallsBack() {
        for (mode in OneHandedMode.entries) assertEquals(mode, OneHandedMode.fromStorage(mode.storageValue))
        for (action in KeyboardToolbarAction.entries) {
            assertEquals(action, KeyboardToolbarAction.fromStorage(action.storageValue))
        }
        assertEquals(OneHandedMode.OFF, OneHandedMode.fromStorage(null))
        assertEquals(OneHandedMode.OFF, OneHandedMode.fromStorage("garbage"))
        assertEquals(KeyboardToolbarAction.DISMISS, KeyboardToolbarAction.fromStorage(null))
        assertEquals(KeyboardToolbarAction.DISMISS, KeyboardToolbarAction.fromStorage("garbage"))
    }

    @Test
    fun dismissAction_alwaysDismisses() {
        for (mode in OneHandedMode.entries) assertNull(KeyboardToolbarAction.DISMISS.tapResult(mode))
    }

    @Test
    fun sideAction_togglesBetweenSideAndOff() {
        assertEquals(OneHandedMode.RIGHT, KeyboardToolbarAction.RIGHT.tapResult(OneHandedMode.OFF))
        assertEquals(OneHandedMode.OFF, KeyboardToolbarAction.RIGHT.tapResult(OneHandedMode.RIGHT))
        assertEquals(OneHandedMode.RIGHT, KeyboardToolbarAction.RIGHT.tapResult(OneHandedMode.LEFT))
        assertEquals(OneHandedMode.LEFT, KeyboardToolbarAction.LEFT.tapResult(OneHandedMode.OFF))
        assertEquals(OneHandedMode.OFF, KeyboardToolbarAction.LEFT.tapResult(OneHandedMode.LEFT))
        assertEquals(OneHandedMode.LEFT, KeyboardToolbarAction.LEFT.tapResult(OneHandedMode.RIGHT))
    }

    @Test
    fun forMode_skipsOff_andFlippedSwapsSides() {
        assertNull(KeyboardToolbarAction.forMode(OneHandedMode.OFF))
        assertEquals(KeyboardToolbarAction.LEFT, KeyboardToolbarAction.forMode(OneHandedMode.LEFT))
        assertEquals(KeyboardToolbarAction.RIGHT, KeyboardToolbarAction.forMode(OneHandedMode.RIGHT))
        assertEquals(OneHandedMode.RIGHT, OneHandedMode.LEFT.flipped)
        assertEquals(OneHandedMode.LEFT, OneHandedMode.RIGHT.flipped)
        assertEquals(OneHandedMode.OFF, OneHandedMode.OFF.flipped)
    }
}
