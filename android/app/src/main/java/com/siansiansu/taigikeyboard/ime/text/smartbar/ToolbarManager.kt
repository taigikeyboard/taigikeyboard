package com.siansiansu.taigikeyboard.ime.text.smartbar

import android.util.Log
import android.view.View
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.PrefHelper

/**
 * Manages toolbar UI interactions extracted from SmartbarManager.
 *
 * Handles toolbar toggle animation, container switching, mode buttons,
 * and layout selection overlay.
 */
class ToolbarManager(
    private val prefs: PrefHelper,
    private val smartbarViewProvider: () -> SmartbarView?,
    private val candidateOverlayViewProvider: () -> CandidateOverlayView?,
    private val layoutSelectionOverlayViewProvider: () -> LayoutSelectionOverlayView?,
    private val symbolSelectionOverlayViewProvider: () -> SymbolSelectionOverlayView?,
    private val settingsSelectionOverlayViewProvider: () -> SettingsSelectionOverlayView?,
    private val onInputModeChanged: (String) -> Unit,
    private val onLayoutSelected: (String) -> Unit,
    private val onActiveContainerChanged: (Int) -> Unit,
    private val getKeyboardHeight: () -> Int,
) {
    // Skip updateActiveContainerVisibility() during animated transitions
    var isAnimatingContainerSwitch = false
        private set

    var activeContainerId: Int = R.id.candidates_container
        set(value) {
            field = value
            if (!isAnimatingContainerSwitch) updateActiveContainerVisibility()
        }

    // Container ID to return to when closing toolbar
    private var containerBeforeToolbar: Int = R.id.candidates_container

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
        if (activeContainerId == R.id.toolbar_container) {
            animateContainerSlide(R.id.toolbar_container, containerBeforeToolbar, expanding = false)
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

            if (activeContainerId == R.id.toolbar_container) {
                // × → + : collapse toolbar, restore previous container
                animateContainerSlide(R.id.toolbar_container, containerBeforeToolbar, expanding = false)
                animateToggleRotation(45f, 0f)
            } else {
                // + → × : expand toolbar
                containerBeforeToolbar = activeContainerId
                animateContainerSlide(activeContainerId, R.id.toolbar_container, expanding = true)
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
            val imm = context.getSystemService(android.content.Context.INPUT_METHOD_SERVICE) as android.view.inputmethod.InputMethodManager
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
        if (prefs.isToolbarAutoCollapse && activeContainerId == R.id.toolbar_container) {
            animateContainerSlide(R.id.toolbar_container, containerBeforeToolbar, expanding = false)
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

    fun getPreferredContainerId(): Int = R.id.candidates_container

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
        fromId: Int,
        toId: Int,
        expanding: Boolean,
    ) {
        val view = smartbarViewProvider() ?: return
        val contentFrame = view.findViewById<View>(R.id.smartbar_content_frame) ?: return
        val fromView = view.findViewById<View>(fromId) ?: return
        val toView = view.findViewById<View>(toId) ?: return
        val height = contentFrame.height.toFloat()

        if (height <= 0f) {
            fromView.visibility = View.GONE
            toView.visibility = View.VISIBLE
            activeContainerId = toId
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
                                activeContainerId = toId
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

        if (BuildConfig.DEBUG) {
            val containerName =
                when (activeContainerId) {
                    R.id.number_row -> "number_row"
                    R.id.candidates_container -> "candidates_container"
                    R.id.english_candidates_container -> "english_candidates_container"
                    R.id.toolbar_container -> "toolbar_container"
                    else -> "unknown($activeContainerId)"
                }
            Log.d(TAG, "[DEBUG] updateActiveContainerVisibility: $containerName")
        }

        val allContainers =
            listOf(
                smartbarView.candidatesContainer,
                smartbarView.englishCandidatesContainer,
                smartbarView.numberRowView,
                smartbarView.toolbarContainer,
            )

        allContainers.forEach { it?.visibility = View.GONE }

        when (activeContainerId) {
            R.id.number_row -> smartbarView.numberRowView?.visibility = View.VISIBLE
            R.id.candidates_container -> smartbarView.candidatesContainer?.visibility = View.VISIBLE
            R.id.english_candidates_container -> smartbarView.englishCandidatesContainer?.visibility = View.VISIBLE
            R.id.toolbar_container -> smartbarView.toolbarContainer?.visibility = View.VISIBLE
        }

        smartbarView.toolbarToggleButton?.visibility =
            when (activeContainerId) {
                R.id.number_row -> View.GONE
                else -> View.VISIBLE
            }

        smartbarView.toolbarToggleButton?.rotation =
            when (activeContainerId) {
                R.id.toolbar_container -> 45f
                else -> 0f
            }
    }

    companion object {
        private const val TAG = "ToolbarManager"
    }
}
