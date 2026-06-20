// 中文: DataStore 實例 + PreferenceKeys 集中定義 — DataStore 為延伸屬性 singleton(taigi_keyboard_prefs)。
// 中文: 所有 typed key 都集中於此,避免字串散落各檔。

package com.siansiansu.taigikeyboard.ime.core

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.booleanPreferencesKey
import androidx.datastore.preferences.core.floatPreferencesKey
import androidx.datastore.preferences.core.intPreferencesKey
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore

/**
 * DataStore instance for TaigiKeyboard preferences.
 * Singleton pattern using extension property.
 */
val Context.preferencesDataStore: DataStore<Preferences> by preferencesDataStore(
    name = "taigi_keyboard_prefs",
)

/**
 * Preference keys for DataStore.
 * Centralized key definitions to avoid string duplication.
 */
object PreferenceKeys {
    // Advanced settings
    val SETTINGS_THEME = stringPreferencesKey("advanced__settings_theme")
    val SHOW_APP_ICON = booleanPreferencesKey("advanced__show_app_icon")

    // App UI display language (i18n). Tag of DisplayLanguage; default "hanji" = current single language.
    val DISPLAY_LANGUAGE = stringPreferencesKey("app__display_language")

    // Correction settings
    val DOUBLE_SPACE_PERIOD = booleanPreferencesKey("correction__double_space_period")

    // Internal settings
    val VERSION_ON_INSTALL = stringPreferencesKey("internal__version_on_install")
    val VERSION_LAST_USE = stringPreferencesKey("internal__version_last_use")

    // Keyboard settings
    val ACTIVE_SUBTYPE_ID = intPreferencesKey("keyboard__active_subtype_id")
    val SUBTYPES = stringPreferencesKey("keyboard__subtypes")
    val INPUT_MODE = stringPreferencesKey("keyboard__input_mode")
    val IS_TRANSLATE_SWAPPED = booleanPreferencesKey("keyboard__is_translate_swapped")
    val OUTPUT_BOTH_SCRIPTS = booleanPreferencesKey("keyboard__output_both_scripts")
    val LITERAL_ROMAN_CANDIDATE = booleanPreferencesKey("keyboard__literal_roman_candidate")
    val PHAH_TAIGI_LAYOUT_ENABLED = booleanPreferencesKey("keyboard__phah_taigi_layout_enabled")
    val KEYBOARD_LAYOUT_TYPE = stringPreferencesKey("keyboard__layout_type")
    val INPUT_MODE_BEFORE_TPS = stringPreferencesKey("keyboard__input_mode_before_tps")
    val LAYOUT_BEFORE_TPS = stringPreferencesKey("keyboard__layout_before_tps")
    val TOOLBAR_AUTO_COLLAPSE = booleanPreferencesKey("keyboard__toolbar_auto_collapse")
    val GLOBE_KEY_ENABLED = booleanPreferencesKey("keyboard__globe_key_enabled")
    val SOUND_FEEDBACK_ENABLED = booleanPreferencesKey("keyboard__sound_feedback_enabled")
    val VIBRATION_FEEDBACK_ENABLED = booleanPreferencesKey("keyboard__vibration_feedback_enabled")

    // Taigi-specific keys
    val ENABLE_DOUBLE_TAP_OO = booleanPreferencesKey("taigi__enable_double_tap_oo")
    val ENABLE_DOUBLE_TAP_NN = booleanPreferencesKey("taigi__enable_double_tap_nn")
    val AUTO_CAPITALIZATION_ENABLED = booleanPreferencesKey("taigi__auto_capitalization_enabled")
    val AUTO_SPACE_ENABLED = booleanPreferencesKey("taigi__auto_space_enabled")
    val FONT_TYPE = stringPreferencesKey("taigi__font_type")

    // Looknfeel settings
    val HEIGHT_FACTOR = stringPreferencesKey("looknfeel__height_factor")
    val LONG_PRESS_DELAY = intPreferencesKey("looknfeel__long_press_delay")

    // 詞頻紀錄開關
    val FREQUENCY_RECORDING_ENABLED = booleanPreferencesKey("dictionary__frequency_recording_enabled")

    // 詞關聯紀錄開關
    val ASSOCIATION_RECORDING_ENABLED = booleanPreferencesKey("dictionary__association_recording_enabled")

    // 自訂詞庫開關
    val CUSTOM_DICT_ENABLED = booleanPreferencesKey("dictionary__custom_dict_enabled")

    // 詞庫開關設定
    val MOE_DICT_ENABLED = booleanPreferencesKey("dictionary__moe_dict_enabled")
    val NEWWORD_DICT_ENABLED = booleanPreferencesKey("dictionary__newword_dict_enabled")
    val ITAIGI_DICT_ENABLED = booleanPreferencesKey("dictionary__itaigi_dict_enabled")
    val SITBUT_DICT_ENABLED = booleanPreferencesKey("dictionary__sitbut_dict_enabled")
    val TAIHOA_DICT_ENABLED = booleanPreferencesKey("dictionary__taihoa_dict_enabled")
    val TAIJIT_DICT_ENABLED = booleanPreferencesKey("dictionary__taijit_dict_enabled")
    val KUNGGE_DICT_ENABLED = booleanPreferencesKey("dictionary__kungge_dict_enabled")
    val STTI_DICT_ENABLED = booleanPreferencesKey("dictionary__stti_dict_enabled")
    val KHPOO_DICT_ENABLED = booleanPreferencesKey("dictionary__khpoo_dict_enabled")

    // 異用字開關
    val VARIANT_DICT_ENABLED = booleanPreferencesKey("dictionary__variant_enabled")

    // 在來字開關
    val KHIIN_ENABLED = booleanPreferencesKey("dictionary__khiin_enabled")

    // LKK漢羅合用建議用字
    val LKK_DICT_ENABLED = booleanPreferencesKey("dictionary__lkk_dict_enabled")

    // 開發者補充辭典 (詞庫增補檔案)
    val DEV_DICT_ENABLED = booleanPreferencesKey("dictionary__dev_dict_enabled")

    // 教育部辭典子集 (腔調 + 姓名附錄) — 巢狀於 MOE master 下;腔調預設開 (DD5 opt-out),姓名附錄預設開 (opt-out)。
    // 腔調順序對齊 config.yaml dialect_columns (= subtag bit - 1)。bit 佈局由 Rust compute_filters 持有。
    val KAUTIAN_ACCENT_LUKANG_ENABLED = booleanPreferencesKey("dictionary__kautian_accent_lukang_enabled")
    val KAUTIAN_ACCENT_SANSIA_ENABLED = booleanPreferencesKey("dictionary__kautian_accent_sansia_enabled")
    val KAUTIAN_ACCENT_TAIPAK_ENABLED = booleanPreferencesKey("dictionary__kautian_accent_taipak_enabled")
    val KAUTIAN_ACCENT_GILAN_ENABLED = booleanPreferencesKey("dictionary__kautian_accent_gilan_enabled")
    val KAUTIAN_ACCENT_TAINAN_ENABLED = booleanPreferencesKey("dictionary__kautian_accent_tainan_enabled")
    val KAUTIAN_ACCENT_KAOHSIUNG_ENABLED = booleanPreferencesKey("dictionary__kautian_accent_kaohsiung_enabled")
    val KAUTIAN_ACCENT_KINMEN_ENABLED = booleanPreferencesKey("dictionary__kautian_accent_kinmen_enabled")
    val KAUTIAN_ACCENT_MAKUNG_ENABLED = booleanPreferencesKey("dictionary__kautian_accent_makung_enabled")
    val KAUTIAN_ACCENT_SINTIK_ENABLED = booleanPreferencesKey("dictionary__kautian_accent_sintik_enabled")
    val KAUTIAN_ACCENT_TAICHUNG_ENABLED = booleanPreferencesKey("dictionary__kautian_accent_taichung_enabled")
    val KAUTIAN_NAME_APPENDIX_ENABLED = booleanPreferencesKey("dictionary__kautian_name_appendix_enabled")

    // TPS settings
    val TPS_OR_MAPS_TO_ER = booleanPreferencesKey("tps__or_maps_to_er")

    // Appearance settings
    val KEY_HEIGHT_SCALE = floatPreferencesKey("appearance__key_height_scale")
    val KEY_FONT_SIZE_SCALE = floatPreferencesKey("appearance__key_font_size_scale")
    val CANDIDATE_TEXT_SIZE_SCALE = floatPreferencesKey("appearance__candidate_text_size_scale")
    val KEY_CORNER_RADIUS = floatPreferencesKey("appearance__key_corner_radius")
    val KEY_BORDER_WIDTH = floatPreferencesKey("appearance__key_border_width")
    val COLOR_SETTINGS = stringPreferencesKey("appearance__color_settings")

    // Theme settings (v3.6.2)
    val SELECTED_THEME_ID = stringPreferencesKey("appearance__selected_theme_id")
    val USER_THEMES = stringPreferencesKey("appearance__user_themes")
}
