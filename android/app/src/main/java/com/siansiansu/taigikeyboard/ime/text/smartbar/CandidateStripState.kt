package com.siansiansu.taigikeyboard.ime.text.smartbar

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
    val isTranslateSwapped: Boolean,
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
