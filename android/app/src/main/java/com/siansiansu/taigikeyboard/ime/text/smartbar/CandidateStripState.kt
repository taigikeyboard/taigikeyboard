package com.siansiansu.taigikeyboard.ime.text.smartbar

import com.siansiansu.taigikeyboard.ime.core.settings.CandidateDisplayMode
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord

/**
 * Render-state snapshot for the candidate strip.
 *
 * Owned by [SmartbarManager] via [kotlinx.coroutines.flow.MutableStateFlow].
 * Intentionally render-only — does not replace SmartbarManager fields like
 * `currentSuggestions`, `hasCandidates`, or NextWord state, which other
 * subsystems (CandidateClickHandler, CandidateOverlayView) read directly.
 *
 * `updateSeq` is a monotonic counter incremented on every push. The Composable
 * keys `LaunchedEffect` on it so structurally-equal candidate lists still trigger
 * scroll-to-zero, matching the prior `ListAdapter.submitList` callback semantics.
 */
data class CandidateStripState(
    val mode: CandidateMode,
    val display: CandidateDisplayParams,
    val updateSeq: Long,
)

sealed class CandidateMode {
    data object Empty : CandidateMode()

    data class Taigi(
        val items: List<TaigiWord>,
    ) : CandidateMode()

    data class English(
        val items: List<TaigiWord>,
    ) : CandidateMode()
}

data class CandidateDisplayParams(
    /** EFFECTIVE translate-swap (already false under roman-only). */
    val isTranslateSwapped: Boolean,
    /** Carried explicitly: the swap flag alone cannot tell roman-first from roman-only. */
    val candidateDisplayMode: CandidateDisplayMode,
    val fontType: String,
    val layoutType: String,
    val orMapsToER: Boolean,
    val textSizeScale: Float,
    val candidateTextColor: Int?,
    val candidateBackgroundColor: Int?,
    val themeTitleColor: Int,
    val themeSubtitleColor: Int,
    val themeKeyBgColor: Int,
    val themePressedHighlightColor: Int,
    val smartbarHeightPx: Int,
)

/** Title / optional subtitle of one candidate cell (strip + expanded overlay share it). */
data class CandidateCellText(
    val title: String,
    val subtitle: String?,
)

/**
 * Arm order — hanji-less rows are roman regardless of mode; TPS precedes
 * ROMAN_ONLY so TPS ignores the setting; swap decides the lead otherwise.
 * `displayRoman` is already TPS-converted by the caller when relevant.
 */
// CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Autocomplete/Views/CandidateCellHelper.swift displayTitle / displaySubtitle.
// Drift causes silent divergence (one platform shows a subtitle under roman-only).
fun candidateCellText(
    hanzi: String?,
    displayRoman: String,
    isTPSLayout: Boolean,
    candidateDisplayMode: CandidateDisplayMode,
    isTranslateSwapped: Boolean,
): CandidateCellText =
    when {
        hanzi.isNullOrEmpty() -> CandidateCellText(displayRoman, null)
        isTPSLayout -> CandidateCellText(hanzi, null)
        candidateDisplayMode == CandidateDisplayMode.ROMAN_ONLY -> CandidateCellText(displayRoman, null)
        isTranslateSwapped -> CandidateCellText(hanzi, displayRoman)
        else -> CandidateCellText(displayRoman, hanzi)
    }
