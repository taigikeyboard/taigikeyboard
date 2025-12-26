package com.siansiansu.taigikeyboard.settings

import android.os.Bundle
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.lifecycle.lifecycleScope
import com.google.android.material.materialswitch.MaterialSwitch
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.text.composing.UserFrequencyService
import com.siansiansu.taigikeyboard.ime.dictionary.NextWordService
import com.siansiansu.taigikeyboard.localization.LanguageManager
import com.siansiansu.taigikeyboard.localization.Tab3Texts
import com.siansiansu.taigikeyboard.localization.Tab4Texts
import kotlinx.coroutines.launch

/**
 * Tab3: 詞庫
 * 內容：詞庫開關、異用字、清除資料
 */
class Tab3Fragment : SettingsMainActivity.BaseSettingsFragment() {

    private lateinit var languageManager: LanguageManager

    override fun onCreateView(
        inflater: LayoutInflater,
        container: ViewGroup?,
        savedInstanceState: Bundle?
    ): View? {
        return inflater.inflate(R.layout.fragment_tab3, container, false)
    }

    override fun onViewCreated(view: View, savedInstanceState: Bundle?) {
        super.onViewCreated(view, savedInstanceState)

        languageManager = LanguageManager.getInstance(requireContext())
        setupLocalizedTexts(view)
        setupDictionarySwitches(view)
        setupClearCache(view)
    }

    private fun setupDictionarySwitches(view: View) {
        // 教育部辭典
        view.findViewById<MaterialSwitch>(R.id.switch_moe).apply {
            isChecked = prefs.moeDictEnabled
            setOnCheckedChangeListener { _, isChecked ->
                prefs.moeDictEnabled = isChecked
            }
        }

        // 新詞辭庫
        view.findViewById<MaterialSwitch>(R.id.switch_newword).apply {
            isChecked = prefs.newwordDictEnabled
            setOnCheckedChangeListener { _, isChecked ->
                prefs.newwordDictEnabled = isChecked
            }
        }

        // 工藝詞庫
        view.findViewById<MaterialSwitch>(R.id.switch_kungge).apply {
            isChecked = prefs.kunggeDictEnabled
            setOnCheckedChangeListener { _, isChecked ->
                prefs.kunggeDictEnabled = isChecked
            }
        }

        // iTaigi
        view.findViewById<MaterialSwitch>(R.id.switch_itaigi).apply {
            isChecked = prefs.itaigiDictEnabled
            setOnCheckedChangeListener { _, isChecked ->
                prefs.itaigiDictEnabled = isChecked
            }
        }

        // 台日大辭典
        view.findViewById<MaterialSwitch>(R.id.switch_taiwan_japan).apply {
            isChecked = prefs.taijitDictEnabled
            setOnCheckedChangeListener { _, isChecked ->
                prefs.taijitDictEnabled = isChecked
            }
        }

        // 台華線頂對照典
        view.findViewById<MaterialSwitch>(R.id.switch_tai_hua).apply {
            isChecked = prefs.taihoaDictEnabled
            setOnCheckedChangeListener { _, isChecked ->
                prefs.taihoaDictEnabled = isChecked
            }
        }

        // 台灣植物名彙
        view.findViewById<MaterialSwitch>(R.id.switch_taiwan_plant).apply {
            isChecked = prefs.sitbutDictEnabled
            setOnCheckedChangeListener { _, isChecked ->
                prefs.sitbutDictEnabled = isChecked
            }
        }

        // 異用字
        view.findViewById<MaterialSwitch>(R.id.switch_variant).apply {
            isChecked = prefs.variantEnabled
            setOnCheckedChangeListener { _, isChecked ->
                prefs.variantEnabled = isChecked
            }
        }
    }

    private fun setupClearCache(view: View) {
        view.findViewById<View>(R.id.row_clear_cache).setOnClickListener {
            showClearCacheDialog()
        }
    }

    private fun showClearCacheDialog() {
        AlertDialog.Builder(requireContext())
            .setTitle(languageManager.text(Tab3Texts.clearCache))
            .setMessage(languageManager.text(Tab3Texts.clearCacheMessage))
            .setPositiveButton(languageManager.text(Tab4Texts.confirmKey)) { _, _ ->
                clearUserFrequencyDatabase()
            }
            .setNegativeButton(languageManager.text(Tab3Texts.cancel), null)
            .show()
    }

    private fun clearUserFrequencyDatabase() {
        lifecycleScope.launch {
            try {
                UserFrequencyService.deleteDatabase()
                NextWordService.clearAllAssociations(requireContext())
                Toast.makeText(
                    requireContext(),
                    languageManager.text(Tab3Texts.clearCacheSuccess),
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
            languageManager.text(Tab3Texts.tabTitle)

        // 詞庫名稱
        view.findViewById<TextView>(R.id.text_dict_moe)?.text =
            languageManager.text(Tab3Texts.moeDict)
        view.findViewById<TextView>(R.id.text_dict_newword)?.text =
            languageManager.text(Tab3Texts.newwordDict)
        view.findViewById<TextView>(R.id.text_dict_kungge)?.text =
            languageManager.text(Tab3Texts.kunggeDict)
        view.findViewById<TextView>(R.id.text_dict_itaigi)?.text =
            languageManager.text(Tab3Texts.iTaigiDict)
        view.findViewById<TextView>(R.id.text_dict_taiwan_japan)?.text =
            languageManager.text(Tab3Texts.taiwanJapanDict)
        view.findViewById<TextView>(R.id.text_dict_tai_hua)?.text =
            languageManager.text(Tab3Texts.taiHuaDict)
        view.findViewById<TextView>(R.id.text_dict_taiwan_plant)?.text =
            languageManager.text(Tab3Texts.taiwanPlantDict)

        // 異用字
        view.findViewById<TextView>(R.id.text_dict_variant)?.text =
            languageManager.text(Tab3Texts.variantDictionary)

        // 自訂詞庫
        view.findViewById<TextView>(R.id.text_dict_custom)?.text =
            languageManager.text(Tab3Texts.customDictionary)
        view.findViewById<TextView>(R.id.text_dict_coming_soon)?.text =
            languageManager.text(Tab3Texts.comingSoon)

        // 清除快取
        view.findViewById<TextView>(R.id.text_clear_cache)?.text =
            languageManager.text(Tab3Texts.clearCache)
    }

    companion object {
        fun newInstance() = Tab3Fragment()
    }
}
