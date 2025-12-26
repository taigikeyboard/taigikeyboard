package com.siansiansu.taigikeyboard.settings

import android.content.Intent
import android.os.Bundle
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.lifecycle.lifecycleScope
import com.google.android.material.button.MaterialButton
import com.google.android.material.button.MaterialButtonToggleGroup
import com.google.android.material.materialswitch.MaterialSwitch
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.text.composing.UserFrequencyService
import com.siansiansu.taigikeyboard.ime.dictionary.NextWordService
import com.siansiansu.taigikeyboard.localization.LanguageManager
import com.siansiansu.taigikeyboard.localization.Tab4Texts
import kotlinx.coroutines.launch

/**
 * Tab4: 設定
 * 內容：輸入模式、字體設定、開關設定
 */
class Tab4Fragment : SettingsMainActivity.BaseSettingsFragment() {

    private lateinit var languageManager: LanguageManager

    override fun onCreateView(
        inflater: LayoutInflater,
        container: ViewGroup?,
        savedInstanceState: Bundle?
    ): View? {
        return inflater.inflate(R.layout.fragment_tab4, container, false)
    }

    override fun onViewCreated(view: View, savedInstanceState: Bundle?) {
        super.onViewCreated(view, savedInstanceState)

        languageManager = LanguageManager.getInstance(requireContext())
        setupLocalizedTexts(view)
        setupInputMode(view)
        setupFontType(view)
        setupSettingsSwitches(view)
        setupResetSettings(view)
        setupDebugZone(view)
    }

    private fun setupInputMode(view: View) {
        val toggleGroup = view.findViewById<MaterialButtonToggleGroup>(R.id.input_mode_toggle_group)

        // 設定初始狀態
        when (prefs.inputMode) {
            "poj" -> toggleGroup.check(R.id.button_poj_mode)
            else -> toggleGroup.check(R.id.button_tl_mode)
        }

        toggleGroup.addOnButtonCheckedListener { _, checkedId, isChecked ->
            if (isChecked) {
                when (checkedId) {
                    R.id.button_poj_mode -> prefs.inputMode = "poj"
                    R.id.button_tl_mode -> prefs.inputMode = "tl"
                }
            }
        }
    }

    private fun setupFontType(view: View) {
        val toggleGroup = view.findViewById<MaterialButtonToggleGroup>(R.id.font_type_toggle_group)

        // 設定初始狀態
        when (prefs.fontType) {
            "system" -> toggleGroup.check(R.id.button_font_system)
            "openHuninn" -> toggleGroup.check(R.id.button_font_huninn)
            "iansui" -> toggleGroup.check(R.id.button_font_iansui)
            else -> toggleGroup.check(R.id.button_font_huninn) // 預設為粉圓
        }

        toggleGroup.addOnButtonCheckedListener { _, checkedId, isChecked ->
            if (isChecked) {
                val fontType = when (checkedId) {
                    R.id.button_font_system -> "system"
                    R.id.button_font_huninn -> "openHuninn"
                    R.id.button_font_iansui -> "iansui"
                    else -> "openHuninn"
                }
                prefs.fontType = fontType
                // 切換字體時重建 Activity 以套用新 Theme
                activity?.recreate()
            }
        }
    }

    private fun setupSettingsSwitches(view: View) {
        // 括號標註
        view.findViewById<MaterialSwitch>(R.id.switch_output_both).apply {
            isChecked = prefs.outputBothScripts
            setOnCheckedChangeListener { _, isChecked ->
                prefs.outputBothScripts = isChecked
            }
        }

        // 自動大寫
        view.findViewById<MaterialSwitch>(R.id.switch_auto_cap).apply {
            isChecked = prefs.autoCapitalizationEnabled
            setOnCheckedChangeListener { _, isChecked ->
                prefs.autoCapitalizationEnabled = isChecked
            }
        }

        // 自動空白
        view.findViewById<MaterialSwitch>(R.id.switch_auto_space).apply {
            isChecked = prefs.autoSpaceEnabled
            setOnCheckedChangeListener { _, isChecked ->
                prefs.autoSpaceEnabled = isChecked
            }
        }

        // 連紲拍 oo
        view.findViewById<MaterialSwitch>(R.id.switch_double_oo).apply {
            isChecked = prefs.enableDoubleTapOO
            setOnCheckedChangeListener { _, isChecked ->
                prefs.enableDoubleTapOO = isChecked
            }
        }

        // 連紲拍 nn
        view.findViewById<MaterialSwitch>(R.id.switch_double_nn).apply {
            isChecked = prefs.enableDoubleTapNN
            setOnCheckedChangeListener { _, isChecked ->
                prefs.enableDoubleTapNN = isChecked
            }
        }
    }

    private fun setupResetSettings(view: View) {
        view.findViewById<View>(R.id.row_reset_settings).setOnClickListener {
            showResetSettingsDialog()
        }
    }

    private fun setupDebugZone(view: View) {
        val cardDebug = view.findViewById<View>(R.id.card_debug)

        // 只在 Debug 模式顯示
        if (BuildConfig.DEBUG) {
            cardDebug.visibility = View.VISIBLE
            view.findViewById<View>(R.id.row_debug_zone).setOnClickListener {
                startActivity(Intent(requireContext(), DebugActivity::class.java))
            }
        } else {
            cardDebug.visibility = View.GONE
        }
    }

    private fun showResetSettingsDialog() {
        AlertDialog.Builder(requireContext())
            .setTitle(languageManager.text(Tab4Texts.resetSettings))
            .setMessage(languageManager.text(Tab4Texts.resetSettingsMessage))
            .setPositiveButton(languageManager.text(Tab4Texts.confirmKey)) { _, _ ->
                resetAllSettings()
            }
            .setNegativeButton(languageManager.text(Tab4Texts.cancel), null)
            .show()
    }

    private fun resetAllSettings() {
        lifecycleScope.launch {
            try {
                // 重置所有設定為預設值
                prefs.resetToDefaults()

                // 清除使用者詞頻資料
                UserFrequencyService.deleteDatabase()
                NextWordService.clearAllAssociations(requireContext())

                // 重新載入 UI
                view?.let { setupInputMode(it) }
                view?.let { setupFontType(it) }
                view?.let { setupSettingsSwitches(it) }

                Toast.makeText(
                    requireContext(),
                    languageManager.text(Tab4Texts.resetSuccess),
                    Toast.LENGTH_SHORT
                ).show()
            } catch (e: Exception) {
                // Handle exception silently
            }
        }
    }

    private fun setupLocalizedTexts(view: View) {
        // 頁面標題
        view.findViewById<TextView>(R.id.text_tab_title)?.text =
            languageManager.text(Tab4Texts.tabTitle)

        // 輸入模式
        view.findViewById<TextView>(R.id.text_input_mode)?.text =
            languageManager.text(Tab4Texts.inputMode)
        view.findViewById<MaterialButton>(R.id.button_poj_mode)?.text =
            languageManager.text(Tab4Texts.pojMode)
        view.findViewById<MaterialButton>(R.id.button_tl_mode)?.text =
            languageManager.text(Tab4Texts.tlMode)

        // 字體設定
        view.findViewById<TextView>(R.id.text_custom_font)?.text =
            languageManager.text(Tab4Texts.customFont)
        view.findViewById<MaterialButton>(R.id.button_font_system)?.text =
            languageManager.text(Tab4Texts.fontSystemDefault)
        view.findViewById<MaterialButton>(R.id.button_font_huninn)?.text =
            languageManager.text(Tab4Texts.fontOpenHuninn)
        view.findViewById<MaterialButton>(R.id.button_font_iansui)?.text =
            languageManager.text(Tab4Texts.fontIansui)

        // 開關設定
        view.findViewById<TextView>(R.id.text_output_both)?.text =
            languageManager.text(Tab4Texts.outputBothScripts)
        view.findViewById<TextView>(R.id.text_auto_cap)?.text =
            languageManager.text(Tab4Texts.autoCapitalization)
        view.findViewById<TextView>(R.id.text_auto_space)?.text =
            languageManager.text(Tab4Texts.autoSpace)
        view.findViewById<TextView>(R.id.text_double_oo)?.text =
            languageManager.text(Tab4Texts.doubleTapOO)
        view.findViewById<TextView>(R.id.text_double_nn)?.text =
            languageManager.text(Tab4Texts.doubleTapNN)

        // 重設設定
        view.findViewById<TextView>(R.id.text_reset_settings)?.text =
            languageManager.text(Tab4Texts.resetSettings)

        // Debug 模式
        view.findViewById<TextView>(R.id.text_debug_zone)?.text =
            languageManager.text(Tab4Texts.debugMode)
    }

    companion object {
        fun newInstance() = Tab4Fragment()
    }
}
