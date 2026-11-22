package com.siansiansu.taigikeyboard.ime.text.layout

import com.siansiansu.taigikeyboard.ime.text.key.KeyData
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutTestAssets.assets
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutTestAssets.merged
import org.junit.Assert.assertEquals
import org.junit.Test

// Pins the ellipsis keys: full-width punctuation types the MOE mark `……` (U+2026 ×2, #851) in one tap,
// half-width keeps `…`; iOS `TaigiLayouts` row 3 + `TaigiCallouts` `……` / MOE `。` lists.
class EllipsisKeyAssetTest {
    private fun symbolKey(
        page: String,
        label: String,
    ): KeyData? =
        assets
            .load(LayoutType.SYMBOLS, page)
            ?.arrangement
            ?.flatten()
            ?.firstOrNull { it.label == label }

    private fun periodPopup(
        base: String,
        isFullWidthPunctuation: Boolean,
    ): List<String> {
        val period = if (isFullWidthPunctuation) "。" else "."
        val keys = merged(base, LayoutScript.TL, isFullWidthPunctuation).flatten()
        return checkNotNull(keys.firstOrNull { it.label == period }).popup.map { it.label }
    }

    @Test
    fun symbolPage_fullWidthTypesTwoEllipses_halfWidthOne() {
        // Code 0 types the label (KeyData.ofEntry contract).
        val full = checkNotNull(symbolKey("western_fullwidth", "……"))
        assertEquals(0, full.code)
        assertEquals(listOf("⋯", "…", "‥"), full.popup.map { it.label })

        val half = checkNotNull(symbolKey("western_default", "…"))
        assertEquals(0x2026, half.code)
        assertEquals(listOf("⋯", "‥"), half.popup.map { it.label })
    }

    @Test
    fun moePeriodPopup_fullWidthOffersTwoEllipses_halfWidthOne() {
        assertEquals(listOf("。", "！", "？", "……"), periodPopup("moe1", isFullWidthPunctuation = true))
        assertEquals(listOf("。", "！", "？", "……", "）", "」"), periodPopup("moe2", isFullWidthPunctuation = true))
        assertEquals("…", periodPopup("moe1", isFullWidthPunctuation = false).last())
    }
}
