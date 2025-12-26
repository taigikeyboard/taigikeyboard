package com.siansiansu.taigikeyboard.settings

import android.os.Bundle
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
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

        // 更新初始狀態
        updateLayoutSelectionUI(prefs.phahTaigiLayoutEnabled)
    }

    private fun selectLayout(phahTaigi: Boolean) {
        prefs.phahTaigiLayoutEnabled = phahTaigi
        updateLayoutSelectionUI(phahTaigi)
    }

    private fun updateLayoutSelectionUI(phahTaigiEnabled: Boolean) {
        // 更新 PhahTaigi 選項的視覺狀態
        if (phahTaigiEnabled) {
            overlayPhahTaigi?.visibility = View.VISIBLE
            checkmarkPhahTaigi?.visibility = View.VISIBLE
            overlayStandard?.visibility = View.GONE
            checkmarkStandard?.visibility = View.GONE
        } else {
            overlayPhahTaigi?.visibility = View.GONE
            checkmarkPhahTaigi?.visibility = View.GONE
            overlayStandard?.visibility = View.VISIBLE
            checkmarkStandard?.visibility = View.VISIBLE
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
