package com.siansiansu.taigikeyboard.ime.text.smartbar

import android.content.Context
import android.util.AttributeSet
import android.util.TypedValue
import android.view.Gravity
import android.widget.FrameLayout
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.ui.platform.ComposeView
import androidx.compose.ui.platform.ViewCompositionStrategy
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.CompositionRoot
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ime.core.logging.debug
import com.siansiansu.taigikeyboard.ui.theme.TaigiKeyboardTheme

/**
 * Settings selection overlay view.
 *
 * Covers the keyboard area with settings toggles using Compose UI,
 * matching the overlay pattern of LayoutSelectionOverlayView and SymbolSelectionOverlayView.
 * Follows the ComposeView integration pattern established by EmojiKeyboardView.
 */
class SettingsSelectionOverlayView : FrameLayout {
    companion object {
        private const val TAG = "SettingsSelectionOverlay"
    }

    private var isShowing: Boolean = false
    private var composeView: ComposeView? = null

    private val logger by lazy { CompositionRoot.shared(context).logger }

    // Incremented on each show() to refresh Compose toggle states from prefs
    private val refreshTrigger = mutableIntStateOf(0)

    var onOpenApp: (() -> Unit)? = null
    var onHide: (() -> Unit)? = null

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

        composeView =
            ComposeView(context).apply {
                setViewCompositionStrategy(ViewCompositionStrategy.DisposeOnViewTreeLifecycleDestroyed)
            }

        addView(
            composeView,
            LayoutParams(
                LayoutParams.MATCH_PARENT,
                LayoutParams.MATCH_PARENT,
            ),
        )

        // A7: IME-only overlay; `context` resolves to the `TaigiKeyboard`
        // service, so we can hand the Application-owned prefs to the
        // Compose content instead of resurrecting a `getInstance()` reach.
        val prefs = (context as TaigiKeyboard).prefs
        composeView?.setContent {
            TaigiKeyboardTheme {
                val trigger by refreshTrigger
                SettingsOverlayContent(
                    prefs = prefs,
                    refreshTrigger = trigger,
                    onDismiss = { hide() },
                    onOpenApp = { onOpenApp?.invoke() },
                )
            }
        }
    }

    override fun onDetachedFromWindow() {
        super.onDetachedFromWindow()
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

        // Trigger Compose recomposition to refresh toggle states from prefs
        refreshTrigger.intValue++
        visibility = VISIBLE
        isShowing = true

        logger.debug(TAG) { "[SHOW] Settings selection overlay shown, height=$keyboardHeight" }
    }

    /**
     * Hide the overlay.
     */
    fun hide() {
        if (!isShowing) return

        visibility = GONE
        isShowing = false
        onHide?.invoke()

        logger.debug(TAG) { "[HIDE] Settings selection overlay hidden" }
    }

    fun isVisible(): Boolean = isShowing
}
