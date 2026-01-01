package com.siansiansu.taigikeyboard.settings

import android.animation.AnimatorSet
import android.animation.ObjectAnimator
import android.os.Bundle
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.view.animation.OvershootInterpolator
import android.widget.TextView
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.localization.LanguageManager
import com.siansiansu.taigikeyboard.localization.Tab2Texts

/**
 * Tab2: 佈局
 * 內容：鍵盤佈局選擇（PhahTaigi / 標準）
 */
class Tab2Fragment : SettingsMainActivity.BaseSettingsFragment() {

    private var overlayPhahTaigi: View? = null
    private var checkmarkPhahTaigi: View? = null
    private var overlayStandard: View? = null
    private var checkmarkStandard: View? = null
    private lateinit var languageManager: LanguageManager

    override fun onCreateView(
        inflater: LayoutInflater,
        container: ViewGroup?,
        savedInstanceState: Bundle?
    ): View? {
        return inflater.inflate(R.layout.fragment_tab2, container, false)
    }

    override fun onViewCreated(view: View, savedInstanceState: Bundle?) {
        super.onViewCreated(view, savedInstanceState)

        languageManager = LanguageManager.getInstance(requireContext())
        setupLocalizedTexts(view)

        // 取得 UI 元件
        overlayPhahTaigi = view.findViewById(R.id.overlay_phahtaigi)
        checkmarkPhahTaigi = view.findViewById(R.id.checkmark_phahtaigi)
        overlayStandard = view.findViewById(R.id.overlay_standard)
        checkmarkStandard = view.findViewById(R.id.checkmark_standard)

        // 設定點擊事件
        view.findViewById<View>(R.id.row_layout_phahtaigi).setOnClickListener {
            selectLayout(phahTaigi = true)
        }

        view.findViewById<View>(R.id.row_layout_standard).setOnClickListener {
            selectLayout(phahTaigi = false)
        }

        // 更新初始狀態（不帶動畫）
        updateLayoutSelectionUI(prefs.phahTaigiLayoutEnabled, animate = false)
    }

    private fun selectLayout(phahTaigi: Boolean) {
        val previousValue = prefs.phahTaigiLayoutEnabled
        if (previousValue != phahTaigi) {
            prefs.phahTaigiLayoutEnabled = phahTaigi
            updateLayoutSelectionUI(phahTaigi, animate = true)
        }
    }

    private fun updateLayoutSelectionUI(phahTaigiEnabled: Boolean, animate: Boolean) {
        if (phahTaigiEnabled) {
            // 顯示 PhahTaigi 選中狀態
            showSelection(overlayPhahTaigi, checkmarkPhahTaigi, animate)
            hideSelection(overlayStandard, checkmarkStandard, animate)
        } else {
            // 顯示標準佈局選中狀態
            hideSelection(overlayPhahTaigi, checkmarkPhahTaigi, animate)
            showSelection(overlayStandard, checkmarkStandard, animate)
        }
    }

    private fun showSelection(overlay: View?, checkmark: View?, animate: Boolean) {
        overlay?.visibility = View.VISIBLE
        checkmark?.visibility = View.VISIBLE

        if (animate && checkmark != null) {
            // 彈跳動畫效果（類似 iOS matchedGeometryEffect）
            checkmark.scaleX = 0f
            checkmark.scaleY = 0f
            checkmark.alpha = 0f

            val scaleX = ObjectAnimator.ofFloat(checkmark, View.SCALE_X, 0f, 1f)
            val scaleY = ObjectAnimator.ofFloat(checkmark, View.SCALE_Y, 0f, 1f)
            val alpha = ObjectAnimator.ofFloat(checkmark, View.ALPHA, 0f, 1f)

            AnimatorSet().apply {
                playTogether(scaleX, scaleY, alpha)
                duration = 200
                interpolator = OvershootInterpolator(1.5f)
                start()
            }
        }

        if (animate && overlay != null) {
            overlay.alpha = 0f
            overlay.animate()
                .alpha(1f)
                .setDuration(150)
                .start()
        }
    }

    private fun hideSelection(overlay: View?, checkmark: View?, animate: Boolean) {
        if (animate) {
            checkmark?.animate()
                ?.scaleX(0f)
                ?.scaleY(0f)
                ?.alpha(0f)
                ?.setDuration(150)
                ?.withEndAction {
                    checkmark.visibility = View.GONE
                }
                ?.start()

            overlay?.animate()
                ?.alpha(0f)
                ?.setDuration(150)
                ?.withEndAction {
                    overlay.visibility = View.GONE
                }
                ?.start()
        } else {
            overlay?.visibility = View.GONE
            checkmark?.visibility = View.GONE
        }
    }

    private fun setupLocalizedTexts(view: View) {
        view.findViewById<TextView>(R.id.text_tab_title)?.text =
            languageManager.text(Tab2Texts.tabTitle)
        view.findViewById<TextView>(R.id.text_layout_description)?.text =
            languageManager.text(Tab2Texts.layoutDescription)
        view.findViewById<TextView>(R.id.text_layout_phahtaigi)?.text =
            languageManager.text(Tab2Texts.phahTaigiLayout)
        view.findViewById<TextView>(R.id.text_layout_standard)?.text =
            languageManager.text(Tab2Texts.standardLayout)
    }

    companion object {
        fun newInstance() = Tab2Fragment()
    }
}
