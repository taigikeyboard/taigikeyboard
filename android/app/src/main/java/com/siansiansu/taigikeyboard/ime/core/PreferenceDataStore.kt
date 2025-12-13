package com.siansiansu.taigikeyboard.ime.core

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.booleanPreferencesKey
import androidx.datastore.preferences.core.intPreferencesKey
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore

/**
 * DataStore instance for TaigiKeyboard preferences.
 * Singleton pattern using extension property.
 */
val Context.preferencesDataStore: DataStore<Preferences> by preferencesDataStore(
    name = "taigi_keyboard_prefs"
)

/**
 * Preference keys for DataStore.
 * Centralized key definitions to avoid string duplication.
 */
object PreferenceKeys {
    // Advanced settings
    val SETTINGS_THEME = stringPreferencesKey("advanced__settings_theme")
    val SHOW_APP_ICON = booleanPreferencesKey("advanced__show_app_icon")

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
    val PHAH_TAIGI_LAYOUT_ENABLED = booleanPreferencesKey("keyboard__phah_taigi_layout_enabled")

    // Taigi-specific keys
    val ENABLE_DOUBLE_TAP_OO = booleanPreferencesKey("taigi__enable_double_tap_oo")
    val ENABLE_DOUBLE_TAP_NN = booleanPreferencesKey("taigi__enable_double_tap_nn")
    val AUTO_CAPITALIZATION_ENABLED = booleanPreferencesKey("taigi__auto_capitalization_enabled")
    val AUTO_SPACE_ENABLED = booleanPreferencesKey("taigi__auto_space_enabled")
    val FONT_TYPE = stringPreferencesKey("taigi__font_type")

    // Looknfeel settings
    val HEIGHT_FACTOR = stringPreferencesKey("looknfeel__height_factor")
    val LONG_PRESS_DELAY = intPreferencesKey("looknfeel__long_press_delay")

    // Suggestion settings
    val SUGGESTION_ENABLED = booleanPreferencesKey("suggestion__enabled")

    // Onboarding settings
    val HAS_SEEN_ONBOARDING = booleanPreferencesKey("onboarding__has_seen")

    // 詞庫開關設定
    val MOE_DICT_ENABLED = booleanPreferencesKey("dictionary__moe_dict_enabled")
    val NEWWORD_DICT_ENABLED = booleanPreferencesKey("dictionary__newword_dict_enabled")
    val ITAIGI_DICT_ENABLED = booleanPreferencesKey("dictionary__itaigi_dict_enabled")
    val SITBUT_DICT_ENABLED = booleanPreferencesKey("dictionary__sitbut_dict_enabled")
    val TAIHOA_DICT_ENABLED = booleanPreferencesKey("dictionary__taihoa_dict_enabled")
    val TAIJIT_DICT_ENABLED = booleanPreferencesKey("dictionary__taijit_dict_enabled")
    val KUNGGE_DICT_ENABLED = booleanPreferencesKey("dictionary__kungge_dict_enabled")
}
