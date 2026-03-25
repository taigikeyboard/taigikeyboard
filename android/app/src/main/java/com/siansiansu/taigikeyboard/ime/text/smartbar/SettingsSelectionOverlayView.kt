package com.siansiansu.taigikeyboard.ime.text.smartbar

import android.content.Context
import android.util.AttributeSet
import android.util.Log
import android.view.Gravity
import android.view.LayoutInflater
import android.view.View
import android.widget.FrameLayout
import android.widget.TextView
import androidx.appcompat.widget.SwitchCompat
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.localization.Tab4Texts

/**
 * Settings selection overlay view.
 *
 * Covers the keyboard area with settings toggles,
 * matching the overlay pattern of LayoutSelectionOverlayView and SymbolSelectionOverlayView.
 */
class SettingsSelectionOverlayView : FrameLayout {

    companion object {
        private const val TAG = "SettingsSelectionOverlay"
    }

    private val prefs: PrefHelper get() = TaigiKeyboard.getInstance().prefs
    private var isShowing: Boolean = false

    // Suppress listener callbacks during programmatic sync
    private var isSyncing: Boolean = false

    private var switchOutputBoth: SwitchCompat? = null
    private var switchAutoCap: SwitchCompat? = null
    private var switchAutoSpace: SwitchCompat? = null
    private var switchToolbarCollapse: SwitchCompat? = null
    private var switchDoubleOO: SwitchCompat? = null
    private var switchDoubleNN: SwitchCompat? = null
    private var switchTpsOrER: SwitchCompat? = null
    private var openAppButton: TextView? = null

    var onOpenApp: (() -> Unit)? = null
    var onHide: (() -> Unit)? = null

    constructor(context: Context) : this(context, null)
    constructor(context: Context, attrs: AttributeSet?) : this(context, attrs, 0)
    constructor(context: Context, attrs: AttributeSet?, defStyleAttr: Int) : super(context, attrs, defStyleAttr)

    init {
        LayoutInflater.from(context).inflate(R.layout.settings_selection_overlay, this, true)
        visibility = GONE
    }

    override fun onFinishInflate() {
        super.onFinishInflate()

        // Section headers
        findViewById<TextView>(R.id.settings_overlay_general_header)?.text =
            Tab4Texts.tabTitle.hanji
        findViewById<TextView>(R.id.settings_overlay_poj_header)?.text =
            Tab4Texts.pojSettingsSectionTitle.hanji
        findViewById<TextView>(R.id.settings_overlay_tps_header)?.text =
            Tab4Texts.tpsSettingsSectionTitle.hanji

        // Labels
        findViewById<TextView>(R.id.settings_overlay_label_output_both)?.text =
            Tab4Texts.outputBothScripts.hanji
        findViewById<TextView>(R.id.settings_overlay_label_auto_cap)?.text =
            Tab4Texts.autoCapitalization.hanji
        findViewById<TextView>(R.id.settings_overlay_label_auto_space)?.text =
            Tab4Texts.autoSpace.hanji
        findViewById<TextView>(R.id.settings_overlay_label_toolbar_collapse)?.text =
            Tab4Texts.toolbarAutoCollapse.hanji
        findViewById<TextView>(R.id.settings_overlay_label_double_oo)?.text =
            Tab4Texts.doubleTapOO.hanji
        findViewById<TextView>(R.id.settings_overlay_label_double_nn)?.text =
            Tab4Texts.doubleTapNN.hanji
        findViewById<TextView>(R.id.settings_overlay_label_tps_or_er)?.text =
            Tab4Texts.tpsOrMapsToER.hanji

        // Switches
        switchOutputBoth = findViewById(R.id.settings_overlay_switch_output_both)
        switchAutoCap = findViewById(R.id.settings_overlay_switch_auto_cap)
        switchAutoSpace = findViewById(R.id.settings_overlay_switch_auto_space)
        switchToolbarCollapse = findViewById(R.id.settings_overlay_switch_toolbar_collapse)
        switchDoubleOO = findViewById(R.id.settings_overlay_switch_double_oo)
        switchDoubleNN = findViewById(R.id.settings_overlay_switch_double_nn)
        switchTpsOrER = findViewById(R.id.settings_overlay_switch_tps_or_er)

        // Open App link
        openAppButton = findViewById<TextView>(R.id.settings_overlay_open_app_button)?.also { btn ->
            btn.text = Tab4Texts.openApp.hanji
            btn.setOnClickListener {
                onOpenApp?.invoke()
            }
        }

        setupListeners()
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
            } ?: FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                overlayHeight
            ).apply {
                gravity = Gravity.TOP
                topMargin = smartbarHeight
            }
        }

        syncSwitchStates()
        visibility = VISIBLE
        isShowing = true

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[SHOW] Settings selection overlay shown, height=$keyboardHeight")
        }
    }

    /**
     * Hide the overlay.
     */
    fun hide() {
        if (!isShowing) return

        visibility = GONE
        isShowing = false
        onHide?.invoke()

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[HIDE] Settings selection overlay hidden")
        }
    }

    fun isVisible(): Boolean = isShowing

    /**
     * Refresh switch states from preferences.
     */
    private fun syncSwitchStates() {
        isSyncing = true
        switchOutputBoth?.isChecked = prefs.outputBothScripts
        switchAutoCap?.isChecked = prefs.autoCapitalizationEnabled
        switchAutoSpace?.isChecked = prefs.isAutoSpaceEnabled
        switchToolbarCollapse?.isChecked = prefs.isToolbarAutoCollapse
        switchDoubleOO?.isChecked = prefs.enableDoubleTapOO
        switchDoubleNN?.isChecked = prefs.enableDoubleTapNN
        switchTpsOrER?.isChecked = prefs.tpsOrMapsToER
        isSyncing = false
    }

    private fun setupListeners() {
        switchOutputBoth?.setOnCheckedChangeListener { _, isChecked ->
            if (!isSyncing) prefs.outputBothScripts = isChecked
        }
        switchAutoCap?.setOnCheckedChangeListener { _, isChecked ->
            if (!isSyncing) prefs.autoCapitalizationEnabled = isChecked
        }
        switchAutoSpace?.setOnCheckedChangeListener { _, isChecked ->
            if (!isSyncing) prefs.isAutoSpaceEnabled = isChecked
        }
        switchToolbarCollapse?.setOnCheckedChangeListener { _, isChecked ->
            if (!isSyncing) prefs.isToolbarAutoCollapse = isChecked
        }
        switchDoubleOO?.setOnCheckedChangeListener { _, isChecked ->
            if (!isSyncing) prefs.enableDoubleTapOO = isChecked
        }
        switchDoubleNN?.setOnCheckedChangeListener { _, isChecked ->
            if (!isSyncing) prefs.enableDoubleTapNN = isChecked
        }
        switchTpsOrER?.setOnCheckedChangeListener { _, isChecked ->
            if (!isSyncing) prefs.tpsOrMapsToER = isChecked
        }
    }

}
