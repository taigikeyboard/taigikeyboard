package com.siansiansu.taigikeyboard.ime.text

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/**
 * CharacterInputPipeline tests — locks the contract of the collapsed
 * single-entry-point that wraps the four TPSConverter context-sensitive
 * adjustment calls.
 *
 * Pre-D9.4 commit-1 baseline: every fixture here is the agreed-canonical
 * behavior. Post-D9.4, the same fixtures must hold against the Rust
 * implementation called via Method::TpsInputAdjust.
 *
 * Mirrors ios/TaigiKeyboardTests/CharacterInputPipelineTests.swift (subset:
 * Android does not gate by InputMode because TPS is a layout, not a mode).
 */
class CharacterInputPipelineTest {
    // region Dual-form initials at syllable start

    @Test
    fun `dual-form keys on empty buffer keep initial form`() {
        for (ch in listOf("ㄇ", "ㄋ", "ㄫ", "ㄅ", "ㄉ", "ㄍ", "ㄏ")) {
            val result = CharacterInputPipeline.adjust(ch, "")
            assertEquals("$ch on empty buffer should stay initial", ch, result.char)
            assertNull(result.replaceLast)
        }
    }

    @Test
    fun `dual-form key after space keeps initial form`() {
        val result = CharacterInputPipeline.adjust("ㄇ", "ㄚ ")
        assertEquals("ㄇ", result.char)
        assertNull(result.replaceLast)
    }

    @Test
    fun `dual-form key after tone mark keeps initial form`() {
        val result = CharacterInputPipeline.adjust("ㄇ", "ㄚˊ")
        assertEquals("ㄇ", result.char)
        assertNull(result.replaceLast)
    }

    @Test
    fun `dual-form key after checked final keeps initial form`() {
        // After ㆴ (entering tone -p) → new syllable
        val result = CharacterInputPipeline.adjust("ㄇ", "ㄚㆴ")
        assertEquals("ㄇ", result.char)
        assertNull(result.replaceLast)
    }

    @Test
    fun `dual-form key after nasal final keeps initial form`() {
        // After ㆬ (syllabic m / -m final) → new syllable
        val result = CharacterInputPipeline.adjust("ㄇ", "ㄚㆬ")
        assertEquals("ㄇ", result.char)
        assertNull(result.replaceLast)
    }

    // endregion
    // region Dual-form initials NOT at syllable start (final form)

    @Test
    fun `M after vowel returns final form`() {
        val result = CharacterInputPipeline.adjust("ㄇ", "ㄚ")
        assertEquals("ㆬ", result.char)
        assertNull(result.replaceLast)
    }

    @Test
    fun `N after vowel returns final form`() {
        val result = CharacterInputPipeline.adjust("ㄋ", "ㄚ")
        assertEquals("ㄣ", result.char)
        assertNull(result.replaceLast)
    }

    @Test
    fun `B after vowel returns final form`() {
        val result = CharacterInputPipeline.adjust("ㄅ", "ㄚ")
        assertEquals("ㆴ", result.char)
        assertNull(result.replaceLast)
    }

    @Test
    fun `D after vowel returns final form`() {
        val result = CharacterInputPipeline.adjust("ㄉ", "ㄚ")
        assertEquals("ㆵ", result.char)
        assertNull(result.replaceLast)
    }

    @Test
    fun `G after vowel returns final form`() {
        val result = CharacterInputPipeline.adjust("ㄍ", "ㄚ")
        assertEquals("ㆻ", result.char)
        assertNull(result.replaceLast)
    }

    @Test
    fun `H after vowel returns final form`() {
        val result = CharacterInputPipeline.adjust("ㄏ", "ㄚ")
        assertEquals("ㆷ", result.char)
        assertNull(result.replaceLast)
    }

    // endregion
    // region ㄫ context-dependent

    @Test
    fun `NG after I returns ing`() {
        val result = CharacterInputPipeline.adjust("ㄫ", "ㄧ")
        assertEquals("ㄥ", result.char)
        assertNull(result.replaceLast)
    }

    @Test
    fun `NG after other vowel returns syllabic ng`() {
        val result = CharacterInputPipeline.adjust("ㄫ", "ㄚ")
        assertEquals("ㆭ", result.char)
        assertNull(result.replaceLast)
    }

    // endregion
    // region Hyphen quirk (locks current behavior — hyphen is NOT syllable boundary)

    /**
     * Locks the current Android contract: "-" is NOT in syllableBoundaryChars,
     * so a dual-form key after a hyphen still gets the FINAL form. If parity
     * audit later decides this is a bug, fix as a separate parity: PR.
     */
    @Test
    fun `dual-form key after hyphen returns final form`() {
        val result = CharacterInputPipeline.adjust("ㄇ", "ㄚ-")
        assertEquals("ㆬ", result.char)
        assertNull(result.replaceLast)
    }

    // endregion
    // region Non-dual-form pass-through

    @Test
    fun `non-dual-form keys pass through`() {
        for (ch in listOf("ㄆ", "ㄊ", "ㄎ", "ㄈ", "ㄌ")) {
            val result = CharacterInputPipeline.adjust(ch, "ㄚ")
            assertEquals("$ch is not dual-form, should pass through", ch, result.char)
            assertNull(result.replaceLast)
        }
    }

    // endregion
    // region ㆮ → ㆯ disambiguation

    @Test
    fun `AINN after I becomes AUNN`() {
        val result = CharacterInputPipeline.adjust("ㆮ", "ㄧ")
        assertEquals("ㆯ", result.char)
        assertNull(result.replaceLast)
    }

    @Test
    fun `AINN after other vowel unchanged`() {
        val result = CharacterInputPipeline.adjust("ㆮ", "ㄚ")
        assertEquals("ㆮ", result.char)
        assertNull(result.replaceLast)
    }

    @Test
    fun `non-AINN char after I no disambiguation`() {
        val result = CharacterInputPipeline.adjust("ㄚ", "ㄧ")
        assertEquals("ㄚ", result.char)
        assertNull(result.replaceLast)
    }

    // endregion
    // region Syllabic nasal replacement (ㄇ/ㄫ + tone)

    @Test
    fun `tone after M replaces with syllabic m`() {
        val result = CharacterInputPipeline.adjust("ˊ", "ㄇ")
        assertEquals("ˊ", result.char)
        assertEquals("ㆬ", result.replaceLast)
    }

    @Test
    fun `tone after NG replaces with syllabic ng`() {
        val result = CharacterInputPipeline.adjust("ˊ", "ㄫ")
        assertEquals("ˊ", result.char)
        assertEquals("ㆭ", result.replaceLast)
    }

    @Test
    fun `tone after other consonant no replace`() {
        // Non-ㄇ/ㄫ consonant + tone → no syllabic nasal replacement.
        val result = CharacterInputPipeline.adjust("ˊ", "ㄉ")
        assertEquals("ˊ", result.char)
        assertNull(result.replaceLast)
    }

    @Test
    fun `non-tone after M no replace`() {
        // ㄇ + non-tone (vowel) → no syllabic nasal replacement; ㄚ pass-through.
        val result = CharacterInputPipeline.adjust("ㄚ", "ㄇ")
        assertEquals("ㄚ", result.char)
        assertNull(result.replaceLast)
    }

    @Test
    fun `tone on empty buffer no replace`() {
        val result = CharacterInputPipeline.adjust("ˊ", "")
        assertEquals("ˊ", result.char)
        assertNull(result.replaceLast)
    }

    // endregion
    // region Palatalization (ㄗ/ㄘ/ㄙ/ㆡ + ㄧ/ㆪ)

    @Test
    fun `I after Ts palatalizes`() {
        val result = CharacterInputPipeline.adjust("ㄧ", "ㄗ")
        assertEquals("ㄧ", result.char)
        assertEquals("ㄐ", result.replaceLast)
    }

    @Test
    fun `I after Tsh palatalizes`() {
        val result = CharacterInputPipeline.adjust("ㄧ", "ㄘ")
        assertEquals("ㄧ", result.char)
        assertEquals("ㄑ", result.replaceLast)
    }

    @Test
    fun `I after S palatalizes`() {
        val result = CharacterInputPipeline.adjust("ㄧ", "ㄙ")
        assertEquals("ㄧ", result.char)
        assertEquals("ㄒ", result.replaceLast)
    }

    @Test
    fun `I after J palatalizes`() {
        val result = CharacterInputPipeline.adjust("ㄧ", "ㆡ")
        assertEquals("ㄧ", result.char)
        assertEquals("ㆢ", result.replaceLast)
    }

    @Test
    fun `nasalized inn after Ts palatalizes`() {
        // ㆪ (nasalized i) also triggers palatalization.
        val result = CharacterInputPipeline.adjust("ㆪ", "ㄗ")
        assertEquals("ㆪ", result.char)
        assertEquals("ㄐ", result.replaceLast)
    }

    @Test
    fun `non-trigger after affricate no palatalization`() {
        // ㄗ + ㄚ (non-trigger vowel) → no replace.
        val result = CharacterInputPipeline.adjust("ㄚ", "ㄗ")
        assertEquals("ㄚ", result.char)
        assertNull(result.replaceLast)
    }

    @Test
    fun `I after non-affricate no palatalization`() {
        // ㄆ + ㄧ (non-affricate previous) → no replace.
        val result = CharacterInputPipeline.adjust("ㄧ", "ㄆ")
        assertEquals("ㄧ", result.char)
        assertNull(result.replaceLast)
    }

    @Test
    fun `I on empty buffer no palatalization`() {
        val result = CharacterInputPipeline.adjust("ㄧ", "")
        assertEquals("ㄧ", result.char)
        assertNull(result.replaceLast)
    }

    // endregion
    // region Combined / disjoint trigger guarantee

    /**
     * Pins the contract that syllabic-nasal and palatalization triggers are
     * disjoint by lastChar: {ㄇ, ㄫ} vs {ㄗ, ㄘ, ㄙ, ㆡ}. Both effects can
     * never fire on the same call, so the Elvis short-circuit is
     * observationally identical to running both checks unconditionally
     * (which the pre-refactor TextInputManager.kt:843-857 inline chain did).
     */
    @Test
    fun `syllabic nasal and palatalization triggers are disjoint`() {
        // Last char in syllabic-nasal set ({ㄇ, ㄫ}) is never in palatalization set.
        for (last in listOf("ㄇ", "ㄫ")) {
            for (incoming in listOf("ㄧ", "ㆪ")) {
                val result = CharacterInputPipeline.adjust(incoming, last)
                assertEquals(incoming, result.char)
                assertNull(
                    "incoming $incoming after $last must not trigger either replacement",
                    result.replaceLast,
                )
            }
        }
    }

    // endregion
}
