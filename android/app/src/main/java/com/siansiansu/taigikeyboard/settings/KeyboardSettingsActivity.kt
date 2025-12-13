package com.siansiansu.taigikeyboard.settings

import android.os.Bundle
import android.view.View
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import androidx.lifecycle.lifecycleScope
import com.google.android.material.dialog.MaterialAlertDialogBuilder
import com.google.android.material.materialswitch.MaterialSwitch
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.databinding.ActivityKeyboardSettingsBinding
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.util.FontUtils
import com.siansiansu.taigikeyboard.util.setupEdgeToEdge
import android.util.Log
import kotlinx.coroutines.launch

class KeyboardSettingsActivity : AppCompatActivity() {
    private lateinit var binding: ActivityKeyboardSettingsBinding
    private lateinit var prefs: PrefHelper
    private lateinit var languageManager: LanguageManager

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        prefs = PrefHelper(this)
        Log.d("KeyboardSettings", "onCreate: inputMode=${prefs.inputMode}, fontType=${prefs.fontType}")
        languageManager = LanguageManager.getInstance(this)

        binding = ActivityKeyboardSettingsBinding.inflate(layoutInflater)
        setContentView(binding.root)

        // 設定 Edge-to-Edge 顯示模式
        setupEdgeToEdge()

        setupToolbar()
        setupViews()
        observeLanguageChanges()
        applyCustomFont()
    }

    private fun setupToolbar() {
        setSupportActionBar(binding.toolbar)
        supportActionBar?.apply {
            setDisplayHomeAsUpEnabled(true)
            setDisplayShowHomeEnabled(true)
            title = languageManager.getText(AppTexts.keyboardSettings)
        }
    }

    private fun observeLanguageChanges() {
        languageManager.currentDisplayLanguage.observe(this) {
            updateAllTexts()
        }
    }

    private fun updateAllTexts() {
        supportActionBar?.title = languageManager.getText(AppTexts.keyboardSettings)

        // Update all UI texts based on current language
        binding.buttonPojMode.text = languageManager.getText(AppTexts.pojMode)
        binding.buttonTlMode.text = languageManager.getText(AppTexts.tlMode)

        // Update section titles
        binding.sectionTitleInputMode.text = languageManager.getText(AppTexts.inputMode)
        binding.sectionTitleFontType.text = languageManager.getText(AppTexts.customFont)

        // Update font type buttons
        binding.buttonFontSystem.text = languageManager.getText(AppTexts.fontSystemDefault)
        binding.buttonFontOpenHuninn.text = languageManager.getText(AppTexts.fontOpenHuninn)
        binding.buttonFontIansui.text = languageManager.getText(AppTexts.fontIansui)

        updateToggleItemText(binding.toggleOutputBothScripts.root, AppTexts.outputBothScripts)
        updateToggleItemText(binding.toggleAutoCapitalization.root, AppTexts.autoCapitalization)
        updateToggleItemText(binding.toggleAutoSpace.root, AppTexts.autoSpace)
        updateToggleItemText(binding.togglePhahTaigiLayout.root, AppTexts.phahTaigiLayout)
        updateToggleItemText(binding.toggleDoubleTapOo.root, AppTexts.doubleTapOO)
        updateToggleItemText(binding.toggleDoubleTapNn.root, AppTexts.doubleTapNN)

        updateActionItemText(binding.actionResetSettings.root, AppTexts.resetSettings)
    }

    private fun updateToggleItemText(itemRoot: View, localizedText: LocalizedText) {
        val title = itemRoot.findViewById<TextView>(R.id.item_title)
        title.text = languageManager.getText(localizedText)
    }

    private fun updateActionItemText(itemRoot: View, localizedText: LocalizedText) {
        val title = itemRoot.findViewById<TextView>(R.id.action_title)
        title.text = languageManager.getText(localizedText)
    }

    private fun setupViews() {
        setupInputModeSection()
        setupFontTypeSection()
        setupBasicSettingsSection()
        setupActionButtonsSection()
    }

    private fun setupInputModeSection() {
        // 載入目前的輸入模式偏好設定
        val currentInputMode = prefs.inputMode
        val checkedButtonId = when (currentInputMode) {
            InputMode.POJ.value -> R.id.button_poj_mode
            InputMode.TL.value -> R.id.button_tl_mode
            else -> R.id.button_tl_mode
        }

        Log.d("KeyboardSettings", "setupInputModeSection: currentInputMode=$currentInputMode, checkedButtonId=$checkedButtonId")
        Log.d("KeyboardSettings", "setupInputModeSection: 設定前 checkedButtonIds=${binding.inputModeToggleGroup.checkedButtonIds}")

        // 先清除舊 listener，避免設定值時觸發 callback
        binding.inputModeToggleGroup.clearOnButtonCheckedListeners()
        binding.inputModeToggleGroup.check(checkedButtonId)

        Log.d("KeyboardSettings", "setupInputModeSection: 設定後 checkedButtonIds=${binding.inputModeToggleGroup.checkedButtonIds}")

        // 設定值後再註冊 listener
        binding.inputModeToggleGroup.addOnButtonCheckedListener { _, checkedId, isChecked ->
            if (isChecked) {
                when (checkedId) {
                    R.id.button_poj_mode -> {
                        prefs.inputMode = InputMode.POJ.value
                    }
                    R.id.button_tl_mode -> {
                        prefs.inputMode = InputMode.TL.value
                    }
                }
            }
        }
    }

    private fun setupFontTypeSection() {
        // 載入目前的字型設定
        val currentFontType = prefs.fontType
        val checkedButtonId = when (currentFontType) {
            "system" -> R.id.button_font_system
            "openHuninn" -> R.id.button_font_open_huninn
            "iansui" -> R.id.button_font_iansui
            else -> R.id.button_font_open_huninn
        }

        // 先清除舊 listener，避免設定值時觸發 callback
        binding.fontTypeToggleGroup.clearOnButtonCheckedListeners()
        binding.fontTypeToggleGroup.check(checkedButtonId)

        // 設定值後再註冊 listener
        binding.fontTypeToggleGroup.addOnButtonCheckedListener { _, checkedId, isChecked ->
            if (isChecked) {
                val newFontType = when (checkedId) {
                    R.id.button_font_system -> "system"
                    R.id.button_font_open_huninn -> "openHuninn"
                    R.id.button_font_iansui -> "iansui"
                    else -> "openHuninn"
                }
                prefs.fontType = newFontType
                // 即時套用字型變更（直接使用新值，避免非同步讀取問題）
                applyCustomFont(newFontType)
            }
        }
    }

    private fun setupBasicSettingsSection() {
        // Output Both Scripts (漢羅攏出)
        // showHanjiMode 固定為 true，因此此功能永遠可用
        setupToggleItem(
            binding.toggleOutputBothScripts.root,
            AppTexts.outputBothScripts,
            prefs.outputBothScripts
        ) { isChecked ->
            prefs.outputBothScripts = isChecked
        }

        // 初始化 outputBothScripts 的啟用狀態（showHanjiMode 固定為 true）
        updateOutputBothScriptsState(true)

        // Auto Capitalization
        setupToggleItem(
            binding.toggleAutoCapitalization.root,
            AppTexts.autoCapitalization,
            prefs.autoCapitalizationEnabled
        ) { isChecked ->
            prefs.autoCapitalizationEnabled = isChecked
        }

        // Auto Space
        setupToggleItem(
            binding.toggleAutoSpace.root,
            AppTexts.autoSpace,
            prefs.autoSpaceEnabled
        ) { isChecked ->
            prefs.autoSpaceEnabled = isChecked
        }

        // Phah Taigi Layout
        setupToggleItem(
            binding.togglePhahTaigiLayout.root,
            AppTexts.phahTaigiLayout,
            prefs.phahTaigiLayoutEnabled
        ) { isChecked ->
            prefs.phahTaigiLayoutEnabled = isChecked
        }

        // Double Tap OO
        setupToggleItem(
            binding.toggleDoubleTapOo.root,
            AppTexts.doubleTapOO,
            prefs.enableDoubleTapOO
        ) { isChecked ->
            prefs.enableDoubleTapOO = isChecked
        }

        // Double Tap NN
        setupToggleItem(
            binding.toggleDoubleTapNn.root,
            AppTexts.doubleTapNN,
            prefs.enableDoubleTapNN
        ) { isChecked ->
            prefs.enableDoubleTapNN = isChecked
        }
    }

    private fun setupActionButtonsSection() {
        // Reset Settings
        setupActionItem(
            binding.actionResetSettings.root,
            AppTexts.resetSettings
        ) {
            showResetSettingsDialog()
        }
    }

    private fun setupToggleItem(
        itemRoot: View,
        localizedText: LocalizedText,
        defaultChecked: Boolean,
        onChangeListener: ((Boolean) -> Unit)? = null
    ) {
        val title = itemRoot.findViewById<TextView>(R.id.item_title)
        val switch = itemRoot.findViewById<MaterialSwitch>(R.id.item_switch)

        title.text = languageManager.getText(localizedText)

        Log.d("KeyboardSettings", "setupToggleItem: ${localizedText.hanji} 設定前 switch=${switch.isChecked}, 目標=$defaultChecked")

        // 先移除舊 listener，避免設定值時觸發 callback
        switch.setOnCheckedChangeListener(null)
        switch.isChecked = defaultChecked

        Log.d("KeyboardSettings", "setupToggleItem: ${localizedText.hanji} 設定後 switch=${switch.isChecked}")

        // 設定值後再註冊 listener
        switch.setOnCheckedChangeListener { _, isChecked ->
            Log.d("KeyboardSettings", "setupToggleItem: ${localizedText.hanji} listener 觸發 isChecked=$isChecked")
            onChangeListener?.invoke(isChecked)
        }
    }

    private fun setupActionItem(
        itemRoot: View,
        localizedText: LocalizedText,
        onClickListener: (() -> Unit)? = null
    ) {
        val title = itemRoot.findViewById<TextView>(R.id.action_title)

        title.text = languageManager.getText(localizedText)

        itemRoot.setOnClickListener {
            onClickListener?.invoke()
        }
    }

    // 顯示恢復設定確認對話框
    private fun showResetSettingsDialog() {
        MaterialAlertDialogBuilder(this)
            .setTitle(languageManager.getText(AppTexts.resetSettings))
            .setMessage(languageManager.getText(AppTexts.resetSettingsMessage))
            .setNegativeButton(languageManager.getText(AppTexts.cancel)) { dialog, _ ->
                dialog.dismiss()
            }
            .setPositiveButton(languageManager.getText(AppTexts.reset)) { dialog, _ ->
                resetAllSettings()
                dialog.dismiss()
            }
            .setBackgroundInsetStart(24)
            .setBackgroundInsetEnd(24)
            .create()
            .apply {
                window?.setBackgroundDrawableResource(android.R.color.transparent)
                show()
                window?.decorView?.setBackgroundColor(
                    ContextCompat.getColor(context, R.color.modern_surface_card)
                )
            }
    }

    // 恢復所有設定為預設值（不清除資料）
    private fun resetAllSettings() {
        lifecycleScope.launch {
            Log.d("KeyboardSettings", "resetAllSettings: 開始重置")
            Log.d("KeyboardSettings", "重置前 inputMode=${prefs.inputMode}, fontType=${prefs.fontType}")
            try {
                prefs.resetToDefaults()
                Log.d("KeyboardSettings", "重置後 inputMode=${prefs.inputMode}, fontType=${prefs.fontType}")
            } catch (e: Exception) {
                Log.e("KeyboardSettings", "重置失敗", e)
                if (com.siansiansu.taigikeyboard.BuildConfig.DEBUG) {
                    e.printStackTrace()
                }
            }

            // 重建 Activity 以更新 UI
            recreate()
        }
    }

    // 更新「漢羅攏出」設定項的啟用狀態
    private fun updateOutputBothScriptsState(showHanjiEnabled: Boolean) {
        val switch = binding.toggleOutputBothScripts.root.findViewById<MaterialSwitch>(R.id.item_switch)
        val title = binding.toggleOutputBothScripts.root.findViewById<TextView>(R.id.item_title)

        binding.toggleOutputBothScripts.root.isEnabled = showHanjiEnabled
        switch.isEnabled = showHanjiEnabled
        title.isEnabled = showHanjiEnabled

        // 停用時的視覺回饋
        val alpha = if (showHanjiEnabled) 1.0f else 0.4f
        binding.toggleOutputBothScripts.root.alpha = alpha
    }

    override fun onSupportNavigateUp(): Boolean {
        onBackPressedDispatcher.onBackPressed()
        return true
    }

    /**
     * 套用自訂字體到所有 UI 元件
     * @param fontType 字型類型，若為 null 則從 prefs 讀取
     */
    private fun applyCustomFont(fontType: String? = null) {
        val typeface = FontUtils.getTypefaceByType(fontType ?: prefs.fontType, this)

        // Section titles
        binding.sectionTitleInputMode.typeface = typeface
        binding.sectionTitleFontType.typeface = typeface

        // Input mode buttons
        binding.buttonPojMode.typeface = typeface
        binding.buttonTlMode.typeface = typeface

        // Font type buttons
        binding.buttonFontSystem.typeface = typeface
        binding.buttonFontOpenHuninn.typeface = typeface
        binding.buttonFontIansui.typeface = typeface

        // Toggle items
        applyFontToToggleItem(binding.toggleOutputBothScripts.root, typeface)
        applyFontToToggleItem(binding.toggleAutoCapitalization.root, typeface)
        applyFontToToggleItem(binding.toggleAutoSpace.root, typeface)
        applyFontToToggleItem(binding.togglePhahTaigiLayout.root, typeface)
        applyFontToToggleItem(binding.toggleDoubleTapOo.root, typeface)
        applyFontToToggleItem(binding.toggleDoubleTapNn.root, typeface)

        // Action items
        applyFontToActionItem(binding.actionResetSettings.root, typeface)
    }

    private fun applyFontToToggleItem(itemRoot: View, typeface: android.graphics.Typeface) {
        itemRoot.findViewById<TextView>(R.id.item_title)?.typeface = typeface
    }

    private fun applyFontToActionItem(itemRoot: View, typeface: android.graphics.Typeface) {
        itemRoot.findViewById<TextView>(R.id.action_title)?.typeface = typeface
    }
}
