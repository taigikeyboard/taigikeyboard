package com.siansiansu.taigikeyboard.settings

import android.os.Bundle
import android.view.View
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import com.google.android.material.dialog.MaterialAlertDialogBuilder
import com.google.android.material.materialswitch.MaterialSwitch
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.databinding.ActivityDictionarySettingsBinding
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.dictionary.NextWordService
import com.siansiansu.taigikeyboard.ime.text.composing.UserFrequencyService
import com.siansiansu.taigikeyboard.util.FontUtils
import com.siansiansu.taigikeyboard.util.setupEdgeToEdge
import kotlinx.coroutines.launch

/**
 * 詞庫管理頁面
 */
class DictionarySettingsActivity : AppCompatActivity() {
    private lateinit var binding: ActivityDictionarySettingsBinding
    private lateinit var languageManager: LanguageManager
    private lateinit var prefs: PrefHelper

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        prefs = PrefHelper(this)
        languageManager = LanguageManager.getInstance(this)

        binding = ActivityDictionarySettingsBinding.inflate(layoutInflater)
        setContentView(binding.root)

        // 設定 Edge-to-Edge 顯示模式
        setupEdgeToEdge()

        setupToolbar()
        setupViews()
        setupCustomDictionaryPlaceholder()
        setupClearCache()
        observeLanguageChanges()
        applyCustomFont()
    }

    private fun setupToolbar() {
        setSupportActionBar(binding.toolbar)
        supportActionBar?.apply {
            setDisplayHomeAsUpEnabled(true)
            setDisplayShowHomeEnabled(true)
            title = languageManager.getText(AppTexts.dictionarySettings)
        }
    }

    private fun observeLanguageChanges() {
        languageManager.currentDisplayLanguage.observe(this) {
            updateAllTexts()
        }
    }

    private fun updateAllTexts() {
        supportActionBar?.title = languageManager.getText(AppTexts.dictionarySettings)

        // 更新詞庫 Toggle 項目文字
        updateToggleItemText(binding.toggleMoeDict.root, AppTexts.moeDict)
        updateToggleItemText(binding.toggleNewwordDict.root, AppTexts.newwordDict)
        updateToggleItemText(binding.toggleItaigiDict.root, AppTexts.iTaigiDict)
        updateToggleItemText(binding.toggleTaiwanPlantDict.root, AppTexts.taiwanPlantDict)
        updateToggleItemText(binding.toggleTaiHuaDict.root, AppTexts.taiHuaDict)
        updateToggleItemText(binding.toggleTaiwanJapanDict.root, AppTexts.taiwanJapanDict)
        updateToggleItemText(binding.toggleKunggeDict.root, AppTexts.kunggeDict)

        // 更新異用字 Toggle 文字
        updateToggleItemText(binding.toggleVariant.root, AppTexts.variantDictionary)

        // 更新自訂詞庫區塊文字
        binding.sectionTitleCustomDictionary.text = languageManager.getText(AppTexts.customDictionary)
        binding.textEmptyState.text = languageManager.getText(AppTexts.comingSoon)

        // 更新清除資料文字
        updateActionItemText(binding.actionClearCache.root, AppTexts.clearCache)
    }

    private fun updateActionItemText(itemRoot: View, localizedText: LocalizedText) {
        val title = itemRoot.findViewById<TextView>(R.id.action_title)
        title.text = languageManager.getText(localizedText)
    }

    private fun updateToggleItemText(itemRoot: View, localizedText: LocalizedText) {
        val title = itemRoot.findViewById<TextView>(R.id.item_title)
        title.text = languageManager.getText(localizedText)
    }

    private fun setupViews() {
        setupDictionaryToggles()
        setupVariantToggle()
    }

    /**
     * 設定自訂詞庫預留區塊（敬請期待）
     */
    private fun setupCustomDictionaryPlaceholder() {
        // 隱藏新增按鈕和列表
        binding.btnAddWord.visibility = View.GONE
        binding.recyclerCustomWords.visibility = View.GONE

        // 顯示「敬請期待」提示
        binding.textEmptyState.text = languageManager.getText(AppTexts.comingSoon)
        binding.textEmptyState.visibility = View.VISIBLE
    }

    /**
     * 設定清除資料功能
     */
    private fun setupClearCache() {
        val title = binding.actionClearCache.root.findViewById<TextView>(R.id.action_title)
        title.text = languageManager.getText(AppTexts.clearCache)

        binding.actionClearCache.root.setOnClickListener {
            showClearCacheDialog()
        }
    }

    /**
     * 顯示清除快取確認對話框
     */
    private fun showClearCacheDialog() {
        MaterialAlertDialogBuilder(this)
            .setTitle(languageManager.getText(AppTexts.clearCache))
            .setMessage(languageManager.getText(AppTexts.clearCacheMessage))
            .setNegativeButton(languageManager.getText(AppTexts.cancel), null)
            .setPositiveButton(languageManager.getText(AppTexts.clear)) { _, _ ->
                clearUserFrequencyDatabase()
            }
            .show()
    }

    /**
     * 清除使用者學習資料
     * - UserFrequencyService: 使用者頻率（常用詞排序）
     * - NextWordService: 使用者關聯（下一詞預測）
     */
    private fun clearUserFrequencyDatabase() {
        lifecycleScope.launch {
            try {
                // 清除常用詞頻率
                UserFrequencyService.deleteDatabase()

                // 清除 NextWord 使用者關聯
                NextWordService.clearAllAssociations(this@DictionarySettingsActivity)
            } catch (e: Exception) {
                // 錯誤處理：清除失敗不影響 UI 運作
                e.printStackTrace()
            }
        }
    }

    /**
     * 設定詞庫 Toggle 開關
     */
    private fun setupDictionaryToggles() {
        // 教育部臺灣台語常用詞辭典（kautian）
        setupToggleItem(
            binding.toggleMoeDict.root,
            AppTexts.moeDict,
            prefs.moeDictEnabled
        ) { isChecked ->
            prefs.moeDictEnabled = isChecked
        }

        // 台語新詞辭庫（taigitv）
        setupToggleItem(
            binding.toggleNewwordDict.root,
            AppTexts.newwordDict,
            prefs.newwordDictEnabled
        ) { isChecked ->
            prefs.newwordDictEnabled = isChecked
        }

        // iTaigi 華台對照典（itaigi）
        setupToggleItem(
            binding.toggleItaigiDict.root,
            AppTexts.iTaigiDict,
            prefs.itaigiDictEnabled
        ) { isChecked ->
            prefs.itaigiDictEnabled = isChecked
        }

        // 台灣植物名彙（sitbut）
        setupToggleItem(
            binding.toggleTaiwanPlantDict.root,
            AppTexts.taiwanPlantDict,
            prefs.sitbutDictEnabled
        ) { isChecked ->
            prefs.sitbutDictEnabled = isChecked
        }

        // 台華線頂對照典（taihoa）
        setupToggleItem(
            binding.toggleTaiHuaDict.root,
            AppTexts.taiHuaDict,
            prefs.taihoaDictEnabled
        ) { isChecked ->
            prefs.taihoaDictEnabled = isChecked
        }

        // 台日大辭典（taijit）
        setupToggleItem(
            binding.toggleTaiwanJapanDict.root,
            AppTexts.taiwanJapanDict,
            prefs.taijitDictEnabled
        ) { isChecked ->
            prefs.taijitDictEnabled = isChecked
        }

        // 台語工藝詞庫（kungge）
        setupToggleItem(
            binding.toggleKunggeDict.root,
            AppTexts.kunggeDict,
            prefs.kunggeDictEnabled
        ) { isChecked ->
            prefs.kunggeDictEnabled = isChecked
        }
    }

    /**
     * 設定異用字 Toggle 開關
     */
    private fun setupVariantToggle() {
        setupToggleItem(
            binding.toggleVariant.root,
            AppTexts.variantDictionary,
            prefs.variantEnabled
        ) { isChecked ->
            prefs.variantEnabled = isChecked
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

    override fun onSupportNavigateUp(): Boolean {
        onBackPressedDispatcher.onBackPressed()
        return true
    }

    /**
     * 套用自訂字體到所有 UI 元件
     */
    private fun applyCustomFont() {
        val typeface = FontUtils.getTypefaceByType(prefs.fontType, this)

        // Section title
        binding.sectionTitleCustomDictionary.typeface = typeface
        binding.textEmptyState.typeface = typeface

        // Toggle items
        applyFontToToggleItem(binding.toggleMoeDict.root, typeface)
        applyFontToToggleItem(binding.toggleNewwordDict.root, typeface)
        applyFontToToggleItem(binding.toggleItaigiDict.root, typeface)
        applyFontToToggleItem(binding.toggleTaiwanPlantDict.root, typeface)
        applyFontToToggleItem(binding.toggleTaiHuaDict.root, typeface)
        applyFontToToggleItem(binding.toggleTaiwanJapanDict.root, typeface)
        applyFontToToggleItem(binding.toggleKunggeDict.root, typeface)

        // 異用字 Toggle
        applyFontToToggleItem(binding.toggleVariant.root, typeface)

        // 清除資料按鈕
        applyFontToActionItem(binding.actionClearCache.root, typeface)
    }

    private fun applyFontToToggleItem(itemRoot: View, typeface: android.graphics.Typeface) {
        itemRoot.findViewById<TextView>(R.id.item_title)?.typeface = typeface
    }

    private fun applyFontToActionItem(itemRoot: View, typeface: android.graphics.Typeface) {
        itemRoot.findViewById<TextView>(R.id.action_title)?.typeface = typeface
    }
}
