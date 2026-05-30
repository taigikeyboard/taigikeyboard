package com.siansiansu.taigikeyboard.ime.text.smartbar

import android.content.Context
import android.util.AttributeSet
import android.util.TypedValue
import android.view.Gravity
import android.widget.FrameLayout
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.platform.ComposeView
import androidx.compose.ui.platform.ViewCompositionStrategy
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.CompositionRoot
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ime.core.logging.debug
import com.siansiansu.taigikeyboard.ui.theme.TaigiKeyboardTheme

/**
 * Layout selection overlay view.
 *
 * Thin FrameLayout shell hosting Compose M3 content ([LayoutOverlayContent] — preview-card
 * sections), following the ComposeView pattern of SymbolSelectionOverlayView. Card colors honor
 * the user-customizable keyboard theme ([KeyboardChromeColors]).
 */
class LayoutSelectionOverlayView : FrameLayout {
    companion object {
        private const val TAG = "LayoutSelectionOverlay"
    }

    // A7: IME-only overlay; `context` resolves to the `TaigiKeyboard` service.
    private val prefs: PrefHelper get() = (context as TaigiKeyboard).prefs
    private val logger by lazy { CompositionRoot.shared(context).logger }

    private var isShowing: Boolean = false
    private var composeView: ComposeView? = null

    // Bumped on each show() to re-resolve keyboard colors + re-seed the selected layout from prefs.
    private val refreshTrigger = mutableIntStateOf(0)

    var onLayoutSelected: ((String) -> Unit)? = null

    constructor(context: Context) : this(context, null)
    constructor(context: Context, attrs: AttributeSet?) : this(context, attrs, 0)
    constructor(context: Context, attrs: AttributeSet?, defStyleAttr: Int) : super(context, attrs, defStyleAttr)

    init {
        visibility = GONE
        val typedValue = TypedValue()
        if (context.theme.resolveAttribute(R.attr.keyboard_bgColor, typedValue, true)) {
            setBackgroundColor(typedValue.data)
        }
    }

    override fun onAttachedToWindow() {
        super.onAttachedToWindow()

        // DisposeOnDetachedFromWindow (not …ViewTreeLifecycleDestroyed): the IME input view
        // detaches on config change while the service Lifecycle stays alive, so the composition
        // must dispose at the detach boundary. See SymbolSelectionOverlayView for the same rationale.
        composeView =
            ComposeView(context).apply {
                setViewCompositionStrategy(ViewCompositionStrategy.DisposeOnDetachedFromWindow)
            }

        addView(
            composeView,
            LayoutParams(
                LayoutParams.MATCH_PARENT,
                LayoutParams.MATCH_PARENT,
            ),
        )

        composeView?.setContent {
            TaigiKeyboardTheme {
                val trigger by refreshTrigger
                LayoutOverlayContent(
                    chromeColors = rememberKeyboardChromeColors(trigger),
                    selectedKey = remember(trigger) { prefs.keyboardLayoutType },
                    resetKey = trigger,
                    onLayoutSelected = { key ->
                        // Persist BEFORE notifying so onKeyboardLayoutTypeChanged sees the new pref.
                        prefs.keyboardLayoutType = key
                        onLayoutSelected?.invoke(key)
                        // The registered callback already hides synchronously; this defensive close
                        // covers a null callback so the overlay still collapses on selection.
                        hide()
                    },
                )
            }
        }
    }

    override fun onDetachedFromWindow() {
        super.onDetachedFromWindow()
        // Remove the child + drop the ref so a later reattach rebuilds a single fresh ComposeView
        // instead of stacking a second one on top of the (now composition-disposed) old child.
        composeView?.let { removeView(it) }
        composeView = null
    }

    /**
     * Show the overlay.
     * @param keyboardHeight Total keyboard height (smartbar + keyboard) to size the overlay.
     */
    fun show(keyboardHeight: Int) {
        if (isShowing) return

        if (keyboardHeight > 0) {
            val smartbarHeight = resources.getDimensionPixelSize(R.dimen.smartbar_height)
            val overlayHeight = keyboardHeight - smartbarHeight
            layoutParams = (layoutParams as? FrameLayout.LayoutParams)?.apply {
                height = overlayHeight
                topMargin = smartbarHeight
            } ?: FrameLayout
                .LayoutParams(
                    FrameLayout.LayoutParams.MATCH_PARENT,
                    overlayHeight,
                ).apply {
                    gravity = Gravity.TOP
                    topMargin = smartbarHeight
                }
        }

        // Re-resolve colors + re-seed the selected layout from prefs.
        refreshTrigger.intValue++
        visibility = VISIBLE
        isShowing = true

        logger.debug(TAG) { "[SHOW] Layout selection overlay shown, height=$keyboardHeight" }
    }

    /**
     * Hide the overlay.
     */
    fun hide() {
        if (!isShowing) return

        visibility = GONE
        isShowing = false

        logger.debug(TAG) { "[HIDE] Layout selection overlay hidden" }
    }

    fun isVisible(): Boolean = isShowing
}
