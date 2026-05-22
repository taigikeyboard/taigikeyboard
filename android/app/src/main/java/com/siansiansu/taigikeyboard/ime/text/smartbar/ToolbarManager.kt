package com.siansiansu.taigikeyboard.ime.text.smartbar

import android.view.View
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.debug

/**
 * Manages toolbar UI interactions extracted from SmartbarManager.
 *
 * Handles toolbar toggle animation, container switching, mode buttons,
 * and layout selection overlay.
 */
class ToolbarManager(
    private val prefs: PrefHelper,
    private val logger: LoggerBackend,
    private val smartbarViewProvider: () -> SmartbarView?,
    private val candidateOverlayViewProvider: () -> CandidateOverlayView?,
    private val layoutSelectionOverlayViewProvider: () -> LayoutSelectionOverlayView?,
    private val symbolSelectionOverlayViewProvider: () -> SymbolSelectionOverlayView?,
    private val settingsSelectionOverlayViewProvider: () -> SettingsSelectionOverlayView?,
    private val onInputModeChanged: (String) -> Unit,
    private val onLayoutSelected: (String) -> Unit,
    private val getKeyboardHeight: () -> Int,
) {
    // Skip updateActiveContainerVisibility() during animated transitions
    var isAnimatingContainerSwitch = false
        private set

    var activeContainer: SmartbarContainer = SmartbarContainer.CANDIDATES
        set(value) {
            field = value
            if (!isAnimatingContainerSwitch) updateActiveContainerVisibility()
        }

    // Container to return to when closing toolbar
    private var containerBeforeToolbar: SmartbarContainer = SmartbarContainer.CANDIDATES

    // Debounce rapid mode button clicks to prevent race conditions
    private var lastModeChangeTime = 0L
    private val MODE_CHANGE_DEBOUNCE_MS = 300L

    // Track active container slide animator to cancel on re-entry
    private var containerSlideAnimator: android.animation.AnimatorSet? = null

    /**
     * Collapse toolbar if it's currently open (no-op otherwise).
     */
    fun collapseToolbarIfOpen() {
        symbolSelectionOverlayViewProvider()?.hide()
        settingsSelectionOverlayViewProvider()?.hide()
        if (activeContainer == SmartbarContainer.TOOLBAR) {
            animateContainerSlide(SmartbarContainer.TOOLBAR, containerBeforeToolbar, expanding = false)
            animateToggleRotation(45f, 0f)
        }
    }

    /**
     * Set up toolbar toggle button and toolbar action buttons.
     */
    fun setupToolbar(smartbarView: SmartbarView) {
        // Toggle button: iOS-style + / × toggle with slide animation
        smartbarView.toolbarToggleButton?.setOnClickListener {
            // Hide overlays when toggling toolbar
            layoutSelectionOverlayViewProvider()?.hide()
            symbolSelectionOverlayViewProvider()?.hide()
            settingsSelectionOverlayViewProvider()?.hide()

            if (activeContainer == SmartbarContainer.TOOLBAR) {
                // × → + : collapse toolbar, restore previous container
                animateContainerSlide(SmartbarContainer.TOOLBAR, containerBeforeToolbar, expanding = false)
                animateToggleRotation(45f, 0f)
            } else {
                // + → × : expand toolbar
                containerBeforeToolbar = activeContainer
                animateContainerSlide(activeContainer, SmartbarContainer.TOOLBAR, expanding = true)
                animateToggleRotation(0f, 45f)
                updateToolbarModeSwitcherState()
            }
        }

        // Layout switcher button
        smartbarView.findViewById<View>(R.id.toolbar_layout_button)?.setOnClickListener {
            showLayoutSelection()
        }

        // Symbol panel button
        smartbarView.findViewById<View>(R.id.toolbar_symbol_button)?.setOnClickListener {
            showSymbolSelection()
        }

        // Dismiss keyboard button
        smartbarView.findViewById<View>(R.id.toolbar_dismiss_button)?.setOnClickListener {
            onInputModeChanged("hide_self")
        }

        // Globe button: open system IME picker
        smartbarView.findViewById<View>(R.id.toolbar_globe_button)?.setOnClickListener {
            val context = smartbarView.context
            val imm = context.getSystemService(
                android.content.Context.INPUT_METHOD_SERVICE,
            ) as android.view.inputmethod.InputMethodManager
            imm.showInputMethodPicker()
        }

        // Mode buttons in toolbar
        smartbarView.findViewById<android.widget.Button>(R.id.toolbar_mode_poj)?.setOnClickListener {
            setInputMode("poj")
        }
        smartbarView.findViewById<android.widget.Button>(R.id.toolbar_mode_tl)?.setOnClickListener {
            setInputMode("tl")
        }
        smartbarView.findViewById<android.widget.Button>(R.id.toolbar_mode_en)?.setOnClickListener {
            setInputMode("english")
        }
        smartbarView.findViewById<android.widget.Button>(R.id.toolbar_mode_tps)?.setOnClickListener {
            setInputMode("tps")
        }

        // Settings button in toolbar — show settings overlay
        smartbarView.findViewById<View>(R.id.toolbar_settings_button)?.setOnClickListener {
            showSettingsSelection()
        }
    }

    /**
     * Set input mode and immediately update UI.
     */
    private fun setInputMode(mode: String) {
        val now = android.os.SystemClock.elapsedRealtime()
        if (now - lastModeChangeTime < MODE_CHANGE_DEBOUNCE_MS) return
        lastModeChangeTime = now

        prefs.inputMode = mode
        updateInputModeSwitcherState(mode)
        updateToolbarModeSwitcherState(mode)

        // Close any open overlay panels
        symbolSelectionOverlayViewProvider()?.hide()
        layoutSelectionOverlayViewProvider()?.hide()
        settingsSelectionOverlayViewProvider()?.hide()

        // Auto-collapse toolbar after mode selection
        if (prefs.isToolbarAutoCollapse && activeContainer == SmartbarContainer.TOOLBAR) {
            animateContainerSlide(SmartbarContainer.TOOLBAR, containerBeforeToolbar, expanding = false)
            animateToggleRotation(45f, 0f)
        }
    }

    /**
     * Update input mode switcher button states.
     */
    fun updateInputModeSwitcherState(currentMode: String? = null) {
        val mode = currentMode ?: prefs.inputMode
        updateToolbarModeSwitcherState(mode)
    }

    /**
     * Update toolbar mode button selected states.
     */
    fun updateToolbarModeSwitcherState(currentMode: String? = null) {
        val mode = currentMode ?: prefs.inputMode
        val smartbarView = smartbarViewProvider()
        smartbarView?.findViewById<android.widget.Button>(R.id.toolbar_mode_poj)?.isSelected = (mode == "poj")
        smartbarView?.findViewById<android.widget.Button>(R.id.toolbar_mode_tl)?.isSelected = (mode == "tl")
        smartbarView?.findViewById<android.widget.Button>(R.id.toolbar_mode_en)?.isSelected = (mode == "english")
        smartbarView?.findViewById<android.widget.Button>(R.id.toolbar_mode_tps)?.isSelected = (mode == "tps")
    }

    /**
     * Show layout selection overlay.
     */
    private fun showLayoutSelection() {
        val overlay = layoutSelectionOverlayViewProvider() ?: return
        if (overlay.isVisible()) {
            overlay.hide()
            return
        }
        candidateOverlayViewProvider()?.hide()
        symbolSelectionOverlayViewProvider()?.hide()
        settingsSelectionOverlayViewProvider()?.hide()
        overlay.show(getKeyboardHeight())
    }

    /**
     * Show symbol selection overlay.
     */
    private fun showSymbolSelection() {
        val overlay = symbolSelectionOverlayViewProvider() ?: return
        if (overlay.isVisible()) {
            overlay.hide()
            return
        }
        candidateOverlayViewProvider()?.hide()
        layoutSelectionOverlayViewProvider()?.hide()
        settingsSelectionOverlayViewProvider()?.hide()
        overlay.show(getKeyboardHeight())
    }

    /**
     * Show settings selection overlay.
     */
    private fun showSettingsSelection() {
        val overlay = settingsSelectionOverlayViewProvider() ?: return
        if (overlay.isVisible()) {
            overlay.hide()
            return
        }
        candidateOverlayViewProvider()?.hide()
        layoutSelectionOverlayViewProvider()?.hide()
        symbolSelectionOverlayViewProvider()?.hide()
        overlay.show(getKeyboardHeight())
    }

    val preferredContainer: SmartbarContainer
        get() = SmartbarContainer.CANDIDATES

    /**
     * Animate the toolbar toggle button rotation (+ ↔ ×).
     */
    private fun animateToggleRotation(
        from: Float,
        to: Float,
    ) {
        smartbarViewProvider()?.toolbarToggleButton?.let { btn ->
            android.animation.ObjectAnimator.ofFloat(btn, "rotation", from, to).apply {
                duration = 200
                interpolator = android.view.animation.PathInterpolator(0.42f, 0f, 0.58f, 1f)
                start()
            }
        }
    }

    /**
     * Animated vertical slide transition between two containers.
     */
    private fun animateContainerSlide(
        from: SmartbarContainer,
        to: SmartbarContainer,
        expanding: Boolean,
    ) {
        val view = smartbarViewProvider() ?: return
        val contentFrame = view.findViewById<View>(R.id.smartbar_content_frame) ?: return
        val fromView = from.viewIn(view) ?: return
        val toView = to.viewIn(view) ?: return
        val height = contentFrame.height.toFloat()

        if (height <= 0f) {
            fromView.visibility = View.GONE
            toView.visibility = View.VISIBLE
            activeContainer = to
            return
        }

        containerSlideAnimator?.cancel()
        isAnimatingContainerSwitch = true

        val toStartY = if (expanding) height else -height
        val fromTargetY = if (expanding) -height else height

        toView.translationY = toStartY
        toView.visibility = View.INVISIBLE

        contentFrame.post {
            toView.visibility = View.VISIBLE

            val animator =
                android.animation.ValueAnimator.ofFloat(0f, 1f).apply {
                    duration = 200
                    interpolator = android.view.animation.DecelerateInterpolator()
                    addUpdateListener { anim ->
                        val fraction = anim.animatedFraction
                        fromView.translationY = fromTargetY * fraction
                        toView.translationY = toStartY * (1f - fraction)
                    }
                    addListener(
                        object : android.animation.AnimatorListenerAdapter() {
                            override fun onAnimationEnd(animation: android.animation.Animator) {
                                fromView.visibility = View.GONE
                                fromView.translationY = 0f
                                toView.translationY = 0f
                                activeContainer = to
                                isAnimatingContainerSwitch = false
                                containerSlideAnimator = null
                            }
                        },
                    )
                }

            containerSlideAnimator =
                android.animation.AnimatorSet().apply {
                    play(animator)
                    start()
                }
        }
    }

    fun updateActiveContainerVisibility() {
        val smartbarView = smartbarViewProvider() ?: return
        val target = activeContainer

        logger.debug(TAG) { "[DEBUG] updateActiveContainerVisibility: ${target.name}" }

        SmartbarContainer.entries.forEach { container ->
            container.viewIn(smartbarView)?.visibility =
                if (container == target) View.VISIBLE else View.GONE
        }

        smartbarView.toolbarToggleButton?.visibility =
            if (target == SmartbarContainer.NUMBER_ROW) View.GONE else View.VISIBLE

        smartbarView.toolbarToggleButton?.rotation =
            if (target == SmartbarContainer.TOOLBAR) 45f else 0f
    }

    companion object {
        private const val TAG = "ToolbarManager"
    }
}
