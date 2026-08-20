package com.siansiansu.taigikeyboard.ime.popup

import com.siansiansu.taigikeyboard.ime.text.key.KeyData
import com.siansiansu.taigikeyboard.ime.text.key.KeyType
import org.junit.Assert.assertEquals
import org.junit.Assert.assertSame
import org.junit.Test

/**
 * Pins the TPS long-press popup base-glyph prepend (long-press ㄗ offers
 * {ㄗ, ㄐ}, not just {ㄐ}). Mirrors iOS `Callouts.TPSCallouts.calloutChars`.
 * Scope (USER 2026-08-21): letter-variant + number-shortcut keys include
 * their own glyph; the `，` punctuation key (code 65292) does not.
 */
class TpsPopupWithBaseGlyphTest {
    private fun tpsGlyphKey(label: String, vararg popup: KeyData) = KeyData(
        code = 0,
        label = label,
        popup = popup.toMutableList(),
    )

    @Test
    fun glyphKeys_baseGlyphPrependedFirst_variantOrderKept() {
        val rows = listOf(
            // letter variant
            tpsGlyphKey("ㄗ", KeyData(code = 0, label = "ㄐ")) to listOf("ㄗ", "ㄐ"),
            // number shortcut
            tpsGlyphKey("ㆠ", KeyData(code = 49, label = "1")) to listOf("ㆠ", "1"),
            // multi-variant
            tpsGlyphKey("ㄫ", KeyData(code = 0, label = "ㆭ"), KeyData(code = 0, label = "ㄥ")) to
                listOf("ㄫ", "ㆭ", "ㄥ"),
        )
        for ((key, expectedLabels) in rows) {
            val augmented = tpsPopupWithBaseGlyph(key, isTpsLayout = true)
            assertEquals(key.label, expectedLabels, augmented.popup.map { it.label })
        }
    }

    @Test
    fun numberShortcutKey_digitCodePreserved() {
        val augmented = tpsPopupWithBaseGlyph(
            tpsGlyphKey("ㆠ", KeyData(code = 49, label = "1")),
            isTpsLayout = true,
        )
        assertEquals(listOf(0, 49), augmented.popup.map { it.code })
    }

    @Test
    fun baseCell_carriesKeyFieldsExceptPopup() {
        val key = tpsGlyphKey("ㄗ", KeyData(code = 0, label = "ㄐ"))
        val base = tpsPopupWithBaseGlyph(key, isTpsLayout = true).popup.first()
        assertEquals(key.copy(popup = mutableListOf()), base)
    }

    @Test
    fun gateRejectedKeys_returnedUnchanged() {
        val rows = listOf(
            // punctuation key (code 65292)
            KeyData(code = 65292, label = "，", popup = mutableListOf(KeyData(code = 12290, label = "。"))) to true,
            // non-TPS layout
            tpsGlyphKey("ㄗ", KeyData(code = 0, label = "ㄐ")) to false,
            // no popup variants
            tpsGlyphKey("ㆦ") to true,
            // non-character key
            KeyData(code = 0, label = "x", popup = mutableListOf(KeyData(code = 0, label = "y")), type = KeyType.FUNCTION) to true,
        )
        for ((key, isTps) in rows) {
            assertSame(key.label, key, tpsPopupWithBaseGlyph(key, isTpsLayout = isTps))
        }
    }

    @Test
    fun originalKeyDataNotMutated() {
        val key = tpsGlyphKey("ㄗ", KeyData(code = 0, label = "ㄐ"))
        tpsPopupWithBaseGlyph(key, isTpsLayout = true)
        assertEquals(listOf("ㄐ"), key.popup.map { it.label })
    }

    // Seam alignment (anchor.data.popup vs anchor.popupCells index parity)
    // is NOT JVM-testable: buildPopupCells → computeKeyLetter →
    // CaseTransformBridge loads the Rust .so, unavailable off-device
    // (project-known limitation). Pinned by device dogfood instead: long-press
    // ㄗ → slide to each cell → index 0 commits ㄗ, index 1 commits ㄐ.
}
