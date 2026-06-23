package com.siansiansu.taigikeyboard.ime.text.smartbar

import android.content.Context
import android.util.AttributeSet
import android.util.TypedValue
import android.view.Gravity
import android.widget.FrameLayout
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.platform.ComposeView
import androidx.compose.ui.platform.ViewCompositionStrategy
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.i18n.DisplayLanguage
import com.siansiansu.taigikeyboard.i18n.ProvideDisplayLanguage
import com.siansiansu.taigikeyboard.ime.core.CompositionRoot
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ime.core.logging.debug
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord
import com.siansiansu.taigikeyboard.typeface.TypefaceLoader
import com.siansiansu.taigikeyboard.ui.theme.TaigiKeyboardTheme

/**
 * Candidate overlay view — expanded candidate grid over the keyboard.
 *
 * Thin FrameLayout shell hosting Compose M3 content ([CandidateOverlayContent] — a LazyColumn of
 * pixel-packed candidate rows + a fixed control panel), following the ComposeView pattern of
 * SymbolSelectionOverlayView / LayoutSelectionOverlayView. Cell colors honor the user-customizable
 * candidate theme attrs. Migrated from the legacy RecyclerView + adapter implementation.
 */
class CandidateOverlayView : FrameLayout {
    companion object {
        private const val TAG = "CandidateOverlayView"
    }

    // A7: IME-only overlay; `context` resolves to the `TaigiKeyboard` service.
    private val prefs: PrefHelper get() = (context as TaigiKeyboard).prefs
    private val logger by lazy { CompositionRoot.shared(context).logger }

    private var isShowing: Boolean = false
    private var composeView: ComposeView? = null

    // Live suggestions + translate-swap snapshot fed into the composition.
    private val suggestionsState = mutableStateOf<List<TaigiWord>>(emptyList())
    private val translateSwappedState = mutableStateOf(false)

    // Resolved theme background gradient stops (ARGB), or null for a flat/default theme.
    // Set on show() from the theme SmartbarManager already resolved, so the overlay
    // paints the gradient backdrop instead of the flat `?smartbar_bgColor` chrome.
    private val backgroundGradientState = mutableStateOf<List<Int>?>(null)

    // Resolved theme candidateTextColor (ARGB), or null for the adaptive default theme.
    // A light-only theme sets it so overlay text/control glyphs stay dark over the light
    // gradient in system dark mode instead of flipping white via the night attrs.
    private val candidateTextColorState = mutableStateOf<Int?>(null)

    // Bumped on each show() only: re-arms click protection + resets scroll/page (NOT on updateSuggestions).
    private val resetTrigger = mutableIntStateOf(0)

    var onCollapse: (() -> Unit)? = null
    var onSuggestionSelected: ((TaigiWord, Int) -> Unit)? = null
    var onTranslateToggle: (() -> Unit)? = null

    constructor(context: Context) : this(context, null)
    constructor(context: Context, attrs: AttributeSet?) : this(context, attrs, 0)
    constructor(context: Context, attrs: AttributeSet?, defStyleAttr: Int) : super(context, attrs, defStyleAttr)

    init {
        visibility = GONE
        val typedValue = TypedValue()
        if (context.theme.resolveAttribute(R.attr.smartbar_bgColor, typedValue, true)) {
            setBackgroundColor(typedValue.data)
        }
    }

    override fun onAttachedToWindow() {
        super.onAttachedToWindow()

        // DisposeOnDetachedFromWindow (not …ViewTreeLifecycleDestroyed): the IME input view detaches
        // on config change while the service Lifecycle stays alive, so the composition must dispose
        // at the detach boundary. See SymbolSelectionOverlayView for the same rationale.
        composeView =
            ComposeView(context).apply {
                setViewCompositionStrategy(ViewCompositionStrategy.DisposeOnDetachedFromWindow)
            }

        addView(
            composeView,
            LayoutParams(LayoutParams.MATCH_PARENT, LayoutParams.MATCH_PARENT),
        )

        composeView?.setContent {
            TaigiKeyboardTheme {
                val resetKey by resetTrigger
                val suggestions = suggestionsState.value
                val isTranslateSwapped = translateSwappedState.value
                // Read live each recomposition; recompose is driven by the states above (show/update),
                // matching the legacy re-measure cadence on submitRows().
                val isTPSLayout = prefs.keyboardLayoutType == "tps" || prefs.inputMode == "tps"
                val fontType = prefs.fontType
                val typeface = remember(fontType) { TypefaceLoader.getTypefaceByType(fontType, context) }
                // i18n live-switch: the IME (same process) follows the SAME DataStore tag the host
                // writes, so a host-side change recomposes this overlay's a11y strings live (no IME
                // service restart). Mirrors SymbolSelectionOverlayView.
                val displayLanguageTag by prefs
                    .observeDisplayLanguage()
                    .collectAsState(initial = prefs.displayLanguageTag)
                ProvideDisplayLanguage(DisplayLanguage.fromTag(displayLanguageTag)) {
                    CandidateOverlayContent(
                        suggestions = suggestions,
                        typeface = typeface,
                        isTPSLayout = isTPSLayout,
                        orMapsToER = prefs.tpsOrMapsToER,
                        isTranslateSwapped = isTranslateSwapped,
                        resetKey = resetKey,
                        backgroundGradient = backgroundGradientState.value,
                        candidateTextColor = candidateTextColorState.value,
                        onSuggestionSelected = { word, index -> onSuggestionSelected?.invoke(word, index) },
                        onCollapse = {
                            hide()
                            onCollapse?.invoke()
                        },
                        onTranslateToggle = { onTranslateToggle?.invoke() },
                    )
                }
            }
        }
    }

    override fun onDetachedFromWindow() {
        super.onDetachedFromWindow()
        // Remove the child + drop the ref so a later reattach rebuilds a single fresh ComposeView.
        composeView?.let { removeView(it) }
        composeView = null
    }

    /**
     * Show overlay.
     * @param suggestions candidate list
     * @param keyboardHeight total keyboard height (overlay covers the full keyboard incl. smartbar)
     * @param backgroundGradient resolved theme gradient stops (ARGB), or null for a flat theme
     * @param candidateTextColor resolved theme candidate text color (ARGB), or null for the adaptive default
     */
    fun show(
        suggestions: List<TaigiWord>,
        keyboardHeight: Int,
        backgroundGradient: List<Int>?,
        candidateTextColor: Int?,
    ) {
        if (isShowing) return
        if (suggestions.isEmpty()) return

        suggestionsState.value = suggestions
        translateSwappedState.value = cachedTranslateSwapped()
        backgroundGradientState.value = backgroundGradient
        candidateTextColorState.value = candidateTextColor

        if (keyboardHeight > 0) {
            layoutParams = (layoutParams as? FrameLayout.LayoutParams)?.apply {
                height = keyboardHeight
            } ?: FrameLayout
                .LayoutParams(FrameLayout.LayoutParams.MATCH_PARENT, keyboardHeight)
                .apply { gravity = Gravity.TOP }
        }

        resetTrigger.intValue++
        visibility = VISIBLE
        isShowing = true

        logger.debug(TAG) { "[SHOW] Candidate overlay shown with ${suggestions.size} suggestions, height=$keyboardHeight" }
    }

    /**
     * Hide overlay.
     */
    fun hide() {
        if (!isShowing) return

        visibility = GONE
        isShowing = false
        suggestionsState.value = emptyList()

        logger.debug(TAG) { "[HIDE] Candidate overlay hidden" }
    }

    /**
     * Replace the candidate list while shown (e.g. translate toggle, continuous re-fetch).
     * Does NOT re-arm click protection — only show() does.
     */
    fun updateSuggestions(suggestions: List<TaigiWord>) {
        if (suggestions.isEmpty()) {
            if (isShowing) hide()
            return
        }
        suggestionsState.value = suggestions
        translateSwappedState.value = cachedTranslateSwapped()
    }

    fun isVisible(): Boolean = isShowing

    private fun cachedTranslateSwapped(): Boolean =
        (context as TaigiKeyboard).smartbarManager.getCachedIsTranslateSwapped()
}
