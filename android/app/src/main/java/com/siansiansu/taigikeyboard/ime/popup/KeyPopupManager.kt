package com.siansiansu.taigikeyboard.ime.popup

import android.view.Gravity
import android.view.MotionEvent
import android.view.View
import android.view.ViewGroup
import android.widget.PopupWindow
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.platform.ComposeView
import androidx.compose.ui.platform.ViewCompositionStrategy
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.setViewTreeLifecycleOwner
import androidx.lifecycle.setViewTreeViewModelStoreOwner
import androidx.savedstate.setViewTreeSavedStateRegistryOwner
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.i18n.ProvideDisplayLanguage
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ime.core.isKeyboardNightMode
import com.siansiansu.taigikeyboard.ime.text.key.KeyCode
import com.siansiansu.taigikeyboard.ime.text.key.KeyData
import com.siansiansu.taigikeyboard.ime.text.keyboard.AnchorSide
import com.siansiansu.taigikeyboard.ime.text.keyboard.ExtendedPopupGeometryInput
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardLayoutSolver
import com.siansiansu.taigikeyboard.ime.text.keyboard.PopupDimensionsInput
import com.siansiansu.taigikeyboard.ime.theme.getColorFromAttr
import com.siansiansu.taigikeyboard.typeface.TypefaceLoader
import com.siansiansu.taigikeyboard.ui.theme.TaigiKeyboardTheme
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

/**
 * Drives the preview + extended popup [PopupWindow]s for the Compose keyboard
 * body. Decoupled from the keyboard layer via [KeyAnchor] so the popup layer
 * never reaches back into [com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardLayout]
 * for view geometry or label resolution — both are pre-resolved by the
 * caller and passed in as anchor fields.
 *
 * Construct once per IME-service lifecycle. The owning IME ([TaigiKeyboard])
 * provides the popup [androidx.compose.runtime.Recomposer] and the host
 * [View] used as the `showAtLocation` parent.
 */
class KeyPopupManager(
    private val ime: TaigiKeyboard,
) : PopupHost {
    private val composeView: ComposeView
    private val composeViewExt: ComposeView
    private val window: PopupWindow
    private val windowExt: PopupWindow

    private val _previewState = MutableStateFlow<PreviewState>(PreviewState.Hidden)
    private val _extendedState = MutableStateFlow<ExtendedState>(ExtendedState.Hidden)

    private var ownersInstalled = false

    /** Window-attached host view used as the `showAtLocation` parent. Set by
     *  [attachHostView] once the IME root has been registered; null in early
     *  startup or after detach. */
    private var hostView: View? = null

    /** Most recent anchor passed to [show] / [extend]. Reused by
     *  [propagateMotionEvent] / [activeKeyData] so callers don't repeat the
     *  anchor on every motion event. */
    private var lastAnchor: KeyAnchor? = null
    private var keyPopupWidth: Int
    private var keyPopupHeight: Int
    private var keyPopupDiffX: Int = 0

    /** Geometry from the most recent [extend]. Drives [propagateMotionEvent]
     *  hit-test math without re-deriving anchor side / row split. */
    private var anchorSide: AnchorSide = AnchorSide.LEFT
    private var anchorOffset: Int = 0
    private var row0count: Int = 0
    private var row1count: Int = 0
    private var activeExtIndex: Int? = null

    /** Memoised [resolveDisplayParams] output. Re-resolved only when one of its
     *  inputs flips (see [DisplayKey]); every other press reuses it, so the five
     *  theme-attr lookups + typeface load stop running on each touch-down. */
    private var cachedDisplayKey: DisplayKey? = null
    private var cachedDisplay: PopupDisplayParams? = null

    override val isShowingPopup: Boolean
        get() = _previewState.value is PreviewState.Visible

    /** True while the extended popup window is on screen. Tracks the window
     *  rather than [_extendedState] because dismiss() detaches the ComposeView
     *  before the state mutation can be observed. */
    override val isShowingExtendedPopup: Boolean
        get() = windowExt.isShowing

    init {
        keyPopupWidth = ime.resources.getDimension(R.dimen.key_width).toInt()
        keyPopupHeight = ime.resources.getDimension(R.dimen.key_height).toInt()

        composeView = popupComposeView(_previewState) { state ->
            if (state is PreviewState.Visible) KeyPopupBox(state)
        }
        composeViewExt = popupComposeView(_extendedState) { state ->
            if (state is ExtendedState.Visible) KeyPopupExtendedBox(state)
        }

        window = createPopupWindow(composeView)
        windowExt = createPopupWindow(composeViewExt)
    }

    private fun <S> popupComposeView(
        state: StateFlow<S>,
        content: @Composable (S) -> Unit,
    ): ComposeView =
        ComposeView(ime).apply {
            layoutParams = ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.MATCH_PARENT,
            )
            // The window is dismissed on every key UP (see [hide]), which detaches this
            // view. The default DisposeOnDetachedFromWindow would then tear the whole
            // composition down and rebuild it on the next DOWN — theme, display-language
            // scope and its `createConfigurationContext` included. Tie the composition to
            // the IME lifecycle instead so dismiss/show only re-attaches the view.
            setViewCompositionStrategy(ViewCompositionStrategy.DisposeOnViewTreeLifecycleDestroyed)
            setContent {
                TaigiKeyboardTheme {
                    // Popups compose in a SEPARATE PopupWindow tree from the keyboard body, so they
                    // need their own ProvideDisplayLanguage scope — KeyPopupBox resolves its a11y
                    // contentDescription via stringRes, which crashes without LocalStringResolver in
                    // scope. Follows the same DataStore tag as the rest of the IME for live switch.
                    ProvideDisplayLanguage(ime.prefs) {
                        val s by state.collectAsStateWithLifecycle()
                        content(s)
                    }
                }
            }
        }

    private fun createPopupWindow(view: View): PopupWindow =
        PopupWindow(ime).apply {
            animationStyle = 0
            contentView = view
            enterTransition = null
            exitTransition = null
            isClippingEnabled = false
            isFocusable = false
            isTouchable = false
            setBackgroundDrawable(null)
        }

    /**
     * Wires the popup [ComposeView]s to the IME service so they can compose
     * inside their host [PopupWindow]. Two pieces are required and neither
     * substitutes for the other:
     *
     * 1. `setParentCompositionContext(ime.popupRecomposer)` — shortcuts
     *    [androidx.compose.ui.platform.AbstractComposeView.resolveParentCompositionContext]
     *    so it does not call `findOrCreateWindowRecomposer` on the popup
     *    window's `PopupDecorView`. That default lookup fails because
     *    PopupDecorView has no `ViewTreeLifecycleOwner` tag (the IME's
     *    [com.siansiansu.taigikeyboard.ime.lifecycle.LifecycleInputMethodService.installViewTreeOwners]
     *    only tags the IME decor view, not the popup window's separate
     *    decor view) — historic crash signature
     *    `IllegalStateException: ViewTreeLifecycleOwner not found from
     *     android.widget.PopupWindow$PopupDecorView`.
     *
     * 2. `setViewTree*Owner` — `AndroidComposeView` still consumes
     *    `LocalLifecycleOwner` / `LocalSavedStateRegistryOwner` /
     *    `LocalViewModelStoreOwner` from the view tree at composition
     *    time, including for `collectAsStateWithLifecycle`. Removing
     *    these tags would replace the `PopupDecorView` crash with a
     *    different one inside `AndroidComposeView`.
     *
     * Idempotent — safe to call repeatedly.
     */
    fun installPopupViewTreeOwnersIfNeeded() {
        if (ownersInstalled) return
        composeView.setParentCompositionContext(ime.popupRecomposer)
        composeViewExt.setParentCompositionContext(ime.popupRecomposer)

        composeView.setViewTreeLifecycleOwner(ime)
        composeView.setViewTreeViewModelStoreOwner(ime)
        composeView.setViewTreeSavedStateRegistryOwner(ime)
        composeViewExt.setViewTreeLifecycleOwner(ime)
        composeViewExt.setViewTreeViewModelStoreOwner(ime)
        composeViewExt.setViewTreeSavedStateRegistryOwner(ime)
        ownersInstalled = true
    }

    /** Attaches the host [View] used as the `showAtLocation` parent. Called
     *  by [com.siansiansu.taigikeyboard.ime.text.TextInputManager] once the
     *  IME root view is registered. */
    fun attachHostView(view: View) {
        hostView = view
    }

    /**
     * Returns the display params for the current theme / font / screen config,
     * re-resolving only when one of those inputs changed since the last press.
     * Same shape as [com.siansiansu.taigikeyboard.ime.core.ThemeAppearanceCache].
     */
    private fun displayParams(): PopupDisplayParams {
        val config = ime.resources.configuration
        val key = DisplayKey(
            isNightMode = isKeyboardNightMode(ime),
            fontType = ime.prefs.fontType,
            densityDpi = config.densityDpi,
            fontScale = config.fontScale,
        )
        val cached = cachedDisplay
        if (cached != null && key == cachedDisplayKey) return cached
        return resolveDisplayParams().also {
            cachedDisplayKey = key
            cachedDisplay = it
        }
    }

    private fun resolveDisplayParams(): PopupDisplayParams {
        val res = ime.resources
        val density = res.displayMetrics.density
        val prefs: PrefHelper = ime.prefs
        return PopupDisplayParams(
            fgColorArgb = getColorFromAttr(ime, R.attr.key_popup_fgColor),
            bgColorArgb = getColorFromAttr(ime, R.attr.key_popup_bgColor),
            extBgColorArgb = getColorFromAttr(ime, R.attr.key_popup_extended_bgColor),
            extBgColorActiveArgb = getColorFromAttr(ime, R.attr.key_popup_extended_bgColorActive),
            shadowColorArgb = getColorFromAttr(ime, R.attr.key_popup_extended_shadowColor),
            cornerRadiusPx = res.getDimension(R.dimen.key_borderRadius),
            keyHeightPx = res.getDimension(R.dimen.key_height).toInt(),
            popupTextSizePx = res.getDimension(R.dimen.key_popup_textSize),
            threeDotsSizePx = (16f * density).toInt(),
            ringWidthPx = (1f * density).toInt().coerceAtLeast(1),
            iconPaddingFraction = 0.2f,
            typeface = TypefaceLoader.getTypefaceByType(prefs.fontType, ime),
        )
    }

    /**
     * Shows a preview popup for the given [anchor]. Mirrors the legacy
     * `KeyView.onFlorisTouchEvent ACTION_DOWN` path: keys with code <= SPACE
     * and no popup cells (and not in [ANCHOR_EXCEPTIONS]) skip the visual
     * popup but the manager still records the anchor so a subsequent
     * [extend] sees consistent dimensions.
     */
    override fun show(anchor: KeyAnchor) {
        val popupDims = KeyboardLayoutSolver.solvePopupDimensions(
            PopupDimensionsInput(
                desiredKeyWidth = anchor.desiredKeyWidth,
                desiredKeyHeight = anchor.desiredKeyHeight,
                keyViewMeasuredWidth = anchor.measuredWidth,
                isLandscape = anchor.isLandscape,
            ),
        )
        keyPopupWidth = popupDims.popupWidth
        keyPopupHeight = popupDims.popupHeight
        keyPopupDiffX = popupDims.popupDiffX
        lastAnchor = anchor

        // Two skip paths share the same outcome (no preview popup, but
        // dimensions stay cached for a follow-up extend()):
        //  - low key codes with no popup variants (legacy "code <= SPACE
        //    with empty popup" filter)
        //  - codes in ANCHOR_EXCEPTIONS that always opt out of the preview
        if ((anchor.data.code <= KeyCode.SPACE && anchor.data.popup.isEmpty()) ||
            ANCHOR_EXCEPTIONS.contains(anchor.data.code)
        ) {
            return
        }

        installPopupViewTreeOwnersIfNeeded()

        val display = displayParams()
        _previewState.value = PreviewState.Visible(
            label = anchor.computedLabel,
            showThreeDots = anchor.data.popup.isNotEmpty(),
            popupWidthPx = keyPopupWidth,
            popupHeightPx = keyPopupHeight,
            display = display,
        )

        val host = hostView ?: return
        val popupX = anchor.xInWindow + keyPopupDiffX
        val popupY = anchor.yInWindow + anchor.measuredHeight + (-keyPopupHeight)
        if (window.isShowing) {
            window.update(popupX, popupY, keyPopupWidth, keyPopupHeight)
        } else {
            window.width = keyPopupWidth
            window.height = keyPopupHeight
            window.showAtLocation(host, Gravity.NO_GRAVITY, popupX, popupY)
        }
    }

    /**
     * Extends the currently showing preview popup with the popup-cell grid
     * from [anchor]. Layout shape (anchor side / row split / anchor offset)
     * is owned by [KeyboardLayoutSolver.solveExtendedPopupGeometry];
     * [propagateMotionEvent] consumes its outputs without re-deriving
     * thresholds.
     */
    override fun extend(anchor: KeyAnchor) {
        val popupCount = anchor.data.popup.size
        if (popupCount == 0) return

        val geometry = KeyboardLayoutSolver.solveExtendedPopupGeometry(
            ExtendedPopupGeometryInput(
                popupCount = popupCount,
                keyViewX = anchor.xInKeyboard.toFloat(),
                keyboardViewMeasuredWidth = anchor.keyboardWidth,
                keyViewMeasuredWidth = anchor.measuredWidth,
                keyViewMeasuredHeight = anchor.measuredHeight,
                keyPopupWidth = keyPopupWidth,
                keyPopupHeight = keyPopupHeight,
            ),
        )
        anchorSide = geometry.anchorSide
        row0count = geometry.row0count
        row1count = geometry.row1count
        anchorOffset = geometry.anchorOffset
        lastAnchor = anchor

        installPopupViewTreeOwnersIfNeeded()

        val display = displayParams()
        val cells = anchor.popupCells.value

        // Closed-form: the initially-active cell sits at row1count + offset
        // for LEFT anchors and at row1count + (row0count - 1 - offset) for
        // RIGHT anchors. Falls back to -1 when the cell list is empty so
        // ExtendedState carries the legacy "no active highlight" sentinel.
        val initialActive = when {
            cells.isEmpty() -> -1
            anchorSide == AnchorSide.LEFT -> row1count + anchorOffset
            else -> row1count + row0count - 1 - anchorOffset
        }.takeIf { it in cells.indices } ?: -1
        activeExtIndex = if (initialActive >= 0) initialActive else null

        _extendedState.value = ExtendedState.Visible(
            cells = cells,
            activeIndex = initialActive,
            anchorSide = geometry.anchorSide,
            row0count = geometry.row0count,
            row1count = geometry.row1count,
            cellWidthPx = keyPopupWidth,
            cellHeightPx = anchor.measuredHeight,
            totalWidthPx = geometry.extWidth,
            totalHeightPx = geometry.extHeight,
            display = display,
        )

        // Hide the three-dots indicator on the preview popup while the
        // extended popup is showing — preview text content stays visible.
        val previewVisible = _previewState.value as? PreviewState.Visible
        if (previewVisible != null) {
            _previewState.value = previewVisible.copy(showThreeDots = false)
        }

        val host = hostView ?: return
        val popupX = anchor.xInWindow + geometry.popupX
        val popupY = anchor.yInWindow + anchor.measuredHeight + geometry.popupY
        if (windowExt.isShowing) {
            windowExt.update(popupX, popupY, geometry.extWidth, geometry.extHeight)
        } else {
            windowExt.width = geometry.extWidth
            windowExt.height = geometry.extHeight
            windowExt.showAtLocation(host, Gravity.NO_GRAVITY, popupX, popupY)
        }
    }

    /**
     * Updates the active popup cell from a key-relative [event]. Returns
     * `true` while the pointer remains within the extended-popup grid;
     * `false` once it leaves so the caller (KeyTouchCoordinator) can cancel.
     *
     * Coord math is a direct port of the legacy
     * [com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardView]
     * implementation — the pointer is expressed in key-relative pixels and
     * the math walks anchor-side rows backwards from the anchor offset.
     */
    override fun propagateMotionEvent(event: MotionEvent): Boolean {
        if (!isShowingExtendedPopup) return false
        val anchor = lastAnchor ?: return false

        val kX: Float = event.x / keyPopupWidth.toFloat()

        if (event.y < -keyPopupHeight || event.y > 0.9f * keyPopupHeight) {
            return false
        }

        val newActiveIndex = when (anchorSide) {
            AnchorSide.LEFT -> when {
                event.x < keyPopupDiffX - (anchorOffset + 1) * keyPopupWidth ||
                    event.x > (keyPopupDiffX + (row0count + 1 - anchorOffset) * keyPopupWidth) -> {
                    return false
                }
                event.y < 0 && row1count > 0 -> when {
                    kX >= row1count - anchorOffset -> row1count - 1
                    kX < -anchorOffset -> 0
                    kX < 0 -> kX.toInt() - 1 + anchorOffset
                    else -> kX.toInt() + anchorOffset
                }
                else -> when {
                    kX >= row0count - anchorOffset -> row1count + row0count - 1
                    kX < -anchorOffset -> row1count
                    kX < 0 -> row1count + kX.toInt() - 1 + anchorOffset
                    else -> row1count + kX.toInt() + anchorOffset
                }
            }

            AnchorSide.RIGHT -> when {
                event.x > anchor.measuredWidth - keyPopupDiffX + (anchorOffset + 1) * keyPopupWidth ||
                    event.x < (
                        anchor.measuredWidth -
                            keyPopupDiffX - (row0count + 1 - anchorOffset) * keyPopupWidth
                    ) -> {
                    return false
                }
                event.y < 0 && row1count > 0 -> when {
                    kX >= anchorOffset -> row1count - 1
                    kX < -(row1count - 1 - anchorOffset) -> 0
                    kX < 0 -> row1count - 2 + kX.toInt() - anchorOffset
                    else -> row1count - 1 + kX.toInt() - anchorOffset
                }
                else -> when {
                    kX >= anchorOffset -> row1count + row0count - 1
                    kX < -(row0count - 1 - anchorOffset) -> row1count
                    kX < 0 -> row1count + row0count - 2 + kX.toInt() - anchorOffset
                    else -> row1count + row0count - 1 + kX.toInt() - anchorOffset
                }
            }
        }

        if (newActiveIndex != activeExtIndex) {
            activeExtIndex = newActiveIndex
            val current = _extendedState.value as? ExtendedState.Visible
            if (current != null) {
                _extendedState.value = current.copy(activeIndex = newActiveIndex)
            }
        }
        return true
    }

    /**
     * Returns the [KeyData] of the currently active extended-popup cell, or
     * the anchor's own [KeyData] if no extended cell is active. Drives the
     * commit-on-UP path inside [com.siansiansu.taigikeyboard.ime.text.keyboard.KeyTouchCoordinator].
     */
    override fun activeKeyData(): KeyData? {
        val anchor = lastAnchor ?: return null
        return anchor.data.popup.getOrNull(activeExtIndex ?: -1) ?: anchor.data
    }

    override fun hide() {
        _previewState.value = PreviewState.Hidden
        _extendedState.value = ExtendedState.Hidden
        // Tear down the preview window, not just the Compose state.
        // Phase D's switch from `showAsDropDown(keyView, …)` to
        // `showAtLocation(rootView, …)` removed the implicit anchor-View
        // lifecycle cleanup, so toggling `_previewState` to Hidden alone
        // leaves the previous frame painted on the popup decor view until
        // a new press triggers redraw — visible as a lingering callout on
        // tap UP. Pins
        // `INVARIANT_keyboard_popup_hide_dismisses_preview_window`.
        if (window.isShowing) {
            window.dismiss()
        }
        if (windowExt.isShowing) {
            windowExt.dismiss()
        }
        activeExtIndex = null
        // Clear the anchor so a subsequent press whose hit-test misses
        // (`activeKey == null`) doesn't read the previous press's KeyData
        // through `activeKeyData()`. Pins
        // `INVARIANT_keyboard_popup_hide_clears_anchor`.
        lastAnchor = null
    }

    override fun dismissAllPopups() = hide()

    /** The [resolveDisplayParams] inputs that can change at runtime today (night-mode
     *  theme variant, font pref, dp/sp scaling); equality drives the re-resolve gate.
     *  The theme style itself is fixed (`R.style.KeyboardTheme`) and the popup dimens
     *  have no qualifier variants, so neither needs a key field. */
    private data class DisplayKey(
        val isNightMode: Boolean,
        val fontType: String,
        val densityDpi: Int,
        val fontScale: Float,
    )

    private companion object {
        /** Key codes whose touch-down does NOT trigger a preview popup but
         *  may trigger an extended popup on long-press (legacy
         *  `KeyPopupManager.exceptionsForKeyCodes`). */
        val ANCHOR_EXCEPTIONS = setOf(
            KeyCode.ENTER,
            KeyCode.LANGUAGE_SWITCH,
            KeyCode.SWITCH_TO_TEXT_CONTEXT,
            KeyCode.SWITCH_TO_MEDIA_CONTEXT,
        )
    }
}
