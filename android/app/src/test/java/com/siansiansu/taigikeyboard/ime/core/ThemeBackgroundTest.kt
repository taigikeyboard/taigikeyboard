package com.siansiansu.taigikeyboard.ime.core

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Test

/**
 * Tests for [ThemeBackground] / [ThemeGradient] — the single background field a theme
 * carries (USER 2026-09-19: one surface for keyboard + candidate bar), its legacy-key
 * decoding, the angle -> unit-point math shared with the overlay backdrops, and the
 * user-theme seed. Mirrors iOS ThemeBackgroundTests.
 */
class ThemeBackgroundTest {
    private fun decode(json: String): KeyboardColorSettings = KeyboardColorSettings.fromJson(JSONObject(json))

    private fun gradient(angle: Float): ThemeGradient = ThemeGradient(listOf(BLACK, WHITE), angle)

    // region Legacy decode

    @Test
    fun decode_legacyBackgroundColor_becomesSolid() {
        val colors = decode("""{ "backgroundColor": $RED }""")
        assertEquals(ThemeBackground.Solid(RED), colors.background)
        assertNull(colors.backgroundGradient)
    }

    // An old 2-stop `backgroundGradient` (no angle) -> gradient at the vertical default angle,
    // winning over a legacy `backgroundColor` beside it (the gradient overrode the flat fill before).
    @Test
    fun decode_legacyBackgroundGradient_becomesVerticalGradient() {
        val colors = decode("""{ "backgroundColor": $RED, "backgroundGradient": { "stops": [$BLACK, $WHITE] } }""")
        val gradient = colors.backgroundGradient
        assertNotNull(gradient)
        assertEquals(ThemeGradient.DEFAULT_ANGLE, gradient?.angle)
        assertEquals(listOf(BLACK, WHITE), gradient?.stops)
    }

    // A legacy 1-stop gradient is not renderable -> falls through to the legacy solid colour.
    @Test
    fun decode_legacySingleStopGradient_fallsBackToSolid() {
        val colors = decode("""{ "backgroundColor": $GREEN, "backgroundGradient": { "stops": [$BLACK] } }""")
        assertEquals(ThemeBackground.Solid(GREEN), colors.background)
    }

    // `candidateBackgroundColor` is dropped on decode — the candidate bar is the keyboard surface.
    @Test
    fun decode_legacyCandidateBackground_isIgnored() {
        assertEquals(KeyboardColorSettings(), decode("""{ "candidateBackgroundColor": $RED }"""))
    }

    // An unknown background `type` (written by a newer build) degrades to adaptive, other roles kept.
    @Test
    fun decode_unknownBackgroundType_degradesToAdaptive() {
        val colors = decode("""{ "background": { "type": "hologram" }, "keyTextColor": $BLUE }""")
        assertNull(colors.background)
        assertEquals(BLUE, colors.keyTextColor)
    }

    // A new-format gradient with one stop is not renderable -> decode degrades to adaptive.
    @Test
    fun decode_newFormatSingleStopGradient_degradesToAdaptive() {
        assertNull(decode("""{ "background": { "type": "gradient", "stops": [$BLACK] } }""").background)
    }

    // endregion

    // region Round trip

    // Encode writes only the `background` key (type + fields), never the legacy keys; decode restores it.
    @Test
    fun roundTrip_gradientWithAngle_andNoLegacyKeys() {
        val colors = KeyboardColorSettings(background = ThemeBackground.Gradient(ThemeGradient(listOf(0xFF112233.toInt(), 0xFF445566.toInt()), 45f)))
        val json = colors.toJson()
        assertFalse("legacy key must not be written: $json", json.contains("backgroundColor"))
        assertFalse("legacy key must not be written: $json", json.contains("backgroundGradient"))
        assertEquals("gradient", JSONObject(json).getJSONObject("background").getString("type"))
        assertEquals(colors, KeyboardColorSettings.fromJson(json))
    }

    @Test
    fun roundTrip_solid() {
        val colors = KeyboardColorSettings(background = ThemeBackground.Solid(0xFFABCDEF.toInt()))
        val json = colors.toJson()
        assertEquals("solid", JSONObject(json).getJSONObject("background").getString("type"))
        assertEquals(colors, KeyboardColorSettings.fromJson(json))
    }

    // endregion

    // region Angle -> unit points

    // CSS convention — 180 = top->bottom edge-to-edge; 90 = left->right; 0 = bottom->top;
    // 135 = top-left -> bottom-right corner-to-corner (Chebyshev-normalised diagonal).
    @Test
    fun unitPoints_presets() {
        val cases =
            listOf(
                Triple(180f, UnitPoint(0.5f, 0f), UnitPoint(0.5f, 1f)),
                Triple(0f, UnitPoint(0.5f, 1f), UnitPoint(0.5f, 0f)),
                Triple(90f, UnitPoint(0f, 0.5f), UnitPoint(1f, 0.5f)),
                Triple(270f, UnitPoint(1f, 0.5f), UnitPoint(0f, 0.5f)),
                Triple(135f, UnitPoint(0f, 0f), UnitPoint(1f, 1f)),
                Triple(315f, UnitPoint(1f, 1f), UnitPoint(0f, 0f)),
                Triple(45f, UnitPoint(0f, 1f), UnitPoint(1f, 0f)),
                Triple(225f, UnitPoint(1f, 0f), UnitPoint(0f, 1f)),
            )
        for ((angle, start, end) in cases) {
            val (actualStart, actualEnd) = gradient(angle).unitPoints()
            assertEquals("angle $angle start.x", start.x, actualStart.x, EPSILON)
            assertEquals("angle $angle start.y", start.y, actualStart.y, EPSILON)
            assertEquals("angle $angle end.x", end.x, actualEnd.x, EPSILON)
            assertEquals("angle $angle end.y", end.y, actualEnd.y, EPSILON)
        }
    }

    // endregion

    // region Photo background

    // `{"type":"image","file":"a.jpg","dim":0.5}` round-trips; dim clamps into 0..0.8; absent dim = default.
    @Test
    fun imageBackground_roundTripAndDimClamp() {
        val colors = KeyboardColorSettings(background = ThemeBackground.Image(ThemeImageBackground("a.jpg", 0.5f)))
        val json = colors.toJson()
        assertEquals("image", JSONObject(json).getJSONObject("background").getString("type"))
        assertEquals(colors, KeyboardColorSettings.fromJson(json))
        val clampedHigh = decode("""{ "background": { "type": "image", "file": "a.jpg", "dim": 2 } }""")
        assertEquals(ThemeImageBackground.DIM_MAX, clampedHigh.background?.asImage?.dim)
        val clampedLow = decode("""{ "background": { "type": "image", "file": "a.jpg", "dim": -1 } }""")
        assertEquals(0f, clampedLow.background?.asImage?.dim)
        val absentDim = decode("""{ "background": { "type": "image", "file": "b.jpg" } }""")
        assertEquals(ThemeBackground.Image(ThemeImageBackground("b.jpg", ThemeImageBackground.DEFAULT_DIM)), absentDim.background)
    }

    // An empty file name is not a photo -> decode degrades to adaptive (same as an unknown type).
    @Test
    fun imageBackground_emptyFile_degradesToAdaptive() {
        assertNull(decode("""{ "background": { "type": "image", "file": "" } }""").background)
    }

    // Aspect-fill cover rect: a 2:1 photo over a 1:1 keyboard fills the height and centres horizontally;
    // a 1:2 photo fills the width and centres vertically; a panel slice keeps the whole-keyboard framing.
    @Test
    fun coverRect_aspectFillCentred() {
        val square = SurfaceRect(0f, 0f, 100f, 100f)
        assertEquals(SurfaceRect(-50f, 0f, 200f, 100f), ThemeImageBackground.coverRect(200f, 100f, square, CENTRE, CENTRE, NO_ZOOM))
        assertEquals(SurfaceRect(0f, -50f, 100f, 200f), ThemeImageBackground.coverRect(100f, 200f, square, CENTRE, CENTRE, NO_ZOOM))
        val slicedKeyboard = SurfaceRect(0f, -50f, 100f, 300f)
        assertEquals(SurfaceRect(-100f, -50f, 300f, 300f), ThemeImageBackground.coverRect(100f, 100f, slicedKeyboard, CENTRE, CENTRE, NO_ZOOM))
        assertEquals(square, ThemeImageBackground.coverRect(0f, 0f, square, CENTRE, CENTRE, NO_ZOOM))
    }

    // Focus aligns the cover rect: a 2:1 photo over a 1:1 keyboard overflows 100 horizontally —
    // focusX 0 -> left 0 (left edge shows), 1 -> 100 - 200 = -100 (right edge shows); the
    // non-overflowing axis ignores focus; a sliced keyboard keeps its origin offset.
    @Test
    fun coverRect_focusAlignsOverflowingAxis() {
        val square = SurfaceRect(0f, 0f, 100f, 100f)
        assertEquals(SurfaceRect(0f, 0f, 200f, 100f), ThemeImageBackground.coverRect(200f, 100f, square, focusX = 0f, focusY = 1f, zoom = NO_ZOOM))
        assertEquals(SurfaceRect(-100f, 0f, 200f, 100f), ThemeImageBackground.coverRect(200f, 100f, square, focusX = 1f, focusY = 0f, zoom = NO_ZOOM))
        val slicedKeyboard = SurfaceRect(0f, -50f, 100f, 300f)
        assertEquals(SurfaceRect(-200f, -50f, 300f, 300f), ThemeImageBackground.coverRect(100f, 100f, slicedKeyboard, focusX = 1f, focusY = 0f, zoom = NO_ZOOM))
    }

    // Zoom scales the cover rect about the focus: a 2:1 photo over a 1:1 keyboard at 2x covers
    // 400x200 — centred at ((100 - 400) / 2, (100 - 200) / 2) = (-150, -50); focus (0, 1) ->
    // (0, 100 - 200 = -100); a 1:1 photo on the sliced 100x300 keyboard at 1.5x covers 450x450 at
    // ((100 - 450) / 2, -50 + (300 - 450) / 2) = (-175, -125).
    @Test
    fun coverRect_zoomScalesAboutFocus() {
        val square = SurfaceRect(0f, 0f, 100f, 100f)
        assertEquals(SurfaceRect(-150f, -50f, 400f, 200f), ThemeImageBackground.coverRect(200f, 100f, square, CENTRE, CENTRE, zoom = 2f))
        assertEquals(SurfaceRect(0f, -100f, 400f, 200f), ThemeImageBackground.coverRect(200f, 100f, square, focusX = 0f, focusY = 1f, zoom = 2f))
        val slicedKeyboard = SurfaceRect(0f, -50f, 100f, 300f)
        assertEquals(SurfaceRect(-175f, -125f, 450f, 450f), ThemeImageBackground.coverRect(100f, 100f, slicedKeyboard, CENTRE, CENTRE, zoom = 1.5f))
    }

    // `zoom` round-trips; absent -> 1 (old themes unzoomed); out of range clamps into 1..2.
    @Test
    fun imageBackground_zoomRoundTripDefaultAndClamp() {
        val colors = KeyboardColorSettings(background = ThemeBackground.Image(ThemeImageBackground("a.jpg", zoom = 1.5f)))
        assertEquals(colors, KeyboardColorSettings.fromJson(colors.toJson()))
        val absent = decode("""{ "background": { "type": "image", "file": "b.jpg" } }""").background?.asImage
        assertEquals(ThemeImageBackground.DEFAULT_ZOOM, absent?.zoom)
        assertEquals(2f, decode("""{ "background": { "type": "image", "file": "a.jpg", "zoom": 5 } }""").background?.asImage?.zoom)
        assertEquals(1f, decode("""{ "background": { "type": "image", "file": "a.jpg", "zoom": 0.2 } }""").background?.asImage?.zoom)
    }

    // `focusX` / `focusY` round-trip; absent -> 0.5 (old themes stay centred); out of range clamps into 0..1.
    @Test
    fun imageBackground_focusRoundTripDefaultAndClamp() {
        val colors = KeyboardColorSettings(background = ThemeBackground.Image(ThemeImageBackground("a.jpg", focusX = 0.25f, focusY = 0.75f)))
        assertEquals(colors, KeyboardColorSettings.fromJson(colors.toJson()))
        val absent = decode("""{ "background": { "type": "image", "file": "b.jpg", "dim": 0.5 } }""").background?.asImage
        assertEquals(ThemeImageBackground.DEFAULT_FOCUS, absent?.focusX)
        assertEquals(ThemeImageBackground.DEFAULT_FOCUS, absent?.focusY)
        val clamped = decode("""{ "background": { "type": "image", "file": "a.jpg", "focusX": -1, "focusY": 3 } }""").background?.asImage
        assertEquals(0f, clamped?.focusX)
        assertEquals(1f, clamped?.focusY)
    }

    // The surface pairs the background with its photo tone: dark key text -> white overlay, light -> black;
    // unset role = seed (black text) -> white; adaptive (no background) -> null.
    @Test
    fun surface_pairsBackgroundWithKeyTextTone() {
        assertNull(KeyboardColorSettings().surface)
        val photo = ThemeBackground.Image(ThemeImageBackground("a.jpg"))
        assertEquals(true, KeyboardColorSettings(background = photo).surface?.dimsTowardWhite)
        assertEquals(false, KeyboardColorSettings(background = photo, keyTextColor = WHITE).surface?.dimsTowardWhite)
        assertEquals(ThemeSurface(photo, true), KeyboardColorSettings(background = photo, keyTextColor = 0xFF1C1C1E.toInt()).surface)
    }

    // endregion

    // region Seed

    // The seed sets every role (no null) so a user theme never follows light / dark.
    @Test
    fun userThemeSeed_hasNoNullRole() {
        val seed = UserThemeSeed.colors
        assertEquals(ThemeBackground.Solid(0xFFD4D5DD.toInt()), seed.background)
        assertNotNull(seed.keyTextColor)
        assertNotNull(seed.normalKeyFillColor)
        assertNotNull(seed.specialKeyFillColor)
        assertNotNull(seed.candidateTextColor)
    }

    // Seeding fills only null roles; set roles (incl. a gradient background) are kept verbatim.
    @Test
    fun seededForUserTheme_fillsOnlyNullRoles() {
        val colors = KeyboardColorSettings(background = ThemeBackground.Gradient(gradient(90f)), keyTextColor = 0xFF123456.toInt())
        val seeded = colors.seededForUserTheme()
        assertEquals(colors.background, seeded.background)
        assertEquals(0xFF123456.toInt(), seeded.keyTextColor)
        assertEquals(UserThemeSeed.KEY_FILL, seeded.normalKeyFillColor)
        assertEquals(UserThemeSeed.KEY_FILL, seeded.specialKeyFillColor)
        assertEquals(UserThemeSeed.CANDIDATE_TEXT, seeded.candidateTextColor)
        assertEquals("seeding the seed is a no-op", UserThemeSeed.colors, UserThemeSeed.colors.seededForUserTheme())
    }

    // A theme saved with two key fills loads with the special fill folded into the letter fill.
    @Test
    fun seededForUserTheme_foldsSpecialKeyFillIntoKeyFill() {
        val colors = KeyboardColorSettings(normalKeyFillColor = 0xFF112233.toInt(), specialKeyFillColor = 0xFFABB1BA.toInt())
        val seeded = colors.seededForUserTheme()
        assertEquals(0xFF112233.toInt(), seeded.normalKeyFillColor)
        assertEquals(0xFF112233.toInt(), seeded.specialKeyFillColor)
    }

    // The single key-fill row writes both fills.
    @Test
    fun withKeyFill_setsLetterAndSpecialFill() {
        val colors = UserThemeSeed.colors.withKeyFill(0xFF445566.toInt())
        assertEquals(0xFF445566.toInt(), colors.normalKeyFillColor)
        assertEquals(0xFF445566.toInt(), colors.specialKeyFillColor)
    }

    // Switching Solid -> Gradient seeds a vertical gradient from the solid colour into a lighter tint of it.
    @Test
    fun seededGradient_runsSolidIntoLighterTint() {
        val seeded = ThemeGradient.seeded(0xFF204080.toInt())
        assertEquals(2, seeded.stops.size)
        assertEquals(0xFF204080.toInt(), seeded.stops[0])
        assertEquals(ThemeGradient.DEFAULT_ANGLE, seeded.angle)
        assertEquals(lightenedArgb(0xFF204080.toInt(), 0.45), seeded.stops[1])
    }

    // endregion

    private companion object {
        const val CENTRE = ThemeImageBackground.DEFAULT_FOCUS
        const val NO_ZOOM = ThemeImageBackground.DEFAULT_ZOOM
        const val EPSILON = 1e-6f
        const val RED = 0xFFFF0000.toInt()
        const val GREEN = 0xFF00FF00.toInt()
        const val BLUE = 0xFF0000FF.toInt()
        const val BLACK = 0xFF000000.toInt()
        const val WHITE = 0xFFFFFFFF.toInt()
    }
}
