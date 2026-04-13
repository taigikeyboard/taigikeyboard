package com.siansiansu.taigikeyboard.ui.tabs.tab2

import android.view.ContextThemeWrapper
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.Arrangement
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

@Composable
fun KeyboardPreviewPanel(
    prefs: PrefHelper,
    previewKey: Int,
    layoutType: String,
    colorSettings: KeyboardColorSettings,
    candidateTextSizeScale: Float,
    fontType: String
) {
    Column(modifier = Modifier.fillMaxWidth()) {
        // Candidate bar preview (matches iOS KeyboardPreviewPanel sample suggestions)
        CandidatePreviewRow(
            colorSettings = colorSettings,
            candidateTextSizeScale = candidateTextSizeScale,
            fontType = fontType
        )

        // Use key() to force full AndroidView recreation when previewKey or layoutType changes.
        // This recomputes the layout from current prefs (layout type, height, colors, etc.)
        // matching iOS KeyboardPreviewPanel which re-evaluates on every state change.
        key(previewKey, layoutType) {
            AndroidView(
                factory = { ctx ->
                    val themedContext = ContextThemeWrapper(ctx, R.style.KeyboardTheme)
                    val layoutManager = LayoutManager(themedContext, prefs)
                    // Always show Taigi layout in preview (match iOS KeyboardPreviewPanel behavior)
                    // Use overrideInputMode to avoid showing English layout
                    val layout = layoutManager.fetchComputedLayoutForPreview(
                        KeyboardMode.CHARACTERS,
                        Subtype.DEFAULT
                    )
                    KeyboardView(themedContext).apply {
                        this.prefs = prefs
                        this.isPreviewMode = true
                        this.computedLayout = layout
                        // Filter keys by variation (hides duplicate TRANSLATE keys etc.)
                        updateVisibility()
                    }
                },
                modifier = Modifier.fillMaxWidth()
            )
        }
    }
}

// ============= Candidate Bar Preview (matches actual Smartbar styling) =============

internal data class SampleCandidate(val roman: String, val hanzi: String)

private val sampleCandidates = listOf(
    SampleCandidate("mī-tê", "麵茶"),
    SampleCandidate("kú-nî", "久年"),
    SampleCandidate("gîm-á", "砛仔")
)

/** Resolve a color attribute from KeyboardTheme (single source of truth: themes.xml). */
internal fun resolveKeyboardThemeColor(context: android.content.Context, attrId: Int): Color {
    val themed = android.view.ContextThemeWrapper(context, R.style.KeyboardTheme)
    val tv = android.util.TypedValue()
    themed.theme.resolveAttribute(attrId, tv, true)
    return Color(tv.data)
}

@Composable
private fun CandidatePreviewRow(
    colorSettings: KeyboardColorSettings,
    candidateTextSizeScale: Float,
    fontType: String
) {
    val context = LocalContext.current
    val typeface = remember(fontType) {
        FontUtils.getTypefaceByType(fontType, context)
    }
    val fontFamily = remember(typeface) { FontFamily(typeface) }

    // Resolve default colors from KeyboardTheme (auto light/dark via DayNight parent)
    val defaultBgColor = remember { resolveKeyboardThemeColor(context, R.attr.smartbar_bgColor) }
    val defaultTextColor = remember { resolveKeyboardThemeColor(context, R.attr.smartbar_candidate_fgColor) }
    val subtitleColor = remember { resolveKeyboardThemeColor(context, R.attr.smartbar_candidate_subtitle_fgColor) }
    val composingBgColor = remember { resolveKeyboardThemeColor(context, R.attr.semiTransparentColor) }
    // Icon tint: matches smartbar toolbar_toggle_button and expand_toggle_button tint
    val iconTint = remember { resolveKeyboardThemeColor(context, R.attr.smartbar_fgColor) }

    val bgColor = colorSettings.candidateBackgroundColor?.let { Color(it) } ?: defaultBgColor
    val textColor = colorSettings.candidateTextColor?.let { Color(it) } ?: defaultTextColor
    val effectiveSubtitleColor = colorSettings.candidateTextColor?.let { Color(it) } ?: subtitleColor

    // Text size: smartbarHeight(50dp) * 0.38 = 19sp, scaled by user preference
    val titleSizeSp = 19.sp * candidateTextSizeScale
    val subtitleSizeSp = titleSizeSp * 0.70f

    Row(
        modifier = Modifier
            .fillMaxWidth()
            .height(50.dp)
            .background(bgColor),
        verticalAlignment = Alignment.CenterVertically
    ) {
        // "+" toolbar toggle button (left side, matches smartbar.xml toolbar_toggle_button)
        Icon(
            painter = painterResource(R.drawable.ic_add),
            contentDescription = null,
            modifier = Modifier
                .width(36.dp)
                .padding(start = 2.dp)
                .padding(6.dp),
            tint = iconTint
        )

        // Candidate items (fill remaining space, vertically stacked title+subtitle)
        Row(
            modifier = Modifier.weight(1f),
            horizontalArrangement = Arrangement.Center,
            verticalAlignment = Alignment.CenterVertically
        ) {
            sampleCandidates.forEachIndexed { index, candidate ->
                // First candidate has composing background (matches candidate_composing_background)
                val itemBg = if (index == 0) composingBgColor else Color.Transparent
                Column(
                    horizontalAlignment = Alignment.CenterHorizontally,
                    modifier = Modifier
                        .background(itemBg, RoundedCornerShape(8.dp))
                        .padding(horizontal = 6.dp, vertical = 3.dp)
                ) {
                    Text(
                        text = candidate.roman,
                        fontSize = titleSizeSp,
                        color = textColor,
                        fontFamily = fontFamily,
                        maxLines = 1,
                        textAlign = TextAlign.Center
                    )
                    Text(
                        text = candidate.hanzi,
                        fontSize = subtitleSizeSp,
                        color = effectiveSubtitleColor,
                        fontFamily = fontFamily,
                        maxLines = 1,
                        textAlign = TextAlign.Center
                    )
                }
                // Add spacing between items (matching margin * 5 = 5dp each side)
                if (index < sampleCandidates.size - 1) {
                    Spacer(Modifier.width(10.dp))
                }
            }
        }

        // Vertical divider (matches smartbar.xml candidate_divider)
        Box(
            modifier = Modifier
                .width(1.dp)
                .height(32.dp)
                .background(iconTint.copy(alpha = 0.3f))
        )

        // Expand/collapse chevron (right side, matches smartbar.xml expand_toggle_button)
        Icon(
            painter = painterResource(R.drawable.ic_keyboard_arrow_down),
            contentDescription = null,
            modifier = Modifier
                .width(48.dp)
                .padding(end = 4.dp)
                .padding(2.dp),
            tint = iconTint
        )
    }
}
