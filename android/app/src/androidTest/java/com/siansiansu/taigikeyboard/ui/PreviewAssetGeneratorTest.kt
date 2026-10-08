package com.siansiansu.taigikeyboard.ui

import android.graphics.Bitmap
import android.os.Bundle
import androidx.annotation.DrawableRes
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.unit.dp
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import com.siansiansu.taigikeyboard.ime.settings.PrefHelper
import com.siansiansu.taigikeyboard.ime.text.layout.KeyboardLayoutOptions
import com.siansiansu.taigikeyboard.ime.theme.BuiltInThemes
import com.siansiansu.taigikeyboard.ime.theme.ThemeId
import com.siansiansu.taigikeyboard.ime.theme.ThemeResolver
import com.siansiansu.taigikeyboard.ui.tabs.layout.KeyboardPreviewPanel
import com.siansiansu.taigikeyboard.ui.tabs.theme.THEME_CARD_WIDTH_DP
import com.siansiansu.taigikeyboard.ui.tabs.theme.THEME_PREVIEW_ASPECT
import com.siansiansu.taigikeyboard.ui.tabs.theme.builtInThemePreviewRes
import org.junit.Assume.assumeTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File
import kotlin.math.roundToInt

/**
 * Regenerates the Theme / Layout card screenshots (`*_preview.png`) by rendering the real keyboard
 * (KeyboardPreviewPanel) — run through the `previews` skill, which clears the app first so the
 * previews show the factory settings, then pulls `files/previews/` into `res/`.
 *
 * Skipped unless the instrumentation runs with `-e previews true`, so connectedAndroidTest never
 * renders. Driven by the same maps the cards read (KeyboardLayoutOptions, builtInThemePreviewRes),
 * so a card without a generated image fails here. Mirrors iOS PreviewAssetGeneratorTests.
 */
@RunWith(AndroidJUnit4::class)
class PreviewAssetGeneratorTest {
    @get:Rule
    val composeRule = createComposeRule()

    private data class RenderJob(
        @param:DrawableRes val previewRes: Int,
        val themeId: String,
        val layoutKey: String,
        val isNightMode: Boolean,
        // The resource bucket: a night copy only for an adaptive theme.
        val bucket: String,
    )

    @Test
    fun regeneratePreviewAssets() {
        assumeTrue(
            "pass -e $ENABLE_ARGUMENT true to regenerate previews",
            InstrumentationRegistry.getArguments().getString(ENABLE_ARGUMENT) == "true",
        )
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val prefs = PrefHelper(context)
        val outputDir = File(context.filesDir, OUTPUT_DIRECTORY).apply { deleteRecursively() }
        val jobs = renderJobs()

        // A Compose test rule takes one setContent per test: each job swaps the state, and key()
        // rebuilds the panel so nothing remembered carries over between jobs.
        var currentJob by mutableStateOf(jobs.first())
        prefs.keyboardLayoutType = currentJob.layoutKey
        composeRule.setContent {
            key(currentJob) { PreviewCapture(prefs, currentJob) }
        }
        for (job in jobs) {
            // LayoutManager reads the layout (and TPS's input mode) from prefs, not from the job.
            prefs.keyboardLayoutType = job.layoutKey
            composeRule.runOnUiThread { currentJob = job }
            composeRule.waitForIdle()
            val capture = composeRule.onNodeWithTag(CAPTURE_TAG).captureToImage().asAndroidBitmap()
            val name = context.resources.getResourceEntryName(job.previewRes)
            writePng(cropKeyArea(capture), File(outputDir, "${job.bucket}/$name.png"))
        }
        reportStatus("preview.generate.complete count=${jobs.size}")
    }

    @Composable
    private fun PreviewCapture(
        prefs: PrefHelper,
        job: RenderJob,
    ) {
        val appearance = ThemeResolver.resolved(job.themeId, job.isNightMode, userThemes = emptyList())
        Box(Modifier.width(RENDER_WIDTH_DP.dp).testTag(CAPTURE_TAG)) {
            KeyboardPreviewPanel(
                prefs = prefs,
                layoutType = job.layoutKey,
                colorSettings = appearance.colors,
                candidateTextSizeScale = appearance.candidateTextSizeScale,
                keyHeightScale = appearance.keyHeightScale,
                keyFontSizeScale = appearance.keyFontSizeScale,
                keyCornerRadius = appearance.keyCornerRadius,
                keyBorderWidth = appearance.keyBorderWidth,
                fontType = prefs.fontType,
                keyShadowIntensity = appearance.keyShadowIntensity,
                isNightMode = job.isNightMode,
            )
        }
    }

    // Layout cards show the default (adaptive) theme; theme cards show the default layout. Every
    // card image is listed once per bucket, so the Default theme reuses the phahTaigi layout image.
    private fun renderJobs(): List<RenderJob> {
        val layoutJobs =
            (KeyboardLayoutOptions.romanization + KeyboardLayoutOptions.phonetic).flatMap { option ->
                adaptiveJobs(option.previewRes, ThemeId.DEFAULT, option.key)
            }
        val themeJobs =
            BuiltInThemes.all.flatMap { theme ->
                val previewRes =
                    checkNotNull(builtInThemePreviewRes(theme.previewImageName)) {
                        "built-in theme ${theme.id} has no preview drawable in builtInThemePreviewRes"
                    }
                val isAdaptive = theme.colors(isDark = false).surface == null
                if (isAdaptive) {
                    adaptiveJobs(previewRes, theme.id, THEME_CARD_LAYOUT)
                } else {
                    // A fixed palette looks the same in both modes; a dark-only one renders night.
                    listOf(RenderJob(previewRes, theme.id, THEME_CARD_LAYOUT, theme.light == null, DAY_BUCKET))
                }
            }
        return (layoutJobs + themeJobs).distinctBy { it.previewRes to it.bucket }
    }

    private fun adaptiveJobs(
        @DrawableRes previewRes: Int,
        themeId: String,
        layoutKey: String,
    ): List<RenderJob> =
        listOf(
            RenderJob(previewRes, themeId, layoutKey, isNightMode = false, bucket = DAY_BUCKET),
            RenderJob(previewRes, themeId, layoutKey, isNightMode = true, bucket = NIGHT_BUCKET),
        )

    // Bottom-anchored crop to the card aspect (drops the candidate row), scaled to the card size.
    private fun cropKeyArea(capture: Bitmap): Bitmap {
        val cropHeight = (capture.width / THEME_PREVIEW_ASPECT).roundToInt()
        val cropped = Bitmap.createBitmap(capture, 0, capture.height - cropHeight, capture.width, cropHeight)
        return Bitmap.createScaledBitmap(cropped, OUTPUT_WIDTH_PX, OUTPUT_HEIGHT_PX, true)
    }

    private fun writePng(
        bitmap: Bitmap,
        file: File,
    ) {
        file.parentFile?.mkdirs()
        file.outputStream().use { check(bitmap.compress(Bitmap.CompressFormat.PNG, 100, it)) { "PNG encode failed: $file" } }
    }

    // Shows up in the `am instrument` output the skill reads.
    private fun reportStatus(message: String) {
        InstrumentationRegistry.getInstrumentation().sendStatus(0, Bundle().apply { putString("stream", "$message\n") })
    }

    private companion object {
        const val ENABLE_ARGUMENT = "previews"
        const val OUTPUT_DIRECTORY = "previews"
        const val CAPTURE_TAG = "previewCapture"
        const val THEME_CARD_LAYOUT = "phahTaigi"
        const val DAY_BUCKET = "drawable-xxxhdpi"
        const val NIGHT_BUCKET = "drawable-night-xxxhdpi"

        // The width at which the card aspect holds the key area alone (no candidate row).
        const val RENDER_WIDTH_DP = 350

        // The theme / layout card (THEME_CARD_WIDTH_DP wide) at xxxhdpi (4x).
        const val OUTPUT_WIDTH_PX = THEME_CARD_WIDTH_DP * 4
        val OUTPUT_HEIGHT_PX = (OUTPUT_WIDTH_PX / THEME_PREVIEW_ASPECT).roundToInt()
    }
}
