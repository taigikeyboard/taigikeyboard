package com.siansiansu.taigikeyboard.ui.tabs.layout

import android.content.Context
import android.view.ContextThemeWrapper
import androidx.compose.foundation.background
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.key
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.viewinterop.AndroidView
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.KeyboardColorSettings
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.Subtype
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardMode
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardView
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutManager
import com.siansiansu.taigikeyboard.util.FontUtils
import com.siansiansu.taigikeyboard.util.getColorFromAttr

// Live keyboard preview panel with candidate bar for appearance settings

@Composable
fun KeyboardPreviewPanel(
    prefs: PrefHelper,
    layoutType: String,
    colorSettings: KeyboardColorSettings,
    candidateTextSizeScale: Float,
    keyHeightScale: Float,
    keyFontSizeScale: Float,
    keyCornerRadius: Float,
    keyBorderWidth: Float,
    fontType: String,
) {
    Column(modifier = Modifier.fillMaxWidth()) {
        CandidatePreviewRow(
            colorSettings = colorSettings,
            candidateTextSizeScale = candidateTextSizeScale,
            fontType = fontType,
        )

        // Re-key on layoutType (structural) and keyHeightScale (changes the AndroidView's measured
        // height — Compose's AndroidView wrapper feeds the hosted KeyboardView an EXACTLY height
        // spec derived from finite parent constraints, so an in-place requestLayout cannot grow
        // or shrink the view. For all other appearance edits (color, font, cornerRadius,
        // borderWidth, candidate size) the cheap update-lambda path applies styling without
        // rebuilding the view hierarchy.
        key(layoutType, keyHeightScale) {
            AndroidView(
                factory = { ctx ->
                    val themedContext = ContextThemeWrapper(ctx, R.style.KeyboardTheme)
                    val layoutManager = LayoutManager(themedContext, prefs)
                    val layout =
                        layoutManager.fetchComputedLayoutForPreview(
                            KeyboardMode.CHARACTERS,
                            Subtype.DEFAULT,
                        )
                    KeyboardView(themedContext).apply {
                        this.prefs = prefs
                        this.isPreviewMode = true
                        this.computedLayout = layout
                        updateVisibility()
                    }
                },
                update = { view ->
                    // Touch each scalar so Compose registers snapshot reads here; live drag of
                    // any appearance slider/color picker then re-fires this lambda and re-applies
                    // styling without rebuilding the view hierarchy.
                    colorSettings
                    candidateTextSizeScale
                    keyFontSizeScale
                    keyCornerRadius
                    keyBorderWidth
                    fontType
                    view.applyAppearanceChanges()
                },
                modifier = Modifier.fillMaxWidth(),
            )
        }
    }
}

private data class SampleCandidate(
    val roman: String,
    val hanzi: String,
)

private val sampleCandidates =
    listOf(
        SampleCandidate("mī-tê", "麵茶"),
        SampleCandidate("kú-nî", "久年"),
        SampleCandidate("gîm-á", "砛仔"),
    )

private fun resolveKeyboardThemeColor(
    context: Context,
    attrId: Int,
): Color {
    val themed = ContextThemeWrapper(context, R.style.KeyboardTheme)
    return Color(getColorFromAttr(themed, attrId))
}

@Composable
private fun CandidatePreviewRow(
    colorSettings: KeyboardColorSettings,
    candidateTextSizeScale: Float,
    fontType: String,
) {
    val context = LocalContext.current
    val typeface =
        remember(fontType) {
            FontUtils.getTypefaceByType(fontType, context)
        }
    val fontFamily = remember(typeface) { FontFamily(typeface) }

    // Key on isDarkTheme so colors refresh on light/dark mode changes
    val isDarkTheme = isSystemInDarkTheme()
    val defaultBgColor = remember(isDarkTheme) { resolveKeyboardThemeColor(context, R.attr.smartbar_bgColor) }
    val defaultTextColor = remember(isDarkTheme) { resolveKeyboardThemeColor(context, R.attr.smartbar_candidate_fgColor) }
    val subtitleColor = remember(isDarkTheme) { resolveKeyboardThemeColor(context, R.attr.smartbar_candidate_subtitle_fgColor) }
    val composingBgColor = remember(isDarkTheme) { resolveKeyboardThemeColor(context, R.attr.semiTransparentColor) }
    val iconTint = remember(isDarkTheme) { resolveKeyboardThemeColor(context, R.attr.smartbar_fgColor) }

    val bgColor = colorSettings.candidateBackgroundColor?.let { Color(it) } ?: defaultBgColor
    val textColor = colorSettings.candidateTextColor?.let { Color(it) } ?: defaultTextColor
    val effectiveSubtitleColor = colorSettings.candidateTextColor?.let { Color(it) } ?: subtitleColor

    val titleSizeSp = 19.sp * candidateTextSizeScale
    val subtitleSizeSp = titleSizeSp * 0.70f

    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .height(50.dp)
                .background(bgColor),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(
            painter = painterResource(R.drawable.ic_add),
            contentDescription = null,
            modifier =
                Modifier
                    .width(36.dp)
                    .padding(start = 2.dp)
                    .padding(6.dp),
            tint = iconTint,
        )

        Row(
            modifier = Modifier.weight(1f),
            horizontalArrangement = Arrangement.Center,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            sampleCandidates.forEachIndexed { index, candidate ->
                // First candidate has composing background (matches candidate_composing_background)
                val itemBg = if (index == 0) composingBgColor else Color.Transparent
                Column(
                    horizontalAlignment = Alignment.CenterHorizontally,
                    modifier =
                        Modifier
                            .background(itemBg, RoundedCornerShape(8.dp))
                            .padding(horizontal = 6.dp, vertical = 3.dp),
                ) {
                    Text(
                        text = candidate.roman,
                        fontSize = titleSizeSp,
                        color = textColor,
                        fontFamily = fontFamily,
                        maxLines = 1,
                        textAlign = TextAlign.Center,
                    )
                    Text(
                        text = candidate.hanzi,
                        fontSize = subtitleSizeSp,
                        color = effectiveSubtitleColor,
                        fontFamily = fontFamily,
                        maxLines = 1,
                        textAlign = TextAlign.Center,
                    )
                }
                if (index < sampleCandidates.size - 1) {
                    Spacer(Modifier.width(10.dp))
                }
            }
        }

        Box(
            modifier =
                Modifier
                    .width(1.dp)
                    .height(32.dp)
                    .background(iconTint.copy(alpha = 0.3f)),
        )

        Icon(
            painter = painterResource(R.drawable.ic_keyboard_arrow_down),
            contentDescription = null,
            modifier =
                Modifier
                    .width(48.dp)
                    .padding(end = 4.dp)
                    .padding(2.dp),
            tint = iconTint,
        )
    }
}
