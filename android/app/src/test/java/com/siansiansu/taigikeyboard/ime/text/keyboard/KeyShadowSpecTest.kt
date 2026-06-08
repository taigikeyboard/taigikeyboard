package com.siansiansu.taigikeyboard.ime.text.keyboard

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/**
 * Pins the key drop-shadow intensity -> geometry mapping (0 = none; 1..4 ->
 * radius = intensity, offsetY = intensity/2, fixed alpha). Mirrors iOS
 * ButtonShadowStyle(size:). Pure mapping, so JVM-testable without Compose.
 */
class KeyShadowSpecTest {
    @Test
    fun zeroIntensity_noShadow() {
        assertNull(keyShadowSpec(0f))
    }

    @Test
    fun negativeIntensity_noShadow() {
        assertNull(keyShadowSpec(-1f))
    }

    @Test
    fun intensityOne_radiusOneHalfOffset() {
        val spec = keyShadowSpec(1f)!!
        assertEquals(1f, spec.radiusDp, 0f)
        assertEquals(0.5f, spec.offsetYDp, 0f)
        assertEquals(0.30f, spec.alpha, 0f)
    }

    @Test
    fun intensityFour_radiusFourHalfOffset() {
        val spec = keyShadowSpec(4f)!!
        assertEquals(4f, spec.radiusDp, 0f)
        assertEquals(2f, spec.offsetYDp, 0f)
        assertEquals(0.30f, spec.alpha, 0f)
    }
}
