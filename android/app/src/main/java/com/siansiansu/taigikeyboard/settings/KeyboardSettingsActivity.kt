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
import com.siansiansu.taigikeyboard.ime.text.composing.UserFrequencyService
import com.siansiansu.taigikeyboard.util.setupEdgeToEdge
import kotlinx.coroutines.launch

class KeyboardSettingsActivity : AppCompatActivity() {
    private lateinit var binding: ActivityKeyboardSettingsBinding
    private lateinit var prefs: PrefHelper
    private lateinit var languageManager: LanguageManager

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        prefs = PrefHelper(this)
        languageManager = LanguageManager.getInstance(this)

        binding = ActivityKeyboardSettingsBinding.inflate(layoutInflater)
        setContentView(binding.root)

        // 設定 Edge-to-Edge 顯示模式
        setupEdgeToEdge()

        setupToolbar()
        setupViews()
        observeLanguageChanges()
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

        updateToggleItemText(binding.toggleOutputBothScripts.root, AppTexts.outputBothScripts)
        updateToggleItemText(binding.toggleAutoCapitalization.root, AppTexts.autoCapitalization)
        updateToggleItemText(binding.toggleAutoSpace.root, AppTexts.autoSpace)
        updateToggleItemText(binding.toggleCustomFont.root, AppTexts.customFont)
        updateToggleItemText(binding.togglePhahTaigiLayout.root, AppTexts.phahTaigiLayout)
        updateToggleItemText(binding.toggleDoubleTapOo.root, AppTexts.doubleTapOO)
        updateToggleItemText(binding.toggleDoubleTapNn.root, AppTexts.doubleTapNN)

        updateActionItemText(binding.actionClearCache.root, AppTexts.clearCache)
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
        setupBasicSettingsSection()
        setupActionButtonsSection()
    }

    private fun setupInputModeSection() {
        // 載入目前的輸入模式偏好設定
        val currentInputMode = prefs.inputMode
        val checkedButtonId = when (currentInputMode) {
            InputMode.POJ.value -> R.id.button_poj_mode
            InputMode.TL.value -> R.id.button_tl_mode
            else -> R.id.button_poj_mode
        }
        binding.inputModeToggleGroup.check(checkedButtonId)

        // 設定監聽器
        binding.inputModeToggleGroup.addOnButtonCheckedListener { _, checkedId, isChecked ->
            if (isChecked) {
                when (checkedId) {
                    R.id.button_poj_mode -> {
                        prefs.inputMode = InputMode.POJ.value
                        // 語言更新會自動透過 Flow 處理
                    }
                    R.id.button_tl_mode -> {
                        prefs.inputMode = InputMode.TL.value
                        // 語言更新會自動透過 Flow 處理
                    }
                }
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

        // Custom Font
        setupToggleItem(
            binding.toggleCustomFont.root,
            AppTexts.customFont,
            prefs.customFontEnabled
        ) { isChecked ->
            prefs.customFontEnabled = isChecked
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
        // Clear Cache
        setupActionItem(
            binding.actionClearCache.root,
            AppTexts.clearCache
        ) {
            showClearCacheDialog()
        }

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
        switch.isChecked = defaultChecked

        switch.setOnCheckedChangeListener { _, isChecked ->
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

    // 顯示清除快取確認對話框
    private fun showClearCacheDialog() {
        MaterialAlertDialogBuilder(this)
            .setTitle(languageManager.getText(AppTexts.clearCache))
            .setMessage(languageManager.getText(AppTexts.clearCacheMessage))
            .setNegativeButton(languageManager.getText(AppTexts.cancel)) { dialog, _ ->
                dialog.dismiss()
            }
            .setPositiveButton(languageManager.getText(AppTexts.clear)) { dialog, _ ->
                clearUserFrequencyDatabase()
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

    // 清除使用者頻率資料庫
    private fun clearUserFrequencyDatabase() {
        lifecycleScope.launch {
            try {
                UserFrequencyService.deleteDatabase()
            } catch (e: Exception) {
                // 錯誤處理：清除失敗不影響 UI 運作
                e.printStackTrace()
            }
        }
    }

    // 恢復所有設定為預設值
    private fun resetAllSettings() {
        lifecycleScope.launch {
            try {
                // 1. 重置所有偏好設定
                prefs.resetToDefaults()
            } catch (e: Exception) {
                if (com.siansiansu.taigikeyboard.BuildConfig.DEBUG) {
                    e.printStackTrace()
                }
            }

            try {
                // 2. 清除使用者頻率資料庫
                UserFrequencyService.deleteDatabase()
            } catch (e: Exception) {
                if (com.siansiansu.taigikeyboard.BuildConfig.DEBUG) {
                    e.printStackTrace()
                }
            }

            // 3. 重建 Activity 以更新 UI
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
}
